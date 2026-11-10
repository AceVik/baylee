//! `cards/instants/mv_2/tidal_bore.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

// oracle_id = "513ad32a-2887-4825-adad-86a87cf1d2cb"
fn tidal_bore() -> CardIndex {
    card_index("513ad32a-2887-4825-adad-86a87cf1d2cb")
}

/// `Tidal Bore` (`Coverage::Partial`: base mana cost `{1}{U}`):
/// "You may tap or untap target creature."
#[test]
fn tidal_bore_taps_or_untaps_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[tidal_bore(), tidal_bore()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("Grizzly Bears");
    assert!(!is_tapped(&engine, bear));

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, tidal_bore());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_tapped(&engine, bear),
        "Tidal Bore taps an untapped creature"
    );

    // Second Tidal Bore untaps the tapped creature.
    cast_with_floating(&mut engine, p0, tidal_bore());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, bear),
        "Tidal Bore untaps a tapped creature"
    );
}
