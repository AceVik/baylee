//! `cards/instants/mv_4/sivvi_s_ruse.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "c5edfaae-3ecd-41ad-970f-8cecb09bf1e4"
fn sivvi_s_ruse() -> CardIndex {
    card_index("c5edfaae-3ecd-41ad-970f-8cecb09bf1e4")
}

/// Sivvi's Ruse ({2}{W}{W}): free alternative cost requires BOTH an opponent's
/// Mountain AND your own Plains.
#[test]
fn sivvis_ruse_free_cost_requires_own_plains_and_opponent_mountain() {
    let only_plains = lk::game(SEED, &[plains()], &[], &[sivvi_s_ruse()], &[]);
    assert!(
        !lk::castable(&only_plains, P0, sivvi_s_ruse()),
        "Plains alone is insufficient without opponent's Mountain"
    );

    let only_mountain = lk::game(SEED, &[], &[mountain()], &[sivvi_s_ruse()], &[]);
    assert!(
        !lk::castable(&only_mountain, P0, sivvi_s_ruse()),
        "opponent's Mountain alone is insufficient without own Plains"
    );
}

/// Cast for free with Plains + opponent's Mountain: prevents BOTH noncombat
/// and combat damage to creatures you control this turn, while leaving
/// opponent's creatures and players unshielded, and wears off next turn.
#[test]
fn sivvis_ruse_casts_free_and_prevents_all_damage_to_own_creatures_until_eot() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), index::GRIZZLY_BEARS])
        .hand(0, &[sivvi_s_ruse(), lightning_bolt(), lightning_bolt()])
        .battlefield(1, &[mountain(), index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let spell = in_hand(&engine, P0, sivvi_s_ruse()).unwrap();

    // Free cast with 0 mana floating.
    engine
        .apply(P0, PlayerAction::CastSpell { card: spell })
        .expect("Plains + opponent's Mountain enables free cast");
    lk::settle(&mut engine);

    // Noncombat damage to own creature is prevented; to player is not.
    lk::float(&mut engine, P0, &[(ManaColor::Red, 2)]);
    lk::cast_at(&mut engine, P0, lightning_bolt(), &[bears], &[]);
    assert_eq!(
        lk::damage_on(&engine, bears),
        0,
        "Bolt to own creature prevented"
    );
    lk::cast_at(&mut engine, P0, lightning_bolt(), &[], &[P0]);
    assert_eq!(
        lk::life(&engine, P0),
        17,
        "damage to player is not prevented"
    );

    // Combat damage: Bears attacks and is blocked by Giant -> Bears survives
    // unharmed while Giant still takes 2 damage from Bears.
    let _ = lk::attack(&mut engine, &[bears]);
    lk::block(&mut engine, &[(giant, bears)]);
    lk::through_combat(&mut engine);
    assert_eq!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS),
        Some(bears),
        "combat damage to own creature prevented"
    );
    assert_eq!(lk::damage_on(&engine, bears), 0);
    assert_eq!(
        lk::damage_on(&engine, giant),
        2,
        "opponent's creature still takes combat damage"
    );

    // Next turn prevention has expired.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let mut next_turn_hand = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), index::GRIZZLY_BEARS])
        .battlefield(1, &[mountain()])
        .hand(0, &[sivvi_s_ruse(), lightning_bolt()])
        .start();
    keep_mulligans(&mut next_turn_hand);
    reach_main_phase(&mut next_turn_hand, P0);
    let bears2 = lk::obj(&next_turn_hand, P0, index::GRIZZLY_BEARS);
    let spell2 = in_hand(&next_turn_hand, P0, sivvi_s_ruse()).unwrap();
    next_turn_hand
        .apply(P0, PlayerAction::CastSpell { card: spell2 })
        .unwrap();
    lk::settle(&mut next_turn_hand);
    reach_their_main_phase(&mut next_turn_hand, P1);
    reach_their_main_phase(&mut next_turn_hand, P0);
    lk::float(&mut next_turn_hand, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut next_turn_hand, P0, lightning_bolt(), &[bears2], &[]);
    assert!(
        in_graveyard(&next_turn_hand, P0, index::GRIZZLY_BEARS).is_some(),
        "prevention wears off at end of turn"
    );
}
