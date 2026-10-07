// Off Windows the curses only log. This keeps the app runnable on Linux for
// UI work and lets `cargo test` cover everything else.
use super::{Restore, Worker};

pub fn apply(id: &str, _restore: &mut Restore) -> Result<Option<Worker>, String> {
    eprintln!("[curse] apply {id}: no-op (not Windows)");
    Ok(None)
}

pub fn revert(restore: &Restore) {
    eprintln!("[curse] revert {restore:?}: no-op (not Windows)");
}
