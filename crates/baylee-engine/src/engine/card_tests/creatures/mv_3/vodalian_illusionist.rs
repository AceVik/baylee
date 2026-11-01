//! `cards/creatures/mv_3/vodalian_illusionist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::object::Status;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn vodalian_illusionist() -> CardIndex {
    card_index("ac935639-ba30-4b04-86f6-363527e85a8b")
}

fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn phased_out(e: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    e.state()
        .object(id)
        .expect("the object exists")
        .status
        .contains(Status::PHASED_OUT)
}

/// Vodalian Illusionist (ac935639-ba30-4b04-86f6-363527e85a8b) — "{U}{U}, {T}:
/// Target creature phases out." Three Islands' mana float and exactly {U}{U} is
/// spent. The opponent's Elves phases out and the Bears beside it does not; the
/// Illusionist taps and cannot do it again; the Elves phase back in at their
/// controller's next untap step.
#[test]
fn vodalian_illusionist_phases_out_the_target_for_uu_and_a_tap() {
    let mut e = lk::game(
        SEED,
        &[vodalian_illusionist(), island(), island(), island()],
        &[llanowar_elves(), grizzly_bears()],
        &[],
        &[],
    );
    let illusionist = lk::obj(&e, P0, vodalian_illusionist());
    let elves = lk::obj(&e, P1, llanowar_elves());
    let bears = lk::obj(&e, P1, grizzly_bears());

    tap_all_mana(&mut e, P0);
    assert_eq!(lk::pool_total(&e, P0), 3);
    activate(&mut e, P0, vodalian_illusionist(), 0);
    let menu = aim_at(&mut e, P0, elves);
    assert!(
        menu.contains(&elves) && menu.contains(&bears) && menu.contains(&illusionist),
        "any creature: {menu:?}"
    );
    pass_until(&mut e, stack_is_empty);

    assert!(phased_out(&e, elves), "the target phases out");
    assert!(!phased_out(&e, bears), "the other creature does not");
    assert!(lk::tapped(&e, illusionist), "{{T}} is part of the cost");
    assert_eq!(
        lk::pool_total(&e, P0),
        1,
        "{{U}}{{U}} of the three floating mana"
    );
    assert!(
        !lk::offered(&e, vodalian_illusionist(), P0, 0),
        "tapped: not again this turn"
    );

    reach_their_main_phase(&mut e, P1);
    assert!(
        !phased_out(&e, elves),
        "it phases in at its controller's next untap step"
    );
}
