//! `cards/lands/utility/castle_doom.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Castle Doom prints `{{T}}: Add {{C}}.`, `{{T}}: Add one mana of any color. Spend this mana
/// only to cast an artifact spell.`, and `{{3}}, {{T}}, Sacrifice an artifact: Create a 3/3
/// colorless Robot Villain artifact creature token named Doombot. Activate only as a sorcery.`
///
/// All three lines are on the offer: with three green mana floating from
/// `forest()` lands, Castle Doom untapped and an artifact standing to be
/// sacrificed (`quiet_artifact()`), ability index 2 can be paid for. What
/// this test is really about is the middle one — activating index 1 asks a
/// colour through `Pending::ChooseColor` and puts the mana in
/// `pool.restricted()` rather than in the general pool, which is the half a
/// plain `available()` reading would have called a missing mana ability.
#[test]
fn castle_doom_produces_restricted_mana_and_offers_its_doombot() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                castle_doom(),
                forest(),
                forest(),
                forest(),
                quiet_artifact(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let doom = on_battlefield(&engine, p0, castle_doom()).expect("castle doom on battlefield");

    // Float {3} green and the Sol Ring's {C}{C} while keeping Castle Doom
    // untapped: five is more than the {3} the omitted ability charges, and
    // the price has to be floated or `can_afford` reads an empty pool and
    // the ability is missing from the offer for the wrong reason.
    tap_mana_except(&mut engine, p0, doom);
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);
    assert!(!is_tapped(&engine, doom));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(doom, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        legal.abilities.contains(&(doom, 1)),
        "ability 1 ({{T}}: Add restricted mana) is offered"
    );
    assert!(
        legal.abilities.contains(&(doom, 2)),
        "ability 2 (the Doombot) is offered with {{3}} floating and an artifact to sacrifice"
    );

    activate(&mut engine, p0, castle_doom(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected ChooseColor, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(options, baylee_cards_dsl::ALL_MANA_COLORS.to_vec());
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Blue);
    assert_eq!(pool.available(ManaColor::Green), 3);
    assert!(is_tapped(&engine, doom));
}

/// Castle Doom: "{3}, {T}, Sacrifice an artifact: Create a 3/3 colorless
/// Robot Villain artifact creature token named Doombot. Activate only as a
/// sorcery." The card says "an artifact" and not "an artifact you control",
/// and CR 701.21a is why that is enough: the opponent's Collar is on the
/// battlefield and is not in the menu. The Doombot is asserted as an
/// artifact *creature*, which is the half a plain `TypeSet::CREATURE` check
/// would have missed.
#[test]
fn castle_doom_sacrifices_only_an_artifact_its_own_controller_has() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9105, forest())
        .battlefield(
            0,
            &[
                castle_doom(),
                forest(),
                forest(),
                forest(),
                basilisk_collar(),
            ],
        )
        .battlefield(1, &[basilisk_collar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let castle = on_battlefield(&engine, p0, castle_doom()).expect("the Castle is seated");
    let mine = on_battlefield(&engine, p0, basilisk_collar()).expect("my Collar");
    let theirs = on_battlefield(&engine, p1, basilisk_collar()).expect("their Collar");
    tap_mana_except(&mut engine, p0, castle);

    activate(&mut engine, p0, castle_doom(), 2);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected the sacrifice choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![mine],
        "a player may sacrifice only what they control (CR 701.21a)"
    );
    assert!(
        !options.contains(&theirs),
        "and the opponent's artifact is no part of the price"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("my own Collar is a legal answer");

    pass_until(&mut engine, stack_is_empty);
    let made = tokens_of(&engine, p0);
    assert_eq!(made.len(), 1, "one Doombot");
    let bot = made[0];
    assert_eq!(pt(&engine, bot), (3, 3));
    let kinds = types(&engine, bot);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::CREATURE),
        "\"artifact creature token\": {kinds:?}"
    );
    let printed = engine
        .state()
        .object(bot)
        .expect("the Doombot is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(printed.name, "Doombot");
    assert!(
        printed.colors.is_empty(),
        "\"a 3/3 colorless Robot Villain\""
    );
}
