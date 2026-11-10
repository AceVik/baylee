//! Table invites (§11.7), in memory: ten minutes each, a pointer for
//! `watch` and a stand-in for the room's password for `join`.

use baylee_protocol::social::{
    Down, InviteAnswer, InviteKind, Person, TABLE_INVITE_SECS, TableBrief, TableInvite,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// How long an invite stands.
pub(crate) const LIFETIME: Duration = Duration::from_secs(TABLE_INVITE_SECS as u64);

/// One open invite.
#[derive(Clone, Debug)]
pub(crate) struct Invite {
    pub(crate) id: String,
    /// The inviter's account.
    pub(crate) from: String,
    pub(crate) from_person: Person,
    /// The invitee's account.
    pub(crate) to: String,
    pub(crate) game: String,
    pub(crate) kind: InviteKind,
    pub(crate) table: TableBrief,
    pub(crate) issued: Instant,
}

impl Invite {
    /// The invite as its invitee is sent it, at `now`.
    pub(crate) fn wire(&self, now: Instant) -> TableInvite {
        let left = LIFETIME.saturating_sub(now.saturating_duration_since(self.issued));
        TableInvite {
            id: self.id.clone(),
            from: self.from_person.clone(),
            game: self.game.clone(),
            kind: self.kind,
            table: self.table.clone(),
            expires_in: u32::try_from(left.as_secs()).unwrap_or(0),
        }
    }

    fn gone(&self, answer: Option<InviteAnswer>) -> Down {
        Down::InviteGone {
            id: self.id.clone(),
            answer,
        }
    }

    fn live(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.issued) < LIFETIME
    }
}

/// The open invites of this gateway.
#[derive(Default)]
pub(crate) struct Invites {
    live: HashMap<String, Invite>,
}

/// Who to tell what, after a change: `(account, frame)`.
pub(crate) type Tell = Vec<(String, Down)>;

impl Invites {
    /// Stores `invite`; one invite from one account to another for one
    /// game replaces the one before.
    pub(crate) fn add(&mut self, invite: Invite) -> Tell {
        let mut tell = Tell::new();
        let before: Vec<String> = self
            .live
            .values()
            .filter(|i| i.from == invite.from && i.to == invite.to && i.game == invite.game)
            .map(|i| i.id.clone())
            .collect();
        for id in before {
            if let Some(old) = self.live.remove(&id) {
                tell.push((old.to.clone(), old.gone(None)));
            }
        }
        self.live.insert(invite.id.clone(), invite);
        tell
    }

    /// The open invites to `account`, oldest first.
    pub(crate) fn open_for(&self, account: &str, now: Instant) -> Vec<TableInvite> {
        let mut open: Vec<&Invite> = self
            .live
            .values()
            .filter(|i| i.to == account && i.live(now))
            .collect();
        open.sort_by_key(|i| i.issued);
        open.into_iter().map(|i| i.wire(now)).collect()
    }

    /// Whether `id` is a live `join` invite for `account` to `game`, which
    /// stands in for the room's password.
    pub(crate) fn fits(&self, id: &str, account: &str, game: &str, now: Instant) -> bool {
        self.live.get(id).is_some_and(|i| {
            i.to == account && i.game == game && i.kind == InviteKind::Join && i.live(now)
        })
    }

    /// Spends `id` on `account`'s join of `game`: answers the invite, and
    /// who to tell, when it is a live `join` invite to this account and
    /// this game.
    pub(crate) fn spend(
        &mut self,
        id: &str,
        account: &str,
        game: &str,
        now: Instant,
    ) -> Option<(Invite, Tell)> {
        let fits = self.live.get(id).is_some_and(|i| {
            i.to == account && i.game == game && i.kind == InviteKind::Join && i.live(now)
        });
        if !fits {
            return None;
        }
        let invite = self.live.remove(id)?;
        let tell = vec![
            (
                invite.from.clone(),
                invite.gone(Some(InviteAnswer::Accepted)),
            ),
            (invite.to.clone(), invite.gone(Some(InviteAnswer::Accepted))),
        ];
        Some((invite, tell))
    }

