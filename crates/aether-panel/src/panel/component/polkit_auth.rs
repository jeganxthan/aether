use std::sync::{Mutex, OnceLock};

use aether::{
    Align, Canvas, Color, Column, Full, Horizontal, Keyboard, Layer, LayerWindow, Padding, Parent,
    Path, Rectangle, Shape, Stack, Text, TextInput, Vertical, Weight, Zone, children,
};
use glib::{object::Cast, subclass::prelude::*};
use polkit_agent_rs::{
    Listener, RegisterFlags, gio, gio::prelude::CancellableExt, gio::prelude::CancellableExtManual,
    polkit, polkit::UnixUser, subclass::ListenerImpl, traits::ListenerExt,
};
use tokio::sync::oneshot;
use zeroize::Zeroize;

const AUTH_FIELD: &str = "aether-polkit-password";
const AGENT_PATH: &str = "/org/aether_shell/PolicyKit/AuthenticationAgent";
const AUTH_ISLAND_WIDTH: f32 = 466.0;
const AUTH_ISLAND_HEIGHT: f32 = 112.0;
const EAR_RADIUS: f32 = 13.0;
const CORNER_RADIUS: f32 = 14.0;

struct Prompt {
    cookie: String,
    action_id: String,
    message: String,
    username: String,
    request: String,
    notice: String,
    echo_on: bool,
    cancelled: bool,
    cancel_sender: Option<oneshot::Sender<()>>,
    response: Option<oneshot::Sender<Option<String>>>,
}

#[derive(Clone)]
struct PromptView {
    action_id: String,
    message: String,
    username: String,
    request: String,
    notice: String,
    echo_on: bool,
}

static PROMPT: OnceLock<Mutex<Option<Prompt>>> = OnceLock::new();

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct AuthListener;

    #[glib::object_subclass]
    impl ObjectSubclass for AuthListener {
        const NAME: &'static str = "AetherPolkitAuthListener";
        type Type = super::AuthListener;
        type ParentType = Listener;
    }

    impl ObjectImpl for AuthListener {}

    impl ListenerImpl for AuthListener {
        type Message = String;

        fn initiate_authentication(
            &self,
            action_id: &str,
            message: &str,
            _icon_name: &str,
            _details: &polkit::Details,
            cookie: &str,
            identities: Vec<polkit::Identity>,
            cancellable: gio::Cancellable,
            task: gio::Task<Self::Message>,
        ) {
            let user = identities
                .into_iter()
                .find_map(|identity| identity.dynamic_cast::<UnixUser>().ok());
            let Some(user) = user else {
                return_task_error(task, "Polkit did not provide an eligible user");
                return;
            };

            let username = user
                .name()
                .map(|name| name.to_string())
                .unwrap_or_else(|| "User".into());

            {
                let Ok(mut state) = prompt_state().lock() else {
                    return_task_error(task, "authentication prompt state is unavailable");
                    return;
                };
                if state.is_some() {
                    return_task_error(task, "another authentication request is already active");
                    return;
                }
                *state = Some(Prompt {
                    cookie: cookie.to_owned(),
                    action_id: action_id.to_owned(),
                    message: message.to_owned(),
                    username: username.clone(),
                    request: "Waiting for authentication prompt…".into(),
                    notice: String::new(),
                    echo_on: false,
                    cancelled: false,
                    cancel_sender: None,
                    response: None,
                });
            }
            aether::refresh();

            begin_session(
                user,
                cookie.to_owned(),
                action_id.to_owned(),
                message.to_owned(),
                username,
                cancellable,
                task,
                0,
            );
        }

        fn initiate_authentication_finish(
            &self,
            result: Result<gio::Task<Self::Message>, glib::Error>,
        ) -> bool {
            result.and_then(|task| unsafe { task.propagate() }).is_ok()
        }
    }
}

glib::wrapper! {
    pub struct AuthListener(ObjectSubclass<imp::AuthListener>)
        @extends Listener;
}

impl Default for AuthListener {
    fn default() -> Self {
        glib::Object::new()
    }
}

fn return_task_error(task: gio::Task<String>, message: &str) {
    unsafe {
        task.return_result(Err(glib::Error::new(gio::IOErrorEnum::Failed, message)));
    }
}

