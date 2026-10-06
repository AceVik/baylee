//! `cards/creatures/mv_7/omnath_locus_of_rage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "1816eede-c5bd-49df-958f-a3af64cb2932"
/// Omnath, Locus of Rage prints two sentences: landfall makes a 5/5 red and
/// green Elemental token, and whenever Omnath or **another Elemental you
/// control** dies, Omnath deals 3 damage to any target.
///
/// Both are played in one main phase. The Elemental arrives by being cast off
/// eight basics, a real `PlayLand` turns landfall on — and the empty token
/// board before that land is the control, because the lands this board was
/// built from were *placed* and never entered. The token is then fed to
/// Ashnod's Altar, whose sacrifice is a cost asked as a `CostSacrifice` menu,
/// so the death the second sentence answers is an Elemental's and not
/// Omnath's own; the three damage goes to the opponent's face, which is one
/// of the answers an "any target" prompt offers.
#[test]
#[allow(clippy::too_many_lines)] // a whole game, as every test in this file is
fn omnath_makes_a_token_for_a_land_and_answers_that_elementals_death_with_three() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                ashnods_altar(),
            ],
        )
        // The Forest is in hand and not on the board: a landfall trigger
        // reads an *entry*, and `starting_battlefield` is a placement.
        .hand(0, &[omnath_locus_of_rage(), forest()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {3}{R}{R}{G}{G} off the eight basics, so the Elemental is in a real
    // game rather than assumed onto the table.
    cast_from_hand(&mut engine, p0, omnath_locus_of_rage());
    pass_until(&mut engine, stack_is_empty);
    let omnath = on_battlefield(&engine, p0, omnath_locus_of_rage()).expect("Omnath resolved");
    assert_eq!(pt(&engine, omnath), (5, 5), "the printed body");
    let altar = on_battlefield(&engine, p0, ashnods_altar()).expect("the Altar stands");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "the lands this board was built from were placed rather than entered, \
         so no landfall has fired yet"
    );

    // Landfall, off a real play.
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one land entered, one Elemental");
    let elemental = tokens[0];
    assert_eq!(
        pt(&engine, elemental),
        (5, 5),
        "a 5/5 with no counter needed"
    );
    let token = engine
        .state()
        .object(elemental)
        .expect("the Elemental is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(token.name, "Elemental");
    assert!(
        token.colors.contains(baylee_core::color::Color::Red)
            && token.colors.contains(baylee_core::color::Color::Green),
        "red and green"
    );
    assert!(
        types(&engine, elemental).contains(TypeSet::CREATURE),
        "and a creature, which is what the second sentence looks for"
    );

    // The second sentence. The Altar's cost names no creature, so the engine
    // asks which one — and the menu is both creatures this seat controls.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(altar, 0)),
        "a creature is out, so the Altar's only line is offered: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, ashnods_altar(), 0);
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "the variant is what tells a client this is a cost and not a search"
    );
    assert_eq!((min, max), (1, 1), "one creature, and the cost asks once");
    assert!(
        options.contains(&elemental),
        "the Elemental is on the menu: {options:?}"
    );
    assert!(
        options.contains(&omnath),
        "and so is the Elemental that made it: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "those two are the whole board: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elemental],
            },
        )
        .expect("the Elemental the question offered pays the cost");

    let asked = settle_aiming_at(&mut engine, p1);
    assert!(
        asked,
        "\"Omnath deals 3 damage to any target\" — the death is asked about, \
         and an any-target prompt offers a player among its answers; \
         the engine stopped at {:?}",
        engine.pending()
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "the Elemental died to pay for the Altar, and it is not Omnath"
    );
    // And it is not in the graveyard either, which is the part worth
    // asserting: a token reaches its owner's graveyard and is swept from it
    // the next time state-based actions are checked (CR 111.7, CR 704.5d), so
    // nobody ever finds it there. The damage below is dealt regardless —
    // "applicable triggered abilities will trigger before the token ceases to
    // exist" is the same sentence, and it is the reason this card works at
    // all.
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .iter()
            .any(|id| engine.state().object(*id).is_some_and(|o| o.card.is_none())),
        "CR 704.5d: no token is left lying in a graveyard"
    );
    assert_eq!(engine.state().players[1].life, 17, "three to the face");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and none of it to the ability's own controller"
    );
    assert!(
        on_battlefield(&engine, p0, omnath_locus_of_rage()).is_some(),
        "the ability's source never moved"
    );
}
