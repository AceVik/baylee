//! Signing in from the website without the password: the client's half of a
//! pairing (`docs/central-login.md` §"Signing in from the website", the
//! contract in `docs/proposals/deep-links.md` §"Pairing, browser → client").
//!
//! A `baylee://pair` link brings the pairing's handle and the page's
//! one-time secret. [`Account::claim_pairing`] claims it with a key pair and
//! a verifier the SDK makes for it; the claim waits until the page approves
//! ([`PairingClaim::approved`]), and is redeemed once
//! ([`PairingClaim::redeem`]) for a native session and the master key,
//! sealed to the claim's key under the link's secret and opened by the SDK.
//! The player is then asked whom the session signs in as
//! ([`PairedSession::who`]): yes restores the account as a password sign-in
//! does ([`PairedSession::accept`]), anything else ends the session
//! ([`PairedSession::decline`], and dropping a [`PairedSession`] does the
//! same).
//!
//! Nothing here is printed: the secret, the key pair, the verifier and the
//! session are the SDK's and wiped when dropped.
use super::{Account, format_profile_handle};
use crate::{CentralHttp, Endpoint, Error};
use central_client::{PairedAccount, Pairing, PairingSecret, Session};

/// A pairing this client claimed, waiting for the page's approval. Dropping
/// it gives the pairing up: its secrets are wiped, and Central lets it expire.
pub struct PairingClaim {
    http: CentralHttp,
    pairing: Pairing,
}

impl std::fmt::Debug for PairingClaim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PairingClaim([REDACTED])")
    }
}

impl Account {
    /// Claim the pairing a `baylee://pair` link names, at the Central this
    /// client is built for. Sent once: the SDK makes a new key pair for every
    /// claim, so a second claim of the same link is another client's to
    /// Central, and refused.
    ///
    /// # Errors
    /// [`Error::Conflict`] when another client claimed it first;
    /// [`Error::Rejected`] for a pairing that is unknown, ended or past its
    /// two minutes; transport failures. The secret is wiped with the refusal.
    pub fn claim_pairing(
        endpoint: Endpoint,
        handle: &[u8; 32],
        secret: &[u8; 32],
    ) -> Result<PairingClaim, Error> {
        let http = CentralHttp::new(endpoint);
        let secret = PairingSecret::from_bytes(secret).map_err(|_| Error::Rejected)?;
        let pairing = central_client::claim(&http, *handle, secret)?;
        Ok(PairingClaim { http, pairing })
    }
}

impl PairingClaim {
    /// The six characters, `XXX-XXX`, the page shows beside its Approve:
    /// the client shows them while it waits.
    #[must_use]
    pub fn comparison_code(&self) -> String {
        self.pairing.comparison_code()
    }

    /// Whether the page approved yet. Asked about once a second; Central
    /// answers at most 300 times.
    ///
    /// # Errors
    /// [`Error::Rejected`] once the pairing ended (cancelled on the page,
    /// expired, or its polls spent); transport failures, after which asking
    /// again is safe.
    pub fn approved(&self) -> Result<bool, Error> {
        self.pairing.approved(&self.http)
    }

    /// Redeem the approved pairing, once: the session with the master key
    /// opened from the seal. Whatever the redeem finds ends the pairing at
    /// Central, so it is never sent twice.
    ///
    /// # Errors
    /// [`Error::Rejected`] for a pairing not approved or ended, a seal that
    /// does not open under this link's secret (its session is ended), or a
    /// session that cannot read its document; transport failures.
    pub fn redeem(self) -> Result<PairedSession, Error> {
        let (session, account) = self.pairing.redeem(&self.http)?;
        Ok(PairedSession {
            live: Some((self.http, session)),
            account,
        })
    }
}

/// A redeemed pairing: a live session the player has not said yes to.
/// Dropped without [`PairedSession::accept`], it signs the session out.
pub struct PairedSession {
    /// The session and the transport it signs out through, until it is
    /// accepted or ended.
    live: Option<(CentralHttp, Session)>,
    account: PairedAccount,
}

impl std::fmt::Debug for PairedSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PairedSession([REDACTED])")
    }
}

impl PairedSession {
    /// Whom the session signs in as, for the player's question:
    /// `Name#tag (username)`, or the username alone for an account without
    /// a display name or tag.
    #[must_use]
    pub fn who(&self) -> String {
        who(&self.account)
    }

    /// The account's canonical username, its private sign-in name.
    #[must_use]
    pub fn username(&self) -> &str {
        &self.account.username
    }

    /// The account's `Name#tag` (or username) for the lobby's header, as
    /// [`Account::profile_handle`] spells it. Read from the redeem's answer:
    /// the export that `profile_handle` asks for wants a password proof of
    /// the last 30 minutes, and a paired session's proof is the website's.
    #[must_use]
    pub fn profile_handle(&self) -> String {
        let name = self
            .account
            .display_name
            .clone()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| self.account.username.clone());
        format_profile_handle(name, self.account.tag)
    }

    /// The player said yes: the account, restored as a password sign-in
    /// leaves it (document, restore confirmed, terms, a waiting move).
    ///
    /// # Errors
    /// As the restore after [`Account::login`]; the session ends.
    pub fn accept(mut self) -> Result<Account, Error> {
        let (http, session) = self.live.take().ok_or(Error::Rejected)?;
        Account::restore(http, session)
    }

    /// The player said no, or left: the session ends at Central.
    ///
    /// # Errors
    /// Central could not confirm the sign-out; the session's keys are wiped
    /// here all the same, and it expires there.
    pub fn decline(mut self) -> Result<(), Error> {
        match self.live.take() {
            Some((http, session)) => session.logout(&http),
            None => Ok(()),
        }
    }
}

impl Drop for PairedSession {
    fn drop(&mut self) {
        if let Some((http, session)) = self.live.take() {
            let _ = session.logout(&http);
        }
    }
}

/// The question's name for an account: its display name and tag as other
/// players see them, then the username in brackets; the username alone
/// without both.
fn who(account: &PairedAccount) -> String {
    match (&account.display_name, account.tag) {
        (Some(name), Some(tag)) if !name.trim().is_empty() => format!(
            "{} ({})",
            format_profile_handle(name.clone(), Some(tag)),
            account.username
        ),
        _ => account.username.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(display_name: Option<&str>, tag: Option<u32>) -> PairedAccount {
        PairedAccount {
            username: "pairing_player".into(),
            display_name: display_name.map(Into::into),
            tag,
        }
    }

    #[test]
    fn a_paired_account_is_named_in_the_header_as_after_a_password_sign_in() {
        let named = |display_name, tag| PairedSession {
            live: None,
            account: account(display_name, tag),
        };
        assert_eq!(
            named(Some("Pair_Player"), Some(0x2a)).profile_handle(),
            "Pair_Player#002a"
        );
        assert_eq!(named(None, Some(7)).profile_handle(), "pairing_player#0007");
        assert_eq!(named(Some(" "), None).profile_handle(), "pairing_player");
    }

    #[test]
    fn the_question_names_the_account_as_others_see_it_and_its_username() {
        assert_eq!(
            who(&account(Some("Pair_Player"), Some(0x2a))),
            "Pair_Player#002a (pairing_player)"
        );
        // Without a name or a tag, the username alone.
        assert_eq!(who(&account(None, Some(7))), "pairing_player");
        assert_eq!(who(&account(Some("Pair_Player"), None)), "pairing_player");
        assert_eq!(who(&account(Some(" "), Some(7))), "pairing_player");
    }
}
