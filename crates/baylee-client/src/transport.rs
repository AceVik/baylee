//! The client's one door to the network: HTTP requests and websockets, with
//! the certificate policy in one place.
//!
//! # Which certificates are trusted
//!
//! Every build verifies certificates. A gateway at `https://` or `wss://`
//! must show a certificate that chains to a public root (`webpki-roots`), or
//! nothing is sent to it. Development uses the explicit numeric loopback
//! origins admitted by Central; it does not weaken TLS for real gateways.
//! Gateway HTTP requests never follow redirects, including requests carrying
//! credentials in their body rather than in Authorization headers.
//! Cleartext HTTP/WS is allowed only to numeric loopback addresses, never a
//! hostname that could resolve to an external machine.
//!
//! # Why not `ehttp` and `ewebsock` directly
//!
//! Both are what the browser build still uses — there the browser checks
//! certificates and nothing here can change that. Natively neither lets its
//! TLS be configured, so the native half is our own: a `ureq` agent for HTTP
//! and a tungstenite client for sockets, each handed the configuration below.
//! The types callers see stay `ehttp`'s and `ewebsock`'s, so a call site
//! changes one path and nothing else.

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::{
    WsReceiver, WsSender, fetch, fetch_blocking, fetch_limited, tls, ws_connect,
};

#[cfg(target_arch = "wasm32")]
pub(crate) use ehttp::fetch;
#[cfg(target_arch = "wasm32")]
pub(crate) use ewebsock::{WsReceiver, WsSender, connect as ws_connect};

