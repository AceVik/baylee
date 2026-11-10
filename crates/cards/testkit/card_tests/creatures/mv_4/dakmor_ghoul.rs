//! `cards/creatures/mv_4/dakmor_ghoul.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dakmor Ghoul — {2}{B}{B} Creature — Zombie (2/2): "When this creature
/// enters, target opponent loses 2 life and you gain 2 life."
///
/// The sentence lands on two different seats, so one cast reads both halves at
/// once: the seat across the table is two life down and the seat that cast the
/// Ghoul is two life up, which a drain touching only one of them could not
/// satisfy. The menu the entry trigger publishes is the second claim —
/// "target opponent" offers p1 and never p0 — and the untouched life totals
/// while the trigger waits on the stack are what say the loss is the
/// resolution rather than the cast.
#[test]
fn dakmor_ghoul_drains_two_life_from_an_opponent_into_its_controllers_total() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[dakmor_ghoul()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Four Swamps pay {2}{B}{B}, and the Ghoul's entry trigger is what asks
    // the next question.
    cast_from_hand(&mut engine, p0, dakmor_ghoul());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseTargets { .. } | Pending::ChoosePlayer { .. }
        )
    });

    let menu: Vec<PlayerId> = match engine.pending().clone() {
        Pending::ChooseTargets {
            player,
            player_options,
            min,
            max,
            ..
        } => {
            assert_eq!(
                player, p0,
                "the seat that cast the Ghoul names the opponent"
            );
            assert_eq!(
                (min, max),
                (1, 1),
                "one opponent, and the trigger asks once"
            );
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .expect("the opponent was one of the targets it enumerated");
            player_options
        }
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(
                player, p0,
                "the seat that cast the Ghoul names the opponent"
            );
            engine
                .apply(p0, PlayerAction::ChoosePlayer(p1))
                .expect("the opponent was one of the targets it enumerated");
            options
        }
        other => panic!("\"target opponent\" is a target choice, got {other:?}"),
    };
    assert!(
        menu.contains(&p1),
        "\"target opponent\" offers the seat across the table: {menu:?}"
    );
    assert!(
        !menu.contains(&p0),
        "and never the Ghoul's own controller — \"opponent\" is not \"any player\": {menu:?}"
    );

    // The drain is the trigger's own resolution, so nothing has happened while
    // it is standing on the stack unanswered.
    assert_eq!(
        (
            engine.state().players[0].life,
            engine.state().players[1].life
        ),
        (20, 20),
        "the trigger has not resolved yet, so neither life total has moved"
    );

    pass_until(&mut engine, stack_is_empty);

    let ghoul = on_battlefield(&engine, p0, dakmor_ghoul()).expect("the Ghoul resolved");
    assert_eq!(pt(&engine, ghoul), (2, 2), "the body the card prints");
    assert_eq!(
        engine.state().players[1].life,
        18,
        "\"target opponent loses 2 life\" — two off the seat that was named"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "\"and you gain 2 life\" — the same two, on the seat that cast it"
    );
}
