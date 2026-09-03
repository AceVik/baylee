//! Confirmation mail, and the rule that makes it optional.
//!
//! A gateway with no SMTP configured is not a broken gateway: it is the
//! development default, and it has to keep working exactly as it did before
//! this module existed. So [`Mailer`] has an `Off` arm, and the whole
//! confirmation flow keys off whether the mailer is configured — an
//! unconfigured gateway confirms an account the moment it is created and
//! never sends anything.
//!
//! The reverse is the part worth being careful about: once mail *is*
//! configured, an unconfirmed account cannot log in. Sending has to fail
//! loudly enough to be seen in the log and quietly enough not to hand a
//! stranger an oracle for which addresses exist — so a send failure is logged
//! and the HTTP answer is the same either way.

use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

/// Where confirmation mail goes, if anywhere.
#[derive(Clone)]
pub enum Mailer {
    /// No SMTP configured. Nothing is sent and nothing is required.
    Off,
    /// A configured SMTP relay.
    Smtp(Box<Smtp>),
}

/// Everything a configured mailer needs.
pub struct Smtp {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    /// The `From:` address, e.g. `baylee <no-reply@example.com>`.
    from: String,
    /// The base a confirmation link is built on, without a trailing slash.
    base_url: String,
}

impl Clone for Smtp {
    fn clone(&self) -> Self {
        Self {
            transport: self.transport.clone(),
            from: self.from.clone(),
            base_url: self.base_url.clone(),
        }
    }
}

impl Mailer {
    /// Reads the environment.
    ///
    /// `BAYLEE_SMTP_URL` decides everything: without it the mailer is `Off`.
    /// The URL is lettre's own (`smtps://user:pass@host:465`, or
    /// `smtp://host:1025` for a local catcher), so STARTTLS, implicit TLS and
    /// a plaintext dev relay are all spelled in one place rather than as
    /// three environment variables that can disagree.
    ///
    /// `BAYLEE_MAIL_FROM` is the sender and `BAYLEE_PUBLIC_URL` is what a
    /// confirmation link points at — the gateway cannot know its own public
    /// address, and guessing it from a request header is how a link ends up
    /// pointing at whatever `Host:` an attacker sent.
    #[must_use]
    pub fn from_env() -> Self {
        let Ok(url) = std::env::var("BAYLEE_SMTP_URL") else {
            return Self::Off;
        };
        if url.is_empty() {
            return Self::Off;
        }
        // One url covers all three shapes: `smtps://` for implicit TLS,
        // `smtp://` with `tls=required` for STARTTLS, and a bare `smtp://`
        // for a local catcher (mailpit, maildev) that has no certificate.
        // Credentials ride in the userinfo, so a password with an `@` in it
        // has to be percent-escaped there.
        match AsyncSmtpTransport::<Tokio1Executor>::from_url(&url)
            .map(lettre::transport::smtp::AsyncSmtpTransportBuilder::build)
        {
            Ok(transport) => Self::Smtp(Box::new(Smtp {
                transport,
                from: std::env::var("BAYLEE_MAIL_FROM")
                    .unwrap_or_else(|_| "baylee <no-reply@localhost>".to_string()),
                base_url: std::env::var("BAYLEE_PUBLIC_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:28766".to_string())
                    .trim_end_matches('/')
                    .to_string(),
            })),
            Err(error) => {
                tracing::error!(%error, "BAYLEE_SMTP_URL is not a usable SMTP url; mail is off");
                Self::Off
            }
        }
    }

    /// Whether an account has to confirm its address before it may log in.
    #[must_use]
    pub const fn required(&self) -> bool {
        matches!(self, Self::Smtp(_))
    }

