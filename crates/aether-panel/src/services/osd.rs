use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use aether::refresh as wake;
use aether_services::osd::{AudioService, AudioState, BrightnessService, BrightnessState};

use super::runtime::runtime;

// how often the volume and brightness are read to notice a change
const POLL: Duration = Duration::from_millis(250);

// how long the display stays after the last change before it slides away
pub const SHOW_TIME: Duration = Duration::from_millis(1500);

// how long the bar takes to slide in or out
pub const SLIDE_TIME: Duration = Duration::from_millis(200);

// which value the display is showing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Volume,
    Brightness,
}

// one reading on screen: what it is, how much, and when it hides
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shown {
    pub kind: Kind,

    // 0 to 100
    pub percent: u8,

    // volume only, and true while the sink is muted
    pub muted: bool,

    // 0 while shown, one bar width off the bottom once gone
    pub offset: f32,

    // true once it is on its way out
    pub leaving: bool,
}

struct State {
    // the last values read, so a change is what opens the display
    audio: Option<AudioState>,
    brightness: Option<BrightnessState>,

    // what is on screen now, if anything
    shown: Option<Shown>,

    // the moment the display starts to go away
    hide_at: Option<Instant>,
}

static STATE: OnceLock<Mutex<State>> = OnceLock::new();

// starts the watcher exactly once
static STARTED: OnceLock<()> = OnceLock::new();

fn state() -> &'static Mutex<State> {
    STATE.get_or_init(|| {
        Mutex::new(State {
            audio: None,
            brightness: None,
            shown: None,
            hide_at: None,
        })
    })
}

// what the view should draw, or none while the display rests
pub fn shown() -> Option<Shown> {
    start();

    state()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .shown
}

// sets how far the display has slid, driven by the watcher
fn set_offset(offset: f32) -> bool {
    let mut state = state().lock().unwrap_or_else(PoisonError::into_inner);

    let Some(shown) = state.shown.as_mut() else {
        return false;
    };

    if shown.offset == offset {
        return false;
    }

    shown.offset = offset;

    true
}

fn start() {
    STARTED.get_or_init(|| {
        // one task reads the values, one slides the bar, so a slow read
        // never stutters the motion
        runtime().spawn(watch());
        runtime().spawn(animate());
    });
}

/*
 * Reads volume and brightness on a short timer, since the shell has no way to
 * be told when a keybind changes them. A reading that differs from the last
 * opens the display on that value, and the display hides itself once the last
 * change is old enough.
 */
async fn watch() {
    let mut ticker = tokio::time::interval(POLL);

    loop {
        ticker.tick().await;

        let audio = AudioService::current().ok();
        let brightness = BrightnessService::current().ok();

        let changed = update(audio, brightness);

        if changed {
            wake();
        }
    }
}

/*
 * Folds a fresh pair of readings in, and says whether the display changed.
 *
 * Each value primes on its own: the very first reading of a kind only fills the
 * cache, so a value the shell merely read (at startup, or the first time a
 * service answers after a failed read) never pops the display. Only a reading
 * that differs from a value already known counts as a change.
 */
fn update(audio: Option<AudioState>, brightness: Option<BrightnessState>) -> bool {
    let mut state = state().lock().unwrap_or_else(PoisonError::into_inner);

    let mut changed = false;

    if let Some(audio) = audio {
        // none until this kind has answered once, which is what primes it
        let known = state.audio;

        state.audio = Some(audio);

        if known.is_some_and(|known| known != audio) {
            show(&mut state, Kind::Volume, audio.volume_percent, audio.muted);

            changed = true;
        }
    }

    if let Some(brightness) = brightness {
        // the device name can change with the value, but only the level matters
        let known = state.brightness.as_ref().map(|known| known.percent);

        let level = brightness.percent;

        state.brightness = Some(brightness);

        if known.is_some_and(|known| known != level) {
            show(&mut state, Kind::Brightness, level, false);

            changed = true;
        }
    }

    changed
}

/*
 * Opens the display on a value, or refreshes the one already up.
 *
 * A change that arrives while the display is already rising or risen only swaps
 * the reading and pushes the hide back out, so holding a key to step the volume
 * up or down keeps one steady bar instead of dropping and popping it on every
 * step. Only a change while it is hidden (or already on its way out) starts a
 * fresh rise from the bottom.
 */
fn show(state: &mut State, kind: Kind, percent: u8, muted: bool) {
    // keep the height it already has, unless it is hidden or on its way out
    let offset = match state.shown {
        Some(shown) if !shown.leaving => shown.offset,
        _ => 1.0,
    };

    state.shown = Some(Shown {
        kind,
        percent,
        muted,
        offset,
        leaving: false,
    });

    state.hide_at = Some(Instant::now() + SHOW_TIME);
}

/*
 * Moves the bar in and out on its own timer, so the view only reads the
 * offset and opacity. It is its own task so the slide stays smooth even while
 * the watcher is blocked reading sysfs or wpctl.
 */
