//! `cards/lands/artifacts/thornglint_bridge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thornglint Bridge prints three lines and this test plays all three in one
/// game: an artifact land that **enters tapped**, that is indestructible, and
/// whose `{T}` adds one mana of **either green or white**. The order is what
/// makes the mana line readable at all — a land that arrives tapped cannot pay
/// its own `{T}` until the untap step has stood it back up, so the offer is
/// asserted empty first and the colour question only after a full turn cycle.
/// Survival is read off a real destroy effect aimed at a permanent the spell is
/// genuinely allowed to name, which is the half that keeps the keyword honest.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn thornglint_bridge_enters_tapped_survives_destruction_and_makes_green_or_white() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[thornglint_bridge()])
        // Vindicate ({1}{W}{B}) off three lands: a destroy effect whose target
        // is any permanent, which the Bridge is twice over.
        .battlefield(1, &[plains(), plains(), swamp()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bridge = play_land(&mut engine, p0, thornglint_bridge());
    assert!(
        entered_tapped(&engine, bridge),
        "\"This land enters tapped\" — a real land drop, so the entry modifier runs"
    );
    let kinds = types(&engine, bridge);
    assert!(
        kinds.contains(TypeSet::LAND) && kinds.contains(TypeSet::ARTIFACT),
        "an artifact land carries both types at once, which is why a spell that \
         names either may point at it: {kinds:?}"
    );
    assert!(
        keywords(&engine, bridge).contains(KeywordSet::INDESTRUCTIBLE),
        "and the keyword the layers project is the one the card prints"
    );

    // A land that arrived tapped gives nothing this turn (CR 118.3): the {T} is
    // the whole price, so the ability cannot even be offered.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a main phase hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == bridge),
        "a tapped land cannot pay its own {{T}}: {:?}",
        legal.abilities
    );

    // p1's turn, and a destruction the Bridge survives.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"destroy target permanent\" asks, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the caster names the target");
    assert!(
        options.contains(&bridge),
        "a land is a permanent, so the Bridge is on the menu — that offer is \
         what makes surviving it the keyword doing work: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![bridge],
            },
        )
        .expect("the option the question itself enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, thornglint_bridge()).is_some(),
        "indestructible: a destroy effect leaves it where it stands"
    );
    assert!(
        in_graveyard(&engine, p0, thornglint_bridge()).is_none(),
        "and it is not in the graveyard, which is where a destroyed land would be"
    );

    // Back around to p0: that is where the untap step has been and gone.
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 takes another main phase"
    );
    assert!(
        !is_tapped(&engine, bridge),
        "the untap step stood it back up, which is what the tapped assertion \
         above was waiting for"
    );

    activate(&mut engine, p0, thornglint_bridge(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{W}}\" is a colour question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that taps it names the colour");
    assert!(
        options.len() == 2
            && options.contains(&ManaColor::Green)
            && options.contains(&ManaColor::White),
        "the two colours the card prints and no third — not the five of an \
         \"any colour\" Mox, and not a colourless: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the two it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the colour that was named"
    );
    assert_eq!(pool.available(ManaColor::Green), 0, "and not the other one");
    assert_eq!(pool.total(), 1, "one land, one tap, one mana");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, bridge), "the {{T}} is what paid for it");
}
