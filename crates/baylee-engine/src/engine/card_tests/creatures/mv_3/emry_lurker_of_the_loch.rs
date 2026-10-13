//! `cards/creatures/mv_3/emry_lurker_of_the_loch.rs`, played: the clauses
//! the whole-game test in `creatures/mv_4/clone.rs` leaves open.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as kit;
use baylee_core::generated::index;

/// Whether Emry is on the castable list when `own` artifacts stand on the
/// caster's side, `theirs` on the opponent's, and exactly {U}{U} floats.
///
/// {2}{U} is three mana, so two floating mana cast her only if affinity took
/// at least one off: the answer is the number of artifacts counted.
fn emry_castable_off_two_blue(own: usize, theirs: usize) -> bool {
    let p0 = PlayerId::new(0);
    let artifact = quiet_artifact();
    let (mine, others) = (vec![artifact; own], vec![artifact; theirs]);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &mine)
        .battlefield(1, &others)
        .hand(0, &[index::EMRY_LURKER_OF_THE_LOCH])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    kit::float(&mut e, p0, &[(ManaColor::Blue, 2)]);
    kit::castable(&e, p0, index::EMRY_LURKER_OF_THE_LOCH)
}

/// "Affinity for artifacts": one less for each artifact YOU control. The
/// opponent's artifacts are on the table and cost her nothing less.
#[test]
fn emry_affinity_counts_only_artifacts_you_control() {
    assert!(
        !emry_castable_off_two_blue(0, 0),
        "no artifact, no reduction: {{2}}{{U}} is three mana"
    );
    assert!(
        emry_castable_off_two_blue(1, 0),
        "one artifact of your own makes her {{1}}{{U}}, which two mana pay"
    );
    assert!(
        !emry_castable_off_two_blue(0, 2),
        "two artifacts on the opponent's side are not artifacts you control"
    );
}

/// "When Emry enters, mill four cards": the controller's own library, not
/// the opponent's.
#[test]
fn emry_mills_four_cards_from_her_controllers_library_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[index::EMRY_LURKER_OF_THE_LOCH])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let (mine, theirs) = (library_size(&e, p0), library_size(&e, p1));
    kit::float(
        &mut e,
        p0,
        &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
    );
    cast_with_floating(&mut e, p0, index::EMRY_LURKER_OF_THE_LOCH);
    pass_until(&mut e, |e| {
        on_battlefield(e, p0, index::EMRY_LURKER_OF_THE_LOCH).is_some() && stack_is_empty(e)
    });
    assert_eq!(
        library_size(&e, p0),
        mine - 4,
        "four cards left the controller's library"
    );
    assert_eq!(
        e.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        4,
        "and are in the controller's graveyard"
    );
    assert_eq!(
        library_size(&e, p1),
        theirs,
        "the opponent's library is as it was"
    );
    assert!(
        e.state().zones.list(ZoneLocation::Graveyard(p1)).is_empty(),
        "and nothing of theirs was milled"
    );
}

/// "Choose target artifact card in your graveyard": a card that is not an
/// artifact is not a target, and neither is an artifact in the opponent's
/// graveyard.
#[test]
fn emry_targets_only_artifact_cards_in_her_controllers_graveyard() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut e = kit::game(
        SEED,
        &[index::EMRY_LURKER_OF_THE_LOCH],
        &[],
        &[index::MIND_STONE, index::GRIZZLY_BEARS],
        &[index::SOL_RING],
    );
    let stone = hand_to_graveyard(&mut e, p0, index::MIND_STONE);
    let bears = hand_to_graveyard(&mut e, p0, index::GRIZZLY_BEARS);
    let ring = hand_to_graveyard(&mut e, p1, index::SOL_RING);

    activate(&mut e, p0, index::EMRY_LURKER_OF_THE_LOCH, 1);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected the target choice, got {:?}", e.pending())
    };
    assert!(
        options.contains(&stone),
        "an artifact card in your graveyard is the target: {options:?}"
    );
    assert!(
        !options.contains(&bears),
        "a creature card that is no artifact is not: {options:?}"
    );
    assert!(
        !options.contains(&ring),
        "an artifact card in the opponent's graveyard is not \"in your graveyard\": {options:?}"
    );
}

/// "{T}: ... You may cast that card this turn": the {T} is a cost, and "this
/// turn" ends with the turn.
#[test]
fn emry_taps_for_her_permission_and_it_lapses_when_the_turn_ends() {
    let p0 = kit::P0;
    let mut e = kit::game(
        SEED,
        &[index::EMRY_LURKER_OF_THE_LOCH],
        &[],
        &[index::MIND_STONE],
        &[],
    );
    let emry = kit::obj(&e, p0, index::EMRY_LURKER_OF_THE_LOCH);
    let stone = hand_to_graveyard(&mut e, p0, index::MIND_STONE);
    assert!(kit::offered(&e, index::EMRY_LURKER_OF_THE_LOCH, p0, 1));

    kit::use_ability(&mut e, p0, index::EMRY_LURKER_OF_THE_LOCH, 1, &[stone], &[]);
    let permitted = |e: &Engine<RegistryLookup>| {
        crate::casting::graveyard_cast_permission(e.state(), p0, e.state().object(stone).unwrap())
            .is_some()
    };
    assert!(permitted(&e), "the chosen card may be cast this turn");
    assert!(kit::tapped(&e, emry), "{{T}} is the cost of the ability");
    assert!(
        !kit::offered(&e, index::EMRY_LURKER_OF_THE_LOCH, p0, 1),
        "a tapped Emry cannot choose a second card"
    );

    let turn = e.state().turn.number;
    pass_until(&mut e, |e| {
        e.state().turn.number > turn
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(
        !permitted(&e),
        "\"this turn\" is over: the card is castable from the graveyard no more"
    );
    assert!(
        kit::offered(&e, index::EMRY_LURKER_OF_THE_LOCH, p0, 1),
        "and Emry, untapped, is ready to choose again"
    );
}
