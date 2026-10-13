//! The chat's wire (the beta.8 social design §8, §11.1): the HTTP routes the
//! machine asks for, and the one social socket, opened with a fresh ticket
//! on every dial (#294).
//!
//! Nothing here decides anything: the machine says what to send
//! ([`baylee_client_core::social::Effect`]) and is handed back what came
//! ([`super::Arrived`]). Answers land in a mailbox from whatever thread the
//! request finished on, and the frame loop drains it.

use std::sync::{Arc, Mutex, PoisonError};

use baylee_client_core::social::{HttpTag, Method};
use baylee_client_core::wsticket::{Step, TicketAnswer, TicketDial};

/// Something that came back from the gateway, for the frame loop.
pub(crate) enum Arrived {
    /// An answer to a request the machine asked for.
    Http {
        /// What it was for.
        tag: HttpTag,
        /// The status (0 when nothing answered).
        status: u16,
        /// The body.
        body: Vec<u8>,
    },
    /// `GET /info`'s `social`: `Some((version, keep_days))`, or `None` when
    /// the gateway has no chat (or did not say).
    Info(Option<(u32, u32)>),
    /// The ticket request's answer: the status and body.
    Ticket(u16, Vec<u8>),
}

/// Where answers wait for the frame loop.
#[derive(Clone, Default)]
pub(crate) struct Mailbox(pub(crate) Arc<Mutex<Vec<Arrived>>>);

impl Mailbox {
    fn post(&self, arrived: Arrived) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(arrived);
    }

    /// Everything that arrived since the last call.
    pub(crate) fn drain(&self) -> Vec<Arrived> {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

fn method_name(method: Method) -> ehttp::Method {
    match method {
        Method::Get => ehttp::Method::GET,
        Method::Post => ehttp::Method::POST,
        Method::Put => ehttp::Method::PUT,
        Method::Patch => ehttp::Method::PATCH,
        Method::Delete => ehttp::Method::DELETE,
    }
}

/// Sends one of the machine's requests with the session in
/// `Authorization`, never in the address.
pub(crate) fn request(
    mailbox: &Mailbox,
    gateway: &str,
    token: &str,
    tag: HttpTag,
    method: Method,
    path: &str,
    body: Option<&serde_json::Value>,
) {
    let url = format!("{}{path}", gateway.trim_end_matches('/'));
    let bytes = body
        .map(|b| serde_json::to_vec(b).unwrap_or_default())
        .unwrap_or_default();
    let mut request = ehttp::Request::post(url, bytes);
    request.method = method_name(method);
    request.headers = ehttp::Headers::new(&[
        ("Accept", "application/json"),
        ("Content-Type", "application/json"),
    ]);
    request
        .headers
        .insert("Authorization", format!("Bearer {token}"));
    let mailbox = mailbox.clone();
    crate::transport::fetch(request, move |result| {
        let (status, body) = match result {
            Ok(response) => (response.status, response.bytes),
            Err(_) => (0, Vec::new()),
        };
        mailbox.post(Arrived::Http { tag, status, body });
    });
}

/// Asks the gateway whether it has chat (`GET /info`'s `social`).
pub(crate) fn ask_info(mailbox: &Mailbox, gateway: &str) {
    let url = format!("{}/info", gateway.trim_end_matches('/'));
    let mailbox = mailbox.clone();
    crate::transport::fetch(ehttp::Request::get(url), move |result| {
        let social = result
            .ok()
            .filter(|r| r.ok)
            .and_then(|r| serde_json::from_slice::<serde_json::Value>(&r.bytes).ok())
            .and_then(|v| {
                let social = v.get("social")?;
                let version = u32::try_from(social.get("version")?.as_u64()?).ok()?;
                let keep = social
                    .get("keep_days")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|d| u32::try_from(d).ok())
                    .unwrap_or(baylee_protocol::social::KEEP_DAYS_DEFAULT);
                Some((version, keep))
            });
        mailbox.post(Arrived::Info(social));
    });
}

/// Asks for a ticket for the social socket with the session.
fn ask_ticket(mailbox: &Mailbox, gateway: &str, token: &str) {
    let url = format!(
        "{}{}",
        gateway.trim_end_matches('/'),
        baylee_protocol::WS_TICKET_PATH
    );
    let body = baylee_client_core::wsticket::Socket::Social.request_body();
    let mut request = ehttp::Request::post(url, body.into_bytes());
    request.headers = ehttp::Headers::new(&[
        ("Accept", "application/json"),
        ("Content-Type", "application/json"),
    ]);
    request
        .headers
        .insert("Authorization", format!("Bearer {token}"));
    let mailbox = mailbox.clone();
    crate::transport::fetch(request, move |result| {
        let (status, body) = match result {
            Ok(response) => (response.status, response.bytes),
            Err(_) => (0, Vec::new()),
        };
        mailbox.post(Arrived::Ticket(status, body));
    });
}

/// What the socket did this frame, for the machine.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum LinkNews {
    /// It opened.
    Up,
    /// It closed or failed (the dial redials on its schedule).
    Down,
    /// The ticket was refused: no chat here. No more dialling.
    Refused,
    /// The session is gone: sign in again.
    Unauthorized,
    /// A text frame.
    Frame(String),
}

