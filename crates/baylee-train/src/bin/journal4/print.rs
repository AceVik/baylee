//! A decision in words.
use std::fmt::Write as _;

use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::{ObjectStatus, PlayerView, PublicObject};
use serde_json::Value;

use super::Row;

/// How many options of a menu are printed, best first.
const OPTIONS: usize = 6;

fn name(view: &PlayerView, id: ObjectId) -> String {
    view.object(id)
        .map(|object| object.name.clone())
        .or_else(|| {
            view.hand
                .iter()
                .find(|card| card.id == id)
                .map(|card| card.name.clone())
        })
        .or_else(|| {
            view.stack
                .iter()
                .find(|item| item.id == id)
                .map(|item| format!("stack:{}", item.name))
        })
        .unwrap_or_else(|| format!("{id:?}"))
}

fn names(view: &PlayerView, ids: &[ObjectId]) -> String {
    ids.iter()
        .map(|id| name(view, *id))
        .collect::<Vec<_>>()
        .join(", ")
}

/// An answer in words; what has no words here prints as it is.
fn action(view: &PlayerView, action: &PlayerAction) -> String {
    match action {
        PlayerAction::PassPriority => "pass".into(),
        PlayerAction::PlayLand { card, .. } => format!("land {}", name(view, *card)),
        PlayerAction::CastSpell { card, .. } => format!("cast {}", name(view, *card)),
        PlayerAction::ActivateManaAbility { source } => format!("tap {}", name(view, *source)),
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        } => format!("activate {} #{ability_index}", name(view, *source)),
        PlayerAction::DeclareAttackers { attackers } => {
            let ids: Vec<ObjectId> = attackers.iter().map(|pair| pair.0).collect();
            format!("attack [{}]", names(view, &ids))
        }
        PlayerAction::DeclareBlockers { blockers } => {
            let pairs: Vec<String> = blockers
                .iter()
                .map(|(blocker, attacker)| {
                    format!("{} x {}", name(view, *blocker), name(view, *attacker))
                })
                .collect();
            format!("block [{}]", pairs.join("; "))
        }
        PlayerAction::ChooseObjects { objects } => format!("objects [{}]", names(view, objects)),
        PlayerAction::ChooseTargets { objects, players } => {
            format!("targets [{}] {players:?}", names(view, objects))
        }
        other => format!("{other:?}"),
    }
}

fn object(view: &PlayerView, object: &PublicObject) -> String {
    let mut text = object.name.clone();
    if let (Some(power), Some(toughness)) = (object.power, object.toughness) {
        let _ = write!(text, " {power}/{toughness}");
    }
    if object.damage > 0 {
        let _ = write!(text, " d{}", object.damage);
    }
    if object.status.contains(ObjectStatus::TAPPED) {
        text.push_str(" T");
    }
    if object.summoning_sick {
        text.push_str(" sick");
    }
    if view
        .combat
        .attackers
        .iter()
        .any(|attacker| attacker.creature == object.id)
    {
        text.push_str(" ATK");
    }
    text
}

fn board(view: &PlayerView, player: PlayerId) -> String {
    use baylee_core::types::TypeSet;
    let mut lands = 0;
    let mut untapped = 0;
    let mut rest = Vec::new();
    for item in view.battlefield_of(player) {
        if item.types.contains(TypeSet::LAND) && !item.types.contains(TypeSet::CREATURE) {
            lands += 1;
            untapped += usize::from(!item.status.contains(ObjectStatus::TAPPED));
        } else {
            rest.push(object(view, item));
        }
    }
    format!("{untapped}/{lands} lands; {}", rest.join(", "))
}

fn question(pending: &Pending) -> &'static str {
    match pending {
        Pending::Priority { .. } => "priority",
        Pending::ChooseAttackers { .. } => "attackers",
        Pending::ChooseBlockers { .. } => "blockers",
        Pending::ChooseTargets { .. } => "targets",
        Pending::ChooseCards { .. } => "cards",
        Pending::DiscardChoice { .. } => "discard",
        Pending::Mulligan { .. } => "mulligan",
        Pending::MulliganBottom { .. } => "bottom",
        _ => "other",
    }
}

