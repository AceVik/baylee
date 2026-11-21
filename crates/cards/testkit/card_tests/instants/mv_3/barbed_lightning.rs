//! `cards/instants/mv_3/barbed_lightning.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "f6e1eb52-9f13-425c-8309-1b08ed6557db"
fn barbed_lightning() -> CardIndex {
    card_index("f6e1eb52-9f13-425c-8309-1b08ed6557db")
}

/// Barbed Lightning ({2}{R}, Entwine {2}): Mode 0 hits only a creature; Mode 1
/// hits a player or planeswalker (excluding creatures); Entwine hits BOTH a
/// creature AND a player/planeswalker for {4}{R}.
#[test]
fn barbed_lightning_plays_creature_mode_planeswalker_mode_and_entwine() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[
            index::HILL_GIANT,
            index::GRIZZLY_BEARS,
            index::JACE_THE_MIND_SCULPTOR,
        ],
        &[barbed_lightning(), barbed_lightning(), barbed_lightning()],
        &[],
    );
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let jace = lk::obj(&engine, P1, index::JACE_THE_MIND_SCULPTOR);

    // 1. Mode 0: 3 damage to target creature (kills 3/3 Hill Giant).
    lk::float(&mut engine, P0, &[(ManaColor::Red, 3)]);
    cast_with_floating(&mut engine, P0, barbed_lightning());
    let m0 = choose_cast_kind(&engine, CastModeKind::Mode(0));
    engine.apply(P0, PlayerAction::ChooseMode(m0)).unwrap();
    lk::aim(&mut engine, &[giant], &[]);
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, P1, index::HILL_GIANT).is_some());
    assert_eq!(lk::life(&engine, P1), 20);

    // 2. Mode 1: 3 damage to target planeswalker (Jace 3 -> 0 loyalty -> GY).
    lk::float(&mut engine, P0, &[(ManaColor::Red, 3)]);
    cast_with_floating(&mut engine, P0, barbed_lightning());
    let m1 = choose_cast_kind(&engine, CastModeKind::Mode(1));
    engine.apply(P0, PlayerAction::ChooseMode(m1)).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected Mode 1 target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&jace));
    assert!(!options.contains(&bears), "Mode 1 excludes creatures");
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![jace],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, P1, index::JACE_THE_MIND_SCULPTOR).is_some());

    // 3. Entwine ({4}{R}): 3 damage to Bears AND 3 damage to P1.
    lk::float(&mut engine, P0, &[(ManaColor::Red, 5)]);
    cast_with_floating(&mut engine, P0, barbed_lightning());
    let both = choose_cast_kind(&engine, CastModeKind::Modes(3));
    engine.apply(P0, PlayerAction::ChooseMode(both)).unwrap();
    lk::aim(&mut engine, &[bears], &[]);
    lk::aim(&mut engine, &[], &[P1]);
    lk::settle(&mut engine);
    assert_eq!(lk::pool_total(&engine, P0), 0, "all {{4}}{{R}} spent");
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(lk::life(&engine, P1), 17, "Entwine also dealt 3 to P1");
}
