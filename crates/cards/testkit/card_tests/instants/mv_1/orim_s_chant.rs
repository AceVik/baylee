//! `cards/instants/mv_1/orim_s_chant.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "26d3a818-7faf-443a-a6bd-9773edaedf17"
fn orim_s_chant() -> CardIndex {
    card_index("26d3a818-7faf-443a-a6bd-9773edaedf17")
}

/// Orim's Chant ({W}, Kicker {W}): unkicked prevents target player from casting
/// spells this turn while still allowing creatures to attack, and wears off
/// next turn.
#[test]
fn orims_chant_unkicked_stops_spells_only_and_allows_attacks() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[orim_s_chant()])
        .battlefield(1, &[mountain(), forest(), forest(), index::HILL_GIANT])
        .hand(1, &[lightning_bolt(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    // P1 passes priority in FirstMain so P0 casts unkicked Orim's Chant on P1.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, orim_s_chant());
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Back in P1's FirstMain: P1 cannot cast either Bolt or Bears.
    tap_all_mana(&mut engine, P1);
    assert!(!lk::castable(&engine, P1, lightning_bolt()));
    assert!(!lk::castable(&engine, P1, index::GRIZZLY_BEARS));

    // Unkicked does NOT prevent attacking.
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let _ = lk::attack(&mut engine, &[giant]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(lk::life(&engine, P0), 17, "unkicked Chant allows attacking");

    // Next turn P1 can cast spells again.
    reach_their_main_phase(&mut engine, P0);
    reach_their_main_phase(&mut engine, P1);
    tap_all_mana(&mut engine, P1);
    assert!(lk::castable(&engine, P1, lightning_bolt()));
}

/// When kicked for {W}, Orim's Chant stops target player from casting spells
/// AND prevents creatures from attacking this turn.
#[test]
fn orims_chant_kicked_stops_spells_and_prevents_attacks() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[orim_s_chant()])
        .battlefield(1, &[mountain(), index::HILL_GIANT])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, orim_s_chant());
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(lk::pool_total(&engine, P0), 0, "both {{W}}{{W}} spent");
    tap_all_mana(&mut engine, P1);
    assert!(!lk::castable(&engine, P1, lightning_bolt()));

    // Entering combat: no attackers are offered because creatures can't attack.
    let offered = lk::attackers_offered(&mut engine);
    assert!(
        offered.is_empty(),
        "kicked Orim's Chant prevents all creatures from attacking: {offered:?}"
    );
}
