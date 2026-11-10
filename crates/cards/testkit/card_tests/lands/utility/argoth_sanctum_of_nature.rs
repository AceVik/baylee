//! `cards/lands/utility/argoth_sanctum_of_nature.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

/// Argoth, Sanctum of Nature prints `This land enters tapped unless you control a legendary
/// green creature.`, `{T}: Add {G}.`, `{2}{G}{G}, {T}: Create a 2/2 green Bear creature
/// token, then mill three cards. Activate only as a sorcery.`, and `(Melds with Titania, Voice of Gaea.)`
///
/// Without a legendary green creature, playing this land enters tapped. After advancing to the
/// next turn, it untaps and taps for one green mana;
/// `argoth_sanctum_of_nature_makes_a_bear_and_mills_three` plays the Bear ability, and the tests
/// below it play the "unless you control a legendary green creature" condition, the Bear token
/// and the sorcery-speed restriction.
#[test]
fn argoth_sanctum_of_nature_enters_tapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[argoth_sanctum_of_nature()])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, argoth_sanctum_of_nature());
    assert!(
        entered_tapped(&engine, land),
        "without a legendary green creature, Argoth enters tapped"
    );

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    // Float {4} green mana while keeping Argoth untapped.
    tap_mana_except(&mut engine, p0, land);
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);

    activate(&mut engine, p0, argoth_sanctum_of_nature(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 5);
    assert_eq!(pool.total(), 5);
    assert!(is_tapped(&engine, land));
}

/// Argoth's `{{2}}{{G}}{{G}}, {{T}}: Create a 2/2 green Bear creature token, then
/// mill three cards. Activate only as a sorcery.` One Bear, and exactly three
/// cards from the library to the graveyard.
#[test]
fn argoth_sanctum_of_nature_makes_a_bear_and_mills_three() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                argoth_sanctum_of_nature(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, argoth_sanctum_of_nature()).expect("Argoth deployed");
    tap_mana_except(&mut engine, p0, land);
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).len();
    let graveyard = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();

    activate(&mut engine, p0, argoth_sanctum_of_nature(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, land));
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Bear");
    assert_eq!(pt(&engine, tokens[0]), (2, 2));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        library - 3
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        graveyard + 3
    );
}

/// "This land enters tapped **unless you control a legendary green creature**":
/// Rubinia Soulsinger, a legendary green creature, is on the battlefield, so
/// Argoth comes in untapped and taps for `{G}` the turn it is played.
#[test]
fn argoth_sanctum_of_nature_enters_untapped_with_a_legendary_green_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[index::RUBINIA_SOULSINGER])
        .hand(0, &[argoth_sanctum_of_nature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, argoth_sanctum_of_nature());

    assert!(
        !entered_tapped(&engine, land),
        "a legendary green creature is under its controller's control: it enters untapped"
    );
    activate(&mut engine, p0, argoth_sanctum_of_nature(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "and it can tap for {{G}} at once"
    );
}

/// The condition is *legendary* **and** *green* **and** *you control*: a
/// creature that is only some of those does not excuse the tapped entry.
/// Each board below has a creature that fails exactly one part.
#[test]
fn argoth_sanctum_of_nature_enters_tapped_unless_a_creature_meets_every_part_of_the_condition() {
    let p0 = PlayerId::new(0);
    let cases: [(&[CardIndex], &[CardIndex], &str); 3] = [
        (
            &[index::AYESHA_TANAKA],
            &[],
            "a legendary creature that is not green",
        ),
        (
            &[llanowar_elves()],
            &[],
            "a green creature that is not legendary",
        ),
        (
            &[],
            &[index::RUBINIA_SOULSINGER],
            "a legendary green creature the opponent controls",
        ),
    ];
    for (mine, theirs, why) in cases {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, mine)
            .battlefield(1, theirs)
            .hand(0, &[argoth_sanctum_of_nature()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let land = play_land(&mut engine, p0, argoth_sanctum_of_nature());

        assert!(
            entered_tapped(&engine, land),
            "{why} does not make Argoth enter untapped"
        );
    }
}

/// "Create a 2/2 green Bear creature token": the token is named Bear, is a
/// 2/2 green creature, and is controlled by the player who activated Argoth.
#[test]
fn argoth_sanctum_of_nature_bear_is_a_green_creature_named_bear() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                argoth_sanctum_of_nature(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, argoth_sanctum_of_nature()).expect("Argoth deployed");
    tap_mana_except(&mut engine, p0, land);

    activate(&mut engine, p0, argoth_sanctum_of_nature(), 1);
    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Bear");
    let bear = engine
        .state()
        .object(tokens[0])
        .expect("the token is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(bear.name, "Bear", "the token the card names");
    assert_eq!((bear.power, bear.toughness), (Some(2), Some(2)), "a 2/2");
    assert!(
        bear.colors.contains(baylee_core::color::Color::Green),
        "a green Bear"
    );
    assert!(
        types(&engine, tokens[0]).contains(TypeSet::CREATURE),
        "and a creature, not merely a permanent"
    );
}

/// "Activate only as a sorcery": not on the opponent's turn, though Argoth is
/// untapped and the mana is there. After the opponent has passed priority in
/// their own main phase it is `p0`'s turn to act, and the mana ability is
/// offered (Argoth is a land that taps for mana) but the Bear ability is not.
#[test]
fn argoth_sanctum_of_nature_makes_no_bear_on_the_opponents_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                argoth_sanctum_of_nature(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, argoth_sanctum_of_nature()).expect("Argoth deployed");

    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_mana_except(&mut engine, p0, land);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "{{2}}{{G}}{{G}} is floating and Argoth is untapped"
    );
    let Pending::Priority { player, legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "p0 has priority in p1's main phase");

    assert!(
        legal.abilities.contains(&(land, 0)),
        "Argoth's mana ability is offered at instant speed: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "\"Activate only as a sorcery\": the Bear ability is not offered on the opponent's turn"
    );
}

/// "Activate only as a sorcery": not while a spell is on the stack, even in
/// the controller's own main phase. A Lightning Bolt is cast first, and with
/// it unresolved the Bear ability is withheld.
#[test]
fn argoth_sanctum_of_nature_makes_no_bear_with_a_spell_on_the_stack() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                argoth_sanctum_of_nature(),
                forest(),
                forest(),
                forest(),
                forest(),
                mountain(),
            ],
        )
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, argoth_sanctum_of_nature()).expect("Argoth deployed");
    let forests = all_on_battlefield(&engine, p0, forest());

    let mountains = all_on_battlefield(&engine, p0, mountain());
    tap_mana_where(&mut engine, p0, |id| mountains.contains(&id));
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("a player is any target");
    tap_mana_where(&mut engine, p0, |id| forests.contains(&id));
    let Pending::Priority { player, legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "p0 holds priority with the Bolt on the stack");
    assert!(!stack_is_empty(&engine), "the Bolt is on the stack");

    assert!(
        legal.abilities.contains(&(land, 0)),
        "Argoth's mana ability is offered: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "\"Activate only as a sorcery\": not while a spell is on the stack"
    );
}