/// One bounded snapshot, so a continuously sending peer cannot keep a UI
/// drain loop alive indefinitely. The browser supplies its own frame buffers.
pub(crate) fn drain_socket(receiver: &WsReceiver) -> Vec<ewebsock::WsEvent> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        receiver.drain()
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::iter::from_fn(|| receiver.try_recv())
            .take(256)
            .collect()
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use ewebsock::{WsEvent, WsMessage};
    use std::ops::ControlFlow;
    use std::sync::mpsc::TryRecvError;
    use std::sync::{Arc, LazyLock};
    use tungstenite::stream::MaybeTlsStream;

    mod handshake;
    mod limits;
    mod receiver;
    mod sender;
    pub(crate) use receiver::WsReceiver;
    pub(crate) use sender::WsSender;

    fn check_address(uri: &ureq::http::Uri, clear: &str, secure: &str) -> Result<(), String> {
        let host = uri.host().ok_or("the URL names no host")?;
        if uri
            .authority()
            .is_some_and(|authority| authority.as_str().contains('@'))
        {
            return Err("credentials in transport URLs are not allowed".into());
        }
        let scheme = uri.scheme_str().ok_or("the URL names no scheme")?;
        if scheme == secure {
            return Ok(());
        }
        let loopback = host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback());
        if scheme == clear && loopback {
            Ok(())
        } else {
            Err("transport requires TLS or a numeric loopback address".into())
        }
    }

    // Catalog responses can be much larger than ordinary account traffic.
    // Routes with smaller protocols use fetch_limited instead.
    const MAX_RESPONSE_BYTES: usize = 256 * 1024 * 1024;
    const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
    const MAX_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
    const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

    /// The certificate settings for a `ureq` agent: strict in every profile.
    ///
    /// `platform` asks for the system's own trust store, which
    /// is what card art uses (#250); everything else trusts the bundled
    /// public roots, which do not depend on how a phone was set up.
    pub(crate) fn tls(platform: bool) -> ureq::tls::TlsConfig {
        use ureq::tls::{RootCerts, TlsConfig};
        let roots = if platform {
            RootCerts::PlatformVerifier
        } else {
            RootCerts::WebPki
        };
        TlsConfig::builder().root_certs(roots).build()
    }

    /// The agent every gateway request goes through.
    static AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
        ureq::Agent::config_builder()
            .tls_config(tls(false))
            .http_status_as_error(false)
            .max_redirects(0)
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_global(Some(REQUEST_TIMEOUT))
            .build()
            .new_agent()
    });

    /// `ehttp::fetch`, natively, under this module's certificate policy: the
    /// request runs on a thread of its own and the answer comes back through
    /// `on_done`, exactly as `ehttp` hands it.
    pub(crate) fn fetch(
        request: ehttp::Request,
        on_done: impl 'static + Send + FnOnce(ehttp::Result<ehttp::Response>),
    ) {
        fetch_limited(request, MAX_RESPONSE_BYTES, on_done);
    }

    /// Receive at most `limit` body bytes, including any HTTP-level inflation.
    /// The streaming reader stops at limit + 1 even without Content-Length.
    pub(crate) fn fetch_limited(
        request: ehttp::Request,
        limit: usize,
        on_done: impl 'static + Send + FnOnce(ehttp::Result<ehttp::Response>),
    ) {
        let spawned = std::thread::Builder::new()
            .name("baylee-http".to_owned())
            .spawn(move || on_done(fetch_blocking_limited(&request, limit)));
        if let Err(err) = spawned {
            bevy::log::error!(%err, "could not start an HTTP request");
        }
    }

    /// The same request, answered on the calling thread: for the one
    /// request that must go out before the process ends (a deletion whose
    /// Undo the player let run out by quitting, S-10).
    pub(crate) fn fetch_blocking(request: &ehttp::Request) -> ehttp::Result<ehttp::Response> {
        fetch_blocking_limited(request, MAX_RESPONSE_BYTES)
    }

    fn fetch_blocking_limited(
        request: &ehttp::Request,
        limit: usize,
    ) -> ehttp::Result<ehttp::Response> {
        let mut builder = ureq::http::Request::builder()
            .method(request.method.as_str())
            .uri(&request.url);
        for (key, value) in &request.headers {
            builder = builder.header(key, value);
        }
        let asked = builder
            .body(request.body.clone())
            .map_err(|err| err.to_string())?;
        check_address(asked.uri(), "http", "https")?;
        let timeout = request
            .timeout
            .filter(|timeout| !timeout.is_zero())
            .unwrap_or(REQUEST_TIMEOUT)
            .min(MAX_REQUEST_TIMEOUT);
        let asked = AGENT
            .configure_request(asked)
            .timeout_global(Some(timeout))
            .timeout_recv_body(Some(timeout))
            .build();
        let mut answer = AGENT.run(asked).map_err(|err| err.to_string())?;

        let status = answer.status();
        let mut headers = ehttp::Headers::default();
        for (key, value) in answer.headers() {
            if let Ok(value) = value.to_str() {
                headers.insert(key, value);
            }
        }
        headers.sort();
        let bytes = if request.method == ehttp::Method::HEAD {
            Vec::new()
        } else {
            limits::read_body(answer.body_mut().as_reader(), limit)?
        };
        Ok(ehttp::Response {
            url: request.url.clone(),
            ok: status.is_success(),
            status: status.as_u16(),
            status_text: status.canonical_reason().unwrap_or("ERROR").to_owned(),
            headers,
            bytes,
        })
    }

    /// `ewebsock::connect`, natively, under this module's certificate
    /// policy. Events arrive on the returned receiver as `ewebsock` delivers
    /// them: `Opened`, messages, then `Closed` or `Error`.
    pub(crate) fn ws_connect(
        url: impl Into<String>,
        options: ewebsock::Options,
    ) -> Result<(WsSender, WsReceiver), String> {
        let url = url.into();
        let (receiver, on_event) = WsReceiver::new();
        let (sender, rx) = sender::channel(Arc::clone(&on_event), receiver.cancellation());
        std::thread::Builder::new()
            .name("baylee-socket".to_owned())
            .spawn(move || {
                if let Err(err) = run_socket(&url, &options, &*on_event, &rx) {
                    let _ = on_event(WsEvent::Error(err));
                }
            })
            .map_err(|err| format!("Failed to spawn thread: {err}"))?;
        Ok((sender, receiver))
    }

    /// The rustls settings a `wss://` socket is opened with.
    fn socket_tls() -> Arc<rustls::ClientConfig> {
        static CONFIG: LazyLock<Arc<rustls::ClientConfig>> = LazyLock::new(|| {
            let provider = Arc::new(rustls::crypto::ring::default_provider());
            let builder = rustls::ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .expect("ring supports the default protocol versions");
            let roots = rustls::RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
            };
            let config = builder.with_root_certificates(roots).with_no_client_auth();
            Arc::new(config)
        });
        CONFIG.clone()
    }

    // ewebsock 0.8's From<Options> targets tungstenite 0.24, not this version.
    fn socket_config(options: &ewebsock::Options) -> tungstenite::protocol::WebSocketConfig {
        let max_frame_size = (options.max_incoming_frame_size != usize::MAX)
            .then_some(options.max_incoming_frame_size);
        tungstenite::protocol::WebSocketConfig::default()
            // 0.24 read in 4 KiB chunks; the new 128 KiB default costs more per socket.
            .read_buffer_size(4 * 1024)
            .max_frame_size(max_frame_size)
    }

    fn run_socket(
        url: &str,
        options: &ewebsock::Options,
        on_event: &(dyn Fn(WsEvent) -> ControlFlow<()> + Send),
        rx: &sender::SocketQueue,
    ) -> Result<(), String> {
        run_socket_with_budget(
            url,
            options,
            on_event,
            rx,
            std::time::Duration::from_secs(15),
        )
    }

    fn run_socket_with_budget(
        url: &str,
        options: &ewebsock::Options,
        on_event: &(dyn Fn(WsEvent) -> ControlFlow<()> + Send),
        rx: &sender::SocketQueue,
        budget: std::time::Duration,
    ) -> Result<(), String> {
        let uri: tungstenite::http::Uri = url
            .parse()
            .map_err(|err| format!("Failed to parse WebSocket URL: {err}"))?;
        check_address(&uri, "ws", "wss")?;
        let stream = handshake::Stream::connect(&uri, budget)?;
        let read_timeout = options.read_timeout;
        let config = socket_config(options);
        let (mut socket, _) = tungstenite::client_tls_with_config(
            uri,
            stream,
            Some(config),
            Some(tungstenite::Connector::Rustls(socket_tls())),
        )
        .map_err(|err| format!("Connect: {err}"))?;

        // Without a read timeout the read below would hold the thread and no
        // queued message would ever go out.
        let timeout = read_timeout
            .filter(|t| !t.is_zero())
            .or(Some(std::time::Duration::from_millis(10)));
        match socket.get_mut() {
            MaybeTlsStream::Plain(s) => s.finish_handshake(timeout),
            MaybeTlsStream::Rustls(s) => s.get_mut().finish_handshake(timeout),
            _ => Ok(()),
        }
        .map_err(|err| format!("failed to set read timeout: {err}"))?;

        if on_event(WsEvent::Opened).is_break() {
            return socket.close(None).map_err(|err| err.to_string());
        }
        loop {
            let mut wrote = false;
            loop {
                match rx.try_recv() {
                    Ok(queued) => {
                        let sender::Queued {
                            message,
                            charge: _charge,
                        } = queued;
                        let message = match message {
                            WsMessage::Text(text) => tungstenite::Message::Text(text.into()),
                            WsMessage::Binary(data) => tungstenite::Message::Binary(data.into()),
                            WsMessage::Ping(data) => tungstenite::Message::Ping(data.into()),
                            WsMessage::Pong(data) => tungstenite::Message::Pong(data.into()),
                            WsMessage::Unknown(_) => continue,
                        };
                        if let Err(err) = socket.write(message) {
                            socket.close(None).ok();
                            return Err(format!("send: {err}"));
                        }
                        wrote = true;
                    }
                    Err(TryRecvError::Disconnected) => {
                        socket.close(None).ok();
                        socket.flush().ok();
                        return Ok(());
                    }
                    Err(TryRecvError::Empty) => break,
                }
            }
            if wrote && let Err(err) = socket.flush() {
                socket.close(None).ok();
                return Err(format!("flush: {err}"));
            }
            let control = match socket.read() {
                Ok(tungstenite::Message::Text(text)) => {
                    on_event(WsEvent::Message(WsMessage::Text(text.to_string())))
                }
                Ok(tungstenite::Message::Binary(data)) => {
                    on_event(WsEvent::Message(WsMessage::Binary(data.to_vec())))
                }
                Ok(tungstenite::Message::Ping(data)) => {
                    on_event(WsEvent::Message(WsMessage::Ping(data.to_vec())))
                }
                Ok(tungstenite::Message::Pong(data)) => {
                    on_event(WsEvent::Message(WsMessage::Pong(data.to_vec())))
                }
                Ok(tungstenite::Message::Close(_))
                | Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                    let _ = on_event(WsEvent::Closed);
                    ControlFlow::Break(())
                }
                Ok(tungstenite::Message::Frame(_)) => ControlFlow::Continue(()),
                Err(tungstenite::Error::Io(err))
                    if matches!(
                        err.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    ControlFlow::Continue(())
                }
                Err(err) => return Err(format!("read: {err}")),
            };
            if control.is_break() {
                socket.close(None).ok();
                return Ok(());
            }
        }
    }

    #[cfg(test)]
    mod security_tests;
    #[cfg(test)]
    mod tests;
}