fn begin_session(
    user: UnixUser,
    cookie: String,
    action_id: String,
    message: String,
    username: String,
    cancellable: gio::Cancellable,
    task: gio::Task<String>,
    failed_attempts: u8,
) {
    let session = polkit_agent_rs::Session::new(&user, &cookie);
    let (cancel_sender, cancel_receiver) = oneshot::channel();
    let Ok(mut state) = prompt_state().lock() else {
        return_task_error(task, "authentication prompt state is unavailable");
        return;
    };
    let Some(prompt) = state.as_mut().filter(|prompt| prompt.cookie == cookie) else {
        return_task_error(task, "authentication request is no longer active");
        return;
    };
    prompt.cancel_sender = Some(cancel_sender);
    drop(state);

    let session_for_cancel = session.clone();
    glib::MainContext::default().spawn_local(async move {
        if cancel_receiver.await.is_ok() {
            session_for_cancel.cancel();
        }
    });
    let cookie_for_request = cookie.clone();
    let action_for_request = action_id.clone();
    let message_for_request = message.clone();
    let username_for_request = username.clone();

    session.connect_request(move |session, request, echo_on| {
        let session = session.clone();
        let cookie = cookie_for_request.clone();
        let action_id = action_for_request.clone();
        let message = message_for_request.clone();
        let username = username_for_request.clone();
        let request = request.to_string();
        glib::MainContext::default().spawn_local(async move {
            match request_secret(&cookie, &action_id, &message, &username, &request, echo_on).await
            {
                Some(mut secret) => {
                    session.response(&secret);
                    secret.zeroize();
                }
                None => session.cancel(),
            }
        });
    });

    let user_for_retry = user.clone();
    let cookie_for_retry = cookie.clone();
    let action_for_retry = action_id.clone();
    let message_for_retry = message.clone();
    let username_for_retry = username.clone();
    let cancellable_for_retry = cancellable.clone();
    let task_for_retry = task.clone();
    let task_for_completed = task.clone();
    let cookie_for_completed = cookie.clone();
    let cancellable_for_completed = cancellable.clone();
    session.connect_completed(move |session, success| {
        if success {
            complete_prompt(&cookie_for_completed);
            unsafe {
                task_for_completed
                    .clone()
                    .return_result(Ok("success".into()));
            }
        } else if cancellable_for_completed.is_cancelled()
            || prompt_was_cancelled(&cookie_for_completed)
            || failed_attempts >= 2
        {
            complete_prompt(&cookie_for_completed);
            return_task_error(
                task_for_completed.clone(),
                "Polkit authentication was cancelled or failed",
            );
        } else {
            update_notice(&cookie_for_completed, "Authentication failed. Try again.");
            begin_session(
                user_for_retry.clone(),
                cookie_for_retry.clone(),
                action_for_retry.clone(),
                message_for_retry.clone(),
                username_for_retry.clone(),
                cancellable_for_retry.clone(),
                task_for_retry.clone(),
                failed_attempts + 1,
            );
        }
        session.cancel();
    });
    session.connect_show_info({
        let cookie = cookie.clone();
        move |_session, info| update_notice(&cookie, &info)
    });
    session.connect_show_error({
        let cookie = cookie.clone();
        move |_session, error| update_notice(&cookie, &error)
    });
    cancellable.connect_cancelled(move |_| {
        cancel_prompt_for_cookie(&cookie);
    });
    session.initiate();
}

fn prompt_state() -> &'static Mutex<Option<Prompt>> {
    PROMPT.get_or_init(|| Mutex::new(None))
}

fn view_state() -> Option<PromptView> {
    prompt_state().lock().ok().and_then(|prompt| {
        prompt.as_ref().map(|prompt| PromptView {
            action_id: prompt.action_id.clone(),
            message: prompt.message.clone(),
            username: prompt.username.clone(),
            request: prompt.request.clone(),
            notice: prompt.notice.clone(),
            echo_on: prompt.echo_on,
        })
    })
}

fn update_notice(cookie: &str, notice: &str) {
    if let Ok(mut state) = prompt_state().lock() {
        if let Some(prompt) = state.as_mut().filter(|prompt| prompt.cookie == cookie) {
            prompt.notice = notice.to_owned();
        }
    }
    aether::refresh();
}

async fn request_secret(
    cookie: &str,
    action_id: &str,
    message: &str,
    username: &str,
    request: &str,
    echo_on: bool,
) -> Option<String> {
    let (sender, receiver) = oneshot::channel();
    {
        let mut state = prompt_state().lock().ok()?;
        let prompt = state.as_mut().filter(|prompt| prompt.cookie == cookie)?;
        prompt.action_id = action_id.to_owned();
        prompt.message = message.to_owned();
        prompt.username = username.to_owned();
        prompt.request = request.to_owned();
        prompt.echo_on = echo_on;
        prompt.notice.clear();
        prompt.response = Some(sender);
    }
    TextInput::set_text(AUTH_FIELD, "");
    aether::refresh();

    receiver.await.ok().flatten()
}

