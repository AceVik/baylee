//! `cards/creatures/mv_2/goblin_piledriver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Goblin Piledriver (`4d74d12b-1ffb-482f-8bac-635a8d4d32e9`) — {1}{R} Creature — Goblin Warrior 1/2:
/// "Protection from blue
/// Whenever this creature attacks, it gets +2/+0 until end of turn for each other attacking Goblin."
#[test]
fn goblin_piledriver_gets_plus_two_power_per_other_attacking_goblin() {
    let piledriver_card = card_index("4d74d12b-1ffb-482f-8bac-635a8d4d32e9");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                piledriver_card,
                index::RAGING_GOBLIN,
                index::MONS_S_GOBLIN_RAIDERS,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let piledriver = on_battlefield(&engine, p0, piledriver_card).unwrap();
    let g1 = on_battlefield(&engine, p0, index::RAGING_GOBLIN).unwrap();
    let g2 = on_battlefield(&engine, p0, index::MONS_S_GOBLIN_RAIDERS).unwrap();
    assert_eq!(pt(&engine, piledriver), (1, 2));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (piledriver, Defender::Player(p1)),
                    (g1, Defender::Player(p1)),
                    (g2, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Two other attacking Goblins -> +4/+0 -> 5/2.
    assert_eq!(pt(&engine, piledriver), (5, 2));

    reach_their_main_phase(&mut engine, p0);
    // 5 (Piledriver) + 1 (Raging Goblin) + 1 (Mons's Goblin Raiders) = 7 damage.
    assert_eq!(engine.state().players[1].life, 13);
}
