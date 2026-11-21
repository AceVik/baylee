//! `cards/sorceries/mv_3/time_ebb.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn time_ebb() -> CardIndex {
    card_index("30cc8f7b-3c28-40f5-8f8f-157e8212280b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Time Ebb (30cc8f7b-3c28-40f5-8f8f-157e8212280b) — {2}{U} Sorcery:
/// "Put target creature on top of its owner's library."
#[test]
fn time_ebb_puts_target_creature_on_top_of_its_owners_library() {
    let mut engine = lk::game(
        SEED,
        &[island(), island(), island(), sol_ring()],
        &[grizzly_bears()],
        &[time_ebb()],
        &[],
    );
    let ring = lk::obj(&engine, P0, sol_ring());
    let bears = lk::obj(&engine, P1, grizzly_bears());

    tap_mana_except(&mut engine, P0, ring);
    cast_with_floating(&mut engine, P0, time_ebb());
    let menu = aim_at(&mut engine, P0, bears);
    assert!(menu.contains(&bears));
    assert!(!menu.contains(&ring), "noncreatures are excluded");
    lk::settle(&mut engine);

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P1))
            .last()
            .copied(),
        Some(bears),
        "creature put on top of its owner's library"
    );

    // Owned by P0, controlled by P1 -> goes to top of P0's library.
    let mut stolen = lk::game(
        SEED,
        &[island(), island(), island(), savannah_lions()],
        &[],
        &[time_ebb()],
        &[],
    );
    let lions = lk::obj(&stolen, P0, savannah_lions());
    stolen
        .dev_state_mut(P0)
        .unwrap()
        .object_mut(lions)
        .unwrap()
        .set_controller(P1);
    stolen.refresh_offer();

    cast_from_hand(&mut stolen, P0, time_ebb());
    aim_at(&mut stolen, P0, lions);
    lk::settle(&mut stolen);

    assert_eq!(
        stolen
            .state()
            .zones
            .list(ZoneLocation::Library(P0))
            .last()
            .copied(),
        Some(lions),
        "returns to its owner's library, not its controller's"
    );
}
