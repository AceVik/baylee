//! Friend invites at Baylee Central (`docs/client.md` §"Inviting friends"):
//! a single-use link the account hands to someone, who becomes its friend by
//! opening it. Central keeps only a digest of each code, so the code comes
//! back once, when the invite is made ([`CreatedInvite`]), and never in the
//! list.
//!
//! Every answer is checked before the client draws it: Central lists at most
//! 100 invites, each text within Central's own bounds, and a code is a
//! canonical 22-character code ([`social::valid_friend_invite_code`]). The
//! code is wiped when dropped and never printed.
use super::{Account, Error};
use crate::{
    CentralHttp, Endpoint,
    wire::{Request, Response},
};
use central_api::social::{
    self, FriendInvitePreview, FriendInviteStatus, FriendInviteSummary, MAX_INVITE_LABEL,
    MAX_INVITE_NOTE, Person, SocialBadge, SocialRequest, SocialResponse,
};
use zeroize::Zeroizing;

/// The most invites Central lists for one account (its `LIMIT 100`,
/// newest first): a longer answer is refused whole.
pub const LISTED: usize = 100;

/// The longest e-mail address an invite is addressed to (RFC 5321's path
/// limit); Central checks the rest.
pub const MAX_INVITE_EMAIL: usize = 254;

/// The account's invites, as Central lists them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FriendInvites {
    /// Newest first, at most [`LISTED`].
    pub invites: Vec<FriendInviteSummary>,
    /// How many may be open at once.
    pub active_limit: u32,
    /// Whether Central can send an invite by e-mail.
    pub mail_available: bool,
    /// The account's progress as a recruiter, where Central says it.
    pub recruiter: Option<SocialBadge>,
}

/// What a new invite asks for. Empty texts are sent as none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewInvite {
    /// A name only the account sees, to know whom it was for.
    pub label: Option<String>,
    /// A greeting the invited person reads on the invite's page.
    pub note: Option<String>,
    /// Where Central mails the invite, if it should.
    pub email: Option<String>,
    /// The mail's language (`en`, `de`).
    pub lang: String,
    /// How many days the link holds, `1..=14`.
    pub days: u8,
}

/// A just-made invite and its code, the one time Central gives it.
#[derive(Clone, PartialEq, Eq)]
pub struct CreatedInvite {
    /// The invite as the list will show it.
    pub invite: FriendInviteSummary,
    /// The code the link carries; wiped when dropped.
    pub code: Zeroizing<String>,
    /// Central handed the invite to its mailer.
    pub mail_sent: bool,
}

impl std::fmt::Debug for CreatedInvite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreatedInvite")
            .field("invite", &self.invite)
            .field("code", &"[REDACTED]")
            .field("mail_sent", &self.mail_sent)
            .finish()
    }
}

impl NewInvite {
    /// The request, its texts trimmed and checked as Central checks them,
    /// so a text Central would refuse is refused before it is sent.
    fn request(&self) -> Result<SocialRequest, Error> {
        let label = social::clean_invite_text(self.label.as_deref(), MAX_INVITE_LABEL)?;
        let note = social::clean_invite_text(self.note.as_deref(), MAX_INVITE_NOTE)?;
        let email = match self.email.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(address) if plausible_email(address) => Some(address.to_owned()),
            Some(_) => return Err(Error::Rejected),
        };
        let days = social::invite_expiry_days(Some(self.days))?;
        Ok(SocialRequest::CreateFriendInvite {
            label,
            note,
            email,
            lang: Some(self.lang.clone()),
            expires_days: Some(days),
        })
    }
}

/// Whether `address` looks like an e-mail address: one `@` with a name
/// before it and a dotted domain after it, nothing that is space or a
/// control character, and within [`MAX_INVITE_EMAIL`]. Central decides.
#[must_use]
pub fn plausible_email(address: &str) -> bool {
    let Some((name, domain)) = address.split_once('@') else {
        return false;
    };
    address.len() <= MAX_INVITE_EMAIL
        && !name.is_empty()
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !address
            .chars()
            .any(|ch| ch.is_whitespace() || ch.is_control())
}

/// An invite as listed, inside Central's own bounds.
fn checked(invite: &FriendInviteSummary) -> Result<(), Error> {
    let within = |text: &Option<String>, max: usize| {
        text.as_ref().is_none_or(|t| t.chars().count() <= max)
    };
    let redeemed = matches!(invite.status, FriendInviteStatus::Redeemed);
    if within(&invite.label, MAX_INVITE_LABEL)
        && within(&invite.note, MAX_INVITE_NOTE)
        && within(&invite.invited_email, MAX_INVITE_EMAIL)
        && invite.expires_at >= invite.created_at
        && (redeemed || invite.redeemed_by.is_none())
    {
        Ok(())
    } else {
        Err(Error::Rejected)
    }
}

