//! `cards/instants/mv_3/capsize.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn capsize() -> CardIndex {
    card_index("77637eff-2963-4402-88f3-ca346f762fc8")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// p0 has three Islands and Capsize in hand; p1 has Grizzly Bears. p0's main
/// phase.
fn a_capsize_over_a_bear() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[capsize()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Capsize (77637eff-2963-4402-88f3-ca346f762fc8) — {1}{U}{U} Instant (Partial: buyback not modeled):
/// "Return target permanent to its owner's hand."
#[test]
fn capsize_returns_target_creature_to_its_owners_hand() {
    let mut engine = a_capsize_over_a_bear();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, P0, capsize());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, grizzly_bears()).is_none());
    assert!(in_hand(&engine, P1, grizzly_bears()).is_some());
}

/// "Target permanent": a land is a legal target, and so is one of your own.
#[test]
fn capsize_may_return_any_permanent_including_a_land_of_the_casters() {
    let mut engine = a_capsize_over_a_bear();
    // Capsize taps the three Islands for its cost, so aim at a tapped one.
    let island_id = on_battlefield(&engine, P0, island()).unwrap();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, P0, capsize());
    let menu = aim_at(&mut engine, P0, island_id);
    assert!(menu.contains(&island_id) && menu.contains(&bears));
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, P0, island()).is_some());
    assert!(
        on_battlefield(&engine, P1, grizzly_bears()).is_some(),
        "only the target was returned"
    );
}

#[test]
fn capsize_goes_to_the_graveyard_without_its_unmodeled_buyback() {
    let mut engine = a_capsize_over_a_bear();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, P0, capsize());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, capsize()).is_some());
    assert!(in_hand(&engine, P0, capsize()).is_none());
}
