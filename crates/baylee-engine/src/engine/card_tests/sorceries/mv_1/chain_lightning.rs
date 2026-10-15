//! Chain Lightning: baseline damage and targets; participant-controlled copies
//! await the reusable resolving-spell copy API. Coverage remains Partial.

#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;
use crate::choice::{TargetPrompt, YesNoPrompt};
use crate::engine::card_tests::legends_kit as kit;
use crate::event::GameEvent;

fn subject() -> CardIndex {
    card_index("8785f42f-87b9-4a0a-89ce-ea423649ba9c")
}

#[test]
fn damage_targets_include_either_player_and_either_sides_creatures() {
    let mut e = kit::game(
        6801,
        &[fangren_hunter()],
        &[gray_ogre(), quiet_artifact()],
        &[subject()],
        &[],
    );
    kit::float(&mut e, kit::P0, &[(ManaColor::Red, 1)]);
    let ours = kit::obj(&e, kit::P0, fangren_hunter());
    let theirs = kit::obj(&e, kit::P1, gray_ogre());
    let artifact = kit::obj(&e, kit::P1, quiet_artifact());
    cast_with_floating(&mut e, kit::P0, subject());
    let Pending::ChooseTargets {
        options,
        player_options,
        min,
        max,
        ..
    } = e.pending()
    else {
        panic!("any target required: {:?}", e.pending());
    };
    assert_eq!((*min, *max), (1, 1));
    assert!(options.contains(&ours) && options.contains(&theirs));
    assert!(!options.contains(&artifact));
    assert!(player_options.contains(&kit::P0) && player_options.contains(&kit::P1));
    kit::aim(&mut e, &[theirs], &[]);
    finish_declining(&mut e);
    assert!(in_graveyard(&e, kit::P1, gray_ogre()).is_some());
    assert_eq!(kit::damage_on(&e, ours), 0);
    assert_eq!(kit::life(&e, kit::P0), 20);
    assert_eq!(kit::life(&e, kit::P1), 20);
}

#[test]
fn damage_to_a_player_is_three_and_does_not_damage_their_creature() {
    let mut e = kit::game(6802, &[], &[fangren_hunter()], &[subject()], &[]);
    kit::float(&mut e, kit::P0, &[(ManaColor::Red, 1)]);
    announce(&mut e, &[], &[kit::P1]);
    finish_declining(&mut e);
    assert_eq!(kit::life(&e, kit::P1), 17);
    assert_eq!(kit::life(&e, kit::P0), 20);
    assert_eq!(
        kit::damage_on(&e, kit::obj(&e, kit::P1, fangren_hunter())),
        0
    );
}

fn announce(e: &mut Engine<RegistryLookup>, objects: &[ObjectId], players: &[PlayerId]) {
    cast_with_floating(e, kit::P0, subject());
    kit::aim(e, objects, players);
}

fn copies(e: &Engine<RegistryLookup>) -> usize {
    e.journal()
        .entries()
        .iter()
        .filter(|entry| matches!(entry.event, GameEvent::SpellCopied { .. }))
        .count()
}

fn finish_declining(e: &mut Engine<RegistryLookup>) {
    for _ in 0..80 {
        if stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. }) {
            return;
        }
        match e.pending().clone() {
            Pending::Priority { player, .. } => {
                e.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo { player, .. } => e.apply(player, PlayerAction::YesNo(false)).unwrap(),
            other => panic!("unexpected Chain Lightning choice: {other:?}"),
        }
    }
    panic!("Chain Lightning did not finish");
}