/// The list Central answered, checked.
fn listed(response: SocialResponse) -> Result<FriendInvites, Error> {
    let SocialResponse::FriendInvites {
        invites,
        active_limit,
        mail_available,
        recruiter_badge,
    } = response
    else {
        return Err(Error::Rejected);
    };
    if invites.len() > LISTED || active_limit as usize > LISTED {
        return Err(Error::Rejected);
    }
    invites.iter().try_for_each(checked)?;
    Ok(FriendInvites {
        invites,
        active_limit,
        mail_available,
        recruiter: recruiter_badge,
    })
}

/// The invite Central made, checked: its code canonical.
fn created(response: SocialResponse) -> Result<CreatedInvite, Error> {
    let SocialResponse::FriendInviteCreated {
        invite,
        code,
        mail_sent,
    } = response
    else {
        return Err(Error::Rejected);
    };
    let code = Zeroizing::new(code);
    if !social::valid_friend_invite_code(&code) {
        return Err(Error::Rejected);
    }
    checked(&invite)?;
    Ok(CreatedInvite {
        invite,
        code,
        mail_sent,
    })
}

impl Account {
    /// An invite's public preview, read from this account's Central
    /// ([`friend_invite_info`]): who made it, before it is taken up.
    ///
    /// # Errors
    /// As [`friend_invite_info`].
    pub fn friend_invite_preview(&self, code: &str) -> Result<FriendInvitePreview, Error> {
        preview(&self.http, code)
    }

    /// The account's invites, newest first.
    ///
    /// # Errors
    /// A rejected session, `TermsRequired`, `Busy`, transport failures and
    /// an answer outside its bounds.
    pub fn friend_invites(&self) -> Result<FriendInvites, Error> {
        listed(self.social(SocialRequest::FriendInvites)?)
    }

    /// Makes a single-use invite and, with an address, has Central mail it.
    ///
    /// # Errors
    /// A text Central would refuse (refused here, before anything is sent),
    /// `Busy` when the account's open invites are at their limit or past
    /// its request budget, and the errors of [`Account::friend_invites`].
    pub fn create_friend_invite(&self, ask: &NewInvite) -> Result<CreatedInvite, Error> {
        created(self.social(ask.request()?)?)
    }

    /// Withdraws an open invite: its link no longer works.
    ///
    /// # Errors
    /// `Rejected` for an invite that is not the account's or not open, and
    /// the errors of [`Account::friend_invites`].
    pub fn revoke_friend_invite(&self, invite: [u8; 16]) -> Result<(), Error> {
        match self.social(SocialRequest::RevokeFriendInvite { invite })? {
            SocialResponse::Done => Ok(()),
            _ => Err(Error::Rejected),
        }
    }

    /// Takes up an invite: the account and the one who made it become
    /// friends. Answers who that is.
    ///
    /// # Errors
    /// `Rejected` for a code that is not canonical (refused here), used,
    /// expired, withdrawn, the account's own or between blocked accounts;
    /// `Busy` at the friend limit; the errors of [`Account::friend_invites`].
    pub fn redeem_friend_invite(&self, code: &str, lang: &str) -> Result<Person, Error> {
        if !social::valid_friend_invite_code(code) {
            return Err(Error::Rejected);
        }
        match self.social(SocialRequest::RedeemFriendInvite {
            code: code.to_owned(),
            link_email: false,
            lang: Some(lang.to_owned()),
        })? {
            SocialResponse::FriendInviteRedeemed { inviter, .. } => Ok(inviter),
            _ => Err(Error::Rejected),
        }
    }
}

/// Reads an invite's public preview, as its page does without signing in:
/// who made it, its note while it is open, its state and when it ends.
/// Central never puts the invited address into it.
///
/// # Errors
/// `Rejected` for a code that is not canonical (refused here) or that
/// Central does not know, an answer of another kind or outside its bounds,
/// and transport failures.
pub fn friend_invite_info(endpoint: &Endpoint, code: &str) -> Result<FriendInvitePreview, Error> {
    preview(&CentralHttp::new(endpoint.clone()), code)
}

