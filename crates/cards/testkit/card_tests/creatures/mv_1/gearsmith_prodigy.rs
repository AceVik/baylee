//! Oracle checklist: its +1/+0 requires an artifact its controller controls; a controlled Amulet grants it, then sacrificing that Amulet removes it while an opponent still controls a Sol Ring.
#[allow(clippy::wildcard_imports)]
use super::*;
fn gearsmith_prodigy() -> CardIndex {
    card_index("589d6b67-17ad-4ca7-9b0c-1b919bf03a27")
}
fn traveler_s_amulet() -> CardIndex {
    card_index("48b561dd-f23d-409a-87c8-e49902b2477b")
}
#[test]
fn prodigy_bonus_tracks_control_of_an_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[gearsmith_prodigy(), traveler_s_amulet(), forest()])
        .battlefield(1, &[sol_ring()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let prodigy = on_battlefield(&e, p0, gearsmith_prodigy()).unwrap();
    assert_eq!(pt(&e, prodigy), (2, 2));
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, traveler_s_amulet(), 0);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    e.apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("may fail to find a Forest");
    assert!(on_battlefield(&e, p1, sol_ring()).is_some());
    assert_eq!(pt(&e, prodigy), (1, 2));
    assert!(on_battlefield(&e, p0, traveler_s_amulet()).is_none());
}
