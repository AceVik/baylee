//! `cards/lands/deserts/bristling_backwoods.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bristling Backwoods prints three sentences, and one played turn cycle reads
/// all three: "This land enters tapped", "When this land enters, it deals 1
/// damage to target opponent", and "{T}: Add {R} or {G}". It is played rather
/// than seeded because `starting_battlefield` places a permanent with
/// `Cause::Setup`, where no entry modifier is ever asked — so the tapped
/// arrival and the trigger only exist off a real `PlayLand`. The arrival turn
/// is also where the missing route is the reading: a `{T}` ability that is not
/// offered at all is CR 502.1 seen from the offer instead of from a status bit.
/// And `{T}` asks a question only because the card names *two* colours — a
/// one-colour source is answered without asking — so the mana half is played on
/// the next turn and the named colour has to be the one in the pool.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn bristling_backwoods_arrives_tapped_shoots_an_opponent_and_taps_for_the_colour_it_names() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .life(0, 20)
        .life(1, 20)
        .hand(0, &[bristling_backwoods()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, bristling_backwoods());
    assert!(entered_tapped(&engine, land), "\"This land enters tapped\"");

    // The entry trigger asks for a target opponent. Which of the two forms a
    // player target arrives in is the engine's business; that the *opponent*
    // is the one on the menu is not.
    let mut aimed = false;
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                player_options,
                ..
            } => {
                assert_eq!(player, p0, "the land's controller chooses");
                assert!(
                    player_options.contains(&p1),
                    "the only opponent is a legal target: {player_options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p1],
                        },
                    )
                    .expect("a face the question enumerated is a legal answer");
                aimed = true;
            }
            Pending::ChoosePlayer { player, options } => {
                assert_eq!(player, p0, "the land's controller chooses");
                assert!(
                    options.contains(&p1),
                    "the only opponent is a legal target: {options:?}"
                );
                engine
                    .apply(player, PlayerAction::ChoosePlayer(p1))
                    .expect("a face the question enumerated is a legal answer");
                aimed = true;
            }
            Pending::Priority { player, .. } => {
                if aimed && stack_is_empty(&engine) {
                    break;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the entry trigger resolves: {other:?}"),
        }
    }
    assert!(
        aimed,
        "the entered land asks which opponent takes the point of damage"
    );
    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"it deals 1 damage to target opponent\""
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage lands on the opponent, never on the land's controller"
    );

    // `deeds` reads *both* offer lists, so this is not a claim about one of
    // them: while the land is tapped there is no route off it at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a main phase hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        deeds(&legal, &[land]).is_empty(),
        "the land entered tapped, so its {{T}} is not offered this turn: {:?}",
        deeds(&legal, &[land])
    );

    // CR 502.1: nothing untaps it before p0's own untap step, so the route is
    // read on the next turn — the same walk the held-down artifacts in this
    // suite make, and for the same reason.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p0
            && !is_tapped(e, land)
    });
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a main phase hands priority back, got {:?}",
            engine.pending()
        )
    };
    let routes = deeds(&legal, &[land]);
    assert!(
        !routes.is_empty(),
        "the untapped land offers its printed mana ability: {routes:?}"
    );
    engine
        .apply(p0, routes[0].1.action(land))
        .expect("the offer listed the route it just published");

    // "Add {R} or {G}" is two answers, so it is a question; and the answer
    // has to be the colour that was named and nothing else.
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{G}}\" is a choice of two, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert_eq!(
        options.len(),
        2,
        "two colours is the whole menu: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::Green),
        "and they are the two the card prints: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the two it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "\"or\" is one colour or the other, never both"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}
