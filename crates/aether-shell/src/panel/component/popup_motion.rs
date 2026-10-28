use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

pub(super) const HOVER_EXIT_DELAY: Duration = Duration::from_millis(140);
pub(super) const EXPANSION_DURATION: Duration = Duration::from_millis(220);

pub(super) fn set_hovered(
    hovered: bool,
    state: &'static AtomicBool,
    generation: &'static AtomicU64,
) {
    if cfg!(test) {
        state.store(hovered, Ordering::SeqCst);
        return;
    }

    if hovered {
        generation.fetch_add(1, Ordering::Relaxed);
        if !state.swap(true, Ordering::Relaxed) {
            aether::refresh();
        }
        return;
    }

    let generation_at_exit = generation.fetch_add(1, Ordering::Relaxed) + 1;
    crate::services::runtime::runtime().spawn(async move {
        tokio::time::sleep(HOVER_EXIT_DELAY).await;
        if generation.load(Ordering::Relaxed) == generation_at_exit
            && state.swap(false, Ordering::Relaxed)
        {
            aether::refresh();
        }
    });
}