    /// Spends any live `join` invite for `account` to `game` (starting with
    /// `id` when named): answers the invite(s) `Accepted`, and who to tell.
    pub(crate) fn spend_joined(
        &mut self,
        id: Option<&str>,
        account: &str,
        game: &str,
        now: Instant,
    ) -> Tell {
        let mut tell = Tell::new();
        if let Some(id) = id
            && let Some((_, mut one)) = self.spend(id, account, game, now)
        {
            tell.append(&mut one);
        }
        let matching: Vec<String> = self
            .live
            .values()
            .filter(|i| {
                i.to == account && i.game == game && i.kind == InviteKind::Join && i.live(now)
            })
            .map(|i| i.id.clone())
            .collect();
        for inv_id in matching {
            if let Some((_, mut one)) = self.spend(&inv_id, account, game, now) {
                tell.append(&mut one);
            }
        }
        tell
    }

    /// Drops every `join` invite to `game` once the room has started.
    pub(crate) fn close_started(&mut self, game: &str) -> Tell {
        let started: Vec<String> = self
            .live
            .values()
            .filter(|i| i.game == game && i.kind == InviteKind::Join)
            .map(|i| i.id.clone())
            .collect();
        let mut tell = Tell::new();
        for id in started {
            if let Some(invite) = self.live.remove(&id) {
                tell.push((invite.from.clone(), invite.gone(None)));
                tell.push((invite.to.clone(), invite.gone(None)));
            }
        }
        tell
    }

    /// `caller` closes invite `id`: its invitee declines (`declined`) or
    /// lets it go, or its inviter withdraws it. `None` when `caller` is
    /// neither, or there is no such invite.
    pub(crate) fn close(&mut self, id: &str, caller: &str, declined: bool) -> Option<Tell> {
        let invite = self.live.get(id)?;
        if invite.to != caller && invite.from != caller {
            return None;
        }
        let invite = self.live.remove(id)?;
        let answer = (declined && invite.to == caller).then_some(InviteAnswer::Declined);
        Some(vec![
            (invite.from.clone(), invite.gone(answer)),
            (invite.to.clone(), invite.gone(answer)),
        ])
    }

    /// Drops the invites that lapsed by `now`.
    pub(crate) fn sweep(&mut self, now: Instant) -> Tell {
        let lapsed: Vec<String> = self
            .live
            .values()
            .filter(|i| !i.live(now))
            .map(|i| i.id.clone())
            .collect();
        let mut tell = Tell::new();
        for id in lapsed {
            if let Some(invite) = self.live.remove(&id) {
                tell.push((invite.from.clone(), invite.gone(None)));
                tell.push((invite.to.clone(), invite.gone(None)));
            }
        }
        tell
    }

