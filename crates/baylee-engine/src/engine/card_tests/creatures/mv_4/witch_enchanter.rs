//! `cards/creatures/mv_4/witch_enchanter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Witch Enchanter ({3}{W}, 2/2): "When this creature enters, destroy target
/// artifact or enchantment an opponent controls."
///
/// The board is built so that each word of that sentence can fail on its
/// own. A Sol Ring stands on **both** sides of the table, so "an opponent
/// controls" is the only thing keeping p0's own copy out of the offer; a
/// Wizard Class stands opposite as well, so "or enchantment" has something
/// to name that a filter narrowed to artifacts would miss; and a Llanowar
/// Elves stands beside them, which is the only permanent on the board that
/// says what "artifact or enchantment" *excludes* — a filter that had lost
/// the noun and kept nothing but "an opponent controls" would offer it.
///
/// What is asserted first is the enumeration the engine offered, because
/// that is where a too-wide filter shows. A test that read only the
/// graveyard afterwards would pass just as happily for a Witch Enchanter
/// that could point at its controller's own artifacts — it would simply
/// never be asked to.
#[test]
fn witch_enchanter_destroys_an_opponents_artifact_and_is_never_offered_its_own() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(311, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), quiet_artifact()],
        )
        .battlefield(1, &[quiet_artifact(), wizard_class(), quiet_creature()])
        .hand(0, &[witch_enchanter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let my_ring = on_battlefield(&engine, p0, quiet_artifact()).expect("a Sol Ring of my own");
    let their_ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring");
    let their_class = on_battlefield(&engine, p1, wizard_class()).expect("their enchantment");
    let their_elf = on_battlefield(&engine, p1, quiet_creature()).expect("their Llanowar Elves");

    // Four Plains pay {3}{W} exactly; the Sol Rings are printed mana
    // abilities and are left alone by this.
    cast_from_hand(&mut engine, p0, witch_enchanter());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the Enchanter's controller points the trigger");
    assert!(
        options.contains(&their_ring),
        "\"target artifact ... an opponent controls\": {options:?}"
    );
    assert!(
        options.contains(&their_class),
        "\"or enchantment\" — the Class across the table is a legal target \
         too: {options:?}"
    );
    assert!(
        !options.contains(&my_ring),
        "\"an opponent controls\" — p0's own Sol Ring must not be in the \
         offer: {options:?}"
    );
    assert!(
        !options.contains(&their_elf),
        "\"artifact or enchantment\" — their Elf is neither, and is theirs: \
         {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![their_ring],
                players: Vec::new(),
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "the artifact it named was destroyed"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "and only that one: the Sol Ring on this side is untouched"
    );
    let enchanter = on_battlefield(&engine, p0, witch_enchanter()).expect("the 2/2 landed");
    assert_eq!(
        pt(&engine, enchanter),
        (2, 2),
        "the trigger came off a creature that actually resolved"
    );
}

/// Witch-Blessed Meadow, the back face: "As this land enters, you may pay 3
/// life. If you don't, it enters tapped." and "{T}: Add {W}".
///
/// That a *creature* card is a legal land drop is CR 712.12, and it is read
/// off `legal.lands` rather than assumed: the front face is a Human Warlock,
/// so an offer that looked only at the front would never name the card and
/// the `PlayLand` below would be refused for a reason that has nothing to do
/// with the printed sentence. Only one of the two faces is a land, so the
/// engine switches to it without asking which — the Glasspool Shore path —
/// and the one question it does ask is the entry replacement (CR 614.1c).
///
/// Paid, the land is up at once and taps the same turn: a land is not a
/// creature, so nothing about summoning sickness applies to its `{T}`.
#[test]
fn witch_blessed_meadow_pays_three_life_to_arrive_untapped_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(313, forest())
        .hand(0, &[witch_enchanter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, witch_enchanter()).expect("the card is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.lands.contains(&card),
        "CR 712.12: a card whose back face is a land is a legal land drop, \
         however the front face reads"
    );

    let land = play_land(&mut engine, p0, witch_enchanter());
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        panic!(
            "the land asked nothing on the way in; pending is {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "its controller answers");
    assert_eq!(
        prompt,
        YesNoPrompt::PayLifeOrEnterTapped { amount: 3 },
        "three life, the number this card prints"
    );

    let life_before = engine.state().players[0].life;
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(
        engine.state().players[0].life,
        life_before - 3,
        "\"you may pay 3 life\""
    );
    assert!(
        !entered_tapped(&engine, land),
        "paid, so the \"if you don't\" half never ran"
    );

    let obj = engine
        .state()
        .object(land)
        .expect("the land is on the table");
    assert_eq!(obj.face_index, 1, "it arrived as its land face");
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Witch-Blessed Meadow",
        "and under the land face's name rather than the Warlock's"
    );
    assert!(
        obj.characteristics().types.contains(TypeSet::LAND),
        "a Land, not a Creature"
    );

    activate(&mut engine, p0, witch_enchanter(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "\"{{T}}: Add {{W}}\" — one white mana, on the turn it was played"
    );
    assert_eq!(
        pool.total(),
        1,
        "and one mana in all: the Meadow is the only source on this board, \
         so anything more came out of this ability"
    );
    assert!(
        is_tapped(&engine, land),
        "and the {{T}} in that cost was actually paid"
    );
}

/// The other answer to the same question, which is invisible from the first.
///
/// Declining costs no life and the land arrives tapped (CR 614.1c) — which
/// for a land means it makes no mana at all on the turn it was played, so
/// both consequences are struck and not just the status bit. An engine that
/// asked the question and then dropped the answer would leave the life total
/// right and the land standing up, and only the second assertion would
/// notice a land that was tapped but still offering its ability.
#[test]
fn witch_blessed_meadow_enters_tapped_when_the_three_life_go_unpaid() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(317, forest())
        .hand(0, &[witch_enchanter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, witch_enchanter());
    let life_before = engine.state().players[0].life;
    let Pending::YesNo { prompt, .. } = engine.pending().clone() else {
        panic!(
            "the land asked nothing on the way in; pending is {:?}",
            engine.pending()
        )
    };
    assert_eq!(prompt, YesNoPrompt::PayLifeOrEnterTapped { amount: 3 });

    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    assert_eq!(
        engine.state().players[0].life,
        life_before,
        "a declined \"you may pay\" costs nothing"
    );
    assert!(
        entered_tapped(&engine, land),
        "\"If you don't, it enters tapped.\""
    );

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    // Whose offer this is, before anything is read off it: `legal` belongs to
    // whoever holds priority, so an assertion that the Meadow is missing from
    // it would be satisfied for free by p1's list, which never held a
    // permanent of p0's in the first place.
    assert_eq!(
        player, p0,
        "the Meadow's controller is the one being offered"
    );
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land),
        "and a tapped land cannot pay the {{T}} in its own mana ability, so \
         the Meadow makes no white mana this turn"
    );
}
