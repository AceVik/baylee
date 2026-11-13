//! `cards/creatures/mv_3/vraska_the_silencer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_card_testkit::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::{index, subtypes};
use baylee_engine::event::Cause;
use baylee_engine::zone::ZonePosition;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn vraska() -> CardIndex {
    card_index("e7c5a568-3855-4a20-9edf-01a8a6a4ba97")
}

#[track_caller]
fn game() -> Engine<RegistryLookup> {
    lk::game(SEED, &[vraska()], &[index::LLANOWAR_ELVES], &[], &[])
}

#[track_caller]
fn destroy(e: &mut Engine<RegistryLookup>, creature: ObjectId) {
    baylee_engine::sba::destroy(e.dev_state_mut(P0).unwrap(), creature);
}

#[track_caller]
fn pay_and_settle(e: &mut Engine<RegistryLookup>, pay: bool) {
    pass_until(e, |e| !stack_is_empty(e));
    kit::resolve_the_stack(e, |_, pending| match pending {
        Pending::YesNo {
            prompt: YesNoPrompt::PayMana { cost },
            ..
        } => {
            assert_eq!(*cost, baylee_core::mana!("{1}"));
            Some(PlayerAction::YesNo(pay))
        }
        _ => None,
    });
}

#[test]
fn deathtouch_makes_its_combat_damage_lethal() {
    let mut e = lk::game(SEED, &[vraska()], &[giant_spider()], &[], &[]);
    let attacker = lk::obj(&e, P0, vraska());
    let blocker = lk::obj(&e, P1, giant_spider());
    lk::attack(&mut e, &[attacker]);
    lk::block(&mut e, &[(blocker, attacker)]);
    kit::drive(
        &mut e,
        |e| e.state().turn.phase == Phase::SecondMain,
        |_, pending| matches!(pending, Pending::YesNo { .. }).then_some(PlayerAction::YesNo(false)),
    );
    assert_eq!(lk::zone_of(&e, blocker), Zone::Graveyard);
    assert_eq!(lk::damage_on(&e, attacker), 2);
}

#[test]
fn the_optional_one_mana_controls_whether_the_death_card_returns() {
    for (pay, funded, returns) in [
        (false, false, false),
        (true, false, false),
        (true, true, true),
    ] {
        let mut e = game();
        if funded {
            lk::float(&mut e, P0, &[(ManaColor::Colorless, 1)]);
        }
        let elf = lk::obj(&e, P1, index::LLANOWAR_ELVES);
        destroy(&mut e, elf);
        pay_and_settle(&mut e, pay);

        assert_eq!(lk::zone_of(&e, elf) == Zone::Battlefield, returns);
        assert_eq!(lk::pool_total(&e, P0), 0);
        if returns {
            let object = e.state().object(elf).unwrap();
            assert_eq!((object.owner, object.controller), (P1, P0));
            assert!(lk::tapped(&e, elf));
        }
    }
}

#[test]
fn the_return_is_a_treasure_artifact_and_keeps_its_printed_mana_ability() {
    let mut e = game();
    lk::float(&mut e, P0, &[(ManaColor::Colorless, 1)]);
    let elf = lk::obj(&e, P1, index::LLANOWAR_ELVES);
    destroy(&mut e, elf);
    pay_and_settle(&mut e, true);

    let object = e.state().object(elf).unwrap();
    let characteristics = object.characteristics();
    assert_eq!(characteristics.types, TypeSet::ARTIFACT);
    assert!(
        characteristics
            .subtypes
            .contains(subtypes::artifact::TREASURE)
    );
    assert!(!characteristics.subtypes.contains(subtypes::creature::ELF));
    assert!(!characteristics.subtypes.contains(subtypes::creature::DRUID));
    assert!(
        !lk::offered(&e, index::LLANOWAR_ELVES, P0, 0),
        "both printed and granted Treasure mana require an untapped artifact"
    );

    kit::walk_to_next_own_main(&mut e, P0);
    assert!(!lk::tapped(&e, elf), "the ordinary untap step readies it");
    assert!(
        lk::offered(&e, index::LLANOWAR_ELVES, P0, 0),
        "losing the creature types does not remove its printed mana ability"
    );
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority after the untap: {:?}", e.pending())
    };
    assert!(legal.abilities.iter().any(|(source, ability)| {
        *source == elf && baylee_engine::choice::granted_slot(*ability).is_some()
    }));
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: elf,
            ability_index: baylee_engine::choice::granted_ability(0),
        },
    )
    .unwrap();
    let Pending::ChooseColor { options, .. } = e.pending().clone() else {
        panic!("Treasure asks for a color: {:?}", e.pending())
    };
    assert_eq!(
        options,
        [
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Green,
        ]
    );
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    assert_eq!(lk::pool_of(&e, P0, ManaColor::Green), 1);
    assert_eq!(lk::pool_total(&e, P0), 1);
    assert_eq!(lk::zone_of(&e, elf), Zone::Graveyard);
}

#[test]
fn neither_my_creature_nor_an_opponents_token_triggers_vraska() {
    let mut e = lk::game(
        SEED,
        &[vraska(), index::LLANOWAR_ELVES],
        &[],
        &[],
        &[index::RAISE_THE_ALARM],
    );
    let own = lk::obj(&e, P0, index::LLANOWAR_ELVES);
    reach_their_main_phase(&mut e, P1);
    lk::float(
        &mut e,
        P1,
        &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
    );
    cast_with_floating(&mut e, P1, index::RAISE_THE_ALARM);
    lk::settle(&mut e);
    let token = tokens_of(&e, P1)[0];
    destroy(&mut e, own);
    destroy(&mut e, token);
    lk::settle(&mut e);
    assert!(stack_is_empty(&e));
    assert_eq!(lk::zone_of(&e, own), Zone::Graveyard);
    assert!(
        e.state()
            .object(token)
            .is_none_or(|o| o.zone != Zone::Battlefield)
    );
    assert!(on_battlefield(&e, P0, vraska()).is_some());
}

#[test]
fn a_new_graveyard_incarnation_is_not_returned_and_retains_no_modifiers() {
    let mut e = game();
    lk::float(&mut e, P0, &[(ManaColor::Colorless, 1)]);
    let elf = lk::obj(&e, P1, index::LLANOWAR_ELVES);
    destroy(&mut e, elf);
    pass_until(&mut e, |e| !stack_is_empty(e));
    let dead_version = e.state().object(elf).unwrap().version;
    let state = e.dev_state_mut(P0).unwrap();
    state
        .move_object(
            elf,
            ZoneLocation::Exile(P1),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            elf,
            ZoneLocation::Graveyard(P1),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    pay_and_settle(&mut e, true);

    assert_eq!(lk::zone_of(&e, elf), Zone::Graveyard);
    assert!(e.state().object(elf).unwrap().version > dead_version);
    assert!(!e.state().effects.iter().any(|effect| {
        matches!(effect.filter, baylee_engine::effects::EffectFilter::ObjectIs(id, _) if id == elf)
    }));
}
