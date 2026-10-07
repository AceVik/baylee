//! `cards/lands/restricted/mistveil_plains.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mistveil Plains is a `Land — Plains` that enters tapped, taps for {W},
/// and prints one more line: "{W}, {T}: Put target card from your graveyard
/// on the bottom of your library. Activate only if you control two or more
/// white permanents."
///
/// The land is *played* rather than placed, because `starting_battlefield` is
/// a setup placement with no entry behind it and only a real land drop can
/// show the tapped clause; the turn cycle afterwards is what stands the same
/// permanent back up so that it can pay its own `{T}`. The two white Soldiers
/// sit on the printed threshold exactly — the second game leaves one of them
/// out, with the same {W} floating, and the line is not offered at all — and
/// the {W} is read out of the pool before the target is answered (CR 601.2c)
/// and gone after it (CR 601.2h).
#[test]
#[allow(clippy::too_many_lines)] // one land, played through every clause it prints
fn mistveil_plains_enters_tapped_and_buries_a_graveyard_card_under_the_library() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), auriok_bladewarden(), auriok_bladewarden()])
        .hand(0, &[mistveil_plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, mistveil_plains());
    assert_eq!(
        on_battlefield(&engine, p0, mistveil_plains()),
        Some(land),
        "the land drop put it onto the battlefield"
    );
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — and the entry that was replaced is the \
         one the drop performed, not a placement"
    );

    // A turn cycle, because the `{T}` in the activated cost belongs to this
    // same permanent: the untap step has to stand it back up (CR 502.3)
    // before the line can be paid for.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, land),
        "the untap step ran, and the land is standing again"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, auriok_bladewarden()).len(),
        2,
        "two white permanents under p0, which is the printed threshold exactly"
    );

    seed_graveyard(&mut engine, p0, 2);
    let graveyard: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .clone();
    assert_eq!(graveyard.len(), 2, "two Forest cards were seeded");
    let library_before = library_size(&engine, p0);

    // Mana into the pool before the offer is read, and the land itself kept
    // back: its `{T}` is half of the cost that is about to be paid.
    tap_all_mana_but(&mut engine, p0, Some(mistveil_plains()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "the Plains paid {{W}}, and the only other source on this board is the \
         land under test"
    );
    assert!(!is_tapped(&engine, land), "which is why it still stands");

    // Ability 0 is the `{T}: Add {W}` printed above the line; 1 is the
    // graveyard line.
    activate(&mut engine, p0, mistveil_plains(), 1);

    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the line targets a card in your graveyard, got {:?}",
            engine.pending()
        )
    };
    assert_eq!((min, max), (1, 1), "exactly one card");
    assert_eq!(options.len(), 2, "the graveyard, and nothing outside it");
    for card in &options {
        assert!(
            graveyard.contains(card),
            "{card:?} is not a card in p0's own graveyard"
        );
    }
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "CR 601.2c: the target is chosen before the cost is paid, so the \
         {{W}} has not been spent yet"
    );
    assert!(
        !is_tapped(&engine, land),
        "and the land has not paid its {{T}} either"
    );

    let chosen = options[0];
    let left = options[1];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("the card the question offered is the one that pays");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "CR 601.2h: the {{W}} went out of the pool with the last step of the \
         activation"
    );
    assert!(is_tapped(&engine, land), "and tapping it is the other half");
    assert!(
        !stack_is_empty(&engine),
        "the ability is on the stack, waiting to resolve"
    );

    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        library_size(&engine, p0),
        library_before + 1,
        "the card left the graveyard for the library"
    );
    let library: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(
        library.first().copied(),
        Some(chosen),
        "\"on the bottom of your library\" — the list's first entry is the \
         bottom, which is the end `ZonePosition::Bottom` writes to"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .clone(),
        vec![left],
        "and only the card that was named: the other one never moved"
    );

    // The gate from the other side, on a board that differs by one white
    // permanent and nothing else.
    let mut short = Duel::new(SEED, forest())
        .battlefield(0, &[mistveil_plains(), plains(), auriok_bladewarden()])
        .start();
    keep_mulligans(&mut short);
    reach_main_phase(&mut short, p0);
    seed_graveyard(&mut short, p0, 1);
    assert_eq!(
        all_on_battlefield(&short, p0, auriok_bladewarden()).len(),
        1,
        "one white permanent, where the card asks for two"
    );
    let gated = on_battlefield(&short, p0, mistveil_plains()).expect(
        "the setup path places it untapped, which is the only reason \
                 its {T} is payable here at all",
    );
    assert!(!is_tapped(&short, gated), "and it is untapped");
    tap_all_mana_but(&mut short, p0, Some(mistveil_plains()));
    assert_eq!(
        short.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "the {{W}} is floating and the graveyard holds a card, so a refusal \
         cannot be the mana's fault or the target's"
    );
    let Pending::Priority { legal, .. } = short.pending().clone() else {
        panic!("expected priority, got {:?}", short.pending())
    };
    assert!(
        !legal.abilities.contains(&(gated, 1)),
        "\"Activate only if you control two or more white permanents\": with \
         one, the line is not offered at all — {:?}",
        legal.abilities
    );
}
