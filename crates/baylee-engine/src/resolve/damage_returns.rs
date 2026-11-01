//! Iterate the exact graveyard successors of creatures damaged by the source.

use super::Resolution;
use crate::state::GameState;
use crate::zone::Zone;
use baylee_cards_dsl::Effect;

pub(super) fn for_each(
    state: &GameState,
    res: &mut Resolution,
    effects: &'static [Effect],
    op: Effect,
) {
    let mut remaining = res.subject.for_each_remaining.take().unwrap_or_else(|| {
        let mut cards = super::source_version(state, res)
            .map(|version| baylee_core::ids::DamageSourceRef {
                object: res.source,
                version,
            })
            .or_else(|| state.source_identity(res.source))
            .map(|source| state.damaged_creature_cards_in_graveyard(source))
            .unwrap_or_default();
        cards.reverse();
        cards
    });
    while let Some(next) = remaining.pop() {
        if !state
            .object(next.object)
            .is_some_and(|object| object.zone == Zone::Graveyard && object.version == next.version)
        {
            continue;
        }
        res.event_object = Some(next.object);
        let power = state
            .source_object(next)
            .and_then(|object| object.characteristics().power)
            .unwrap_or(0);
        res.subject.set_event_identity(Some((next, power)));
        let mut spliced = super::flatten(effects);
        if !remaining.is_empty() {
            res.subject.for_each_remaining = Some(remaining);
            spliced.push(op);
        }
        let insertion = res.pc + 1;
        res.effects.splice(insertion..insertion, spliced);
        return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, creature, preset};
    use crate::event::Cause;
    use crate::resolve::{Flow, SubjectContext};
    use crate::state::{DelayedAction, DelayedWhen};
    use crate::zone::{ZoneLocation, ZonePosition};
    use baylee_cards_dsl::{DelayedObservation, TargetSpec};
    use baylee_core::ids::{PlayerId, SeatSet};
    use smallvec::SmallVec;

    #[test]
    fn each_damage_death_returns_and_binds_its_own_successor_to_an_independent_observer() {
        static BODY: &[Effect] = &[
            Effect::GraveyardToBattlefield {
                target: TargetSpec::EventObject,
                owner_control: false,
                counters: None,
            },
            Effect::CreateDelayedObserver {
                watch: TargetSpec::ThisObject,
                when: DelayedObservation::YouLoseControl,
                about: TargetSpec::EventObject,
                effects: &[Effect::SacrificeEvent],
            },
        ];
        let card = creature(850_007, "Synthetic return sequence", 1, 4, &[]);
        let lookup = SyntheticLookup::new(vec![card]);
        let mut state =
            GameState::from_preset(&preset(725, &[850_007, 850_007, 850_007]), &lookup).unwrap();
        let objects = state.zones.list(ZoneLocation::Battlefield).clone();
        let source = state.source_identity(objects[0]).unwrap();
        let player = PlayerId::new(0);
        for object in &objects[1..] {
            state.record_permanent_damage(source.object, Some(source.version), *object, 1, false);
            state
                .move_object(
                    *object,
                    ZoneLocation::Graveyard(player),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        }
        let mut res = Resolution {
            source: source.object,
            on_stack: source.object,
            controller: player,
            effects: vec![Effect::ForEachCreatureDamagedBySourceThatDiedThisTurn { effects: BODY }],
            pc: 0,
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_lki: None,
            subject: SubjectContext::default().with_source_version(Some(source.version)),
            text: crate::text_changes::TextChangeMap::IDENTITY,
            event_mana: None,
            retarget_left: None,
            loot: None,
            coin_stake: None,
            draw_watch: None,
            target_players: SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: false,
            mana_ability: false,
            countered_source: None,
            this_way_from: None,
        };
        let mut flow = crate::resolve::run(&mut state, &mut res);
        while matches!(flow, Flow::Enter) {
            assert!(crate::entry::advance(&mut state).is_none());
            flow = crate::resolve::resume_entries(&mut state, &mut res);
        }
        assert!(matches!(flow, Flow::Complete));
        assert_eq!(state.delayed.len(), 2);
        for (watch, &object) in state.delayed.iter().zip(&objects[1..]) {
            assert!(
                matches!(watch.when, DelayedWhen::Observed { watch: watched, .. } if watched == source)
            );
            assert!(
                matches!(watch.action, DelayedAction::TriggerAbout { object: about, version, .. }
                if about == object && version == state.object(object).unwrap().version)
            );
            assert_eq!(state.object(object).unwrap().zone, Zone::Battlefield);
        }
        state
            .move_object(
                source.object,
                ZoneLocation::Graveyard(player),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        let triggers = crate::trigger::collect(&state, &lookup, 0);
        assert_eq!(triggers.len(), 2);
        assert_ne!(triggers[0].event_object, triggers[1].event_object);
    }
}
