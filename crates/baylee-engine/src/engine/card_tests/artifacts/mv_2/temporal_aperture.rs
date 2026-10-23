#[allow(clippy::wildcard_imports)]
use super::*;

fn temporal_aperture() -> CardIndex {
    card_index("549fe3ff-70aa-4543-b427-40b814b29894")
}

#[test]
fn temporal_aperture_taps_to_shuffle_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                temporal_aperture(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ap_id = on_battlefield(&engine, p0, temporal_aperture()).expect("aperture on bf");
    tap_all_mana(&mut engine, p0);
    let journal_from = engine.state().journal.len();
    activate(&mut engine, p0, temporal_aperture(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(ap_id)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert!(engine.state().journal.entries()[journal_from..].iter().any(
        |entry| matches!(entry.event, crate::event::GameEvent::Shuffled { player, .. } if player == p0)
    ));
}