/// The social socket and its dial.
#[derive(Default)]
pub(crate) struct Socket {
    /// The live socket. Behind a `Mutex` only to be `Sync`.
    link: Mutex<Option<(crate::net::SocketSender, ewebsock::WsReceiver)>>,
    /// Where the dial stands.
    dial: Option<TicketDial>,
    /// Whether the machine wants the socket at all.
    wanted: bool,
    /// The back-off before the next dial after a failure: 0.5 s doubling to
    /// 15 s, for as long as the chat is wanted (it never gives up: a
    /// gateway that comes back finds the chat again by itself).
    backoff: f32,
    /// Seconds before the next dial.
    wait: f32,
    /// Whether the socket is open now.
    open: bool,
    /// Refused for good.
    stopped: bool,
}

impl Socket {
    /// Asks for the socket ([`baylee_client_core::social::Effect::Dial`]).
    pub(crate) fn want(&mut self) {
        if !self.stopped {
            self.wanted = true;
            self.wait = 0.0;
        }
    }

    /// Closes it and dials no more until asked again.
    pub(crate) fn hang(&mut self) {
        self.wanted = false;
        self.dial = None;
        self.open = false;
        *self.link.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }

    /// Sends a text frame; `false` when there is no socket to send on.
    pub(crate) fn send(&self, text: String) -> bool {
        let mut link = self.link.lock().unwrap_or_else(PoisonError::into_inner);
        let Some((sender, _)) = link.as_mut() else {
            return false;
        };
        sender.send(ewebsock::WsMessage::Text(text));
        true
    }

    /// The ticket's answer arrived.
    pub(crate) fn ticket(
        &mut self,
        status: u16,
        body: &[u8],
        gateway: &str,
        token: &str,
        mailbox: &Mailbox,
        news: &mut Vec<LinkNews>,
    ) {
        if !self.wanted {
            return;
        }
        if status == 403 {
            // `BAYLEE_SOCIAL=off`, or an account the chat refuses.
            self.stopped = true;
            self.hang();
            news.push(LinkNews::Refused);
            return;
        }
        let Some(dial) = self.dial.as_mut() else {
            return;
        };
        let step = dial.answered(TicketAnswer::read(status, body));
        self.perform(step, gateway, token, mailbox, news);
    }

    fn perform(
        &mut self,
        step: Step,
        gateway: &str,
        token: &str,
        mailbox: &Mailbox,
        news: &mut Vec<LinkNews>,
    ) {
        let mut step = step;
        loop {
            match step {
                Step::Dial(ticket) => {
                    let url = format!(
                        "{}{}",
                        crate::net::ws_base(gateway),
                        baylee_protocol::social::social_socket_path(&ticket)
                    );
                    let options = ewebsock::Options {
                        max_incoming_frame_size: 16 * 1024 * 1024,
                        ..ewebsock::Options::default()
                    };
                    match crate::transport::ws_connect(url, options) {
                        Ok((sender, receiver)) => {
                            *self.link.lock().unwrap_or_else(PoisonError::into_inner) =
                                Some((crate::net::wrap_sender(sender), receiver));
                            return;
                        }
                        Err(_) => {
                            step = self.dial.as_mut().map_or(Step::GiveUp, TicketDial::failed);
                        }
                    }
                }
                Step::FetchTicket => {
                    *self.link.lock().unwrap_or_else(PoisonError::into_inner) = None;
                    ask_ticket(mailbox, gateway, token);
                    return;
                }
                Step::Unauthorized => {
                    self.hang();
                    news.push(LinkNews::Unauthorized);
                    return;
                }
                Step::GiveUp => {
                    self.lost(news);
                    return;
                }
                Step::Wait => return,
            }
        }
    }

    fn lost(&mut self, news: &mut Vec<LinkNews>) {
        *self.link.lock().unwrap_or_else(PoisonError::into_inner) = None;
        self.dial = None;
        if self.open {
            news.push(LinkNews::Down);
        }
        self.open = false;
        self.backoff = (self.backoff * 2.0).clamp(0.5, 15.0);
        self.wait = self.backoff;
    }

    /// Once a frame: dials when it should, and reads what came.
    pub(crate) fn pump(
        &mut self,
        dt: f32,
        gateway: &str,
        token: &str,
        mailbox: &Mailbox,
        news: &mut Vec<LinkNews>,
    ) {
        if !self.wanted || self.stopped {
            return;
        }
        if self.dial.is_none() {
            if self.wait > 0.0 {
                self.wait -= dt;
                return;
            }
            let (dial, step) = TicketDial::start();
            self.dial = Some(dial);
            self.perform(step, gateway, token, mailbox, news);
            return;
        }
        let mut failed = false;
        {
            let mut link = self.link.lock().unwrap_or_else(PoisonError::into_inner);
            let Some((_, receiver)) = link.as_mut() else {
                return;
            };
            while let Some(event) = receiver.try_recv() {
                match event {
                    ewebsock::WsEvent::Opened => {
                        if let Some(dial) = self.dial.as_mut() {
                            dial.opened();
                        }
                        self.open = true;
                        self.backoff = 0.0;
                        news.push(LinkNews::Up);
                    }
                    ewebsock::WsEvent::Message(ewebsock::WsMessage::Text(text)) => {
                        news.push(LinkNews::Frame(text));
                    }
                    ewebsock::WsEvent::Message(_) => {}
                    ewebsock::WsEvent::Error(_) | ewebsock::WsEvent::Closed => {
                        failed = true;
                        break;
                    }
                }
            }
        }
        if failed {
            if self.open {
                self.lost(news);
            } else {
                let step = self.dial.as_mut().map_or(Step::GiveUp, TicketDial::failed);
                self.perform(step, gateway, token, mailbox, news);
            }
        }
    }
}