async fn animate() {
    // a steady step, so a slide takes about SLIDE_TIME either way
    let step = 16.0 / SLIDE_TIME.as_millis() as f32;

    loop {
        tokio::time::sleep(Duration::from_millis(16)).await;

        let mut changed = false;

        {
            let mut state = state().lock().unwrap_or_else(PoisonError::into_inner);

            // the moment the display has sat long enough to start leaving
            let hide_now = state
                .shown
                .as_ref()
                .is_some_and(|shown| !shown.leaving)
                && state.hide_at.is_some_and(|at| Instant::now() >= at);

            if hide_now && let Some(shown) = state.shown.as_mut() {
                shown.leaving = true;

                changed = true;
            }
        }

        let Some(shown) = state().lock().unwrap_or_else(PoisonError::into_inner).shown else {
            continue;
        };

        let target = if shown.leaving { 1.0 } else { 0.0 };

        if shown.offset != target {
            let next = if shown.leaving {
                f32::min(shown.offset + step, 1.0)
            } else {
                f32::max(shown.offset - step, 0.0)
            };

            if set_offset(next) {
                changed = true;
            }
        }

        // gone off the bottom and out of the way
        if shown.leaving && shown.offset >= 1.0 {
            let mut state = state().lock().unwrap_or_else(PoisonError::into_inner);

            state.shown = None;
            state.hide_at = None;

            changed = true;
        }

        if changed {
            wake();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn audio(percent: u8, muted: bool) -> AudioState {
        AudioState {
            volume_percent: percent,
            muted,
        }
    }

    fn brightness(percent: u8) -> BrightnessState {
        BrightnessState {
            percent,
            device: "test".into(),
        }
    }

    // the tests share the one global state, so they run one after another
    fn reset() {
        let mut state = state().lock().unwrap_or_else(PoisonError::into_inner);

        *state = State {
            audio: None,
            brightness: None,
            shown: None,
            hide_at: None,
        };
    }

    #[test]
    fn the_first_reading_only_primes_and_shows_nothing() {
        reset();

        // at startup the shell reads a value it did not change, so no display
        assert!(!update(Some(audio(40, false)), Some(brightness(70))));

        assert!(state().lock().unwrap().shown.is_none());
    }

    #[test]
    fn a_kind_that_answers_late_still_only_primes() {
        reset();

        // brightness answers first and primes; audio fails its first read
        assert!(!update(None, Some(brightness(70))));
        assert!(state().lock().unwrap().shown.is_none());

        // the first audio read succeeds later: it is a value merely read, not changed
        assert!(!update(Some(audio(40, false)), Some(brightness(70))));
        assert!(state().lock().unwrap().shown.is_none());

        // only the next, different reading opens the display
        assert!(update(Some(audio(55, false)), Some(brightness(70))));
    }

    #[test]
    fn a_new_volume_opens_the_display_on_volume() {
        reset();

        // prime both, then change volume
        update(Some(audio(40, false)), Some(brightness(70)));
        assert!(update(Some(audio(55, false)), Some(brightness(70))));

        let shown = state().lock().unwrap().shown.expect("the display opens");

        assert_eq!(shown.kind, Kind::Volume);
        assert_eq!(shown.percent, 55);
        assert!(!shown.muted);
    }

    #[test]
    fn stepping_the_volume_keeps_one_steady_bar() {
        reset();

        update(Some(audio(40, false)), Some(brightness(70)));
        update(Some(audio(55, false)), Some(brightness(70)));

        // let the first change rise all the way into place
        state().lock().unwrap().shown.as_mut().unwrap().offset = 0.0;

        // each further step only swaps the reading, it never drops back to the bottom
        for level in [60, 65, 70] {
            assert!(update(Some(audio(level, false)), Some(brightness(70))));

            let shown = state().lock().unwrap().shown.unwrap();

            assert_eq!(shown.offset, 0.0, "the bar sank on step to {level}");
            assert_eq!(shown.percent, level);
            assert!(!shown.leaving);
        }
    }

    #[test]
    fn a_change_while_leaving_rises_again() {
        reset();

        update(Some(audio(40, false)), Some(brightness(70)));
        update(Some(audio(55, false)), Some(brightness(70)));

        // the bar is on its way out and halfway down the screen
        {
            let mut state = state().lock().unwrap();
            let shown = state.shown.as_mut().unwrap();

            shown.leaving = true;
            shown.offset = 0.5;
        }

        // a fresh change pulls it back in from the bottom rather than resuming mid drop
        assert!(update(Some(audio(60, false)), Some(brightness(70))));

        let shown = state().lock().unwrap().shown.unwrap();

        assert_eq!(shown.offset, 1.0);
        assert!(!shown.leaving);
    }

    #[test]
    fn a_new_brightness_opens_the_display_on_brightness() {
        reset();

        update(Some(audio(40, false)), Some(brightness(70)));
        assert!(update(Some(audio(40, false)), Some(brightness(80))));

        let shown = state().lock().unwrap().shown.expect("the display opens");

        assert_eq!(shown.kind, Kind::Brightness);
        assert_eq!(shown.percent, 80);
    }

    #[test]
    fn mute_is_carried_onto_the_display() {
        reset();

        update(Some(audio(30, false)), None);

        assert!(update(Some(audio(30, true)), None));

        let shown = state().lock().unwrap().shown.expect("the display opens");

        assert!(shown.muted);
    }

    #[test]
    fn an_unchanged_reading_leaves_the_display_alone() {
        reset();

        update(Some(audio(40, false)), Some(brightness(70)));

        assert!(!update(Some(audio(40, false)), Some(brightness(70))));
    }

    #[test]
    fn the_newest_change_takes_over_the_display() {
        reset();

        update(Some(audio(40, false)), Some(brightness(50)));

        // volume changes, then brightness: the newest one is what shows
        update(Some(audio(60, false)), Some(brightness(50)));
        update(Some(audio(60, false)), Some(brightness(55)));

        let shown = state().lock().unwrap().shown.expect("the display opens");

        assert_eq!(shown.kind, Kind::Brightness);
    }
}
