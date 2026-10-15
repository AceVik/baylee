//! Sevinne's Reclamation: return eligibility and printed flashback cost.
//! Graveyard-cast copies await the reusable API; coverage remains Partial.

#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;
use crate::choice::{TargetPrompt, YesNoPrompt};
use crate::engine::card_tests::legends_kit as kit;
use crate::event::GameEvent;

fn subject() -> CardIndex {
    card_index("1b9f9f5b-8712-4f00-90cb-1b7b9970eccc")
}

#[test]
fn hand_cast_returns_only_an_own_permanent_of_mana_value_at_most_three() {
    let mut e = kit::game(
        6803,
        &[],
        &[],
        &[
            subject(),
            gray_ogre(),
            wild_elephant(),
            lightning_bolt(),
            forest(),
        ],
        &[gray_ogre()],
    );
    let small = hand_to_graveyard(&mut e, kit::P0, gray_ogre());
    let land = hand_to_graveyard(&mut e, kit::P0, forest());
    let large = hand_to_graveyard(&mut e, kit::P0, wild_elephant());
    let instant = hand_to_graveyard(&mut e, kit::P0, lightning_bolt());
    let opposing = hand_to_graveyard(&mut e, kit::P1, gray_ogre());
    kit::float(&mut e, kit::P0, &[(ManaColor::White, 3)]);
    cast_with_floating(&mut e, kit::P0, subject());
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("graveyard target required: {:?}", e.pending());
    };
    assert!(options.contains(&small) && options.contains(&land));
    assert!(
        !options.contains(&large) && !options.contains(&instant) && !options.contains(&opposing)
    );
    kit::aim(&mut e, &[small], &[]);
    // A hand cast must not ask the graveyard-only copy question.
    pass_until(&mut e, stack_is_empty);
    assert_eq!(on_battlefield(&e, kit::P0, gray_ogre()), Some(small));
    assert_eq!(e.state().object(large).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().object(opposing).unwrap().zone, Zone::Graveyard);
    assert!(in_graveyard(&e, kit::P0, subject()).is_some());
}

#[test]
fn printed_flashback_requires_five_mana_and_exiles_after_resolving() {
    let mut e = kit::game(6804, &[], &[], &[subject(), gray_ogre()], &[]);
    let spell = hand_to_graveyard(&mut e, kit::P0, subject());
    let target = hand_to_graveyard(&mut e, kit::P0, gray_ogre());
    kit::float(&mut e, kit::P0, &[(ManaColor::White, 4)]);
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority");
    };
    assert!(
        !legal.castable.contains(&spell),
        "flashback is {{4}}{{W}}, not {{2}}{{W}}"
    );
    kit::float(&mut e, kit::P0, &[(ManaColor::White, 1)]);
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority");
    };
    assert!(legal.castable.contains(&spell));
    e.apply(kit::P0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    kit::aim(&mut e, &[target], &[]);
    finish_declining(&mut e);
    assert_eq!(on_battlefield(&e, kit::P0, gray_ogre()), Some(target));
    assert_eq!(e.state().object(spell).unwrap().zone, Zone::Exile);
    assert_eq!(kit::pool_total(&e, kit::P0), 0);
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
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::MayDo,
                ..
            } => e.apply(player, PlayerAction::YesNo(false)).unwrap(),
            other => panic!("unexpected Reclamation question: {other:?}"),
        }
    }
    panic!("Reclamation did not finish");
}

