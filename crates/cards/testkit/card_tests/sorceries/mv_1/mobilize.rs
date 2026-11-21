//! `cards/sorceries/mv_1/mobilize.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mobilize() -> CardIndex {
    card_index("7416c8db-ec33-4d92-866c-b7fdc69c8b41")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Mobilize (7416c8db-ec33-4d92-866c-b7fdc69c8b41) — {G} Sorcery:
/// "Untap all creatures you control." Untaps controlled creatures while
/// leaving controlled noncreature permanents and opposing creatures tapped.
#[test]
fn mobilize_untaps_only_creatures_you_control() {
    let mut engine = lk::game(
        SEED,
        &[forest(), sol_ring(), llanowar_elves(), grizzly_bears()],
        &[grizzly_bears()],
        &[mobilize()],
        &[],
    );
    let my_forest = lk::obj(&engine, P0, forest());
    let my_ring = lk::obj(&engine, P0, sol_ring());
    let my_elf = lk::obj(&engine, P0, llanowar_elves());
    let my_bears = lk::obj(&engine, P0, grizzly_bears());
    let their_bears = lk::obj(&engine, P1, grizzly_bears());

    {
        let state = engine.dev_state_mut(P0).unwrap();
        for id in [my_ring, my_elf, my_bears, their_bears] {
            state.object_mut(id).unwrap().status.insert(Status::TAPPED);
        }
    }
    engine.refresh_offer();

    cast_from_hand(&mut engine, P0, mobilize());
    assert!(lk::tapped(&engine, my_forest));
    lk::settle(&mut engine);

    assert!(!lk::tapped(&engine, my_elf), "controlled Elf untaps");
    assert!(!lk::tapped(&engine, my_bears), "controlled Bears untaps");
    assert!(
        lk::tapped(&engine, my_forest),
        "controlled land stays tapped"
    );
    assert!(
        lk::tapped(&engine, my_ring),
        "controlled artifact stays tapped"
    );
    assert!(
        lk::tapped(&engine, their_bears),
        "opponent's creature stays tapped"
    );
}
