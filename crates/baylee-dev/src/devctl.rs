//! The client's dev-control harness over plain HTTP/1.1 on loopback: the
//! few requests a scenario needs, each with its own timeout. Loopback only,
//! by construction: the port is the only input.
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

/// A client's harness at `127.0.0.1:<port>`.
pub struct Devctl {
    address: SocketAddr,
}

impl Devctl {
    pub fn new(port: u16) -> Self {
        Self {
            address: SocketAddr::from(([127, 0, 0, 1], port)),
        }
    }

    pub fn get(&self, route: &str, timeout: Duration) -> Result<Value> {
        self.request("GET", route, None, timeout)
    }

    /// POSTs a JSON body; a reply carrying `error` is the harness refusing.
    pub fn post(&self, route: &str, body: &Value) -> Result<Value> {
        let reply = self.request("POST", route, Some(body), Duration::from_secs(15))?;
        if reply.get("error").is_some() {
            bail!("the harness refused {route}");
        }
        Ok(reply)
    }

    fn request(
        &self,
        method: &str,
        route: &str,
        body: Option<&Value>,
        timeout: Duration,
    ) -> Result<Value> {
        let mut stream = TcpStream::connect_timeout(&self.address, timeout)?;
        stream.set_read_timeout(Some(timeout))?;
        stream.set_write_timeout(Some(timeout))?;
        let body = body.map(Value::to_string).unwrap_or_default();
        write!(
            stream,
            "{method} /{} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            route.trim_start_matches('/'),
            self.address,
            body.len()
        )?;
        let mut reply = Vec::new();
        stream.read_to_end(&mut reply)?;
        let reply = String::from_utf8(reply).context("a reply that is not UTF-8")?;
        let (head, payload) = reply
            .split_once("\r\n\r\n")
            .context("a reply without a body")?;
        let status = head.split_whitespace().nth(1).unwrap_or_default();
        if !status.starts_with('2') {
            bail!("{method} /{route} answered {status}");
        }
        let payload = if head
            .to_ascii_lowercase()
            .contains("transfer-encoding: chunked")
        {
            dechunk(payload)?
        } else {
            payload.to_owned()
        };
        if payload.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&payload).with_context(|| format!("{method} /{route}: not JSON"))
    }
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
    #[test]
    fn a_chunked_body_is_joined() {
        assert_eq!(
            super::dechunk("4\r\n{\"a\"\r\n3\r\n:1}\r\n0\r\n\r\n").unwrap(),
            "{\"a\":1}"
        );
        assert!(super::dechunk("zz\r\n").is_err());
    }
}
