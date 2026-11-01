//! Policy diagnostics are derived exclusively from this seat's `PlayerView`.
use anyhow::Result;
use baylee_ai::HeuristicAgent;
use baylee_core::ids::ObjectId;
use baylee_core::types::TypeSet;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_train::features4::{encode_candidates4, encode_state4};
use baylee_view::PlayerView;
use serde_json::{Value, json};

pub fn pool(view: &PlayerView) -> u64 {
    view.seats
        .iter()
        .find(|seat| seat.player == view.seat)
        .map_or(0, |seat| seat.mana_pool.total())
}

fn name(view: &PlayerView, id: ObjectId) -> Option<&str> {
    view.hand
        .iter()
        .find(|row| row.id == id)
        .map(|row| row.name.as_str())
        .or_else(|| {
            view.battlefield
                .iter()
                .chain(&view.stack)
                .chain(view.command.iter().flatten())
                .find(|row| row.id == id)
                .map(|row| row.name.as_str())
        })
}

fn named(view: &PlayerView, ids: &[ObjectId]) -> Vec<Value> {
    ids.iter()
        .map(|id| json!({"id":id,"name":name(view,*id)}))
        .collect()
}

pub fn action(view: &PlayerView, action: &PlayerAction) -> Value {
    // Resolve only identities authorized in the actor's own view. Keeping the
    // original action alongside the names makes target/ability indexing auditable.
    let raw = serde_json::to_value(action).expect("PlayerAction serializes");
    let objects: Vec<_> = action_objects(action)
        .into_iter()
        .map(|id| json!({"id":id,"name":name(view,id)}))
        .collect();
    json!({"action": raw,"objects":objects})
}

fn action_objects(action: &PlayerAction) -> Vec<ObjectId> {
    use baylee_core::ids::Defender;
    use baylee_engine::choice::PriorityHold;
    match action {
        PlayerAction::CastSpell { card }
        | PlayerAction::PlayLand { card }
        | PlayerAction::Suspend { card }
        | PlayerAction::Foretell { card } => vec![*card],
        PlayerAction::ActivateManaAbility { source }
        | PlayerAction::ActivateAbility { source, .. } => vec![*source],
        PlayerAction::ChooseManaAbility { source, .. }
        | PlayerAction::ChooseDamageSource { source, .. } => vec![source.object],
        PlayerAction::ChooseObjects { objects }
        | PlayerAction::ChooseTargets { objects, .. }
        | PlayerAction::ChooseTargetBatch { objects, .. } => objects.clone(),
        PlayerAction::DeclareAttackers { attackers } => attackers
            .iter()
            .flat_map(|(id, defender)| {
                let mut objects = vec![*id];
                if let Defender::Planeswalker(id) | Defender::Battle(id) = defender {
                    objects.push(*id);
                }
                objects
            })
            .collect(),
        PlayerAction::DeclareBlockers { blockers } => {
            blockers.iter().flat_map(|(a, b)| [*a, *b]).collect()
        }
        PlayerAction::Arrange { piles } => piles.iter().flatten().copied().collect(),
        PlayerAction::PayWindow { steps } => steps.iter().flat_map(action_objects).collect(),
        PlayerAction::SetPriorityHold(PriorityHold::UntilTopOfStack { object }) => vec![*object],
        // CardIndex, ability index, player seat, choice ID and counts are NOT ObjectIds.
        _ => Vec::new(),
    }
}

pub fn snapshot(view: &PlayerView) -> Value {
    json!({"turn":view.turn,"phase":view.phase,"step":view.step,
        "active":view.active,"awaiting":view.awaiting,"owed":view.owed,
        "pool_total":pool(view),
        "pool":view.seats.iter().find(|seat|seat.player==view.seat).map(|seat|seat.mana_pool)})
}

pub fn priority(
    view: &PlayerView,
    pending: &Pending,
    teams: &[Option<u8>],
    sharp: &HeuristicAgent,
) -> Result<Value> {
    let Pending::Priority { legal, .. } = pending else {
        return Ok(Value::Null);
    };
    let state = encode_state4(view, Some(pending), None, teams);
    let candidates = encode_candidates4(&state, view, pending);
    for id in &legal.castable {
        anyhow::ensure!(
            candidates
                .candidates
                .iter()
                .any(|candidate| candidate.action == Some(PlayerAction::CastSpell { card: *id })),
            "legal CastSpell missing from v4 candidates: {id:?}"
        );
    }
    let missing_payable: Vec<_> = legal
        .payable
        .iter()
        .copied()
        .filter(|id| {
            !candidates
                .candidates
                .iter()
                .any(|candidate| candidate.action == Some(PlayerAction::CastSpell { card: *id }))
        })
        .collect();
    let instants: Vec<_> = view
        .hand
        .iter()
        .filter(|row| row.types.contains(TypeSet::INSTANT))
        .map(|row| {
            json!({"id":row.id,"name":row.name,
            "castable":legal.castable.contains(&row.id),"payable":legal.payable.contains(&row.id)})
        })
        .collect();
    Ok(json!({"candidate_count":candidates.candidates.len(),
        "sharp":action(view,&sharp.act(view,pending)),"hand_instants":instants,
        "stack":view.stack.iter().map(|row|json!({"id":row.id,"name":row.name})).collect::<Vec<_>>(),
        "castable":named(view,&legal.castable),"payable":named(view,&legal.payable),
        "missing_payable_casts":named(view,&missing_payable),
        "mana_sources":named(view,&legal.mana_abilities),
        "abilities":legal.abilities.iter().map(|(id,index)|json!({"id":id,"name":name(view,*id),"index":index})).collect::<Vec<_>>() }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::{CardIndex, PlayerId};

    #[test]
    fn only_typed_object_fields_become_action_identities() {
        let source = ObjectId::new(9, 0);
        assert_eq!(
            action_objects(&PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            }),
            vec![source]
        );
        assert!(action_objects(&PlayerAction::ChooseNumber(0)).is_empty());
        assert!(
            action_objects(&PlayerAction::ChooseCardName {
                card: CardIndex::new(0),
                face: 0,
            })
            .is_empty()
        );
        assert_eq!(
            action_objects(&PlayerAction::ChooseTargets {
                objects: vec![source],
                players: vec![PlayerId::new(0)],
            }),
            vec![source]
        );
    }
}