fn complete_prompt(cookie: &str) {
    if let Ok(mut state) = prompt_state().lock() {
        if state.as_ref().is_some_and(|prompt| prompt.cookie == cookie) {
            *state = None;
        }
    }
    TextInput::set_text(AUTH_FIELD, "");
    aether::refresh();
}

fn prompt_was_cancelled(cookie: &str) -> bool {
    prompt_state()
        .lock()
        .ok()
        .and_then(|state| {
            state
                .as_ref()
                .filter(|prompt| prompt.cookie == cookie)
                .map(|prompt| prompt.cancelled)
        })
        .unwrap_or(true)
}

pub fn submit_secret(mut secret: String) {
    TextInput::set_text(AUTH_FIELD, "");
    let response = prompt_state()
        .lock()
        .ok()
        .and_then(|mut state| state.as_mut().and_then(|prompt| prompt.response.take()));

    if let Some(response) = response {
        if let Err(mut error) = response.send(Some(secret)) {
            if let Some(secret) = error.as_mut() {
                secret.zeroize();
            }
        }
    } else {
        secret.zeroize();
    }
    aether::refresh();
}

pub fn cancel_prompt() {
    let cookie = prompt_state()
        .lock()
        .ok()
        .and_then(|state| state.as_ref().map(|prompt| prompt.cookie.clone()));
    if let Some(cookie) = cookie {
        cancel_prompt_for_cookie(&cookie);
    }
    TextInput::set_text(AUTH_FIELD, "");
    aether::refresh();
}

fn cancel_prompt_for_cookie(cookie: &str) {
    let (response, cancel_sender) = prompt_state()
        .lock()
        .ok()
        .and_then(|mut state| {
            let prompt = state.as_mut().filter(|prompt| prompt.cookie == cookie)?;
            prompt.cancelled = true;
            Some((prompt.response.take(), prompt.cancel_sender.take()))
        })
        .unwrap_or((None, None));
    if let Some(response) = response {
        let _ = response.send(None);
    }
    if let Some(cancel_sender) = cancel_sender {
        let _ = cancel_sender.send(());
    }
}

pub fn start() {
    if let Err(error) = std::thread::Builder::new()
        .name("aether-polkit-agent".into())
        .spawn(|| {
            let main_loop = glib::MainLoop::new(None, false);
            let listener = AuthListener::default();
            let subject = match polkit::UnixSession::new_for_process_sync(
                std::process::id() as i32,
                gio::Cancellable::NONE,
            ) {
                Ok(subject) => subject,
                Err(error) => {
                    eprintln!("Could not identify Aether's Polkit session: {error}");
                    return;
                }
            };
            match listener.register(
                RegisterFlags::NONE,
                &subject,
                AGENT_PATH,
                gio::Cancellable::NONE,
            ) {
                Ok(_registration) => main_loop.run(),
                Err(error) => {
                    eprintln!("Could not register Aether Polkit authentication agent: {error}");
                }
            }
        })
    {
        eprintln!("Could not start Aether Polkit authentication agent: {error}");
    }
}

