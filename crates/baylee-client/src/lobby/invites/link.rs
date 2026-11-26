//! A friend invite from a link (`baylee://invite/<code>`, `docs/client.md`
//! §"Taking up an invite from a link"): the player opened someone's invite
//! on the website, or anywhere, and the client takes it up.
//!
//! Never on arrival. Signed in at Baylee Central, the lobby asks Central
//! who made the invite (its public preview, the one the invite's page
//! reads), and then asks the player, naming that person: "Accept" takes
//! the invite up, and the two are friends. "Not now", `Esc` and a tap
//! beside the panel say no, and no key says yes. An invite that is used,
//! withdrawn, expired, the player's own or from a friend already is said in
//! one sentence, and nothing is asked or spent. Signed out, Central's
//! sign-in comes first (`links`); anywhere else (a guest, a gateway's own
//! account, offline) the player is told that it takes a Baylee account.
//!
//! The code is a bearer: whoever holds it can take the invite up. It is
//! wiped when dropped ([`InviteLink`]), goes to Central and nowhere else,
//! and neither the log nor dev-control's `/state` ever shows it.

#[allow(clippy::wildcard_imports)] // the lobby widget vocabulary
use super::super::*;
use crate::social::SocialHost;
use baylee_account::Error;
use baylee_account::social::{FriendInvitePreview, FriendInviteStatus, Person};
use baylee_client_core::deeplink::InviteLink;
use baylee_client_core::social::invites;
use std::sync::{Mutex, mpsc};

/// An invite asked about: its link and what Central said of it.
#[derive(Clone, Debug)]
pub(crate) struct Asked {
    /// The link, whose code takes the invite up.
    pub(crate) link: InviteLink,
    /// Who made it, its greeting and until when it holds.
    pub(crate) preview: FriendInvitePreview,
}

impl Asked {
    /// Who made the invite, as the client names people.
    pub(crate) fn inviter(&self) -> String {
        invites::handle(&self.preview.inviter)
    }
}

/// What the lobby waits for from Central.
#[derive(Default)]
pub(crate) struct Flow {
    /// The preview asked for, on the account worker.
    looking: Option<Mutex<mpsc::Receiver<(InviteLink, Result<FriendInvitePreview, Error>)>>>,
    /// The invite being taken up.
    taking: Option<Mutex<mpsc::Receiver<Result<Person, Error>>>>,
}

impl Flow {
    /// Whether the invite is being taken up: the question stands, its
    /// Accept held, until Central answers.
    pub(crate) fn taking(&self) -> bool {
        self.taking.is_some()
    }
}

/// Asks Central who made the invite. Answers whether the link is done
/// with (always: a link that could not be asked about is told and dropped).
pub(in crate::lobby) fn look(state: &mut LobbyState, link: InviteLink) -> bool {
    let Some(handle) = state.central_handle() else {
        state.lobby.tell_refusal(Phrase::InviteLinkSignIn, &[]);
        return true;
    };
    let (send, receive) = mpsc::sync_channel(1);
    let asked = handle.submit(move |account| {
        let said = account.friend_invite_preview(link.code());
        let _ = send.send((link, said));
    });
    if asked {
        state.invite_link.looking = Some(Mutex::new(receive));
    } else {
        state.lobby.tell_refusal(Phrase::InviteLinkFailed, &[]);
    }
    true
}

/// Takes in Central's answers: the preview, once the lobby may ask a
/// question (`may_ask`), and the outcome of taking an invite up, at once.
/// A frame with nothing answered writes nothing.
pub(in crate::lobby) fn answers(
    state: &mut ResMut<LobbyState>,
    host: Option<&SocialHost>,
    may_ask: bool,
) {
    // A worker that stopped (a sign-out) dropped its sender: that is an
    // answer too, so nothing waits for ever.
    if let Some(taken) = state.invite_link.taking.as_ref().and_then(delivered) {
        state.invite_link.taking = None;
        took(state, taken.unwrap_or(Err(Error::Unavailable)));
    }
    if !may_ask {
        return;
    }
    if let Some(looked) = state.invite_link.looking.as_ref().and_then(delivered) {
        state.invite_link.looking = None;
        let Some((link, preview)) = looked else {
            state.lobby.tell_refusal(Phrase::InviteLinkFailed, &[]);
            return;
        };
        let mine = own_tag(state);
        let friends = friend_tags(host);
        match decide(link, preview, mine.as_deref(), &friends, now()) {
            Decision::Ask(asked) => {
                state.confirmation = Some(confirm::Destructive::FriendInvite(asked));
            }
            Decision::Tell(phrase, player) => {
                state
                    .lobby
                    .tell_refusal(phrase, &[player.as_deref().unwrap_or("")]);
            }
        }
    }
}

