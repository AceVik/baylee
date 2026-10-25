//! Explore: reveal before choosing, and place the counter before the decision.
use super::{
    ArrangePlace, AwaitingOp, Cause, GameEvent, GameState, Pending, Resolution, TargetSpec,
    ZoneLocation, ZonePosition, look_question,
};
use baylee_cards_dsl::CounterKind;
use baylee_core::types::TypeSet;

pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    target: TargetSpec,
) -> Option<Pending> {
    let (_, _, player) = resolve_explorer(state, res, target)?;
    let scries = u8::try_from(
        state
            .replacement_rules
            .iter()
            .filter(|entry| {
                entry.rule == baylee_cards_dsl::ReplacementRule::ExploreScry1First
                    && entry.controller == player
                    && !state
                        .object(entry.source)
                        .is_some_and(|o| o.status.contains(crate::object::Status::PHASED_OUT))
            })
            .inspect(|#[cfg_attr(not(test), allow(unused_variables))] entry| {
                #[cfg(test)]
                crate::ability_log::replaced(entry);
            })
            .count(),
    )
    .unwrap_or(u8::MAX);
    if scries > 0
        && let Some(card) = state
            .zones
            .list(ZoneLocation::Library(player))
            .last()
            .copied()
    {
        res.awaiting = Some(AwaitingOp::ExploreScry {
            player,
            target,
            remaining: scries - 1,
        });
        return Some(look_question(
            player,
            vec![card],
            ArrangePlace::LibraryBottom,
        ));
    }
    explore_now(state, res, target)
}

pub(super) fn resume_scry(
    state: &mut GameState,
    res: &mut Resolution,
    player: baylee_core::ids::PlayerId,
    target: TargetSpec,
    remaining: u8,
    top: &[baylee_core::ids::ObjectId],
    bottom: &[baylee_core::ids::ObjectId],
) -> Option<Pending> {
    let library = ZoneLocation::Library(player);
    for &card in bottom {
        let _ = state.move_object(card, library, ZonePosition::Bottom, Cause::Effect);
    }
    for &card in top.iter().rev() {
        let _ = state.move_object(card, library, ZonePosition::Top, Cause::Effect);
    }
    if remaining > 0
        && let Some(card) = state.zones.list(library).last().copied()
    {
        res.awaiting = Some(AwaitingOp::ExploreScry {
            player,
            target,
            remaining: remaining - 1,
        });
        return Some(look_question(
            player,
            vec![card],
            ArrangePlace::LibraryBottom,
        ));
    }
    explore_now(state, res, target)
}

fn resolve_explorer(
    state: &GameState,
    res: &Resolution,
    target: TargetSpec,
) -> Option<(baylee_core::ids::ObjectId, bool, baylee_core::ids::PlayerId)> {
    let creature = match target {
        TargetSpec::ThisObject => res.source,
        TargetSpec::EventObject => res.event_object?,
        _ => super::zones::spec_object(state, res, target)?,
    };
    let reference = match target {
        TargetSpec::ThisObject => super::subjects::source(state, res),
        TargetSpec::EventObject => super::subjects::event(state, res),
        _ => super::subjects::target(state, res, creature),
    };
    let present =
        reference.is_some_and(|reference| super::subjects::on_battlefield(state, reference));
    let player = if present {
        state.object(creature)?.controller
    } else {
        reference
            .and_then(|reference| state.damage_source(creature, Some(reference.version)))
            .map(|object| object.controller)
            .or_else(|| {
                res.target_lki
                    .as_ref()?
                    .iter()
                    .find(|target| {
                        target.id == creature
                            && reference
                                .is_some_and(|reference| reference.version == target.version)
                    })
                    .map(|target| target.controller)
            })
            .unwrap_or(res.controller)
    };
    Some((creature, present, player))
}