pub fn auth_prompt() -> LayerWindow {
    let Some(prompt) = view_state() else {
        return LayerWindow::new()
            .width(Full)
            .height(Full)
            .layer(Layer::Overlay)
            .space(Zone::Ignore)
            .keyboard(Keyboard::None)
            .namespace("aether-polkit-auth")
            .click_through()
            .child(blank());
    };

    let window = LayerWindow::new()
        .width(Full)
        .height(Full)
        .anchor_vertical(Vertical::Top)
        .anchor_horizontal(Horizontal::Left)
        .layer(Layer::Overlay)
        .space(Zone::Ignore)
        .keyboard(Keyboard::Exclusive)
        .namespace("aether-polkit-auth")
        .on_key(|key| {
            if key == aether::Key::Escape {
                cancel_prompt();
            }
        });

    let theme = crate::theme::theme();
    let mut input = TextInput::new(AUTH_FIELD)
        .placeholder(&prompt.request)
        .focused()
        .size(13.0)
        .color(theme.text)
        .on_submit(submit_secret);
    if !prompt.echo_on {
        input = input.password();
    }
    let field = Rectangle::new()
        .width(Parent)
        .height(38.0)
        .fill(Color::rgba(26, 26, 26, 255))
        .border(1.0, theme.chip_border)
        .radius(9.0)
        .padding(Padding {
            top: 0.0,
            right: 12.0,
            bottom: 0.0,
            left: 12.0,
        })
        .align_child(Align::Center, Align::Center)
        .child(input);

    let island_fill =
        Box::new(auth_island_path(AUTH_ISLAND_WIDTH, AUTH_ISLAND_HEIGHT).fill(theme.bar))
            as Box<dyn Shape>;
    let island_border = Box::new(
        auth_island_path(AUTH_ISLAND_WIDTH, AUTH_ISLAND_HEIGHT).stroke(1.0, theme.surface_border),
    ) as Box<dyn Shape>;
    let island_canvas = Canvas::new()
        .width(AUTH_ISLAND_WIDTH)
        .height(AUTH_ISLAND_HEIGHT)
        .shapes(vec![island_fill, island_border]);
    let content = Rectangle::new()
        .width(AUTH_ISLAND_WIDTH)
        .height(AUTH_ISLAND_HEIGHT)
        .fill(Color::TRANSPARENT)
        .clip()
        .padding(Padding {
            top: 9.0,
            right: EAR_RADIUS + 12.0,
            bottom: 9.0,
            left: EAR_RADIUS + 12.0,
        })
        .align_child(Align::Start, Align::Start)
        .child(
            Column::new(children![
                field,
                Rectangle::new()
                    .width(Parent)
                    .height(1.0)
                    .fill(theme.chip_border),
                Text::new(format!("{} · {}", prompt.message, prompt.username))
                    .font("Maple Mono")
                    .size(10.5)
                    .weight(Weight::SemiBold)
                    .color(theme.text)
                    .elide(),
                Text::new(prompt.action_id)
                    .font("Maple Mono")
                    .size(9.0)
                    .color(theme.text_faint)
                    .elide(),
                Text::new(prompt.notice)
                    .font("Maple Mono")
                    .size(9.0)
                    .color(theme.text_muted),
            ])
            .gap(5.0)
            .align(Align::Start),
        );

    let island = Stack::new(children![
        Rectangle::new()
            .width(AUTH_ISLAND_WIDTH)
            .height(AUTH_ISLAND_HEIGHT)
            .fill(Color::TRANSPARENT)
            .padding(Padding {
                top: 0.0,
                right: EAR_RADIUS,
                bottom: 0.0,
                left: EAR_RADIUS,
            })
            .align_child(Align::Center, Align::Start)
            .child(
                Rectangle::new()
                    .width(AUTH_ISLAND_WIDTH - EAR_RADIUS * 2.0)
                    .height(AUTH_ISLAND_HEIGHT)
                    .radius_bottom_left(CORNER_RADIUS)
                    .radius_bottom_right(CORNER_RADIUS)
                    .fill(Color::TRANSPARENT)
                    .shadow(super::island_shadow(0.35)),
            ),
        island_canvas,
        content,
    ]);

    window.child(
        Rectangle::new()
            .width(Parent)
            .height(Parent)
            .fill(Color::TRANSPARENT)
            .padding(Padding {
                top: crate::panel::top::PANEL_HEIGHT,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            })
            .align_child(Align::Center, Align::Start)
            .child(island),
    )
}

fn auth_island_path(width: f32, height: f32) -> Path {
    let ear_curve = 0.552_284_75 * EAR_RADIUS;
    let corner_curve = 0.552_284_75 * CORNER_RADIUS;

    Path::new()
        .move_to(0.0, 0.0)
        .cubic_to(
            ear_curve,
            0.0,
            EAR_RADIUS,
            EAR_RADIUS - ear_curve,
            EAR_RADIUS,
            EAR_RADIUS,
        )
        .line_to(EAR_RADIUS, height - CORNER_RADIUS)
        .cubic_to(
            EAR_RADIUS,
            height - CORNER_RADIUS + corner_curve,
            EAR_RADIUS + CORNER_RADIUS - corner_curve,
            height,
            EAR_RADIUS + CORNER_RADIUS,
            height,
        )
        .line_to(width - EAR_RADIUS - CORNER_RADIUS, height)
        .cubic_to(
            width - EAR_RADIUS - CORNER_RADIUS + corner_curve,
            height,
            width - EAR_RADIUS,
            height - CORNER_RADIUS + corner_curve,
            width - EAR_RADIUS,
            height - CORNER_RADIUS,
        )
        .line_to(width - EAR_RADIUS, EAR_RADIUS)
        .cubic_to(
            width - EAR_RADIUS,
            EAR_RADIUS - ear_curve,
            width - ear_curve,
            0.0,
            width,
            0.0,
        )
        .line_to(0.0, 0.0)
        .close()
}

fn blank() -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(Parent)
        .fill(Color::TRANSPARENT)
}