fn payment(e: &mut Engine<RegistryLookup>, expected: PlayerId) {
    pass_until(e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    assert!(
        matches!(e.pending(), Pending::YesNo { player, prompt: YesNoPrompt::PayMana { cost }, .. }
        if *player == expected && *cost == baylee_core::mana!("{R}{R}")),
        "{:?}",
        e.pending()
    );
}

fn accept_copy(e: &mut Engine<RegistryLookup>, expected: PlayerId) {
    e.apply(expected, PlayerAction::YesNo(true)).unwrap();
    assert!(
        matches!(e.pending(), Pending::YesNo { player, prompt: YesNoPrompt::MayDo, .. } if *player == expected),
        "{:?}",
        e.pending()
    );
    e.apply(expected, PlayerAction::YesNo(true)).unwrap();
}

#[test]
fn targeted_player_pays_controls_and_retargets_copy_which_can_chain_back() {
    let mut e = kit::game(6810, &[], &[], &[subject()], &[]);
    kit::float(&mut e, kit::P0, &[(ManaColor::Red, 3)]);
    kit::float(&mut e, kit::P1, &[(ManaColor::Red, 2)]);
    announce(&mut e, &[], &[kit::P1]);
    payment(&mut e, kit::P1);
    assert_eq!(
        kit::life(&e, kit::P1),
        17,
        "damage happens before the payment question"
    );
    accept_copy(&mut e, kit::P1);
    assert!(
        matches!(e.pending(), Pending::ChooseTargets { player, reason: TargetPrompt::Retarget { .. }, .. } if *player == kit::P1)
    );
    kit::aim(&mut e, &[], &[kit::P0]);
    assert_eq!(copies(&e), 1);
    let stack = e.state().zones.list(ZoneLocation::Stack);
    assert_eq!(
        stack.len(),
        1,
        "original finishes before the copy can resolve"
    );
    assert_eq!(e.state().object(stack[0]).unwrap().controller, kit::P1);
    assert_eq!(
        e.state().per_turn.spells_cast,
        vec![1, 0],
        "copying is not casting"
    );
    payment(&mut e, kit::P0);
    assert_eq!(kit::life(&e, kit::P0), 17);
    accept_copy(&mut e, kit::P0);
    kit::aim(&mut e, &[], &[kit::P1]);
    assert_eq!(
        copies(&e),
        2,
        "the copied spell retains its own chain clause"
    );
    finish_declining(&mut e);
    assert_eq!(kit::life(&e, kit::P1), 14);
    assert_eq!(e.state().per_turn.spells_cast, vec![1, 0]);
    assert_eq!(kit::pool_total(&e, kit::P0), 0);
    assert_eq!(kit::pool_total(&e, kit::P1), 0);
}

#[test]
fn payment_and_copy_are_separate_optional_choices() {
    for copy_payment in [false, true] {
        let mut e = kit::game(6811, &[], &[], &[subject()], &[]);
        kit::float(&mut e, kit::P0, &[(ManaColor::Red, 1)]);
        kit::float(&mut e, kit::P1, &[(ManaColor::Red, 2)]);
        announce(&mut e, &[], &[kit::P1]);
        payment(&mut e, kit::P1);
        e.apply(kit::P1, PlayerAction::YesNo(copy_payment)).unwrap();
        if copy_payment {
            assert!(
                matches!(e.pending(), Pending::YesNo { player, prompt: YesNoPrompt::MayDo, .. } if *player == kit::P1)
            );
            e.apply(kit::P1, PlayerAction::YesNo(false)).unwrap();
        }
        finish_declining(&mut e);
        assert_eq!(copies(&e), 0);
        assert_eq!(kit::life(&e, kit::P1), 17);
        assert_eq!(
            kit::pool_total(&e, kit::P1),
            if copy_payment { 0 } else { 2 }
        );
    }
}

#[test]
fn a_lethally_damaged_permanents_controller_decides_before_state_based_death() {
    let mut e = kit::game(6812, &[], &[gray_ogre()], &[subject()], &[]);
    kit::float(&mut e, kit::P0, &[(ManaColor::Red, 1)]);
    kit::float(&mut e, kit::P1, &[(ManaColor::Red, 2)]);
    let target = kit::obj(&e, kit::P1, gray_ogre());
    announce(&mut e, &[target], &[]);
    payment(&mut e, kit::P1);
    assert_eq!(e.state().object(target).unwrap().zone, Zone::Battlefield);
    assert_eq!(kit::damage_on(&e, target), 3);
    accept_copy(&mut e, kit::P1);
    kit::aim(&mut e, &[], &[kit::P0]);
    assert_eq!(e.state().object(target).unwrap().zone, Zone::Graveyard);
    finish_declining(&mut e);
    assert_eq!(kit::life(&e, kit::P0), 17);
}

#[test]
fn a_copy_can_keep_its_original_player_target() {
    let mut e = kit::game(6813, &[], &[], &[subject()], &[]);
    kit::float(&mut e, kit::P0, &[(ManaColor::Red, 1)]);
    kit::float(&mut e, kit::P1, &[(ManaColor::Red, 2)]);
    announce(&mut e, &[], &[kit::P1]);
    payment(&mut e, kit::P1);
    accept_copy(&mut e, kit::P1);
    assert!(
        matches!(e.pending(), Pending::ChooseTargets { player, min: 0, reason: TargetPrompt::Retarget { .. }, .. } if *player == kit::P1)
    );
    kit::aim(&mut e, &[], &[]);
    finish_declining(&mut e);
    assert_eq!(kit::life(&e, kit::P1), 14);
    assert_eq!(copies(&e), 1);
}

#[test]
fn an_illegal_original_target_stops_damage_payment_and_copying() {
    let mut e = kit::game(6814, &[], &[gray_ogre()], &[subject()], &[]);
    kit::float(&mut e, kit::P0, &[(ManaColor::Red, 1)]);
    kit::float(&mut e, kit::P1, &[(ManaColor::Red, 2)]);
    let target = kit::obj(&e, kit::P1, gray_ogre());
    announce(&mut e, &[target], &[]);
    // Fixture models a response removing the target before resolution.
    e.state
        .move_object(
            target,
            ZoneLocation::Hand(kit::P1),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(copies(&e), 0);
    assert_eq!(kit::pool_total(&e, kit::P1), 2);
    assert_eq!(kit::life(&e, kit::P1), 20);
}

#[test]
fn insufficient_red_cannot_pay_partially_or_use_the_casters_mana() {
    let mut e = kit::game(6815, &[], &[], &[subject()], &[]);
    kit::float(&mut e, kit::P0, &[(ManaColor::Red, 5)]);
    kit::float(
        &mut e,
        kit::P1,
        &[(ManaColor::Red, 1), (ManaColor::Blue, 1)],
    );
    announce(&mut e, &[], &[kit::P1]);
    payment(&mut e, kit::P1);
    // Accepting the offer attempts payment. With no mana abilities and
    // insufficient red it legally takes the unpaid branch, spending nothing.
    e.apply(kit::P1, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(kit::pool_of(&e, kit::P1, ManaColor::Red), 1);
    assert_eq!(kit::pool_of(&e, kit::P1, ManaColor::Blue), 1);
    assert_eq!(kit::pool_of(&e, kit::P0, ManaColor::Red), 4);
    // No successful payment means no optional-copy question can be asked.
    pass_until(&mut e, stack_is_empty);
    assert_eq!(copies(&e), 0);
}

#[test]
fn targeting_your_own_creature_asks_you_and_not_the_opponent() {
    let mut e = kit::game(6816, &[fangren_hunter()], &[], &[subject()], &[]);
    kit::float(&mut e, kit::P0, &[(ManaColor::Red, 3)]);
    kit::float(&mut e, kit::P1, &[(ManaColor::Red, 2)]);
    let target = kit::obj(&e, kit::P0, fangren_hunter());
    announce(&mut e, &[target], &[]);
    payment(&mut e, kit::P0);
    finish_declining(&mut e);
    assert_eq!(kit::damage_on(&e, target), 3);
    assert_eq!(kit::pool_total(&e, kit::P1), 2);
}

#[test]
fn prevented_damage_does_not_remove_the_payment_and_copy_option() {
    let protected = baylee_cards::decks::by_name("Bronze Horse").unwrap();
    let mut e = kit::game(6817, &[], &[protected, gray_ogre()], &[subject()], &[]);
    kit::float(&mut e, kit::P0, &[(ManaColor::Red, 1)]);
    kit::float(&mut e, kit::P1, &[(ManaColor::Red, 2)]);
    let target = kit::obj(&e, kit::P1, protected);
    announce(&mut e, &[target], &[]);
    payment(&mut e, kit::P1);
    assert_eq!(
        kit::damage_on(&e, target),
        0,
        "targeted spell damage to Bronze Horse is prevented"
    );
    accept_copy(&mut e, kit::P1);
    kit::aim(&mut e, &[], &[kit::P0]);
    finish_declining(&mut e);
    assert_eq!(kit::life(&e, kit::P0), 17);
    assert_eq!(copies(&e), 1);
}

#[test]
fn the_targets_current_controller_decides_rather_than_its_owner() {
    let control = baylee_cards::decks::by_name("Control Magic").unwrap();
    let mut e = kit::game(6818, &[], &[fangren_hunter()], &[subject(), control], &[]);
    let target = kit::obj(&e, kit::P1, fangren_hunter());
    kit::float(
        &mut e,
        kit::P0,
        &[(ManaColor::Blue, 4), (ManaColor::Red, 3)],
    );
    kit::float(&mut e, kit::P1, &[(ManaColor::Red, 2)]);
    kit::cast_at(&mut e, kit::P0, control, &[target], &[]);
    assert_eq!(e.state().object(target).unwrap().owner, kit::P1);
    assert_eq!(e.state().object(target).unwrap().controller, kit::P0);
    announce(&mut e, &[target], &[]);
    payment(&mut e, kit::P0);
    accept_copy(&mut e, kit::P0);
    kit::aim(&mut e, &[], &[kit::P1]);
    finish_declining(&mut e);
    assert_eq!(kit::life(&e, kit::P1), 17);
    assert_eq!(kit::pool_total(&e, kit::P1), 2);
}
