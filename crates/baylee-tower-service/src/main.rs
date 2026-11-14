//! Standalone Tower authority; credentials arrive only in bounded stdin configuration.
#![forbid(unsafe_code)]
mod bootstrap;

fn main() {
    if bootstrap::run().is_err() {
        eprintln!("Tower service configuration, storage or worker failed");
        std::process::exit(1);
    }
}
