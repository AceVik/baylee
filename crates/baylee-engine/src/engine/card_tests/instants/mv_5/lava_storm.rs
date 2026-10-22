//! `cards/instants/mv_5/lava_storm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, LAVA_STORM};

/// `Lava Storm` ({3}{R}{R}, Instant) deals 2 damage to each attacking creature
/// or 2 damage to each blocking creature.
#[test]
fn lava_storm_deals_2_damage_to_attacking_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                GRIZZLY_BEARS,
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[LAVA_STORM])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
            },
        )
        .unwrap();

    cast_from_hand(&mut engine, p0, LAVA_STORM);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_some());
}
