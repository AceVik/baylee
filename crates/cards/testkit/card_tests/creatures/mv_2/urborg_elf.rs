//! `cards/creatures/mv_2/urborg_elf.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Urborg Elf is a `{1}{G}` 1/1 printing one line: "{T}: Add {B}, {G}, or
/// {U}." Three colours and no default is the whole card, so the question has
/// to be asked and its menu has to hold exactly those three — a `{G}` read as
/// a fixed colour would float green off the tap and never ask anything. The
/// board is two Forests that are already spent paying for the Elf, so the pool
/// is empty when the tap is pressed and the one blue afterwards can only be
/// the Elf's: a creature's `{T}` is a mana ability whose whole price is its
/// own tap (CR 605.1), which is why the mana and the empty stack are read in
/// the same instant.
#[test]
fn urborg_elf_taps_for_one_mana_of_the_three_colors_it_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[urborg_elf()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{G} out of exactly two Forests, so the pool is empty once the Elf has
    // landed and every drop of mana below is accounted for by its own tap.
    cast_from_hand(&mut engine, p0, urborg_elf());
    pass_until(&mut engine, stack_is_empty);
    let elf = on_battlefield(&engine, p0, urborg_elf()).expect("the Elf resolved");
    assert_eq!(pt(&engine, elf), (1, 1), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "two Forests paid exactly the {{1}}{{G}}, so nothing is left floating"
    );
    for land in all_on_battlefield(&engine, p0, forest()) {
        assert!(
            is_tapped(&engine, land),
            "both Forests are spent, so no land on this board can be the source \
             of the mana the Elf is about to make"
        );
    }

    // Its own next turn: a `{T}` in a cost is unpayable for a creature that
    // arrived this turn (CR 302.6). The Forests stand back up with it and are
    // left standing — what is measured below is the Elf's tap and no other.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool empties at the end of a step (CR 500.5), so every drop \
         below is the Elf's own"
    );

    // The whole price of the ability is the Elf's own {T}, so it is offered
    // with nothing in the pool at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(elf, 0)),
        "an untapped Elf is a paid {{T}}, so its mana line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, urborg_elf(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}}, {{G}}, or {{U}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        3,
        "exactly the three colours the card prints: {options:?}"
    );
    for color in [ManaColor::Black, ManaColor::Green, ManaColor::Blue] {
        assert!(
            options.contains(&color),
            "\"Add {{B}}, {{G}}, or {{U}}\" includes {color:?}: {options:?}"
        );
    }

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, off the Elf's own tap"
    );
    assert_eq!(pool.total(), 1, "one mana, and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, elf), "the Elf paid its own {{T}}");
}