    /// Sends the confirmation mail. Errors are logged, never returned to the
    /// caller: the HTTP answer must not differ by whether the address exists
    /// or whether the relay happened to be up.
    pub async fn send_confirmation(&self, to: &str, display_name: &str, lang: &str, token: &str) {
        let Self::Smtp(smtp) = self else {
            return;
        };
        let link = format!("{}/auth/confirm?token={token}", smtp.base_url);
        let (subject, body) = confirmation_text(lang, display_name, &link);
        let built = Message::builder()
            .from(match smtp.from.parse() {
                Ok(from) => from,
                Err(error) => {
                    tracing::error!(%error, "BAYLEE_MAIL_FROM is not an address");
                    return;
                }
            })
            .to(match to.parse() {
                Ok(to) => to,
                Err(error) => {
                    tracing::warn!(%error, "not sending to an unparseable address");
                    return;
                }
            })
            .subject(subject)
            .body(body);
        let message = match built {
            Ok(message) => message,
            Err(error) => {
                tracing::error!(%error, "could not build the confirmation mail");
                return;
            }
        };
        if let Err(error) = smtp.transport.send(message).await {
            tracing::error!(%error, "could not send the confirmation mail");
        }
    }
}

/// The mail itself, in the language the account registered in.
///
/// A table here rather than the client's `Phrase` enum, on purpose: this is
/// the one place the *server* speaks to a player, `baylee-client-core` is a
/// wasm-targeted UI crate the gateway has no business linking, and the two
/// sets of sentences have nothing in common beyond being prose.
#[must_use]
pub fn confirmation_text(lang: &str, display_name: &str, link: &str) -> (String, String) {
    if lang.split(['-', '_']).next().unwrap_or_default() == "de" {
        (
            "Bestätige deine E-Mail-Adresse".to_string(),
            format!(
                "Hallo {display_name},\n\n\
                 bitte bestätige deine E-Mail-Adresse, um dich anmelden zu können:\n\n\
                 {link}\n\n\
                 Der Link gilt 24 Stunden. Wenn du dich nicht registriert hast, \
                 kannst du diese Nachricht ignorieren.\n"
            ),
        )
    } else {
        (
            "Confirm your e-mail address".to_string(),
            format!(
                "Hello {display_name},\n\n\
                 please confirm your e-mail address so you can sign in:\n\n\
                 {link}\n\n\
                 The link is good for 24 hours. If you did not register, \
                 you can ignore this message.\n"
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The development default has to stay the development default: no SMTP
    /// url, no mail, and — the load-bearing half — no confirmation required,
    /// so every account that could log in before this module existed still
    /// can.
    #[test]
    fn an_unconfigured_gateway_requires_no_confirmation() {
        assert!(!Mailer::Off.required());
    }

    #[test]
    fn the_mail_is_written_in_the_language_it_was_asked_for() {
        let (subject, body) = confirmation_text("de", "Viktor", "https://x/y");
        assert_eq!(subject, "Bestätige deine E-Mail-Adresse");
        assert!(body.contains("https://x/y"), "{body}");
        assert!(body.contains("Viktor"), "{body}");

        let (subject, _) = confirmation_text("en-GB", "Viktor", "https://x/y");
        assert_eq!(subject, "Confirm your e-mail address");
        // Anything unrecognised is English rather than an error, the same
        // rule the client's `Lang::of` follows.
        let (fallback, _) = confirmation_text("xx", "Viktor", "https://x/y");
        assert_eq!(subject, fallback);
    }

    /// An unusable url must not take the gateway down with it: `from_env`
    /// logs and falls back to `Off`, which is the same state as no url at
    /// all. Asserted without touching the environment — `set_var` is `unsafe`
    /// in this edition and the crate forbids unsafe outright.
    #[test]
    fn a_url_that_is_not_an_smtp_url_is_refused_rather_than_panicking() {
        assert!(AsyncSmtpTransport::<Tokio1Executor>::from_url("not a url").is_err());
        assert!(AsyncSmtpTransport::<Tokio1Executor>::from_url("smtp://127.0.0.1:1025").is_ok());
    }
}
