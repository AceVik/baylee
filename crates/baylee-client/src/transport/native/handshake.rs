//! Absolute connection/TLS/upgrade budget, including peers that trickle bytes.

use std::io::{self, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

pub(super) struct Stream {
    inner: TcpStream,
    deadline: Option<Instant>,
}

impl Stream {
    pub(super) fn connect(uri: &ureq::http::Uri, budget: Duration) -> Result<Self, String> {
        use ureq::unversioned::resolver::{DefaultResolver, Resolver};
        use ureq::unversioned::transport::NextTimeout;

        let deadline = Instant::now() + budget;
        let host = uri.host().ok_or("the URL names no host")?;
        let port = uri
            .port_u16()
            .unwrap_or(if uri.scheme_str() == Some("wss") {
                443
            } else {
                80
            });
        let host = host.trim_start_matches('[').trim_end_matches(']');
        let addresses = if let Ok(ip) = host.parse::<IpAddr>() {
            vec![SocketAddr::new(ip, port)]
        } else {
            // Reuse the HTTP dependency's bounded resolver; the lookup sees no
            // path/query (which can contain a seat ticket). Its native OS lookup
            // may finish later, but this caller never waits beyond the budget.
            let address = ureq::http::Uri::builder()
                .scheme(if uri.scheme_str() == Some("wss") {
                    "https"
                } else {
                    "http"
                })
                .authority(uri.authority().ok_or("missing authority")?.clone())
                .path_and_query("/")
                .build()
                .map_err(|error| error.to_string())?;
            DefaultResolver::default()
                .resolve(
                    &address,
                    super::AGENT.config(),
                    NextTimeout {
                        after: budget.into(),
                        reason: ureq::Timeout::Resolve,
                    },
                )
                .map_err(|error| format!("Resolve: {error}"))?
                .iter()
                .copied()
                .collect()
        };
        let mut error = io::Error::new(io::ErrorKind::NotConnected, "no address connected");
        for address in addresses {
            let timeout = remaining(deadline).map_err(|error| error.to_string())?;
            match TcpStream::connect_timeout(&address, timeout.min(super::CONNECT_TIMEOUT)) {
                Ok(inner) => {
                    inner.set_nodelay(true).ok();
                    return Ok(Self {
                        inner,
                        deadline: Some(deadline),
                    });
                }
                Err(failed) => error = failed,
            }
        }
        Err(format!("Connect: {error}"))
    }

    pub(super) fn finish_handshake(&mut self, read_timeout: Option<Duration>) -> io::Result<()> {
        if let Some(deadline) = self.deadline {
            remaining(deadline)?;
        }
        self.deadline = None;
        self.inner.set_read_timeout(read_timeout)?;
        self.inner.set_write_timeout(Some(Duration::from_secs(10)))
    }

    fn before_read(&self) -> io::Result<()> {
        if let Some(deadline) = self.deadline {
            self.inner.set_read_timeout(Some(remaining(deadline)?))?;
        }
        Ok(())
    }

    fn before_write(&self) -> io::Result<()> {
        if let Some(deadline) = self.deadline {
            self.inner.set_write_timeout(Some(remaining(deadline)?))?;
        }
        Ok(())
    }
}

fn remaining(deadline: Instant) -> io::Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|left| !left.is_zero())
        .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "socket handshake timed out"))
}

impl Read for Stream {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.before_read()?;
        self.inner.read(bytes)
    }
}

impl Write for Stream {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.before_write()?;
        self.inner.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.before_write()?;
        self.inner.flush()
    }
}
