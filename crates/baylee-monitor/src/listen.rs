//! Binding the listener patiently: right after login or boot the LAN address
//! may not be assigned yet, and the kernel refuses to bind it until DHCP has
//! finished.

use std::{io, net::SocketAddr, time::Duration};
use tokio::{net::TcpListener, time::Instant};

/// How long to wait for the configured address before handing the failure
/// to the service manager, which restarts the process.
pub(crate) const ADDRESS_WAIT: Duration = Duration::from_secs(600);
const FIRST_RETRY: Duration = Duration::from_millis(250);
const LONGEST_RETRY: Duration = Duration::from_secs(2);

/// Binds `address`, retrying with a short, capped backoff while the kernel
/// reports it as not assigned to any interface (`EADDRNOTAVAIL`). Every other
/// error, such as a port already in use, fails at once.
pub(crate) async fn bind(address: SocketAddr, patience: Duration) -> io::Result<TcpListener> {
    let started = Instant::now();
    let mut delay = FIRST_RETRY;
    let mut waiting = false;
    loop {
        match TcpListener::bind(address).await {
            Ok(listener) => {
                if waiting {
                    eprintln!(
                        "Baylee Pulse: {address} is available after {:.1} s",
                        started.elapsed().as_secs_f64()
                    );
                }
                return Ok(listener);
            }
            Err(error)
                if error.kind() == io::ErrorKind::AddrNotAvailable
                    && started.elapsed() < patience =>
            {
                if !waiting {
                    eprintln!(
                        "Baylee Pulse: {address} is not assigned yet ({error}); \
                         waiting up to {} s for the network",
                        patience.as_secs()
                    );
                    waiting = true;
                }
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(LONGEST_RETRY);
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn waits_for_an_unassigned_address_then_gives_up() {
        // TEST-NET-1 is never assigned to a local interface, unless the host
        // allows non-local binds; then there is nothing to wait for.
        let address: SocketAddr = "192.0.2.1:0".parse().unwrap();
        let patience = Duration::from_millis(400);
        let started = Instant::now();
        match bind(address, patience).await {
            Ok(_) => {}
            Err(error) => {
                assert_eq!(error.kind(), io::ErrorKind::AddrNotAvailable);
                assert!(started.elapsed() >= patience, "gave up before its patience");
            }
        }
    }

    #[tokio::test]
    async fn other_errors_fail_at_once() {
        let taken = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let started = Instant::now();
        let error = bind(taken.local_addr().unwrap(), ADDRESS_WAIT)
            .await
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AddrInUse);
        assert!(
            started.elapsed() < FIRST_RETRY,
            "an occupied port is not retried"
        );
    }
}
