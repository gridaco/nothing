pub mod backends;
pub mod cache;
pub use cg;
pub mod embedded_fonts;
pub mod export;
pub mod formats;
pub mod hittest;
pub mod htmlcss;
pub mod import;
pub mod io;
pub mod layout;
pub mod node;
pub mod os;
pub mod overlay;
pub mod painter;
pub mod query;
pub mod resources;
pub mod runtime;
pub mod shape;
pub mod sys;
pub mod text;
pub mod text_edit;
pub mod text_edit_session;
pub mod vectornetwork;
pub mod window;

/// Shared test lock for Stylo's process-wide configuration.
///
/// Document state is session-owned, but Stylo preferences remain process-wide.
/// Tests that exercise Stylo keep their setup and reads serialized. We use
/// `unwrap_or_else(|e| e.into_inner())` to recover from poison so a single
/// test panic does not cascade to every other Stylo test.
#[cfg(test)]
pub(crate) mod stylo_test {
    use std::sync::{Mutex, MutexGuard};

    static STYLO_TEST_LOCK: Mutex<()> = Mutex::new(());

    pub(crate) fn lock() -> MutexGuard<'static, ()> {
        STYLO_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }
}