/// What a worker sent, `Some(None)` once it is gone without an answer, and
/// `None` while it is still at work.
fn delivered<T>(receive: &Mutex<mpsc::Receiver<T>>) -> Option<Option<T>> {
    match receive
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .try_recv()
    {
        Ok(answer) => Some(Some(answer)),
        Err(mpsc::TryRecvError::Empty) => None,
        Err(mpsc::TryRecvError::Disconnected) => Some(None),
    }
}

/// "Accept": the invite is taken up at Central. The question stands, its
/// Accept held, until Central answers ([`answers`]).
pub(in crate::lobby) fn take(state: &mut LobbyState) {
    if state.invite_link.taking() {
        return;
    }
    let Some(confirm::Destructive::FriendInvite(asked)) = state.confirmation.as_ref() else {
        return;
    };
    let Some(handle) = state.central_handle() else {
        state.confirmation = None;
        state.lobby.tell_refusal(Phrase::InviteLinkSignIn, &[]);
        return;
    };
    let link = asked.link.clone();
    let lang = state.lobby.lang().code();
    let (send, receive) = mpsc::sync_channel(1);
    let asked = handle.submit(move |account| {
        let _ = send.send(account.redeem_friend_invite(link.code(), lang));
    });
    if asked {
        state.invite_link.taking = Some(Mutex::new(receive));
    } else {
        state.confirmation = None;
        state.lobby.tell_refusal(Phrase::InviteLinkFailed, &[]);
    }
}

/// Central answered the taking up: the question goes, and one sentence
/// says how it ended.
fn took(state: &mut LobbyState, taken: Result<Person, Error>) {
    if matches!(
        state.confirmation,
        Some(confirm::Destructive::FriendInvite(_))
    ) {
        state.confirmation = None;
    }
    match taken {
        Ok(inviter) => {
            state
                .lobby
                .tell(Phrase::InviteLinkFriends, &[&invites::handle(&inviter)]);
        }
        // Used, withdrawn or expired since the preview, the player's own,
        // or between blocked accounts: Central says no more than that.
        Err(Error::Rejected) => state.lobby.tell_refusal(Phrase::InviteLinkUnknown, &[]),
        Err(_) => state.lobby.tell_refusal(Phrase::InviteLinkFailed, &[]),
    }
}

/// What the preview leads to.
#[derive(Debug)]
enum Decision {
    /// The question, naming the inviter.
    Ask(Box<Asked>),
    /// One sentence, with the inviter where it names them; nothing asked.
    Tell(Phrase, Option<String>),
}

/// What Central's preview leads to: the question only for an open invite
/// of someone else who is not a friend yet. `mine` is the signed-in
/// account's tag, `friends` its friends' (both as handles spell them),
/// `now` the wall clock in Unix seconds.
fn decide(
    link: InviteLink,
    preview: Result<FriendInvitePreview, Error>,
    mine: Option<&str>,
    friends: &[String],
    now: i64,
) -> Decision {
    let preview = match preview {
        Ok(preview) => preview,
        Err(Error::Rejected) => return Decision::Tell(Phrase::InviteLinkUnknown, None),
        Err(_) => return Decision::Tell(Phrase::InviteLinkFailed, None),
    };
    let tag = invites::tag(&preview.inviter);
    let inviter = invites::handle(&preview.inviter);
    match preview.status {
        FriendInviteStatus::Redeemed => Decision::Tell(Phrase::InviteLinkUsed, None),
        FriendInviteStatus::Revoked => Decision::Tell(Phrase::InviteLinkWithdrawn, None),
        FriendInviteStatus::Expired => Decision::Tell(Phrase::InviteLinkExpired, None),
        FriendInviteStatus::Active if preview.expires_at <= now => {
            Decision::Tell(Phrase::InviteLinkExpired, None)
        }
        FriendInviteStatus::Active if mine == Some(tag.as_str()) => {
            Decision::Tell(Phrase::InviteLinkOwn, None)
        }
        FriendInviteStatus::Active if friends.contains(&tag) => {
            Decision::Tell(Phrase::InviteLinkAlready, Some(inviter))
        }
        FriendInviteStatus::Active => Decision::Ask(Box::new(Asked { link, preview })),
    }
}

