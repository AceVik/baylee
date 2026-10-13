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
