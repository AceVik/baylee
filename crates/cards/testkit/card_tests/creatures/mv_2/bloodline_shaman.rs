//! `cards/creatures/mv_2/bloodline_shaman.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;
use baylee_core::generated::subtypes;

fn bloodline_shaman() -> CardIndex {
    card_index("a9c2bf1b-2d5e-42c5-bc76-8429aaae161c")
}

fn woodland_changeling() -> CardIndex {
    card_index("338f0ab6-f744-4e5a-a358-05e71b93758d")
}

/// Bloodline Shaman puts matching creature cards (including changelings) from
/// the top of your library into hand and non-matching or noncreature cards into
/// your graveyard.
#[test]
fn bloodline_shaman_puts_matching_creature_to_hand_and_misses_to_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[bloodline_shaman()])
        .hand(0, &[llanowar_elves(), crib_swap(), woodland_changeling()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Order from top down: llanowar_elves (hit), crib_swap (noncreature miss), woodland_changeling (hit).
    let changeling_top = hand_to_library_top(&mut engine, p0, woodland_changeling());
    let spell_top = hand_to_library_top(&mut engine, p0, crib_swap());
    let elf_top = hand_to_library_top(&mut engine, p0, llanowar_elves());

    // Activation 1: Elf creature -> hand.
    activate(&mut engine, p0, bloodline_shaman(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .expect("choose Elf");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(zone_of(&engine, elf_top), Zone::Hand);

    // Turn 2 draw pulled `spell_top` (`crib_swap`) into hand; place it back on top.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(zone_of(&engine, spell_top), Zone::Hand);
    let noncreature_id = hand_to_library_top(&mut engine, p0, crib_swap());
    assert_eq!(
        engine
            .state()
            .object(noncreature_id)
            .unwrap()
            .card
            .unwrap()
            .index,
        crib_swap()
    );

    // Activation 2: noncreature Changeling (Crib Swap) is NOT a creature card -> graveyard.
    activate(&mut engine, p0, bloodline_shaman(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .expect("choose Elf");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(zone_of(&engine, noncreature_id), Zone::Graveyard);

    // Turn 3 draw pulled `changeling_top` (`woodland_changeling`) into hand; place it back on top.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(zone_of(&engine, changeling_top), Zone::Hand);
    let top = hand_to_library_top(&mut engine, p0, woodland_changeling());
    activate(&mut engine, p0, bloodline_shaman(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .expect("choose Elf");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(zone_of(&engine, top), Zone::Hand);
}

/// Killing Bloodline Shaman in response does not stop its activated ability on the stack (CR 113.7a).
#[test]
fn bloodline_shaman_ability_resolves_even_if_shaman_dies_in_response() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), bloodline_shaman()])
        .hand(0, &[lightning_bolt(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shaman = on_battlefield(&engine, p0, bloodline_shaman()).expect("Shaman seated");
    let top_elf = hand_to_library_top(&mut engine, p0, llanowar_elves());

    activate(&mut engine, p0, bloodline_shaman(), 0);
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim(&mut engine, &[shaman], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    assert_eq!(
        zone_of(&engine, shaman),
        Zone::Graveyard,
        "Lightning Bolt already destroyed Bloodline Shaman"
    );
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .expect("ability still asks and resolves per CR 113.7a");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(zone_of(&engine, top_elf), Zone::Hand);
}
