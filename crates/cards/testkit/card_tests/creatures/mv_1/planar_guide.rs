//! `cards/creatures/mv_1/planar_guide.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn planar_guide() -> CardIndex {
    card_index("75701981-b9fc-471f-a97f-e897bb3dfb77")
}

/// Planar Guide exiles itself as cost, exiles all creatures, and returns
/// those exiled creatures (not Planar Guide itself) at the next end step
/// under their owners' control.
#[test]
fn planar_guide_blinks_all_other_creatures_at_next_end_step() {
    let mut e = lk::game(
        SEED,
        &[planar_guide(), grizzly_bears()],
        &[savannah_lions()],
        &[],
        &[],
    );
    let guide = lk::obj(&e, lk::P0, planar_guide());
    let bears = lk::obj(&e, lk::P0, grizzly_bears());
    let lions = lk::obj(&e, lk::P1, savannah_lions());

    lk::float(&mut e, lk::P0, &[(ManaColor::White, 4)]);
    activate(&mut e, lk::P0, planar_guide(), 0);
    lk::settle(&mut e);

    assert_eq!(e.state().object(guide).map(|o| o.zone), Some(Zone::Exile));
    assert_eq!(e.state().object(bears).map(|o| o.zone), Some(Zone::Exile));
    assert_eq!(e.state().object(lions).map(|o| o.zone), Some(Zone::Exile));

    // Advance into P1's first main phase so the next end step delayed trigger fires.
    pass_until(&mut e, |eng| {
        eng.state().turn.active == lk::P1 && eng.state().turn.phase == Phase::FirstMain
    });

    assert_eq!(
        e.state().object(guide).map(|o| o.zone),
        Some(Zone::Exile),
        "Planar Guide exiled as cost must remain in exile"
    );
    assert!(
        on_battlefield(&e, lk::P0, grizzly_bears()).is_some(),
        "your exiled creature returned at end step"
    );
    assert!(
        on_battlefield(&e, lk::P1, savannah_lions()).is_some(),
        "opponent creature returned under owner control at end step"
    );
}
