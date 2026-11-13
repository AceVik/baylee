//! `cards/artifacts/mv_6/chronicle_of_victory.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::subtypes;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn chronicle() -> CardIndex {
    card_index("e949b362-4988-4482-ac51-c1049af9040c")
}

#[track_caller]
fn cast_choosing_elf(e: &mut Engine<RegistryLookup>, seat: PlayerId) {
    lk::float(e, seat, &[(ManaColor::Colorless, 6)]);
    cast_with_floating(e, seat, chronicle());
    pass_until(e, |e| matches!(e.pending(), Pending::ChooseSubtype { .. }));
    e.apply(seat, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .unwrap();
    pass_until(e, stack_is_empty);
}

#[test]
fn chronicle_buffs_only_your_chosen_type_and_plays_both_keywords_in_combat() {
    let mut e = lk::game(
        SEED,
        &[
            llanowar_elves(),
            baylee_core::generated::index::GRIZZLY_BEARS,
        ],
        &[
            llanowar_elves(),
            baylee_core::generated::index::GRIZZLY_BEARS,
        ],
        &[chronicle()],
        &[],
    );
    let elf = lk::obj(&e, P0, llanowar_elves());
    let bears = lk::obj(&e, P0, baylee_core::generated::index::GRIZZLY_BEARS);
    let their_elf = lk::obj(&e, P1, llanowar_elves());
    let blocker = lk::obj(&e, P1, baylee_core::generated::index::GRIZZLY_BEARS);
    cast_choosing_elf(&mut e, P0);

    assert_eq!(pt(&e, elf), (3, 3));
    assert_eq!(pt(&e, bears), (2, 2));
    assert_eq!(pt(&e, their_elf), (1, 1));
    let elf_keywords = keywords(&e, elf);
    assert!(elf_keywords.contains(KeywordSet::FIRST_STRIKE));
    assert!(elf_keywords.contains(KeywordSet::TRAMPLE));
    let bear_keywords = keywords(&e, bears);
    assert!(!bear_keywords.contains(KeywordSet::FIRST_STRIKE));
    assert!(!bear_keywords.contains(KeywordSet::TRAMPLE));
    assert!(!keywords(&e, their_elf).contains(KeywordSet::FIRST_STRIKE));

    lk::attack(&mut e, &[elf]);
    lk::block(&mut e, &[(blocker, elf)]);
    lk::through_combat(&mut e);

    assert_eq!(lk::zone_of(&e, blocker), Zone::Graveyard);
    assert_eq!(lk::damage_on(&e, elf), 0, "first strike killed the blocker");
    assert_eq!(
        lk::life(&e, P1),
        19,
        "one of the three damage trampled over"
    );
}

#[test]
fn chronicle_draws_for_creature_and_kindred_spells_but_not_other_spells() {
    let mut e = lk::game(
        SEED,
        &[],
        &[baylee_core::generated::index::GRIZZLY_BEARS],
        &[
            chronicle(),
            llanowar_elves(),
            crib_swap(),
            sol_ring(),
            forest(),
            forest(),
            forest(),
        ],
        &[llanowar_elves()],
    );
    cast_choosing_elf(&mut e, P0);

    hand_to_library_top(&mut e, P0, forest());
    let library = e.state().zones.list(ZoneLocation::Library(P0)).len();
    lk::float(&mut e, P0, &[(ManaColor::Green, 1)]);
    cast_with_floating(&mut e, P0, llanowar_elves());
    lk::settle(&mut e);
    assert!(
        in_hand(&e, P0, forest()).is_some(),
        "the Elf cast drew the top card"
    );
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(P0)).len(),
        library - 1
    );

    // Crib Swap is not a creature spell. Changeling still gives the Kindred
    // instant every creature type, including the Elf type chosen above.
    hand_to_library_top(&mut e, P0, forest());
    let library = e.state().zones.list(ZoneLocation::Library(P0)).len();
    lk::float(
        &mut e,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
    );
    cast_with_floating(&mut e, P0, crib_swap());
    let target = lk::obj(&e, P1, baylee_core::generated::index::GRIZZLY_BEARS);
    lk::aim(&mut e, &[target], &[]);
    lk::settle(&mut e);
    assert!(
        in_hand(&e, P0, forest()).is_some(),
        "the Kindred spell also drew"
    );
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(P0)).len(),
        library - 1
    );

    hand_to_library_top(&mut e, P0, forest());
    let library = e.state().zones.list(ZoneLocation::Library(P0)).len();
    lk::float(&mut e, P0, &[(ManaColor::Colorless, 1)]);
    lk::cast_at(&mut e, P0, sol_ring(), &[], &[]);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(P0)).len(),
        library,
        "an artifact with no creature type does not trigger"
    );

    reach_their_main_phase(&mut e, P1);
    let library = e.state().zones.list(ZoneLocation::Library(P0)).len();
    lk::float(&mut e, P1, &[(ManaColor::Green, 1)]);
    cast_with_floating(&mut e, P1, llanowar_elves());
    lk::settle(&mut e);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(P0)).len(),
        library,
        "only Chronicle's controller draws"
    );
}