fn explore_now(state: &mut GameState, res: &mut Resolution, target: TargetSpec) -> Option<Pending> {
    let (creature, present, player) = resolve_explorer(state, res, target)?;
    let card = state
        .zones
        .list(ZoneLocation::Library(player))
        .last()
        .copied();
    if let Some(card) = card {
        state.journal.record(GameEvent::Revealed {
            player,
            cards: vec![card],
        });
        if state
            .object(card)
            .is_some_and(|object| object.characteristics().types.contains(TypeSet::LAND))
        {
            let owner = state.object(card)?.owner;
            let _ = state.move_object(
                card,
                ZoneLocation::Hand(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            return None;
        }
    }
    if present {
        crate::replacement::put_counters_by(state, player, creature, CounterKind::P1P1, 1);
    }
    let card = card?;
    res.awaiting = Some(AwaitingOp::Explore { player });
    Some(look_question(player, vec![card], ArrangePlace::Graveyard))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use crate::object::ObjectKind;
    use baylee_core::ids::{ObjectId, PlayerId};

    fn setup(types: TypeSet) -> (GameState, Resolution, ObjectId, ObjectId) {
        let mut state =
            GameState::from_preset(&preset(551, &[]), &SyntheticLookup::new(vec![])).unwrap();
        let player = PlayerId::new(1);
        let name = state.names.intern("explorer");
        let creature = state.create_bare(
            player,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let card = state.create_bare(
            player,
            ObjectKind::Card,
            name,
            ZoneLocation::Library(player),
        );
        std::sync::Arc::make_mut(&mut state.object_mut(card).unwrap().base).types = types;
        state.invalidate_projections();
        let res = Resolution {
            source: creature,
            on_stack: creature,
            controller: PlayerId::new(0),
            effects: vec![baylee_cards_dsl::Effect::Explore {
                target: TargetSpec::ThisObject,
            }],
            pc: 0,
            targets: smallvec::SmallVec::new(),
            second_targets: smallvec::SmallVec::new(),
            x: None,
            chosen_player: None,
            target_players: baylee_core::ids::SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: false,
            mana_ability: false,
            countered_source: None,
            target_lki: None,
            subject: super::super::SubjectContext::default(),
            text: crate::text_changes::TextChangeMap::IDENTITY,
            event_mana: None,
            retarget_left: None,
            loot: None,
            draw_watch: None,
            this_way_from: None,
            coin_stake: None,
        };
        (state, res, creature, card)
    }

    #[test]
    fn explore_uses_creature_controller_and_reveals_land_without_counter() {
        let (mut state, mut res, creature, card) = setup(TypeSet::LAND);
        assert!(matches!(
            super::super::run(&mut state, &mut res),
            super::super::Flow::Complete
        ));
        assert!(
            state
                .zones
                .list(ZoneLocation::Hand(PlayerId::new(1)))
                .contains(&card)
        );
        assert_eq!(
            state
                .object(creature)
                .unwrap()
                .counters
                .get(CounterKind::P1P1),
            0
        );
        assert!(state.journal.entries().iter().any(|entry| matches!(&entry.event,
            GameEvent::Revealed { player, cards } if *player == PlayerId::new(1) && cards == &[card])));
    }

    #[test]
    fn explore_counter_precedes_choice_and_nonland_can_stay_or_go() {
        for bury in [false, true] {
            let (mut state, mut res, creature, card) = setup(TypeSet::SORCERY);
            let super::super::Flow::Wait(Pending::Arrange { player, cards, .. }) =
                super::super::run(&mut state, &mut res)
            else {
                panic!("explore choice");
            };
            assert_eq!(player, PlayerId::new(1));
            assert_eq!(cards, vec![card]);
            assert_eq!(
                state
                    .object(creature)
                    .unwrap()
                    .counters
                    .get(CounterKind::P1P1),
                1
            );
            let piles = if bury {
                [vec![], vec![card]]
            } else {
                [vec![card], vec![]]
            };
            assert!(matches!(
                super::super::resume_arranged(&mut state, &mut res, &piles),
                super::super::Flow::Complete
            ));
            let destination = if bury {
                ZoneLocation::Graveyard(player)
            } else {
                ZoneLocation::Library(player)
            };
            assert!(state.zones.list(destination).contains(&card));
            assert_eq!(
                state
                    .object(creature)
                    .unwrap()
                    .counters
                    .get(CounterKind::P1P1),
                1
            );
        }
    }

    #[test]
    fn empty_library_still_adds_a_counter_without_a_question() {
        let (mut state, mut res, creature, _) = setup(TypeSet::SORCERY);
        let player = PlayerId::new(1);
        let cards = state.zones.list(ZoneLocation::Library(player)).clone();
        for card in cards {
            let _ = state.move_object(
                card,
                ZoneLocation::Graveyard(player),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
        assert!(matches!(
            super::super::run(&mut state, &mut res),
            super::super::Flow::Complete
        ));
        assert_eq!(
            state
                .object(creature)
                .unwrap()
                .counters
                .get(CounterKind::P1P1),
            1
        );
    }
    #[test]
    fn a_departed_implicit_explorer_still_reveals_but_gets_no_counter() {
        let (mut state, mut res, creature, card) = setup(TypeSet::SORCERY);
        let version = state.object(creature).unwrap().version;
        let name = state.names.intern("explore ability");
        let ability = state.create_bare(
            PlayerId::new(0),
            ObjectKind::AbilityOnStack,
            name,
            ZoneLocation::Stack,
        );
        state
            .object_mut(ability)
            .unwrap()
            .riders
            .push(crate::object::Rider::AbilitySourceVersion(version));
        res.on_stack = ability;
        let _ = state.move_object(
            creature,
            ZoneLocation::Graveyard(PlayerId::new(1)),
            ZonePosition::Top,
            Cause::Effect,
        );
        assert!(
            matches!(super::super::run(&mut state, &mut res), super::super::Flow::Wait(Pending::Arrange { player, .. }) if player == PlayerId::new(1))
        );
        assert_eq!(
            state
                .object(creature)
                .unwrap()
                .counters
                .get(CounterKind::P1P1),
            0
        );
        assert!(matches!(
            super::super::resume_arranged(&mut state, &mut res, &[vec![], vec![card]]),
            super::super::Flow::Complete
        ));
        assert!(
            state
                .zones
                .list(ZoneLocation::Graveyard(PlayerId::new(1)))
                .contains(&card)
        );
    }

    #[test]
    fn entry_counter_grant_tracks_exact_spell_incarnation() {
        let (mut state, mut res, _, _) = setup(TypeSet::SORCERY);
        let player = PlayerId::new(0);
        let name = state.names.intern("watched spell");
        let spell = state.create_bare(player, ObjectKind::Spell, name, ZoneLocation::Stack);
        let version = state.object(spell).unwrap().version;
        let ability = state.create_bare(
            player,
            ObjectKind::AbilityOnStack,
            name,
            ZoneLocation::Stack,
        );
        state
            .object_mut(ability)
            .unwrap()
            .riders
            .push(crate::object::Rider::EventObjectIdentity(version, 0));
        res.on_stack = ability;
        res.event_object = Some(spell);
        super::super::bird_entry::grant(&mut state, &res, 1);
        assert!(
            state
                .object(spell)
                .unwrap()
                .riders
                .contains(&crate::object::Rider::EntryP1P1(1))
        );
        let _ = state.move_object(
            spell,
            ZoneLocation::Graveyard(player),
            ZonePosition::Top,
            Cause::Effect,
        );
        let _ = state.move_object(spell, ZoneLocation::Stack, ZonePosition::Top, Cause::Effect);
        super::super::bird_entry::grant(&mut state, &res, 1);
        assert!(
            !state
                .object(spell)
                .unwrap()
                .riders
                .iter()
                .any(|rider| matches!(rider, crate::object::Rider::EntryP1P1(_)))
        );
    }
    #[test]
    fn blinked_stolen_implicit_explorer_uses_its_old_controller_library() {
        for target in [TargetSpec::ThisObject, TargetSpec::EventObject] {
            let (mut state, mut res, _, card) = setup(TypeSet::SORCERY);
            let owner = PlayerId::new(0);
            let controller = PlayerId::new(1);
            let name = state.names.intern("stolen explorer");
            let explorer = state.create_bare(
                owner,
                ObjectKind::Permanent,
                name,
                ZoneLocation::Battlefield,
            );
            // A physical card can return from exile; a token cannot (CR 111.8).
            state.object_mut(explorer).unwrap().card = Some(crate::object::CardRef {
                index: baylee_core::ids::CardIndex::new(0),
                print: baylee_core::ids::PrintRef::new(0),
            });
            super::super::gain_control(&mut state, &[(explorer, controller)]);
            let version = state.object(explorer).unwrap().version;
            assert_eq!(state.object(explorer).unwrap().controller, controller);
            let ability =
                state.create_bare(owner, ObjectKind::AbilityOnStack, name, ZoneLocation::Stack);
            state.object_mut(ability).unwrap().riders.extend([
                crate::object::Rider::AbilitySourceVersion(version),
                crate::object::Rider::EventObjectIdentity(version, 0),
            ]);
            res.source = explorer;
            res.on_stack = ability;
            res.event_object = Some(explorer);
            res.effects = vec![baylee_cards_dsl::Effect::Explore { target }];
            let mut blink = res.clone();
            blink.effects = vec![baylee_cards_dsl::Effect::Blink {
                target: TargetSpec::Object(&baylee_cards_dsl::Filter::Any),
                owner_control: true,
            }];
            blink.targets.push(explorer);
            blink.targeted = true;
            assert!(matches!(
                super::super::run(&mut state, &mut blink),
                super::super::Flow::Complete
            ));
            assert_eq!(state.object(explorer).unwrap().controller, owner);
            assert_eq!(
                state.object(explorer).unwrap().zone,
                crate::zone::Zone::Battlefield
            );
            assert_ne!(state.object(explorer).unwrap().version, version);
            let super::super::Flow::Wait(Pending::Arrange { player, cards, .. }) =
                super::super::run(&mut state, &mut res)
            else {
                panic!("old controller explores");
            };
            assert_eq!(player, controller);
            assert_eq!(cards, vec![card]);
            assert_eq!(
                state
                    .object(explorer)
                    .unwrap()
                    .counters
                    .get(CounterKind::P1P1),
                0
            );
            assert!(matches!(
                super::super::resume_arranged(&mut state, &mut res, &[vec![], vec![card]]),
                super::super::Flow::Complete
            ));
            assert!(
                state
                    .zones
                    .list(ZoneLocation::Graveyard(controller))
                    .contains(&card)
            );
        }
    }
}
