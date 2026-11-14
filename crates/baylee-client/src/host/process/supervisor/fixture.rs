//! Standalone synthetic child used by supervisor tests, compiled into a tempdir.
#![forbid(unsafe_code)]
use std::io::{Read as _, Write as _};
fn main() {
    let mut input = std::io::stdin().lock();
    let mut prefix = [0; 4];
    input.read_exact(&mut prefix).unwrap();
    let length = u32::from_be_bytes(prefix) as usize;
    assert!(length < 1024 * 1024);
    input.read_exact(&mut vec![0; length]).unwrap();
    let reply = std::fs::read(std::env::var_os("BAYLEE_FIXTURE_REPLY").unwrap()).unwrap();
    std::io::stdout().write_all(&reply).unwrap();
    std::io::stdout().flush().unwrap();
    // No progress, even when commanded. Only the owning supervisor can stop it.
    std::thread::sleep(std::time::Duration::from_secs(30));
}
