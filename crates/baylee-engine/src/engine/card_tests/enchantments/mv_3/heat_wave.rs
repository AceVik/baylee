//! `cards/enchantments/mv_3/heat_wave.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn heat_wave() -> CardIndex {
    card_index("d8f08699-6210-41c3-bfa0-c4b3bcbbaf1c")
}

fn knight_of_the_mists() -> CardIndex {
    card_index("e9893534-3d05-42c2-93ff-f28ffae89aa3")
}

/// `Heat Wave` ({2}{R}, Enchantment, `Coverage::Partial`) prevents blue
/// creatures from blocking creatures you control.
#[test]
fn heat_wave_prevents_blue_creatures_from_blocking_your_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), GRIZZLY_BEARS])
        .battlefield(1, &[knight_of_the_mists()])
        .hand(0, &[heat_wave()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, heat_wave());
    pass_until(&mut engine, stack_is_empty);
    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let blue_blocker = on_battlefield(&engine, p1, knight_of_the_mists()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(blue_blocker, bear)],
                },
            )
            .is_err(),
        "blue creature cannot block creatures controlled by Heat Wave's controller"
    );
}
