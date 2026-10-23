//! `cards/instants/mv_2/prismatic_boon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "aee5dda1-2181-40d5-9846-85d154e1d5da"
fn prismatic_boon() -> CardIndex {
    card_index("aee5dda1-2181-40d5-9846-85d154e1d5da")
}

#[test]
fn prismatic_boon_casts_with_x_targeting_a_creature() {
    let mut engine = lk::game(SEED, &[index::GRIZZLY_BEARS], &[], &[prismatic_boon()], &[]);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 1),
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
        ],
    );
    cast_with_floating(&mut engine, P0, prismatic_boon());
    engine.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    lk::settle(&mut engine);

    assert!(in_graveyard(&engine, P0, prismatic_boon()).is_some());
}

/// "Protection from the chosen color until end of turn": a red source cannot
/// target the creature, a green one can, and in the opponent's turn a red one
/// can again.
#[test]
fn prismatic_boon_protects_from_the_chosen_color_only_and_only_this_turn() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[],
        &[prismatic_boon(), index::LIGHTNING_BOLT, giant_growth()],
        &[index::LIGHTNING_BOLT],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 1),
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
        ],
    );
    cast_with_floating(&mut engine, P0, prismatic_boon());
    engine.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    lk::settle(&mut engine);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Red, 1), (ManaColor::Green, 1)],
    );
    cast_with_floating(&mut engine, P0, index::LIGHTNING_BOLT);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a target choice was expected, got {:?}", engine.pending());
    };
    assert!(
        !options.contains(&bears),
        "red Lightning Bolt cannot target a creature with protection from red"
    );
    lk::aim(&mut engine, &[], &[P1]);
    lk::settle(&mut engine);

    cast_with_floating(&mut engine, P0, giant_growth());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a target choice was expected, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&bears),
        "a green spell may still target it: the protection is from red only"
    );
    lk::aim(&mut engine, &[bears], &[]);
    lk::settle(&mut engine);

    reach_their_main_phase(&mut engine, P1);
    lk::float(&mut engine, P1, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P1, index::LIGHTNING_BOLT);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a target choice was expected, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&bears),
        "the protection ended with the turn"
    );
}
