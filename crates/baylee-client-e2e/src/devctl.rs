//! The client's dev-control harness over plain HTTP/1.1 on loopback: the one
//! driver of it, for the E2E tests, `cargo dev central` and the tools'
//! `shell` and `recording`. Loopback only, by construction: the port is the
//! only input, nothing here reads a proxy setting, and a reply that is not
//! 2xx is an error, so a redirect is never followed.
use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;
use std::{
    fmt,
    io::{ErrorKind, Read, Write},
    net::{SocketAddr, TcpStream},
    time::{Duration, Instant},
};

/// The largest reply body read by default; `/state` of a full table is a few
/// hundred kilobytes.
const REPLY_LIMIT: usize = 10 * 1024 * 1024;
/// Room for the status line and headers on top of the body's limit.
const HEAD_LIMIT: usize = 16 * 1024;

/// The harness answered and said no. Its reason can repeat a field of the
/// request (`unknown key: …`, a path), so a tool whose bodies may carry a
/// secret keeps the reason out of its output: `error.is::<Refused>()`.
#[derive(Debug)]
pub struct Refused {
    route: String,
    reason: String,
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the harness refused /{}: {}", self.route, self.reason)
    }
}

impl std::error::Error for Refused {}

/// A client's harness at `127.0.0.1:<port>`.
#[derive(Clone, Copy, Debug)]
pub struct Devctl {
    address: SocketAddr,
    reply_limit: usize,
}

impl Devctl {
    /// The harness of a client listening on loopback `port`.
    pub fn new(port: u16) -> Self {
        Self {
            address: SocketAddr::from(([127, 0, 0, 1], port)),
            reply_limit: REPLY_LIMIT,
        }
    }

    /// Refuses a reply whose body is larger than `bytes`.
    #[must_use]
    pub fn reply_limit(mut self, bytes: usize) -> Self {
        self.reply_limit = bytes;
        self
    }

    /// The port it listens on.
    pub fn port(&self) -> u16 {
        self.address.port()
    }

    /// GETs a route and reads its JSON, within `timeout`. A reply carrying
    /// `error` or `"ok": false` is a [`Refused`], here and for a POST.
    pub fn get(&self, route: &str, timeout: Duration) -> Result<Value> {
        self.request("GET", route, None, timeout)
    }

    /// POSTs a JSON body, within fifteen seconds.
    pub fn post(&self, route: &str, body: &Value) -> Result<Value> {
        self.post_within(route, body, Duration::from_secs(15))
    }

    /// [`Devctl::post`] with its own deadline: a screenshot waits for a
    /// frame to be drawn and read back, which a busy machine makes slow.
    pub fn post_within(&self, route: &str, body: &Value, timeout: Duration) -> Result<Value> {
        self.request("POST", route, Some(body), timeout)
    }

    fn request(
        &self,
        method: &str,
        route: &str,
        body: Option<&Value>,
        timeout: Duration,
    ) -> Result<Value> {
        ensure!(
            !route.starts_with("//")
                && !route.contains(|c: char| {
                    c.is_whitespace() || c.is_control() || matches!(c, '?' | '#')
                }),
            "invalid local route"
        );
        let route = route.trim_start_matches('/');
        let deadline = Instant::now() + timeout;
        let mut stream = TcpStream::connect_timeout(&self.address, timeout)
            .with_context(|| format!("no harness at {}", self.address))?;
        stream.set_write_timeout(Some(timeout))?;
        let body = body.map_or_default(Value::to_string);
        let request = format!(
            "{method} /{route} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            self.address,
            body.len()
        );
        stream.write_all(request.as_bytes())?;
        let reply = self
            .read_reply(&mut stream, deadline)
            .with_context(|| format!("{method} /{route}"))?;
        let reply = String::from_utf8(reply).context("a reply that is not UTF-8")?;
        let (head, payload) = reply
            .split_once("\r\n\r\n")
            .context("a reply without a body")?;
        let status = head.split_whitespace().nth(1).unwrap_or_default();
        if !status.starts_with('2') {
            bail!("{method} /{route} answered {status}");
        }
        ensure!(
            payload.len() <= self.reply_limit,
            "{method} /{route}: a reply over {} bytes",
            self.reply_limit
        );
        let chunked = head
            .to_ascii_lowercase()
            .contains("transfer-encoding: chunked");
        let payload = if chunked {
            dechunk(payload)?
        } else {
            ensure!(
                declared_length(head).is_none_or(|length| payload.len() >= length),
                "{method} /{route}: a reply cut short"
            );
            payload.to_owned()
        };
        let value: Value = serde_json::from_str(&payload)
            .with_context(|| format!("{method} /{route}: not JSON"))?;
        match refusal(&value) {
            Some(reason) => Err(Refused {
                route: route.to_owned(),
                reason,
            }
            .into()),
            None => Ok(value),
        }
    }

