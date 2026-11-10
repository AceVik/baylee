//! What friends are shown (§6), and the table and team memberships, both
//! read from lobby memory and published to whoever is online.
//!
//! One loop, woken by every lobby change and once a second: it computes
//! each online account's presence and pushes a change to its online
//! friends (at most once a second per account), and diffs the table and
//! team conversations, pushing `Members` to the accounts in them. Memory
//! only, never logged.

use super::{
    Social,
    members::{seated, seated_accounts},
};
use crate::lobby::{Lobby, LobbyGame, LobbyState};
use crate::{Shared, auth};
use baylee_protocol::social::{
    Activity, Conv, Down, Person, Place, Presence, Shown, Status, TableConv,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::{Duration, Instant};

/// How often one account's presence may be pushed.
const DEBOUNCE: Duration = Duration::from_secs(1);

/// An online account, as presence needs it.
pub(crate) struct OnlineView {
    pub(crate) account: String,
    pub(crate) me: Person,
    pub(crate) status: Status,
    pub(crate) activity: Option<Activity>,
    pub(crate) share_table: bool,
    pub(crate) friends: Vec<String>,
}

/// What was last pushed.
#[derive(Default)]
pub(crate) struct Published {
    /// Per account: the presence its friends were last shown (with the
    /// running time left out, which moves every second) and when.
    shown: HashMap<String, (Presence, Instant)>,
    /// Per table or team conversation: its members.
    tables: BTreeMap<String, BTreeSet<String>>,
}

impl Published {
    pub(crate) fn forget(&mut self, account: &str) {
        self.shown.remove(account);
    }
}

/// The tables that say where one account is: the first game of the walk it
/// plays at, which is the answer if there is one, and the last room it waits in.
#[derive(Clone, Copy, Default)]
struct Seating<'a> {
    playing: Option<&'a LobbyGame>,
    room: Option<&'a LobbyGame>,
}

impl<'a> Seating<'a> {
    fn note(&mut self, game: &'a LobbyGame) {
        match game.state {
            LobbyState::Playing => {
                self.playing.get_or_insert(game);
            }
            LobbyState::Waiting => self.room = Some(game),
            LobbyState::Over => {}
        }
    }

    /// `account`'s, for a caller asking about one.
    fn of(lobby: &'a Lobby, account: &str) -> Self {
        let mut seating = Self::default();
        for game in lobby.games.values() {
            if seated_accounts(game).any(|seated| seated == account) {
                seating.note(game);
            }
        }
        seating
    }

