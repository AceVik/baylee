//! The client's one door to the network: HTTP requests and websockets, with
//! the certificate policy in one place.
//!
//! # Which certificates are trusted
//!
//! A release build (`debug_assertions` off, which `dist` inherits) trusts
//! what a browser would: a gateway at `https://` must show a certificate that
//! chains to a public root (`webpki-roots`), or nothing is sent to it. A
//! debug or test build accepts any certificate, so a gateway on a laptop or
//! a test box can run on plain `http://` or on a self-signed `https://`, and
//! says so once, loudly, in the log. There is no switch between the two
//! other than the build profile: a release that could be talked out of
//! checking would be one that a hostile network could talk out of it too.
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
pub(crate) use native::{WsSender, fetch, tls, ws_connect};

#[cfg(target_arch = "wasm32")]
pub(crate) use ehttp::fetch;
#[cfg(target_arch = "wasm32")]
pub(crate) use ewebsock::{WsSender, connect as ws_connect};

/// Whether this build checks certificates. The whole policy, as a value.
/// (In a browser the browser decides, always strictly.)
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const VERIFIES_CERTIFICATES: bool = !cfg!(debug_assertions);

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::VERIFIES_CERTIFICATES;
    use ewebsock::{WsEvent, WsMessage, WsReceiver};
    use std::io::Read as _;
    use std::net::TcpStream;
    use std::ops::ControlFlow;
    use std::sync::mpsc::{self, TryRecvError};
    use std::sync::{Arc, LazyLock, Once};
    use tungstenite::stream::MaybeTlsStream;

    /// Says once per run that this build takes any certificate.
    fn warn_once() {
        static WARNED: Once = Once::new();
        if !VERIFIES_CERTIFICATES {
            WARNED.call_once(|| {
                bevy::log::warn!(
                    "DEBUG BUILD: TLS certificates are NOT verified. Self-signed gateways are \
                     accepted; a release build refuses them."
                );
            });
        }
    }

    /// The certificate settings for a `ureq` agent: strict in a release,
    /// none in a debug build.
    ///
    /// `platform` asks for the system's own trust store in a release, which
    /// is what card art uses (#250); everything else trusts the bundled
    /// public roots, which do not depend on how a phone was set up.
    pub(crate) fn tls(platform: bool) -> ureq::tls::TlsConfig {
        use ureq::tls::{RootCerts, TlsConfig};
        warn_once();
        let roots = if platform {
            RootCerts::PlatformVerifier
        } else {
            RootCerts::WebPki
        };
        TlsConfig::builder()
            .root_certs(roots)
            .disable_verification(!VERIFIES_CERTIFICATES)
            .build()
    }

    /// The agent every gateway request goes through.
    static AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
        ureq::Agent::config_builder()
            .tls_config(tls(false))
            .http_status_as_error(false)
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
        let spawned = std::thread::Builder::new()
            .name("baylee-http".to_owned())
            .spawn(move || on_done(fetch_blocking(&request)));
        if let Err(err) = spawned {
            bevy::log::error!(%err, "could not start an HTTP request");
        }
    }

    fn fetch_blocking(request: &ehttp::Request) -> ehttp::Result<ehttp::Response> {
        let mut builder = ureq::http::Request::builder()
            .method(request.method.as_str())
            .uri(&request.url);
        for (key, value) in &request.headers {
            builder = builder.header(key, value);
        }
        let asked = builder
            .body(request.body.clone())
            .map_err(|err| err.to_string())?;
        let asked = AGENT
            .configure_request(asked)
            .timeout_recv_body(request.timeout)
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
        let mut bytes = Vec::new();
        if request.method != ehttp::Method::HEAD {
            answer
                .body_mut()
                .as_reader()
                .read_to_end(&mut bytes)
                .map_err(|err| format!("Failed to read response body: {err}"))?;
        }
        Ok(ehttp::Response {
            url: request.url.clone(),
            ok: status.is_success(),
            status: status.as_u16(),
            status_text: status.canonical_reason().unwrap_or("ERROR").to_owned(),
            headers,
            bytes,
        })
    }

    /// The sending half of a socket from [`ws_connect`]: `ewebsock`'s own
    /// shape, a channel into the socket's thread. Dropping it closes the
    /// socket.
    pub(crate) struct WsSender {
        tx: Option<mpsc::Sender<WsMessage>>,
    }

    impl WsSender {
        /// Queues a message; one sent before the socket opened waits.
        pub(crate) fn send(&mut self, message: WsMessage) {
            if let Some(tx) = &self.tx {
                tx.send(message).ok();
            }
        }
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
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("baylee-socket".to_owned())
            .spawn(move || {
                if let Err(err) = run_socket(&url, options, &*on_event, &rx) {
                    let _ = on_event(WsEvent::Error(err));
                }
            })
            .map_err(|err| format!("Failed to spawn thread: {err}"))?;
        Ok((WsSender { tx: Some(tx) }, receiver))
    }

    /// The rustls settings a `wss://` socket is opened with.
    fn socket_tls() -> Arc<rustls::ClientConfig> {
        static CONFIG: LazyLock<Arc<rustls::ClientConfig>> = LazyLock::new(|| {
            let provider = Arc::new(rustls::crypto::ring::default_provider());
            let builder = rustls::ClientConfig::builder_with_provider(provider.clone())
                .with_safe_default_protocol_versions()
                .expect("ring supports the default protocol versions");
            let config = if VERIFIES_CERTIFICATES {
                let roots = rustls::RootCertStore {
                    roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
                };
                builder.with_root_certificates(roots).with_no_client_auth()
            } else {
                builder
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(AnyCertificate(provider)))
                    .with_no_client_auth()
            };
            Arc::new(config)
        });
        warn_once();
        CONFIG.clone()
    }

    fn run_socket(
        url: &str,
        options: ewebsock::Options,
        on_event: &(dyn Fn(WsEvent) -> ControlFlow<()> + Send),
        rx: &mpsc::Receiver<WsMessage>,
    ) -> Result<(), String> {
        let uri: tungstenite::http::Uri = url
            .parse()
            .map_err(|err| format!("Failed to parse URL {url:?}: {err}"))?;
        let host = uri.host().ok_or("the URL names no host")?.to_owned();
        let port = uri
            .port_u16()
            .unwrap_or(if uri.scheme_str() == Some("wss") {
                443
            } else {
                80
            });
        let host = host.trim_start_matches('[').trim_end_matches(']');
        let stream = TcpStream::connect((host, port)).map_err(|err| format!("Connect: {err}"))?;
        stream.set_nodelay(true).ok();
        let read_timeout = options.read_timeout;
        let config = tungstenite::protocol::WebSocketConfig::from(options);
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
            MaybeTlsStream::Plain(s) => s.set_read_timeout(timeout),
            MaybeTlsStream::Rustls(s) => s.get_mut().set_read_timeout(timeout),
            _ => Ok(()),
        }
        .map_err(|err| format!("failed to set read timeout: {err}"))?;

        if on_event(WsEvent::Opened).is_break() {
            return socket.close(None).map_err(|err| err.to_string());
        }
        loop {
            match rx.try_recv() {
                Ok(message) => {
                    let message = match message {
                        WsMessage::Text(text) => tungstenite::Message::Text(text),
                        WsMessage::Binary(data) => tungstenite::Message::Binary(data),
                        WsMessage::Ping(data) => tungstenite::Message::Ping(data),
                        WsMessage::Pong(data) => tungstenite::Message::Pong(data),
                        WsMessage::Unknown(_) => continue,
                    };
                    if let Err(err) = socket.send(message) {
                        socket.close(None).ok();
                        return Err(format!("send: {err}"));
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    socket.close(None).ok();
                    socket.flush().ok();
                    return Ok(());
                }
                Err(TryRecvError::Empty) => {}
            }
            let control = match socket.read() {
                Ok(tungstenite::Message::Text(text)) => {
                    on_event(WsEvent::Message(WsMessage::Text(text)))
                }
                Ok(tungstenite::Message::Binary(data)) => {
                    on_event(WsEvent::Message(WsMessage::Binary(data)))
                }
                Ok(tungstenite::Message::Ping(data)) => {
                    on_event(WsEvent::Message(WsMessage::Ping(data)))
                }
                Ok(tungstenite::Message::Pong(data)) => {
                    on_event(WsEvent::Message(WsMessage::Pong(data)))
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

    /// A debug build's verifier: every certificate passes, signatures are
    /// still checked so the handshake is a real one.
    #[derive(Debug)]
    struct AnyCertificate(Arc<rustls::crypto::CryptoProvider>);

    impl rustls::client::danger::ServerCertVerifier for AnyCertificate {
        fn verify_server_cert(
            &self,
            _end_entity: &rustls::pki_types::CertificateDer<'_>,
            _intermediates: &[rustls::pki_types::CertificateDer<'_>],
            _server_name: &rustls::pki_types::ServerName<'_>,
            _ocsp_response: &[u8],
            _now: rustls::pki_types::UnixTime,
        ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
            Ok(rustls::client::danger::ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            message: &[u8],
            cert: &rustls::pki_types::CertificateDer<'_>,
            dss: &rustls::DigitallySignedStruct,
        ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
            rustls::crypto::verify_tls12_signature(
                message,
                cert,
                dss,
                &self.0.signature_verification_algorithms,
            )
        }

        fn verify_tls13_signature(
            &self,
            message: &[u8],
            cert: &rustls::pki_types::CertificateDer<'_>,
            dss: &rustls::DigitallySignedStruct,
        ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
            rustls::crypto::verify_tls13_signature(
                message,
                cert,
                dss,
                &self.0.signature_verification_algorithms,
            )
        }

        fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
            self.0.signature_verification_algorithms.supported_schemes()
        }
    }
}
