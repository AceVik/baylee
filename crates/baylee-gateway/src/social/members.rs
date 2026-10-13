//! Who a conversation's members are (§5): whispers and groups from the
//! database, tables and teams from lobby memory.
//!
//! A table's members are the **accounts in its chairs**: a delegate's chair
//! (a seat bridge) has no account, an AI chair has none, and a spectator
//! has no chair, so none of them is ever a member. That follows from where
//! the members are read, not from a rule that could be forgotten.

use super::uid;
use crate::Shared;
use crate::lobby::{Lobby, LobbyGame, SeatKind};
use baylee_protocol::social::{Conv, WhispersFrom};
use std::collections::BTreeSet;

/// The accounts in a chair of `game`, or of team `team` there.
pub(crate) fn seated(game: &LobbyGame, team: Option<u8>) -> BTreeSet<String> {
    game.seats
        .iter()
        .filter(|seat| seat.kind == SeatKind::Human)
        .filter(|seat| team.is_none_or(|team| seat.team == Some(team)))
        .filter_map(|seat| seat.account_id.clone())
        .collect()
}

/// Whether `x` and `y` both have a chair at one table now.
pub(crate) fn share_a_table(lobby: &Lobby, x: &str, y: &str) -> bool {
    lobby.games.values().any(|game| {
        let at = seated(game, None);
        at.contains(x) && at.contains(y)
    })
}

/// The accounts that sit at a table with `account` now, `account` left out.
pub(crate) fn table_mates(lobby: &Lobby, account: &str) -> BTreeSet<String> {
    let mut mates = BTreeSet::new();
    for game in lobby.games.values() {
        let at = seated(game, None);
        if at.contains(account) {
            mates.extend(at);
        }
    }
    mates.remove(account);
    mates
}

/// The members of `conv` as `sender` (whose tag is `sender_tag`) may write
/// to it, or `None` when `sender` may not: not in it, not friends, not
/// allowed, no such group or table. One answer for all of them, so the
/// sender cannot tell which.
///
/// A recipient who blocks the sender is still a member: the message is
/// wrapped for its devices and the gateway gives them no slot (§5.1).
///
/// # Errors
///
/// When the database fails.
pub(crate) async fn members(
    state: &Shared,
    sender: &str,
    sender_tag: &str,
    conv: &Conv,
) -> anyhow::Result<Option<BTreeSet<String>>> {
    match conv {
        Conv::Dm { .. } => {
            let Some(partner_tag) = conv.partner(sender_tag) else {
                return Ok(None);
            };
            let Some(tag) = super::tag_number(partner_tag) else {
                return Ok(None);
            };
            let Some(partner) = baylee_db::social::who_by_tag(&state.db, tag).await? else {
                return Ok(None);
            };
            let (Some(me), partner_id) = (uid(sender), partner.id) else {
                return Ok(None);
            };
            if may_whisper(state, me, partner_id).await? {
                Ok(Some(
                    [sender.to_owned(), partner_id.to_string()]
                        .into_iter()
                        .collect(),
                ))
            } else {
                Ok(None)
            }
        }
        Conv::Group { id } => {
            let (Ok(group), Some(me)) = (uuid::Uuid::parse_str(id), uid(sender)) else {
                return Ok(None);
            };
            let Some(group) = baylee_db::social::group(&state.db, group).await? else {
                return Ok(None);
            };
            if group.role(me).is_none() {
                return Ok(None);
            }
            Ok(Some(
                group.member_ids().iter().map(ToString::to_string).collect(),
            ))
        }
        Conv::Table { game } => Ok(table(state, game, None, sender)),
        Conv::Team { game, team } => Ok(table(state, game, Some(*team), sender)),
    }
}

/// A table's (or team's) members, if `sender` is one of them.
fn table(state: &Shared, game: &str, team: Option<u8>, sender: &str) -> Option<BTreeSet<String>> {
    let lobby = state.lobby.lock();
    let game = lobby.games.get(game)?;
    if team.is_some() && game.seats.iter().all(|seat| seat.team.is_none()) {
        return None;
    }
    let members = seated(game, team);
    members.contains(sender).then_some(members)
}

/// Whether `me` may whisper `partner`: friends, or both at one table now
/// with the partner taking whispers from table mates (§5.2).
///
/// # Errors
///
/// When the database fails.
pub(crate) async fn may_whisper(
    state: &Shared,
    me: uuid::Uuid,
    partner: uuid::Uuid,
) -> anyhow::Result<bool> {
    if me == partner {
        return Ok(false);
    }
    if baylee_db::social::relation(&state.db, me, partner)
        .await?
        .friend
    {
        return Ok(true);
    }
    let together = share_a_table(&state.lobby.lock(), &me.to_string(), &partner.to_string());
    if !together {
        return Ok(false);
    }
    let settings = baylee_db::social::settings_of(&state.db, &[partner]).await?;
    Ok(settings.first().is_some_and(|(_, settings)| {
        WhispersFrom::from_code(settings.whispers_from) == WhispersFrom::FriendsAndTables
    }))
}

/// The accounts `account` may write to and so fetch the keys of: itself,
/// its friends, its groups' members and the players at its tables.
///
/// # Errors
///
/// When the database fails.
pub(crate) async fn may_write_to(
    state: &Shared,
    account: uuid::Uuid,
) -> anyhow::Result<BTreeSet<String>> {
    let mut may: BTreeSet<String> = BTreeSet::new();
    may.insert(account.to_string());
    for friend in baylee_db::social::friend_ids(&state.db, account).await? {
        may.insert(friend.to_string());
    }
    for co in baylee_db::social::co_members(&state.db, account).await? {
        may.insert(co.to_string());
    }
    may.extend(table_mates(&state.lobby.lock(), &account.to_string()));
    Ok(may)
}