    /// Everything the harness sends until it closes, by `deadline` however
    /// slowly it comes, and no more than the limit allows.
    fn read_reply(&self, stream: &mut TcpStream, deadline: Instant) -> Result<Vec<u8>> {
        let most = self.reply_limit.saturating_add(HEAD_LIMIT);
        let mut reply = Vec::new();
        let mut chunk = [0; 16 * 1024];
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            ensure!(!left.is_zero(), "no whole reply in time");
            stream.set_read_timeout(Some(left))?;
            let read = match stream.read(&mut chunk) {
                Ok(0) => return Ok(reply),
                Ok(read) => read,
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.into()),
            };
            reply.extend_from_slice(&chunk[..read]);
            ensure!(
                reply.len() <= most,
                "a reply over {} bytes",
                self.reply_limit
            );
        }
    }
}

/// The harness's reason for saying no, if it did.
fn refusal(reply: &Value) -> Option<String> {
    match reply.get("error") {
        Some(Value::String(reason)) => Some(reason.clone()),
        Some(reason) => Some(reason.to_string()),
        None => {
            (reply.get("ok") == Some(&Value::Bool(false))).then(|| "no reason given".to_owned())
        }
    }
}

/// The `Content-Length` a reply's head declares.
fn declared_length(head: &str) -> Option<usize> {
    head.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("content-length")
            .then(|| value.trim().parse().ok())?
    })
}

/// A chunked body joined back together.
fn dechunk(mut text: &str) -> Result<String> {
    let mut out = String::new();
    loop {
        let (size, rest) = text.split_once("\r\n").context("a broken chunk")?;
        let size = usize::from_str_radix(size.trim(), 16)?;
        if size == 0 {
            return Ok(out);
        }
        out.push_str(rest.get(..size).context("a short chunk")?);
        text = rest.get(size + 2..).context("a chunk without its end")?;
    }
}

