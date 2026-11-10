//! `cards/lands/utility/flamekin_village.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flamekin Village — Land: "{T}: Add {R}" and "{R}, {T}: Target creature
/// gains haste until end of turn". The arrival clause is the
/// `Coverage::Partial` gap ("reveal an Elemental card from your hand, or this
/// land enters tapped" has no `EnterModifier` to live in), so what is played
/// here is the two printed abilities.
///
/// The {R} comes off a Mountain: `{T}` is half the price the pump charges
/// (CR 602.2b), so a Village that had paid for its own activation would be
/// refused rather than tested. Three Elves stand on the table — two here, one
/// across it — and that is what reads "target creature" as *a* creature: the
/// haste lands on the one that was named and on neither of the others.
#[test]
fn flamekin_village_spends_red_and_its_own_tap_on_one_creatures_haste() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[flamekin_village(), solitude()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A real `PlayLand`. `starting_battlefield` places a permanent with
    // `Cause::Setup`, which is a placement and no arrival at all.
    let village = play_land(&mut engine, p0, flamekin_village());
    // The Elemental in hand, revealed: the haste ability taps the Village,
    // so a Village that entered tapped could not be asked this question at
    // all. Solitude is in the hand for that and is never cast.
    let elemental = in_hand(&engine, p0, solitude()).expect("Solitude is in hand");
    reveal_on_entry(&mut engine, p0, elemental);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays plain");
    let (chosen, bystander) = (elves[0], elves[1]);
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    let peak = on_battlefield(&engine, p0, mountain()).expect("the Mountain is out");
    assert_eq!(pt(&engine, chosen), (1, 1), "a printed 1/1 before anything");
    assert!(
        !keywords(&engine, chosen).contains(KeywordSet::HASTE),
        "and nothing is hasted yet"
    );

    // One red, off the Mountain, with the Village left standing: the mana is
    // in the pool before the offer is read.
    tap_all_mana_but(&mut engine, p0, Some(flamekin_village()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "the Mountain's red is where the pump's cost will come from"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "one red off the Mountain and a green from each Elf; only the \
         Village is held back"
    );

    // Ability 0 is "{T}: Add {R}", ability 1 is the pump the card is here for.
    activate(&mut engine, p0, flamekin_village(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("\"target creature\" asks, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&chosen) && options.contains(&bystander),
        "both creatures under this seat are on the menu: {options:?}"
    );
    assert!(
        options.contains(&their_elf),
        "\"target creature\" is not \"you control\" and reaches across the table: {options:?}"
    );
    assert!(
        !options.contains(&village) && !options.contains(&peak),
        "a land is no creature, the Village under test included: {options:?}"
    );
    assert_eq!(options.len(), 3, "and those three are the whole menu");

    // CR 601.2c before 601.2h: while the target question stands, the price is
    // unpaid — the pool still holds the Mountain's red and the Village is up.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "nothing is spent before the target is named"
    );
    assert!(!is_tapped(&engine, village), "the Village is still up");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("a creature the question offered is a legal target");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{R}} came out of the pool, and the two green the Elves made \
         are still in it"
    );
    assert!(
        is_tapped(&engine, village),
        "and so did {{T}} off the Village"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so it waits on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, chosen).contains(KeywordSet::HASTE),
        "\"target creature gains haste until end of turn\""
    );
    assert_eq!(
        pt(&engine, chosen),
        (1, 1),
        "a +0/+0 pump carries a keyword and moves no body"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::HASTE),
        "the Elf beside the target was never named"
    );
    assert!(
        !keywords(&engine, their_elf).contains(KeywordSet::HASTE),
        "nor the Elf across the table, whatever the menu offered"
    );
}

/// Flamekin Village prints "As this land enters, you may reveal an Elemental card from your hand. If you don't, this land enters tapped.", `{{T}}: Add {{R}}.`, and `{{R}}, {{T}}: Target creature gains haste until end of turn.`
///
/// Under `Coverage::Implemented`, the land checks for an Elemental card in hand upon entry.
/// Revealing `solitude` satisfies the condition and allows the land to enter untapped while keeping the card in hand, after which activating its mana ability produces Red mana directly; without an Elemental card in hand, it enters tapped.
#[test]
fn flamekin_village_enters_untapped_by_revealing_elemental() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(140, forest())
        .hand(0, &[flamekin_village(), solitude()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, flamekin_village());
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected reveal prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (0, 1));
    assert_eq!(prompt, ChoicePrompt::RevealOrEnterTapped);
    let elemental = in_hand(&engine, p0, solitude()).expect("solitude is in hand");
    assert_eq!(options, vec![elemental]);

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elemental],
            },
        )
        .expect("reveal elemental card");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, land),
        "revealing an Elemental lets Flamekin Village enter untapped"
    );
    assert!(
        in_hand(&engine, p0, solitude()).is_some(),
        "revealed Elemental remains in hand"
    );

    activate(&mut engine, p0, flamekin_village(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));

    let mut engine2 = Duel::new(141, forest())
        .hand(0, &[flamekin_village()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);

    let land2 = play_land(&mut engine2, p0, flamekin_village());
    assert!(
        is_tapped(&engine2, land2),
        "with no Elemental in hand, Flamekin Village enters tapped"
    );
}
