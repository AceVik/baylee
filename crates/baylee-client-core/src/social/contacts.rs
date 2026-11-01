//! Friends, requests, blocks, groups and invites, as the gateway last said
//! (§10.1): a [`Snapshot`] makes them whole, the events after it change
//! them one at a time, and [`FriendsView`] sorts them into the drawer's
//! sections (§11.3).

use std::cmp::Ordering;

use baylee_protocol::social::{
    Friend, FriendEvent, GroupEvent, GroupInfo, GroupInvite, Person, Place, Presence, Shown,
    Snapshot, SocialSettings, TableInvite,
};

/// A table invite and when it arrived, so its countdown can be drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeldInvite {
    /// The invite.
    pub invite: TableInvite,
    /// When it lapses, Unix milliseconds.
    pub expires_at: u64,
}

/// The social graph as this account sees it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Contacts {
    /// Friends, with their presence.
    pub friends: Vec<Friend>,
    /// Requests to this account.
    pub incoming: Vec<Person>,
    /// Requests from this account.
    pub outgoing: Vec<Person>,
    /// Accounts this one blocked.
    pub blocked: Vec<Person>,
    /// Groups this account is in, in the drawer's order (`/1`–`/9`).
    pub groups: Vec<GroupInfo>,
    /// Invites to groups.
    pub group_invites: Vec<GroupInvite>,
    /// Invites to tables.
    pub invites: Vec<HeldInvite>,
    /// The server-side settings.
    pub settings: SocialSettings,
}

impl Contacts {
    /// Everything from a snapshot; invites' countdowns start at `now_ms`.
    #[must_use]
    pub fn from_snapshot(snapshot: &Snapshot, now_ms: u64) -> Self {
        Self {
            friends: snapshot.friends.clone(),
            incoming: snapshot.incoming.clone(),
            outgoing: snapshot.outgoing.clone(),
            blocked: snapshot.blocked.clone(),
            groups: snapshot.groups.clone(),
            group_invites: snapshot.group_invites.clone(),
            invites: snapshot
                .invites
                .iter()
                .map(|invite| HeldInvite {
                    expires_at: now_ms + u64::from(invite.expires_in) * 1000,
                    invite: invite.clone(),
                })
                .collect(),
            settings: snapshot.settings,
        }
    }

    /// The friend `tag`.
    #[must_use]
    pub fn friend(&self, tag: &str) -> Option<&Friend> {
        self.friends.iter().find(|f| f.person.tag == tag)
    }

    /// Whether `tag` is a friend.
    #[must_use]
    pub fn is_friend(&self, tag: &str) -> bool {
        self.friend(tag).is_some()
    }

    /// Whether this account blocked `tag`.
    #[must_use]
    pub fn is_blocked(&self, tag: &str) -> bool {
        self.blocked.iter().any(|p| p.tag == tag)
    }

    /// The group `id`.
    #[must_use]
    pub fn group(&self, id: &str) -> Option<&GroupInfo> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// Friends shown as anything but offline.
    #[must_use]
    pub fn online_count(&self) -> usize {
        self.friends
            .iter()
            .filter(|f| f.presence.status != Shown::Offline)
            .count()
    }

    /// Whether anything waits for an answer: a request, a group invite or a
    /// table invite (the header button's dot).
    #[must_use]
    pub fn waiting(&self) -> bool {
        !self.incoming.is_empty() || !self.group_invites.is_empty() || !self.invites.is_empty()
    }

    /// A friend's presence changed.
    pub fn set_presence(&mut self, tag: &str, presence: Presence) {
        if let Some(friend) = self.friends.iter_mut().find(|f| f.person.tag == tag) {
            friend.presence = presence;
        }
    }

    /// A friend, request or block event (§8.2).
    pub fn friend_event(
        &mut self,
        event: FriendEvent,
        person: &Person,
        presence: Option<Presence>,
        now_ms: u64,
    ) {
        let tag = person.tag.as_str();
        let drop_from = |list: &mut Vec<Person>| list.retain(|p| p.tag != tag);
        match event {
            FriendEvent::RequestIn => {
                drop_from(&mut self.incoming);
                self.incoming.push(person.clone());
            }
            FriendEvent::RequestOut => {
                drop_from(&mut self.outgoing);
                self.outgoing.push(person.clone());
            }
            FriendEvent::Accepted => {
                drop_from(&mut self.incoming);
                drop_from(&mut self.outgoing);
                self.friends.retain(|f| f.person.tag != tag);
                self.friends.push(Friend {
                    person: person.clone(),
                    since: now_ms,
                    presence: presence.unwrap_or_else(Presence::offline),
                });
            }
            FriendEvent::Removed => self.friends.retain(|f| f.person.tag != tag),
            FriendEvent::RequestGone => {
                drop_from(&mut self.incoming);
                drop_from(&mut self.outgoing);
            }
            FriendEvent::Blocked => {
                self.friends.retain(|f| f.person.tag != tag);
                drop_from(&mut self.incoming);
                drop_from(&mut self.outgoing);
                drop_from(&mut self.blocked);
                self.blocked.push(person.clone());
            }
            FriendEvent::Unblocked => drop_from(&mut self.blocked),
        }
    }

