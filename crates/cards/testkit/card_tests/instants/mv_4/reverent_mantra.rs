//! `cards/instants/mv_4/reverent_mantra.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "b4f7b1eb-6cd4-40f9-b259-3c39291edf27"
fn reverent_mantra() -> CardIndex {
    card_index("b4f7b1eb-6cd4-40f9-b259-3c39291edf27")
}

/// Reverent Mantra ({3}{W}): may be cast by exiling a white card from hand
/// (rejecting non-white cards); chooses a color and grants ALL creatures
/// protection from that color until end of turn, then wears off next turn.
#[test]
fn reverent_mantra_pitches_white_card_and_protects_all_creatures_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[reverent_mantra(), index::SAVANNAH_LIONS, shock(), shock()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let lions = in_hand(&engine, P0, index::SAVANNAH_LIONS).unwrap();
    let spell = in_hand(&engine, P0, reverent_mantra()).unwrap();

    engine
        .apply(P0, PlayerAction::CastSpell { card: spell })
        .expect("castable for free by pitching a white card");
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected pitch card choice, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![lions], "only the white card is a legal pitch");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![lions],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    lk::settle(&mut engine);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(P0))
            .contains(&lions),
        "pitched white card was exiled"
    );

    // Neither P0's Bears nor P1's Giant can be targeted by red Shock this turn.
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P0, shock());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected Shock target choice, got {:?}", engine.pending());
    };
    assert!(
        !options.contains(&bears) && !options.contains(&giant),
        "all creatures have protection from red until EOT: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    // Next turn protection has worn off and Shock can target Bears again.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, P0, shock(), &[bears], &[]);
    assert!(
        in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some(),
        "protection wore off at end of turn"
    );
}

/// Without another white card in hand and no mana, Reverent Mantra cannot be
/// cast; with {3}{W} floating it casts normally without exiling anything.
#[test]
fn reverent_mantra_requires_white_pitch_or_four_mana() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[],
        &[reverent_mantra(), shock()],
        &[],
    );
    assert!(
        !lk::castable(&engine, P0, reverent_mantra()),
        "a red card cannot pay the white pitch cost"
    );

    lk::float(&mut engine, P0, &[(ManaColor::White, 4)]);
    cast_with_floating(&mut engine, P0, reverent_mantra());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(lk::pool_total(&engine, P0), 0, "{{3}}{{W}} spent");
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(P0))
            .is_empty(),
        "normal mana cast exiles nothing"
    );
}
