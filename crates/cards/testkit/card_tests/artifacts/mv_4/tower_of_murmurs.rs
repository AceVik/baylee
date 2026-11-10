//! `cards/artifacts/mv_4/tower_of_murmurs.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tower of Murmurs — {4} artifact: "{8}, {T}: Target player mills eight
/// cards."
///
/// Both halves of that price are invisible in the card file, so twelve Forests
/// pay the {4} the cast costs and leave exactly the {8} the ability charges
/// floating beside it — the offer is read once the mana is really in the pool,
/// because `can_afford` reads the pool and not the untapped lands. The target
/// is the other half: "target player" names one seat of the two, so the
/// library and the graveyard are read on *both* seats, which a Whetstone that
/// says "each player" could not satisfy; and CR 601.2c before CR 601.2h is
/// what leaves the artifact untapped and the eight still floating while the
/// question of who is being milled stands unanswered.
#[test]
#[allow(clippy::too_many_lines)]
fn tower_of_murmurs_taps_and_eight_mana_to_mill_eight_off_one_named_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 12])
        .hand(0, &[tower_of_murmurs()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Twelve Forests into the pool first: the {4} for the artifact and the {8}
    // its ability charges are one payment, and CR 500.5 keeps what is left in
    // the pool because the whole scenario stays inside this one main phase.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        12,
        "twelve Forests tapped, and the Tower makes no mana of its own"
    );
    cast_with_floating(&mut engine, p0, tower_of_murmurs());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let tower = on_battlefield(&engine, p0, tower_of_murmurs()).expect("the Tower resolved");
    assert!(!is_tapped(&engine, tower), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "the cast's {{4}} is spent and exactly the {{8}} the ability charges is left"
    );

    // `legal.abilities` is filtered through `can_afford`, which reads the pool
    // rather than the untapped lands — so the claim about the offer is made
    // with the mana already floating, which is where the engine reads it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(tower, 0)),
        "with {{8}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    let my_library = library_size(&engine, p0);
    let their_library = library_size(&engine, p1);
    let my_yard = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    let their_yard = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();

    activate(&mut engine, p0, tower_of_murmurs(), 0);

    // CR 601.2c names the target before CR 601.2h pays for it: while the
    // question of who is being milled is open, nothing has been spent.
    assert!(
        !is_tapped(&engine, tower),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "and the {{8}} is still floating while the question stands"
    );

    match engine.pending().clone() {
        Pending::ChooseTargets {
            player,
            player_options,
            ..
        } => {
            assert_eq!(player, p0, "the activating seat is the one that aims it");
            assert!(
                player_options.contains(&p0) && player_options.contains(&p1),
                "\"target player\" reaches both seats of the table: {player_options:?}"
            );
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .expect("the opponent seat was one of the targets it enumerated");
        }
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, p0, "the activating seat is the one that aims it");
            assert!(
                options.contains(&p1),
                "both seats are legal \"target player\"s: {options:?}"
            );
            engine
                .apply(p0, PlayerAction::ChoosePlayer(p1))
                .expect("the opponent seat was one of the targets it enumerated");
        }
        other => panic!("\"target player\" is a target choice, got {other:?}"),
    }

    assert!(
        is_tapped(&engine, tower),
        "{{T}} is paid by the Tower itself"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{8}} it charges came out of the pool: an activation that had \
         skipped its generic cost would have left the eight floating"
    );
    assert!(
        !stack_is_empty(&engine),
        "milling is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p1),
        their_library - 8,
        "\"mills eight cards\" — eight off the top of the named player's library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        their_yard + 8,
        "and the eight are in that player's graveyard, not merely gone from the library"
    );
    assert_eq!(
        library_size(&engine, p0),
        my_library,
        "\"target player\" is one seat: the seat that activated mills nothing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        my_yard,
        "and its graveyard is untouched, which \"each player\" would not leave it"
    );
    assert!(
        on_battlefield(&engine, p0, tower_of_murmurs()).is_some(),
        "the price was a tap and eight mana, so the Tower is still standing"
    );
}
