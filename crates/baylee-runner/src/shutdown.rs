//! Signals request an orderly return through VM and registration cleanup.
use anyhow::{Context, Result};
use std::sync::atomic::{AtomicBool, Ordering};

static REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn install() -> Result<()> {
    ctrlc::set_handler(|| REQUESTED.store(true, Ordering::Relaxed))
        .context("installing runner shutdown handler")
}

pub fn requested() -> bool {
    REQUESTED.load(Ordering::Relaxed)
}