fn cast_graveyard(e: &mut Engine<RegistryLookup>, spell: ObjectId, target: ObjectId) {
    e.apply(kit::P0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    kit::aim(e, &[target], &[]);
}

fn copy_question(e: &mut Engine<RegistryLookup>) {
    pass_until(e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    assert!(
        matches!(e.pending(), Pending::YesNo { player, prompt: YesNoPrompt::MayDo, .. } if *player == kit::P0),
        "{:?}",
        e.pending()
    );
}

#[test]
fn flashback_returns_the_original_before_copy_choice_and_retargets_once() {
    let mut e = kit::game(
        6820,
        &[],
        &[],
        &[subject(), gray_ogre(), quiet_artifact()],
        &[],
    );
    let spell = hand_to_graveyard(&mut e, kit::P0, subject());
    let first = hand_to_graveyard(&mut e, kit::P0, gray_ogre());
    let second = hand_to_graveyard(&mut e, kit::P0, quiet_artifact());
    kit::float(&mut e, kit::P0, &[(ManaColor::White, 5)]);
    cast_graveyard(&mut e, spell, first);
    copy_question(&mut e);
    assert_eq!(e.state().object(first).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().object(second).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().object(spell).unwrap().zone, Zone::Stack);
    e.apply(kit::P0, PlayerAction::YesNo(true)).unwrap();
    let Pending::ChooseTargets {
        player,
        options,
        reason: TargetPrompt::Retarget { .. },
        ..
    } = e.pending()
    else {
        panic!("retarget: {:?}", e.pending());
    };
    assert_eq!(*player, kit::P0);
    assert!(options.contains(&second));
    assert!(
        !options.contains(&first),
        "original target is already on the battlefield"
    );
    kit::aim(&mut e, &[second], &[]);
    assert_eq!(e.state().object(spell).unwrap().zone, Zone::Exile);
    assert_eq!(copies(&e), 1);
    assert_eq!(e.state().per_turn.spells_cast, vec![1, 0]);
    // The copy must not offer another copy: it was created, not cast from a graveyard.
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(second).unwrap().zone, Zone::Battlefield);
    assert_eq!(copies(&e), 1);
    assert_eq!(e.state().per_turn.spells_cast, vec![1, 0]);
}

#[test]
fn flashback_can_decline_the_copy_without_returning_a_second_permanent() {
    let mut e = kit::game(
        6821,
        &[],
        &[],
        &[subject(), gray_ogre(), quiet_artifact()],
        &[],
    );
    let spell = hand_to_graveyard(&mut e, kit::P0, subject());
    let first = hand_to_graveyard(&mut e, kit::P0, gray_ogre());
    let second = hand_to_graveyard(&mut e, kit::P0, quiet_artifact());
    kit::float(&mut e, kit::P0, &[(ManaColor::White, 5)]);
    cast_graveyard(&mut e, spell, first);
    copy_question(&mut e);
    e.apply(kit::P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(first).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().object(second).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().object(spell).unwrap().zone, Zone::Exile);
    assert_eq!(copies(&e), 0);
}

#[test]
fn copy_may_keep_the_original_now_illegal_target_and_fail_to_resolve() {
    let mut e = kit::game(
        6822,
        &[],
        &[],
        &[subject(), gray_ogre(), quiet_artifact()],
        &[],
    );
    let spell = hand_to_graveyard(&mut e, kit::P0, subject());
    let first = hand_to_graveyard(&mut e, kit::P0, gray_ogre());
    let second = hand_to_graveyard(&mut e, kit::P0, quiet_artifact());
    kit::float(&mut e, kit::P0, &[(ManaColor::White, 5)]);
    cast_graveyard(&mut e, spell, first);
    copy_question(&mut e);
    e.apply(kit::P0, PlayerAction::YesNo(true)).unwrap();
    assert!(
        matches!(
            e.pending(),
            Pending::ChooseTargets {
                min: 0,
                reason: TargetPrompt::Retarget { .. },
                ..
            }
        ),
        "choosing new targets is optional even if the old target is illegal: {:?}",
        e.pending()
    );
    kit::aim(&mut e, &[], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(first).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().object(second).unwrap().zone, Zone::Graveyard);
    assert_eq!(copies(&e), 1);
}

#[test]
fn an_illegal_original_graveyard_target_prevents_the_copy_offer() {
    let mut e = kit::game(6823, &[], &[], &[subject(), gray_ogre()], &[]);
    let spell = hand_to_graveyard(&mut e, kit::P0, subject());
    let first = hand_to_graveyard(&mut e, kit::P0, gray_ogre());
    kit::float(&mut e, kit::P0, &[(ManaColor::White, 5)]);
    cast_graveyard(&mut e, spell, first);
    e.state
        .move_object(
            first,
            ZoneLocation::Exile(kit::P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(first).unwrap().zone, Zone::Exile);
    assert_eq!(e.state().object(spell).unwrap().zone, Zone::Exile);
    assert_eq!(copies(&e), 0);
}

#[test]
fn casting_from_graveyard_by_another_permission_also_enables_the_copy() {
    let mut e = kit::game(
        6824,
        &[],
        &[],
        &[subject(), gray_ogre(), forest(), yawgmoth_s_will()],
        &[],
    );
    let spell = hand_to_graveyard(&mut e, kit::P0, subject());
    let first = hand_to_graveyard(&mut e, kit::P0, gray_ogre());
    let second = hand_to_graveyard(&mut e, kit::P0, forest());
    kit::float(
        &mut e,
        kit::P0,
        &[(ManaColor::Black, 3), (ManaColor::White, 3)],
    );
    kit::cast_at(&mut e, kit::P0, yawgmoth_s_will(), &[], &[]);
    e.apply(kit::P0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    // If both ordinary permission and printed flashback are offered, choose
    // the ordinary route: only {2}{W} remains, so flashback is unaffordable.
    if matches!(e.pending(), Pending::ChooseCastMode { .. }) {
        let mode = choose_cast_kind(&e, CastModeKind::Normal);
        e.apply(kit::P0, PlayerAction::ChooseMode(mode)).unwrap();
    }
    kit::aim(&mut e, &[first], &[]);
    copy_question(&mut e);
    e.apply(kit::P0, PlayerAction::YesNo(true)).unwrap();
    kit::aim(&mut e, &[second], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(second).unwrap().zone, Zone::Battlefield);
    assert_eq!(copies(&e), 1);
}

#[test]
fn countering_a_flashback_cast_exiles_it_without_returning_or_copying() {
    let mut e = kit::game(6825, &[], &[], &[subject(), gray_ogre()], &[counterspell()]);
    let spell = hand_to_graveyard(&mut e, kit::P0, subject());
    let target = hand_to_graveyard(&mut e, kit::P0, gray_ogre());
    kit::float(&mut e, kit::P0, &[(ManaColor::White, 5)]);
    kit::float(&mut e, kit::P1, &[(ManaColor::Blue, 2)]);
    cast_graveyard(&mut e, spell, target);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == kit::P1),
    );
    cast_with_floating(&mut e, kit::P1, counterspell());
    kit::aim(&mut e, &[spell], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(target).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().object(spell).unwrap().zone, Zone::Exile);
    assert_eq!(copies(&e), 0);
}

#[test]
fn original_return_triggers_resolve_above_the_copy_after_retargeting() {
    let warden = baylee_cards::decks::by_name("Soul Warden").unwrap();
    let mut e = kit::game(
        6826,
        &[warden],
        &[],
        &[subject(), gray_ogre(), quiet_artifact()],
        &[],
    );
    let spell = hand_to_graveyard(&mut e, kit::P0, subject());
    let first = hand_to_graveyard(&mut e, kit::P0, gray_ogre());
    let second = hand_to_graveyard(&mut e, kit::P0, quiet_artifact());
    kit::float(&mut e, kit::P0, &[(ManaColor::White, 5)]);
    cast_graveyard(&mut e, spell, first);
    copy_question(&mut e);
    assert_eq!(
        kit::life(&e, kit::P0),
        20,
        "the return trigger cannot resolve during the original spell"
    );
    e.apply(kit::P0, PlayerAction::YesNo(true)).unwrap();
    kit::aim(&mut e, &[second], &[]);
    let stack = e.state().zones.list(ZoneLocation::Stack);
    assert_eq!(stack.len(), 2, "Soul Warden trigger above the copied spell");
    assert_eq!(
        e.state().object(*stack.last().unwrap()).unwrap().kind,
        crate::object::ObjectKind::AbilityOnStack
    );
    pass_until(&mut e, |e| kit::life(e, kit::P0) == 21);
    assert_eq!(
        e.state().object(second).unwrap().zone,
        Zone::Graveyard,
        "the copy has not resolved yet"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(second).unwrap().zone, Zone::Battlefield);
}

#[test]
fn a_copy_is_created_even_when_no_new_legal_target_remains() {
    let mut e = kit::game(6827, &[], &[], &[subject(), gray_ogre()], &[]);
    let spell = hand_to_graveyard(&mut e, kit::P0, subject());
    let target = hand_to_graveyard(&mut e, kit::P0, gray_ogre());
    kit::float(&mut e, kit::P0, &[(ManaColor::White, 5)]);
    cast_graveyard(&mut e, spell, target);
    copy_question(&mut e);
    e.apply(kit::P0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(
        copies(&e),
        1,
        "a lack of new targets does not forbid copying"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(target).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().object(spell).unwrap().zone, Zone::Exile);
}

#[test]
fn neither_hand_nor_flashback_is_offered_without_a_legal_return_target() {
    let mut e = kit::game(
        6828,
        &[],
        &[],
        &[subject(), subject(), lightning_bolt(), wild_elephant()],
        &[gray_ogre()],
    );
    let grave_spell = hand_to_graveyard(&mut e, kit::P0, subject());
    let hand_spell = in_hand(&e, kit::P0, subject()).unwrap();
    hand_to_graveyard(&mut e, kit::P0, lightning_bolt());
    hand_to_graveyard(&mut e, kit::P0, wild_elephant());
    hand_to_graveyard(&mut e, kit::P1, gray_ogre());
    kit::float(&mut e, kit::P0, &[(ManaColor::White, 8)]);
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority");
    };
    assert!(!legal.castable.contains(&hand_spell));
    assert!(!legal.castable.contains(&grave_spell));
}
