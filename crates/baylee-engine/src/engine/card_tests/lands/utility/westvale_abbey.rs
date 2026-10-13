//! `cards/lands/utility/westvale_abbey.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn westvale_abbey_taps_for_colorless_and_sells_a_life_for_a_black_white_cleric() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                westvale_abbey(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let abbey = on_battlefield(&engine, p0, westvale_abbey()).expect("the Abbey is on the table");
    assert!(
        !is_tapped(&engine, abbey),
        "a land placed on the battlefield arrives untapped"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing floats before anything is tapped"
    );

    activate(&mut engine, p0, westvale_abbey(), 0);
    assert!(
        !matches!(engine.pending(), Pending::ChooseColor { .. }),
        "`{{C}}` is a fixed colourless and not \"any color\", so there is \
         nothing to name: {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1, "{{T}}: Add {{C}}");
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "and not a single colour beside it"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, abbey), "the Abbey paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting"
    );

    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the Abbey's controller takes another turn"
    );
    assert!(
        !is_tapped(&engine, abbey),
        "the untap step stood the land back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool the {{C}} was in is gone"
    );

    tap_all_mana_but(&mut engine, p0, Some(westvale_abbey()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six tapped Forests and an untapped Abbey, which makes no mana of its own"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nobody has paid a life yet"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(abbey, 1)),
        "with {{5}} in the pool the token line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, westvale_abbey(), 1);
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"Pay 1 life\" is part of the price: one, and never a life per mana spent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{5}} came out of the pool, so one green is left"
    );
    assert!(
        is_tapped(&engine, abbey),
        "{{T}} was the other half of the price"
    );
    assert!(
        !stack_is_empty(&engine),
        "making a token is no mana ability, so the ability is on the stack"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and the Cleric arrives on resolution, not on announcement"
    );

    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Human Cleric");
    let cleric = engine
        .state()
        .object(tokens[0])
        .expect("the Cleric is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(cleric.name, "Human Cleric", "the name the card gives it");
    assert_eq!(
        (cleric.power, cleric.toughness),
        (Some(1), Some(1)),
        "\"a 1/1\""
    );
    assert!(
        cleric.colors.contains(baylee_core::color::Color::White),
        "\"...white...\""
    );
    assert!(
        cleric.colors.contains(baylee_core::color::Color::Black),
        "and \"...black\": a token that was white alone would fill the body and \
         the name just as well"
    );
    assert!(
        types(&engine, tokens[0]).contains(TypeSet::CREATURE),
        "a creature token and not a permanent of some other card type"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the life belongs to the seat that paid it, not to the opponent"
    );
    assert!(
        on_battlefield(&engine, p0, westvale_abbey()).is_some(),
        "the price was a tap and no sacrifice, so the Abbey stays to make another"
    );
}

#[test]
fn westvale_abbey_sacrifices_five_real_creatures_and_untaps_the_same_object() {
    let p0 = PlayerId::new(0);
    let mut board = vec![westvale_abbey()];
    board.extend([forest(); 5]);
    board.extend([llanowar_elves(); 5]);
    let mut e = Duel::new(SEED, forest()).battlefield(0, &board).start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let abbey = on_battlefield(&e, p0, westvale_abbey()).unwrap();
    let version = e.state().object(abbey).unwrap().version;
    tap_all_mana_but(&mut e, p0, Some(westvale_abbey()));
    activate(&mut e, p0, westvale_abbey(), 2);
    for _ in 0..5 {
        let Pending::ChooseCards { options, .. } = e.pending().clone() else {
            panic!("{:?}", e.pending())
        };
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    }
    assert_eq!(e.state().zones.list(ZoneLocation::Graveyard(p0)).len(), 5);
    pass_until(&mut e, stack_is_empty);
    let object = e.state().object(abbey).unwrap();
    assert_eq!((object.version, object.face_index), (version, 1));
    assert_eq!(pt(&e, abbey), (9, 7));
    assert_eq!(
        e.state().object(abbey).unwrap().characteristics().colors,
        baylee_core::color::ColorSet::of(baylee_core::color::Color::Black),
        "Ormendahl has a black color indicator and no mana cost, so it is black and only black (CR 202.2e)"
    );
    assert!(!is_tapped(&e, abbey));
    assert!(
        keywords(&e, abbey).contains(
            KeywordSet::HASTE
                .union(KeywordSet::FLYING)
                .union(KeywordSet::LIFELINK)
                .union(KeywordSet::INDESTRUCTIBLE)
        )
    );
}
