//! `cards/sorceries/mv_4/deadshot.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn deadshot() -> CardIndex {
    card_index("711c10cb-7a85-417d-be24-79d4e4a89993")
}

/// Deadshot (`711c10cb-7a85-417d-be24-79d4e4a89993`) — {3}{R} Sorcery:
/// "Tap target creature. It deals damage equal to its power to another target creature."
#[test]
fn deadshot_taps_first_target_and_deals_its_power_to_another_target_creature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::HILL_GIANT,
            ],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[deadshot()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, deadshot());
    aim_at(&mut engine, P0, giant);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected second target prompt");
    };
    assert!(
        !options.contains(&giant),
        "another target creature excludes the first target"
    );
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, giant));
    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