    /// A group event (§8.2); `inviter` and `now_ms` date an invite.
    pub fn group_event(
        &mut self,
        event: GroupEvent,
        group: &GroupInfo,
        inviter: Option<&Person>,
        now_ms: u64,
    ) {
        match event {
            GroupEvent::Joined | GroupEvent::Updated => {
                self.group_invites.retain(|i| i.group != group.id);
                if let Some(old) = self.groups.iter_mut().find(|g| g.id == group.id) {
                    *old = group.clone();
                } else {
                    self.groups.push(group.clone());
                }
            }
            GroupEvent::Left | GroupEvent::Deleted => {
                self.groups.retain(|g| g.id != group.id);
                self.group_invites.retain(|i| i.group != group.id);
            }
            GroupEvent::InviteGone => self.group_invites.retain(|i| i.group != group.id),
            // Listed at once where the gateway names who invited (since
            // 29.10.2026); an older gateway's invite is listed by the next
            // snapshot, which carries its inviter.
            GroupEvent::Invited => {
                if let Some(inviter) = inviter {
                    self.group_invites.retain(|i| i.group != group.id);
                    self.group_invites.push(GroupInvite {
                        group: group.id.clone(),
                        name: group.name.clone(),
                        inviter: inviter.clone(),
                        at: now_ms,
                    });
                }
            }
        }
    }

    /// Drops table invites past their time, and says which went.
    pub fn expire_invites(&mut self, now_ms: u64) -> Vec<String> {
        let (gone, kept) = std::mem::take(&mut self.invites)
            .into_iter()
            .partition(|i| i.expires_at <= now_ms);
        self.invites = kept;
        gone.into_iter().map(|i: HeldInvite| i.invite.id).collect()
    }

    /// The drawer's sections.
    #[must_use]
    pub fn view(&self) -> FriendsView<'_> {
        let mut online: Vec<&Friend> = self
            .friends
            .iter()
            .filter(|f| f.presence.status != Shown::Offline)
            .collect();
        online.sort_by(|a, b| online_order(a, b));
        let mut offline: Vec<&Friend> = self
            .friends
            .iter()
            .filter(|f| f.presence.status == Shown::Offline)
            .collect();
        offline.sort_by(|a, b| name_order(&a.person, &b.person));
        FriendsView {
            incoming: &self.incoming,
            outgoing: &self.outgoing,
            online,
            groups: &self.groups,
            group_invites: &self.group_invites,
            offline,
            blocked: &self.blocked,
            invites: &self.invites,
        }
    }
}

/// The friends drawer's sections (§11.3), in drawing order: Requests,
/// Online, Groups, Offline, Blocked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FriendsView<'a> {
    /// Requests to this account.
    pub incoming: &'a [Person],
    /// Requests from this account ("sent", with Undo).
    pub outgoing: &'a [Person],
    /// Friends not offline: by status (online, away, do not disturb), then
    /// where (playing, waiting, lobby, elsewhere), then name.
    pub online: Vec<&'a Friend>,
    /// Groups, in `/1`–`/9` order.
    pub groups: &'a [GroupInfo],
    /// Invites to groups.
    pub group_invites: &'a [GroupInvite],
    /// Friends offline, by name (folded past 8 by default).
    pub offline: Vec<&'a Friend>,
    /// Blocked accounts.
    pub blocked: &'a [Person],
    /// Table invites waiting for an answer.
    pub invites: &'a [HeldInvite],
}

fn online_order(a: &Friend, b: &Friend) -> Ordering {
    status_rank(a.presence.status)
        .cmp(&status_rank(b.presence.status))
        .then_with(|| {
            place_rank(a.presence.place.as_ref()).cmp(&place_rank(b.presence.place.as_ref()))
        })
        .then_with(|| name_order(&a.person, &b.person))
}

fn status_rank(status: Shown) -> u8 {
    match status {
        Shown::Online => 0,
        Shown::Away => 1,
        Shown::Dnd => 2,
        Shown::Offline => 3,
    }
}

fn place_rank(place: Option<&Place>) -> u8 {
    match place {
        Some(Place::Playing { .. }) => 0,
        Some(Place::Room { .. }) => 1,
        Some(Place::Lobby) => 2,
        _ => 3,
    }
}

fn name_order(a: &Person, b: &Person) -> Ordering {
    a.handle
        .to_lowercase()
        .cmp(&b.handle.to_lowercase())
        .then_with(|| a.tag.cmp(&b.tag))
}
