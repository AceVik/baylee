//! `cards/instants/mv_1/coffin_purge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "9db5aa54-3deb-45ee-82a2-e88a4367c6d9"
fn coffin_purge() -> CardIndex {
    card_index("9db5aa54-3deb-45ee-82a2-e88a4367c6d9")
}

/// Coffin Purge ({B}, Flashback {B}): casts from hand to exile a target card
/// from an opponent's graveyard (landing in your graveyard), then casts via
/// flashback from your graveyard to exile another card and exiles itself.
#[test]
fn coffin_purge_exiles_from_any_graveyard_from_hand_and_flashback() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), mountain()])
        .hand(0, &[coffin_purge(), shock()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Put Grizzly Bears into P1's graveyard and Shock into P0's graveyard.
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let mtn = lk::obj(&engine, P0, mountain());
    tap_mana_where(&mut engine, P0, |id| id == mtn);
    lk::cast_at(&mut engine, P0, shock(), &[bears], &[]);
    let dead_bears = in_graveyard(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    let spent_shock = in_graveyard(&engine, P0, shock()).unwrap();

    // 1. Cast Coffin Purge from hand targeting P1's graveyard card.
    let swamps = all_on_battlefield(&engine, P0, swamp());
    tap_mana_where(&mut engine, P0, |id| id == swamps[0]);
    cast_with_floating(&mut engine, P0, coffin_purge());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&dead_bears) && options.contains(&spent_shock));
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![dead_bears],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(P1))
            .contains(&dead_bears),
        "P1's Bears was exiled from graveyard"
    );
    let gy_purge = in_graveyard(&engine, P0, coffin_purge()).expect("Coffin Purge in GY");

    // 2. Cast Coffin Purge via Flashback {B} targeting P0's own Shock in GY.
    tap_mana_where(&mut engine, P0, |id| id == swamps[1]);
    engine
        .apply(P0, PlayerAction::CastSpell { card: gy_purge })
        .expect("flashback castable from graveyard for {B}");
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let fb = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine.apply(P0, PlayerAction::ChooseMode(fb)).unwrap();
    }
    lk::aim(&mut engine, &[spent_shock], &[]);
    lk::settle(&mut engine);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(P0))
            .contains(&spent_shock),
        "second target card was exiled"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(P0))
            .contains(&gy_purge),
        "flashed-back Coffin Purge was exiled instead of returning to graveyard"
    );
    assert!(in_graveyard(&engine, P0, coffin_purge()).is_none());
}