/// The net's own order of the options.
fn ranking(view: &PlayerView, menu: &Value) -> String {
    let mut options: Vec<(f64, String)> = menu["options"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|option| {
            let text = serde_json::from_value::<PlayerAction>(option["action"].clone()).map_or_else(
                |_| {
                    serde_json::from_value::<ObjectId>(option["object"].clone())
                        .map_or_else(|_| option["index"].to_string(), |id| name(view, id))
                },
                |answer| action(view, &answer),
            );
            (option["logit"].as_f64().unwrap_or(0.0), text)
        })
        .collect();
    options.sort_by(|a, b| b.0.total_cmp(&a.0));
    options
        .iter()
        .take(OPTIONS)
        .map(|(logit, text)| format!("{text} {logit:.1}"))
        .collect::<Vec<_>>()
        .join(" | ")
}

/// A decision, in a few lines.
pub(crate) fn decision(row: &Row) -> String {
    let view = &row.view;
    let me = view.seat;
    let other = view
        .seats
        .iter()
        .map(|seat| seat.player)
        .find(|player| *player != me)
        .unwrap_or(me);
    let life = |player| view.seat(player).map_or(0, |seat| seat.life);
    let hand: Vec<&str> = view.hand.iter().map(|card| card.name.as_str()).collect();
    let stack: Vec<&str> = view.stack.iter().map(|item| item.name.as_str()).collect();
    let mut text = format!(
        "#{} T{} {:?}/{:?} {} life {}:{} hand [{}]",
        row.decision,
        view.turn,
        view.phase,
        view.step,
        if view.active == me { "mine" } else { "theirs" },
        life(me),
        life(other),
        hand.join(", "),
    );
    if !stack.is_empty() {
        let _ = write!(text, " stack [{}]", stack.join(", "));
    }
    let _ = write!(
        text,
        "\n  me  {}\n  opp {}\n  {} ({}) -> {}\n",
        board(view, me),
        board(view, other),
        question(&row.pending),
        row.source,
        action(view, &row.action)
    );
    let top = ranking(view, &row.menu);
    if !top.is_empty() {
        let _ = writeln!(text, "  net {top}");
    }
    text
}

/// Creatures and power on one side.
fn force(view: &PlayerView, player: PlayerId) -> (usize, i32) {
    use baylee_core::types::TypeSet;
    view.battlefield_of(player)
        .filter(|object| object.types.contains(TypeSet::CREATURE))
        .fold((0, 0), |(count, power), object| {
            (
                count + 1,
                power + i32::from(object.power.unwrap_or(0)).max(0),
            )
        })
}

/// One line per turn: the state as the turn ended, and what the seat did in it.
pub(crate) fn ledger(rows: &[Row]) -> String {
    use baylee_core::types::TypeSet;
    let mut text = String::new();
    let mut index = 0;
    while index < rows.len() {
        let turn = rows[index].view.turn;
        let end = rows[index..]
            .iter()
            .position(|row| row.view.turn != turn)
            .map_or(rows.len(), |offset| index + offset);
        let last = &rows[end - 1].view;
        let me = last.seat;
        let other = last
            .seats
            .iter()
            .map(|seat| seat.player)
            .find(|player| *player != me)
            .unwrap_or(me);
        let life = |player| last.seat(player).map_or(0, |seat| seat.life);
        let lands = |player| {
            last.battlefield_of(player)
                .filter(|object| object.types.contains(TypeSet::LAND))
                .count()
        };
        let (mine, my_power) = force(last, me);
        let (theirs, their_power) = force(last, other);
        let done: Vec<String> = rows[index..end]
            .iter()
            .filter(|row| {
                matches!(
                    row.action,
                    PlayerAction::CastSpell { .. }
                        | PlayerAction::ActivateAbility { .. }
                        | PlayerAction::DeclareBlockers { .. }
                ) || matches!(&row.action, PlayerAction::DeclareAttackers { attackers }
                        if !attackers.is_empty())
            })
            .map(|row| action(&row.view, &row.action))
            .collect();
        let _ = writeln!(
            text,
            "T{turn} {} {}:{} hand {} lands {}/{} crea {mine}/{my_power} v {theirs}/{their_power} | {}",
            if last.active == me { "me " } else { "opp" },
            life(me),
            life(other),
            last.hand.len(),
            lands(me),
            lands(other),
            done.join("; ")
        );
        index = end;
    }
    text
}
