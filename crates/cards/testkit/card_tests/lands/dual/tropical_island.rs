//! `cards/lands/dual/tropical_island.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tropical Island is a `Land — Forest Island` and prints nothing beyond the
/// parentheses its two basic types come with: `({T}: Add {G} or {U}.)`. It
/// enters untapped, so the turn it is played its own tap is live and the whole
/// card reads in one main phase — one land, one tap, and a question whose menu
/// is exactly the two colours it is named after, never both at once and never a
/// third. The green that comes back out of the pool pays for a green spell in
/// the same phase, which is what tells mana that was really made from a number
/// that only sits in a `ManaPool`.
#[test]
fn tropical_island_asks_which_of_its_two_colors_to_add_and_taps_for_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[tropical_island(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The land drop. Nothing about the card keeps it down — it has no entry
    // modifier at all — so it stands untapped and its `{T}` is payable in
    // this very phase.
    play_land(&mut engine, p0, tropical_island());
    let island = on_battlefield(&engine, p0, tropical_island())
        .expect("the land drop landed on the battlefield");
    assert!(
        !is_tapped(&engine, island),
        "a land with no enters-tapped clause arrives ready to tap"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the land taps, so what comes next came off it"
    );

    // Mana before the claim. Both offers are read the way the kit reads them
    // (`mana_routes`): a printed `{T}: Add …` is an ordinary `(source, index)`
    // entry and the CR 305.6 shortcut names the land itself, and which of the
    // two carries a dual land's tap is not this test's subject — that the tap
    // is offered, and what it asks, is.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let tap = legal
        .abilities
        .iter()
        .copied()
        .find(|(source, _)| *source == island)
        .map(|(source, ability_index)| PlayerAction::ActivateAbility {
            source,
            ability_index,
        })
        .or_else(|| {
            legal
                .mana_abilities
                .contains(&island)
                .then_some(PlayerAction::ActivateManaAbility { source: island })
        })
        .expect("the land offers the mana ability its own text prints");
    engine
        .apply(p0, tap)
        .expect("the land is untapped and this is its controller's own main phase");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{U}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that taps is the seat that names it");
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::Blue),
        "both basic land types it is named after are on the menu: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and nothing else is: no third colour, and colourless is no colour at \
         all (CR 105.4): {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the two colours offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "\"or\" is a choice: the blue the same tap might have made is not there \
         as well"
    );
    assert_eq!(
        pool.total(),
        1,
        "one land, one tap, one mana — the whole price is its own {{T}}"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is in the pool \
         with nothing waiting to resolve"
    );

    // And it is mana: the green pays for the Elf, in this phase and out of the
    // pool the tap filled.
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, llanowar_elves()).is_some()
    });
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{G}} came out of the pool to pay for the spell"
    );
}
