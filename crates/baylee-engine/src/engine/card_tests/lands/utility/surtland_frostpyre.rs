//! `cards/lands/utility/surtland_frostpyre.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Surtland Frostpyre enters tapped and taps for {R}; for {2}{U}{U}{R}, its
/// own tap and the land itself, it scries 2 and deals 2 damage to *each*
/// creature — its controller's own included. The board pins both halves of
/// that sentence at once: a printed 2/2 of mine dies while a printed 3/3 of
/// mine survives, so the effect is two damage and not a destroy, and the Sol
/// Ring across the table is never touched, because an artifact is no creature.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn surtland_frostpyre_enters_tapped_then_scries_two_and_burns_every_creature_for_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                mountain(),
                katara_the_fearless(),
                desert_drake(),
            ],
        )
        .battlefield(1, &[quiet_creature(), quiet_artifact()])
        .hand(0, &[surtland_frostpyre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // "This land enters tapped." A land that came in untapped would offer its
    // own {T} at this very moment, so the missing mana line is the entry
    // modifier being read through a real `PlayLand` rather than assumed.
    let land = play_land(&mut engine, p0, surtland_frostpyre());
    assert!(
        entered_tapped(&engine, land),
        "the printed \"enters tapped\" is a real entry and not a placement"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the land drop hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land)
            && !legal.mana_abilities.contains(&land),
        "a tapped land has no {{T}} left to pay its own mana ability with: {:?}",
        legal.abilities
    );

    // A turn round the table and back: the untap step is what turns the printed
    // line into an ability the seat is offered at all.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step stood it back up");

    // "{T}: Add {R}" — a printed mana ability, so it is an ordinary entry in
    // `abilities` with an index to name, never the CR 305.6 shortcut.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "an untapped land is a paid {{T}}, so the line is offered: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, surtland_frostpyre(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "{{T}}: Add {{R}}");
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");

    // A second turn: the land is standing again and the pool it filled is gone
    // (CR 500.5), so the price below is paid out of five fresh lands.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step stood it back up");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the pool emptied with the step that ended (CR 500.5)"
    );

    // The whole price is {2}{U}{U}{R} *and* this land's {T} *and* the land
    // itself, so the land is the one source kept back: four Islands and a
    // Mountain are five mana, and no land can pay a tap symbol.
    let taken = tap_mana_except(&mut engine, p0, land);
    assert_eq!(
        taken, 5,
        "four Islands and one Mountain, and the land itself kept back"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "the Mountain's red");
    assert_eq!(
        pool.total(),
        5,
        "exactly the {{2}}{{U}}{{U}}{{R}} the ability charges"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 1)),
        "ability 0 is the mana line above; ability 1 is the scry-and-burn, and \
         with the mana floating the whole price is payable: {:?}",
        legal.abilities
    );

    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let second = library_before[library_before.len() - 2];

    activate(&mut engine, p0, surtland_frostpyre(), 1);

    // The ability names no target, so CR 601.2h pays the whole cost the moment
    // it is announced: the mana, the {T} and the land itself.
    assert!(
        on_battlefield(&engine, p0, surtland_frostpyre()).is_none(),
        "\"Sacrifice this land\" is part of the cost"
    );
    assert!(
        in_graveyard(&engine, p0, surtland_frostpyre()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{2}}{{U}}{{U}}{{R}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is on the stack"
    );

    // "Scry 2." The question is the top two cards, and answering it is what
    // lets the second effect of the same ability happen at all.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing else here")
    };
    assert_eq!(player, p0, "the land's controller does the looking");
    assert_eq!(prompt, crate::choice::ArrangePrompt::Scry);
    assert_eq!(cards, vec![top, second], "the top two cards, top first");
    assert_eq!(
        piles,
        scry_piles(2),
        "either, both or neither of the two may be bottomed"
    );
    engine
        .apply(p0, look_answer(&cards, &[top]))
        .expect("one of the two cards just looked at");

    pass_until(&mut engine, stack_is_empty);

    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(
        library.first().copied(),
        Some(top),
        "the chosen card is bottomed"
    );
    assert_eq!(
        library.last().copied(),
        Some(second),
        "the other is the new top"
    );
    assert_eq!(
        library.len(),
        library_before.len(),
        "scry draws nothing: two cards were looked at and none left the library"
    );

    // "This land deals 2 damage to each creature." Both sides of the table lose
    // a body, and the two bodies that stay pin the number on both ends.
    assert!(
        in_graveyard(&engine, p0, desert_drake()).is_some(),
        "a printed 2/2 of the activating seat's own is killed by its own land: \
         the effect is neither one-sided nor one damage"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_creature()).is_some(),
        "and so is a printed 1/1 across the table"
    );
    let survivor = on_battlefield(&engine, p0, katara_the_fearless()).expect(
        "a printed 3/3 survives two damage, so the effect is damage and not \
         \"destroy each creature\"",
    );
    assert_eq!(
        pt(&engine, survivor),
        (3, 3),
        "and two damage marked does not shrink the body it was marked on"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "an artifact is no creature, so \"each creature\" never reaches it"
    );
}