#[cfg(test)]
mod tests {
    use super::{Devctl, Refused};
    use serde_json::json;
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::{Ipv4Addr, TcpListener},
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };

    const SOON: Duration = Duration::from_secs(5);

    /// A harness that answers one request with `pieces`, a little apart, and
    /// hands back the request it read.
    fn answering(pieces: Vec<Vec<u8>>) -> (Devctl, JoinHandle<String>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let devctl = Devctl::new(listener.local_addr().unwrap().port());
        let peer = thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(socket);
            let mut request = String::new();
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if let Some(value) = line.strip_prefix("Content-Length:") {
                    length = value.trim().parse().unwrap();
                }
                request.push_str(&line);
                if line == "\r\n" {
                    break;
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            request.push_str(std::str::from_utf8(&body).unwrap());
            let mut socket = reader.into_inner();
            for piece in pieces {
                if socket.write_all(&piece).is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }
            request
        });
        (devctl, peer)
    }

    fn reply(status: &str, body: &str) -> Vec<Vec<u8>> {
        vec![
            format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .into_bytes(),
        ]
    }

    #[test]
    fn a_chunked_body_is_joined() {
        assert_eq!(
            super::dechunk("4\r\n{\"a\"\r\n3\r\n:1}\r\n0\r\n\r\n").unwrap(),
            "{\"a\":1}"
        );
        assert!(super::dechunk("zz\r\n").is_err());
    }

    #[test]
    fn a_reply_reads_as_json_and_a_post_carries_its_body() {
        let (devctl, peer) = answering(reply("200 OK", r#"{"ok":true,"n":1}"#));
        let body = json!({"name":"KeyK"});
        assert_eq!(devctl.post("/key", &body).unwrap()["n"], 1);
        let request = peer.join().unwrap();
        assert!(request.starts_with("POST /key HTTP/1.1\r\n"));
        assert_eq!(request.split_once("\r\n\r\n").unwrap().1, body.to_string());
    }

    #[test]
    fn a_refusal_says_why_on_a_get_as_on_a_post() {
        let (devctl, peer) = answering(reply("200 OK", r#"{"error":"unknown key: Foo"}"#));
        let error = devctl.get("state", SOON).unwrap_err();
        peer.join().unwrap();
        assert!(error.is::<Refused>());
        assert_eq!(
            error.to_string(),
            "the harness refused /state: unknown key: Foo"
        );
        let (devctl, peer) = answering(reply("200 OK", r#"{"ok":false}"#));
        let error = devctl.post("key", &json!({})).unwrap_err();
        peer.join().unwrap();
        assert!(error.is::<Refused>(), "{error}");
    }

    #[test]
    fn an_http_error_and_a_redirect_are_failures_not_refusals() {
        for status in ["500 Failure", "302 Found"] {
            let (devctl, peer) = answering(reply(status, r#"{"error":"body"}"#));
            let error = devctl.get("state", SOON).unwrap_err();
            peer.join().unwrap();
            assert!(!error.is::<Refused>());
            assert!(!error.to_string().contains("body"), "{error}");
        }
    }

    #[test]
    fn a_reply_cut_short_or_empty_is_a_failure() {
        let short = b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\n{}".to_vec();
        let (devctl, peer) = answering(vec![short]);
        let error = devctl.get("state", SOON).unwrap_err();
        peer.join().unwrap();
        assert!(error.to_string().contains("cut short"), "{error}");
        let (devctl, peer) = answering(reply("200 OK", ""));
        assert!(devctl.get("state", SOON).is_err());
        peer.join().unwrap();
    }

    #[test]
    fn the_reply_limit_lets_its_own_size_through_and_nothing_past_it() {
        let (devctl, peer) = answering(reply("200 OK", &format!("{}1", " ".repeat(99))));
        assert_eq!(devctl.reply_limit(100).get("s", SOON).unwrap(), json!(1));
        peer.join().unwrap();
        let (devctl, peer) = answering(reply("200 OK", &" ".repeat(101)));
        let error = devctl.reply_limit(100).get("s", SOON).unwrap_err();
        peer.join().unwrap();
        assert!(error.to_string().contains("over 100 bytes"), "{error}");
    }

    #[test]
    fn a_reply_that_trickles_does_not_outlive_the_deadline() {
        let (devctl, peer) = answering(vec![b"HTTP/1.1 200 OK\r\n".to_vec(); 40]);
        let started = Instant::now();
        assert!(devctl.get("state", Duration::from_millis(150)).is_err());
        assert!(started.elapsed() < Duration::from_millis(600));
        peer.join().unwrap();
    }

    #[test]
    fn a_route_that_would_change_the_request_is_refused_before_it_is_sent() {
        for route in ["//state", "a\r\nX: 1", "a b", "state?x=1", "state#x"] {
            let error = Devctl::new(1).get(route, SOON).unwrap_err();
            assert_eq!(error.to_string(), "invalid local route");
        }
    }
}