/// The signed-in Central account's tag, as its handle spells it.
fn own_tag(state: &LobbyState) -> Option<String> {
    let handle = state.central.as_ref()?.profile_handle.as_deref()?;
    baylee_client_core::social::tag_of(handle).map(str::to_owned)
}

/// The tags of the account's friends, as the chat knows them.
fn friend_tags(host: Option<&SocialHost>) -> Vec<String> {
    host.and_then(SocialHost::machine)
        .map(|machine| {
            machine
                .contacts()
                .friends
                .iter()
                .map(|friend| friend.person.tag.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// The wall clock in Unix seconds.
fn now() -> i64 {
    i64::try_from(crate::social::now_ms() / 1000).unwrap_or(i64::MAX)
}

/// The local offset from UTC, in seconds, now: the question says until
/// when the invite holds in the player's own time.
pub(in crate::lobby) fn local_offset() -> i32 {
    time::OffsetDateTime::from_unix_timestamp(now())
        .ok()
        .and_then(|utc| time::UtcOffset::local_offset_at(utc).ok())
        .map_or(0, time::UtcOffset::whole_seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::deeplink::{Link, Origins, read};

    const CODE: &str = "Zm9vYmFyLWJheWxlZS0xMA";
    const NOW: i64 = 1_796_000_000;

    fn link() -> InviteLink {
        let Some(Link::Invite(link)) = read(&format!("baylee://invite/{CODE}"), Origins::Public)
        else {
            panic!("an invite link");
        };
        link
    }

    fn preview(status: FriendInviteStatus) -> FriendInvitePreview {
        FriendInvitePreview {
            registration_open: true,
            invite_only: false,
            inviter: Person {
                tag: 0x2a,
                display_name: "Mara".into(),
            },
            note: Some("Komm spielen!".into()),
            invited_email: None,
            status,
            created_at: NOW - 3_600,
            expires_at: NOW + 86_400,
            redeemed_by: None,
        }
    }

    fn told(decision: Decision) -> Phrase {
        match decision {
            Decision::Tell(phrase, _) => phrase,
            Decision::Ask(_) => panic!("asked"),
        }
    }

    #[test]
    fn an_open_invite_of_someone_else_is_asked_about_naming_them() {
        let Decision::Ask(asked) = decide(
            link(),
            Ok(preview(FriendInviteStatus::Active)),
            Some("0001"),
            &["0002".into()],
            NOW,
        ) else {
            panic!("asked");
        };
        assert_eq!(asked.inviter(), "Mara#002a");
        assert_eq!(asked.link.code(), CODE);
        assert!(
            !format!("{asked:?}").contains(CODE),
            "the code is never printed"
        );
    }

    #[test]
    fn an_invite_that_cannot_be_taken_up_is_told_and_nothing_is_asked() {
        use FriendInviteStatus::*;
        let decided = |status, mine: Option<&str>, friends: &[String], now| {
            told(decide(link(), Ok(preview(status)), mine, friends, now))
        };
        assert_eq!(decided(Redeemed, None, &[], NOW), Phrase::InviteLinkUsed);
        assert_eq!(
            decided(Revoked, None, &[], NOW),
            Phrase::InviteLinkWithdrawn
        );
        assert_eq!(decided(Expired, None, &[], NOW), Phrase::InviteLinkExpired);
        assert_eq!(
            decided(Active, None, &[], NOW + 86_400),
            Phrase::InviteLinkExpired,
            "past its end, whatever Central still says"
        );
        assert_eq!(
            decided(Active, Some("002a"), &[], NOW),
            Phrase::InviteLinkOwn
        );
        assert_eq!(
            decided(Active, None, &["002a".into()], NOW),
            Phrase::InviteLinkAlready
        );
        assert_eq!(
            told(decide(link(), Err(Error::Rejected), None, &[], NOW)),
            Phrase::InviteLinkUnknown
        );
        assert_eq!(
            told(decide(link(), Err(Error::Unavailable), None, &[], NOW)),
            Phrase::InviteLinkFailed
        );
    }
}
