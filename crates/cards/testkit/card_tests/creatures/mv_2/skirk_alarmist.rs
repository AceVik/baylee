//! `cards/creatures/mv_2/skirk_alarmist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn skirk_alarmist() -> CardIndex {
    card_index("a7d20fb0-005b-4bcc-9592-cdbd3fbdd43a")
}

/// Skirk Alarmist has haste, taps immediately to turn a face-down creature you
/// control face up, and sacrifices that creature at the beginning of the next end step.
#[test]
fn skirk_alarmist_has_haste_flips_morph_and_sacrifices_at_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[index::SKIRK_COMMANDO, skirk_alarmist()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lands = all_on_battlefield(&engine, p0, mountain());
    tap_mana_where(&mut engine, p0, |id| lands[..3].contains(&id));
    cast_with_floating(&mut engine, p0, index::SKIRK_COMMANDO);
    let slot = choose_cast_kind(&engine, CastModeKind::Disguise);
    engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let face_down = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.status.contains(Status::FACE_DOWN))
        })
        .expect("face-down creature");
    assert_eq!(pt(&engine, face_down), (2, 2));

    tap_mana_where(&mut engine, p0, |id| lands[3..5].contains(&id));
    cast_with_floating(&mut engine, p0, skirk_alarmist());
    pass_until(&mut engine, |e| at_rest(e, p0));

    activate(&mut engine, p0, skirk_alarmist(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![face_down],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        !engine
            .state()
            .object(face_down)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN),
        "Skirk Commando is now face up"
    );
    assert_eq!(pt(&engine, face_down), (2, 1));

    reach_their_main_phase(&mut engine, p1);
    assert!(in_graveyard(&engine, p0, index::SKIRK_COMMANDO).is_some());
    assert!(on_battlefield(&engine, p0, skirk_alarmist()).is_some());
}
