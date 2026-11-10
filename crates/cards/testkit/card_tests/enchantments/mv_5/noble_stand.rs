//! `cards/enchantments/mv_5/noble_stand.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn noble_stand_gains_two_life_when_your_creature_blocks() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                card_index("1aa1e34c-b2ff-4750-9e73-5707a8f49bfb"),
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);

    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).expect("Grizzly Bears");
    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).expect("Hill Giant");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p0))],
            },
        )
        .expect("Hill Giant attacks");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, giant)],
            },
        )
        .expect("Grizzly Bears blocks");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 22);
}