    /// Everyone's, in one walk of the lobby: a pass over the online accounts
    /// holds the lobby lock, and walking every game per account made that
    /// `online × games`, once a second and on every change, with every seat
    /// frame of every table queueing behind it.
    fn of_everyone(lobby: &'a Lobby) -> HashMap<&'a str, Self> {
        let mut seatings: HashMap<&str, Self> = HashMap::new();
        for game in lobby.games.values() {
            for account in seated_accounts(game) {
                seatings.entry(account).or_default().note(game);
            }
        }
        seatings
    }
}

/// Where `account` is, from lobby memory and what it said it does.
fn place(
    seating: Seating<'_>,
    lobby: &Lobby,
    watching: &BTreeMap<String, BTreeMap<String, usize>>,
    account: &str,
    activity: Option<Activity>,
    share_table: bool,
    now: u64,
) -> Place {
    let shared =
        |game: &LobbyGame| (share_table && game.password_hash.is_none()).then(|| game.id.clone());
    let seats = |game: &LobbyGame| u8::try_from(game.seats.len()).unwrap_or(u8::MAX);
    if let Some(game) = seating.playing {
        return Place::Playing {
            game: shared(game),
            name: game.name.clone(),
            seats: seats(game),
            running_secs: game.started_at.map_or(0, |t| {
                u32::try_from(now.saturating_sub(t)).unwrap_or(u32::MAX)
            }),
        };
    }
    if let Some(game) = seating.room {
        return Place::Room {
            game: shared(game),
            name: game.name.clone(),
            seats: seats(game),
            taken: u8::try_from(game.seats.iter().filter(|s| s.occupied()).count())
                .unwrap_or(u8::MAX),
            locked: game.password_hash.is_some(),
        };
    }
    if let Some(game) = watching
        .get(account)
        .and_then(|games| games.keys().find_map(|id| lobby.games.get(id)))
    {
        let locked = game.password_hash.is_some();
        return Place::Watching {
            game: (share_table && !locked).then(|| game.id.clone()),
            name: game.name.clone(),
        };
    }
    match activity {
        Some(Activity::Building) => Place::Building,
        Some(Activity::House) => Place::House,
        None => Place::Lobby,
    }
}

/// The presence `status` and `place` make, as friends are shown it.
fn shown(status: Status, place: Place) -> Presence {
    let status = match status {
        Status::Invisible => return Presence::offline(),
        Status::Online => Shown::Online,
        Status::Away => Shown::Away,
        Status::Dnd => Shown::Dnd,
    };
    Presence {
        status,
        place: Some(place),
    }
}

/// `presence` with the running time left out, which is what a change is
/// judged by.
fn settled(presence: &Presence) -> Presence {
    let mut presence = presence.clone();
    if let Some(Place::Playing { running_secs, .. }) = &mut presence.place {
        *running_secs = 0;
    }
    presence
}

/// `account`'s presence now: offline unless it has a social socket open.
pub(crate) fn presence_now(state: &Shared, account: &str) -> Presence {
    let Some(view) = state.social.online_account(account) else {
        return Presence::offline();
    };
    let watching = state.social.watching.lock().clone();
    let lobby = state.lobby.lock();
    shown(
        view.status,
        place(
            Seating::of(&lobby, account),
            &lobby,
            &watching,
            account,
            view.activity,
            view.share_table,
            auth::now_secs(),
        ),
    )
}

/// The table and team conversations of the lobby now, with their names.
fn conversations(lobby: &Lobby) -> BTreeMap<String, (String, BTreeSet<String>)> {
    let mut convs = BTreeMap::new();
    for game in lobby.games.values() {
        let all = seated(game, None);
        if all.is_empty() {
            continue;
        }
        convs.insert(
            Conv::Table {
                game: game.id.clone(),
            }
            .to_string(),
            (game.name.clone(), all),
        );
        let teams: BTreeSet<u8> = game.seats.iter().filter_map(|seat| seat.team).collect();
        for team in teams {
            let members = seated(game, Some(team));
            if !members.is_empty() {
                convs.insert(
                    Conv::Team {
                        game: game.id.clone(),
                        team,
                    }
                    .to_string(),
                    (game.name.clone(), members),
                );
            }
        }
    }
    convs
}

/// The handles of `accounts` as people, from the name book.
async fn people(state: &Shared, accounts: &BTreeSet<String>) -> BTreeMap<String, Person> {
    let names = state
        .names
        .names(&state.db, accounts.iter().cloned().collect())
        .await;
    accounts
        .iter()
        .filter_map(|id| {
            let handle = names.get(id)?;
            let (_, tag) = handle.rsplit_once('#')?;
            Some((
                id.clone(),
                Person {
                    tag: tag.to_owned(),
                    handle: handle.clone(),
                    guest: names.is_guest(id),
                },
            ))
        })
        .collect()
}

/// The table and team conversations `account` is in now.
pub(crate) async fn tables_of(state: &Shared, account: &str) -> Vec<TableConv> {
    let mine: Vec<(String, String, BTreeSet<String>)> = {
        let lobby = state.lobby.lock();
        conversations(&lobby)
            .into_iter()
            .filter(|(_, (_, members))| members.contains(account))
            .map(|(conv, (name, members))| (conv, name, members))
            .collect()
    };
    let everyone: BTreeSet<String> = mine.iter().flat_map(|(_, _, m)| m.clone()).collect();
    let people = people(state, &everyone).await;
    mine.into_iter()
        .map(|(conv, name, members)| TableConv {
            conv,
            name,
            members: members
                .iter()
                .filter_map(|id| people.get(id).cloned())
                .collect(),
        })
        .collect()
}

/// What every online account is shown as now.
fn presences(
    lobby: &Lobby,
    online: Vec<OnlineView>,
    watching: &BTreeMap<String, BTreeMap<String, usize>>,
    now: u64,
) -> Vec<(OnlineView, Presence)> {
    let seatings = Seating::of_everyone(lobby);
    online
        .into_iter()
        .map(|view| {
            let place = place(
                seatings
                    .get(view.account.as_str())
                    .copied()
                    .unwrap_or_default(),
                lobby,
                watching,
                &view.account,
                view.activity,
                view.share_table,
                now,
            );
            let presence = shown(view.status, place);
            (view, presence)
        })
        .collect()
}

/// [`publish`] on a request's way: at once if none did a moment ago, else at
/// the next round of [`spawn`]. It walks everyone online and every game under
/// the locks seat frames take, and any session can ask as often as it likes.
pub(crate) async fn publish_soon(state: &Shared) {
    if state.social.may_publish(Instant::now()) {
        publish(state).await;
    }
}

/// Publishes what changed since the last time.
pub(crate) async fn publish(state: &Shared) {
    let online = state.social.online_accounts();
    let watching = state.social.watching.lock().clone();
    let now = auth::now_secs();
    let (presences, convs) = {
        let lobby = state.lobby.lock();
        (
            presences(&lobby, online, &watching, now),
            conversations(&lobby),
        )
    };
    push_presences(&state.social, presences);
    push_members(state, convs).await;
    let lapsed = state.social.invites.lock().sweep(Instant::now());
    for (account, frame) in lapsed {
        state.social.push(&account, &frame);
    }
}

fn push_presences(social: &Social, presences: Vec<(OnlineView, Presence)>) {
    let at = Instant::now();
    let mut due = Vec::new();
    {
        let mut published = social.published.lock();
        for (view, presence) in presences {
            let key = settled(&presence);
            match published.shown.get(&view.account) {
                Some((last, _)) if *last == key => continue,
                Some((_, when)) if at.duration_since(*when) < DEBOUNCE => continue,
                _ => {}
            }
            published.shown.insert(view.account.clone(), (key, at));
            due.push((view, presence));
        }
    }
    for (view, presence) in due {
        let frame = Down::Presence {
            tag: view.me.tag.clone(),
            presence,
        };
        for friend in &view.friends {
            social.push(friend, &frame);
        }
    }
}

async fn push_members(state: &Shared, convs: BTreeMap<String, (String, BTreeSet<String>)>) {
    let changed: Vec<(String, BTreeSet<String>, BTreeSet<String>)> = {
        let mut published = state.social.published.lock();
        let mut changed = Vec::new();
        for (conv, (_, members)) in &convs {
            let before = published.tables.get(conv).cloned().unwrap_or_default();
            if &before != members {
                changed.push((conv.clone(), before, members.clone()));
            }
        }
        for (conv, before) in &published.tables {
            if !convs.contains_key(conv) {
                changed.push((conv.clone(), before.clone(), BTreeSet::new()));
            }
        }
        published.tables = convs
            .into_iter()
            .map(|(conv, (_, members))| (conv, members))
            .collect();
        changed
    };
    if changed.is_empty() {
        return;
    }
    let everyone: BTreeSet<String> = changed
        .iter()
        .flat_map(|(_, before, after)| before.iter().chain(after).cloned())
        .collect();
    let people = people(state, &everyone).await;
    for (conv, before, after) in changed {
        let frame = Down::Members {
            conv,
            members: after
                .iter()
                .filter_map(|id| people.get(id).cloned())
                .collect(),
        };
        for account in before.union(&after) {
            state.social.push(account, &frame);
        }
    }
}

/// The loop that publishes presence and memberships: on a lobby change, as
/// [`publish_soon`] lets it, and once a second.
pub(crate) fn spawn(state: Shared) {
    if !state.social.on() {
        return;
    }
    tokio::spawn(async move {
        let mut changed = state.lobby_changed.subscribe();
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        loop {
            tokio::select! {
                news = changed.recv() => {
                    if matches!(news, Err(tokio::sync::broadcast::error::RecvError::Closed)) {
                        return;
                    }
                    // Any session can move the lobby, if only by opening
                    // and closing sockets: what this skips, the tick says.
                    publish_soon(&state).await;
                }
                _ = tick.tick() => publish(&state).await,
            }
        }
    });
}

/// `account`'s last social socket closed: its friends see it offline now.
pub(crate) fn went_offline(social: &Social, me: &Person, friends: &[String], account: &str) {
    social.published.lock().forget(account);
    let frame = Down::Presence {
        tag: me.tag.clone(),
        presence: Presence::offline(),
    };
    for friend in friends {
        social.push(friend, &frame);
    }
}

#[cfg(test)]
mod tests;
