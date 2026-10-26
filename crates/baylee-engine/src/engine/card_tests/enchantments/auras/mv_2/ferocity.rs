//! Tests for Ferocity.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::hill_giant;

// oracle_id = "1b1a0f35-3126-4b88-90aa-c56e67850a9d"
fn ferocity() -> CardIndex {
    card_index("1b1a0f35-3126-4b88-90aa-c56e67850a9d")
}

/// Enchant creature
/// Whenever enchanted creature blocks or becomes blocked, you may put a +1/+1
/// counter on it.
#[test]
fn ferocity_puts_p1p1_counter_on_enchanted_creature_when_it_blocks() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), hill_giant()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[ferocity()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let giant = on_battlefield(&engine, p0, hill_giant()).expect("Hill Giant");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("Gray Ogre");

    cast_from_hand(&mut engine, p0, ferocity());
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    // Advance to P1's declare-attackers step; P1 attacks with Gray Ogre, P0 blocks with Hill Giant.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(p0))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, ogre)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, giant), (4, 4));
}