    /// Drops every invite from or to `accounts` (deleted).
    pub(crate) fn forget(&mut self, accounts: &[String]) -> Tell {
        let gone: Vec<String> = self
            .live
            .values()
            .filter(|i| accounts.contains(&i.from) || accounts.contains(&i.to))
            .map(|i| i.id.clone())
            .collect();
        let mut tell = Tell::new();
        for id in gone {
            if let Some(invite) = self.live.remove(&id) {
                for account in [&invite.from, &invite.to] {
                    if !accounts.contains(account) {
                        tell.push((account.clone(), invite.gone(None)));
                    }
                }
            }
        }
        tell
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An invite `id` from `from` to `to` for `game`, issued at `issued`.
    fn invite(
        id: &str,
        (from, to): (&str, &str),
        game: &str,
        kind: InviteKind,
        issued: Instant,
    ) -> Invite {
        Invite {
            id: id.to_owned(),
            from: from.to_owned(),
            from_person: Person {
                tag: from.to_owned(),
                handle: format!("{from}#0000"),
                guest: false,
            },
            to: to.to_owned(),
            game: game.to_owned(),
            kind,
            table: TableBrief {
                name: game.to_owned(),
                seats: 2,
                taken: 1,
                state: "waiting".to_owned(),
                locked: true,
            },
            issued,
        }
    }

    /// `me` invited to `room` twice to sit down and once to watch, `other`
    /// invited to it as well, and `me` invited to another room.
    fn open(issued: Instant) -> Invites {
        let all = [
            invite("mine", ("host", "me"), "room", InviteKind::Join, issued),
            invite(
                "mine-too",
                ("friend", "me"),
                "room",
                InviteKind::Join,
                issued,
            ),
            invite("watch", ("fan", "me"), "room", InviteKind::Watch, issued),
            invite(
                "theirs",
                ("host", "other"),
                "room",
                InviteKind::Join,
                issued,
            ),
            invite("elsewhere", ("host", "me"), "far", InviteKind::Join, issued),
        ];
        Invites {
            live: all.into_iter().map(|i| (i.id.clone(), i)).collect(),
        }
    }

    fn told(to: &str, id: &str, answer: Option<InviteAnswer>) -> (String, Down) {
        (
            to.to_owned(),
            Down::InviteGone {
                id: id.to_owned(),
                answer,
            },
        )
    }

    /// What `tell` says, in an order that does not hang on the map's.
    fn sorted(mut tell: Tell) -> Tell {
        tell.sort_by_key(|(to, frame)| (to.clone(), format!("{frame:?}")));
        tell
    }

    /// The invites still open, by id.
    fn left(invites: &Invites) -> Vec<&str> {
        let mut ids: Vec<&str> = invites.live.keys().map(String::as_str).collect();
        ids.sort_unstable();
        ids
    }

    const ACCEPTED: Option<InviteAnswer> = Some(InviteAnswer::Accepted);

    #[test]
    fn joining_spends_only_the_joiners_own_join_invites_to_that_room() {
        let now = Instant::now();
        let mut invites = open(now);
        let tell = invites.spend_joined(None, "me", "room", now);
        assert_eq!(
            sorted(tell),
            sorted(vec![
                told("host", "mine", ACCEPTED),
                told("me", "mine", ACCEPTED),
                told("friend", "mine-too", ACCEPTED),
                told("me", "mine-too", ACCEPTED),
            ]),
        );
        assert_eq!(
            left(&invites),
            ["elsewhere", "theirs", "watch"],
            "another's invite to the room, an invite to another room and one to watch stay"
        );
    }

    #[test]
    fn an_invite_named_on_the_join_is_told_once_beside_the_others() {
        let now = Instant::now();
        let mut invites = open(now);
        let tell = invites.spend_joined(Some("mine"), "me", "room", now);
        assert_eq!(
            sorted(tell),
            sorted(vec![
                told("host", "mine", ACCEPTED),
                told("me", "mine", ACCEPTED),
                told("friend", "mine-too", ACCEPTED),
                told("me", "mine-too", ACCEPTED),
            ]),
        );
        assert_eq!(left(&invites), ["elsewhere", "theirs", "watch"]);
    }

    #[test]
    fn an_invite_of_somebody_else_named_on_the_join_is_left_alone() {
        let now = Instant::now();
        let mut invites = open(now);
        let tell = invites.spend_joined(Some("theirs"), "me", "room", now);
        assert!(
            !tell.iter().any(|(to, _)| to == "other"),
            "the other invitee was told: {tell:?}"
        );
        assert_eq!(tell.len(), 4, "{tell:?}");
        assert_eq!(left(&invites), ["elsewhere", "theirs", "watch"]);
    }

    #[test]
    fn a_lapsed_join_invite_is_not_answered_accepted() {
        let then = Instant::now();
        let now = then + LIFETIME;
        let mut invites = open(now);
        let lapsed = invite("lapsed", ("old", "me"), "room", InviteKind::Join, then);
        invites.live.insert(lapsed.id.clone(), lapsed);
        let tell = invites.spend_joined(Some("lapsed"), "me", "room", now);
        assert!(
            !tell.iter().any(|(to, _)| to == "old"),
            "a lapsed invite was spent: {tell:?}"
        );
        assert_eq!(tell.len(), 4, "the live ones are: {tell:?}");
        assert_eq!(
            left(&invites),
            ["elsewhere", "lapsed", "theirs", "watch"],
            "the lapsed one waits for the sweep"
        );
    }

    #[test]
    fn a_room_that_starts_closes_its_join_invites_and_no_other() {
        let now = Instant::now();
        let mut invites = open(now);
        let tell = invites.close_started("room");
        assert_eq!(
            sorted(tell),
            sorted(vec![
                told("host", "mine", None),
                told("me", "mine", None),
                told("friend", "mine-too", None),
                told("me", "mine-too", None),
                told("host", "theirs", None),
                told("other", "theirs", None),
            ]),
            "every join invite to the room, gone unanswered, to both ends"
        );
        assert_eq!(
            left(&invites),
            ["elsewhere", "watch"],
            "an invite to watch the room and one to another room stay"
        );
        assert!(invites.close_started("room").is_empty(), "and only once");
    }
}
