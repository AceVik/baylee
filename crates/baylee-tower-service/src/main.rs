//! Standalone Tower authority; credentials arrive only in bounded stdin configuration.
//!
//! Exit status: 0 when everything asked was done; 75 (`EX_TEMPFAIL`) when an
//! operation left part of it for a later run of the same input (an erasure
//! that met a duel under way); 1 for a refused input or a failure, after
//! which an administrative input may also run again unchanged.
#![forbid(unsafe_code)]
mod bootstrap;

fn main() {
    match bootstrap::run() {
        Ok(bootstrap::Ended::Done) => {}
        Ok(bootstrap::Ended::Again) => {
            eprintln!("Tower: part of this is left for later; run the same input again");
            std::process::exit(75);
        }
        Err(_) => {
            eprintln!("Tower service configuration, storage or worker failed");
            std::process::exit(1);
        }
    }
}