fn preview(http: &CentralHttp, code: &str) -> Result<FriendInvitePreview, Error> {
    if !social::valid_friend_invite_code(code) {
        return Err(Error::Rejected);
    }
    let Response::FriendInviteInfo { invite } = http.exchange(
        &Request::FriendInviteInfo {
            code: code.to_owned(),
        },
        None,
    )?
    else {
        return Err(Error::Rejected);
    };
    let note_fits = invite
        .note
        .as_ref()
        .is_none_or(|t| t.chars().count() <= MAX_INVITE_NOTE);
    if !note_fits || invite.expires_at < invite.created_at {
        return Err(Error::Rejected);
    }
    Ok(invite)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(status: FriendInviteStatus) -> FriendInviteSummary {
        FriendInviteSummary {
            id: [7; 16],
            label: Some("Mara".into()),
            note: None,
            invited_email: None,
            status,
            created_at: 1_000,
            expires_at: 2_000,
            redeemed_at: None,
            redeemed_by: None,
        }
    }

    fn ask() -> NewInvite {
        NewInvite {
            label: None,
            note: None,
            email: None,
            lang: "de".into(),
            days: 7,
        }
    }

    #[test]
    fn a_new_invite_sends_trimmed_texts_and_none_for_empty_ones() {
        let request = NewInvite {
            label: Some("  Mara  ".into()),
            note: Some("   ".into()),
            email: Some(" mara@example.org ".into()),
            ..ask()
        }
        .request()
        .unwrap();
        let SocialRequest::CreateFriendInvite {
            label,
            note,
            email,
            lang,
            expires_days,
        } = request
        else {
            panic!("a new invite is a create request");
        };
        assert_eq!(label.as_deref(), Some("Mara"));
        assert_eq!(note, None);
        assert_eq!(email.as_deref(), Some("mara@example.org"));
        assert_eq!(lang.as_deref(), Some("de"));
        assert_eq!(expires_days, Some(7));
    }

    #[test]
    fn a_new_invite_central_would_refuse_is_refused_before_it_is_sent() {
        for bad in [
            NewInvite { days: 0, ..ask() },
            NewInvite { days: 15, ..ask() },
            NewInvite {
                label: Some("x".repeat(MAX_INVITE_LABEL + 1)),
                ..ask()
            },
            NewInvite {
                note: Some("hi\u{202e}there".into()),
                ..ask()
            },
            NewInvite {
                email: Some("not an address".into()),
                ..ask()
            },
        ] {
            assert!(matches!(bad.request(), Err(Error::Rejected)), "{bad:?}");
        }
    }

    #[test]
    fn an_address_is_plausible_only_with_a_name_and_a_dotted_domain() {
        assert!(plausible_email("a@b.de"));
        assert!(plausible_email("mara.k+baylee@mail.example.org"));
        for bad in [
            "",
            "a",
            "@b.de",
            "a@b",
            "a@.de",
            "a@b.",
            "a@@b.de",
            "a b@c.de",
            "a@b.de\n",
        ] {
            assert!(!plausible_email(bad), "{bad:?}");
        }
        assert!(!plausible_email(&format!("{}@b.de", "a".repeat(250))));
    }

    #[test]
    fn a_list_outside_its_bounds_is_refused_whole() {
        let ok = SocialResponse::FriendInvites {
            invites: vec![summary(FriendInviteStatus::Active)],
            active_limit: 10,
            mail_available: true,
            recruiter_badge: None,
        };
        assert_eq!(listed(ok).unwrap().invites.len(), 1);
        let long = SocialResponse::FriendInvites {
            invites: vec![summary(FriendInviteStatus::Active); LISTED + 1],
            active_limit: 10,
            mail_available: false,
            recruiter_badge: None,
        };
        assert_eq!(listed(long), Err(Error::Rejected));
        let mut stranger = summary(FriendInviteStatus::Active);
        stranger.redeemed_by = Some(Person {
            tag: 1,
            display_name: "x".into(),
        });
        let odd = SocialResponse::FriendInvites {
            invites: vec![stranger],
            active_limit: 10,
            mail_available: false,
            recruiter_badge: None,
        };
        assert_eq!(listed(odd), Err(Error::Rejected));
        assert_eq!(listed(SocialResponse::Done), Err(Error::Rejected));
    }

    #[test]
    fn a_made_invite_needs_a_canonical_code_and_never_prints_it() {
        let code = "AAAAAAAAAAAAAAAAAAAAAA".to_owned();
        let made = created(SocialResponse::FriendInviteCreated {
            invite: summary(FriendInviteStatus::Active),
            code: code.clone(),
            mail_sent: false,
        })
        .unwrap();
        assert_eq!(made.code.as_str(), code);
        assert!(!format!("{made:?}").contains(&code));
        for bad in ["AAAAAAAAAAAAAAAAAAAAAB", "short", "AAAAAAAAAAAAAAAAAAAAA=="] {
            assert_eq!(
                created(SocialResponse::FriendInviteCreated {
                    invite: summary(FriendInviteStatus::Active),
                    code: bad.into(),
                    mail_sent: false,
                }),
                Err(Error::Rejected),
                "{bad}"
            );
        }
    }
}
