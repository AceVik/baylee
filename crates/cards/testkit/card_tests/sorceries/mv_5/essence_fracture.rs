//! `cards/sorceries/mv_5/essence_fracture.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn essence_fracture() -> CardIndex {
    card_index("31a49fb9-1081-46c3-a352-0444869afdc9")
}

/// Essence Fracture (31a49fb9-1081-46c3-a352-0444869afdc9) — {3}{U}{U} Sorcery:
/// "Return two target creatures to their owners' hands. Cycling {2}{U}."
#[test]
fn essence_fracture_bounces_two_target_creatures_and_cycles_from_hand() {
    let mut engine = lk::game(
        SEED,
        &[island(), island(), island(), island(), island(), aurochs()],
        &[fangren_hunter(), llanowar_elves(), sol_ring()],
        &[essence_fracture()],
        &[],
    );
    let my_beast = lk::obj(&engine, P0, aurochs());
    let their_hunter = lk::obj(&engine, P1, fangren_hunter());
    let their_elf = lk::obj(&engine, P1, llanowar_elves());
    let their_ring = lk::obj(&engine, P1, sol_ring());

    cast_from_hand(&mut engine, P0, essence_fracture());
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (2, 2));
    assert!(
        options.contains(&my_beast)
            && options.contains(&their_hunter)
            && options.contains(&their_elf)
    );
    assert!(!options.contains(&their_ring));
    lk::aim(&mut engine, &[their_hunter, their_elf], &[]);
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, their_hunter), Zone::Hand);
    assert_eq!(lk::zone_of(&engine, their_elf), Zone::Hand);
    assert_eq!(lk::zone_of(&engine, my_beast), Zone::Battlefield);

    // Cycling {2}{U} works at instant speed (even with <2 creatures in play).
    let mut cycle = lk::game(
        SEED,
        &[island(), island(), island()],
        &[],
        &[essence_fracture()],
        &[],
    );
    let card_in_hand = in_hand(&cycle, P0, essence_fracture()).unwrap();
    let lib_before = library_size(&cycle, P0);
    tap_all_mana(&mut cycle, P0);
    cycle
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: card_in_hand,
                ability_index: 0,
            },
        )
        .unwrap();
    assert_eq!(lk::zone_of(&cycle, card_in_hand), Zone::Graveyard);
    lk::settle(&mut cycle);
    assert_eq!(library_size(&cycle, P0), lib_before - 1);
    assert_eq!(cycle.state().zones.list(ZoneLocation::Hand(P0)).len(), 1);
}
