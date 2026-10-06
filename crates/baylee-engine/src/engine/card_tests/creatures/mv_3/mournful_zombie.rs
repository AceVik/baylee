//! `cards/creatures/mv_3/mournful_zombie.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mournful Zombie prints one line — "{W}, {T}: Target player gains 1 life."
/// — and the word that carries it is *target*: the life goes to whoever was
/// named, which is the opposite of a Zombie helping the seat that paid for it.
/// So the ability is aimed at the opponent and both life totals are read
/// afterwards: p1 gains the one life, and p0 — a Plains and a tap poorer —
/// gains nothing. Both halves of the price leave a mark, so both are read: the
/// {W} is the white that leaves a pool three Swamps could never have put
/// there, and the tapped Zombie is the {T}.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn mournful_zombie_gives_the_life_to_the_player_it_names_and_not_to_its_controller() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), plains()])
        .hand(0, &[mournful_zombie()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {2}{B} off the three Swamps alone: the Plains is the {W} the ability
    // charges, and it is kept back so the cast cannot spend it first.
    tap_all_mana_but(&mut engine, p0, Some(plains()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Swamps, and no white in the pool"
    );
    cast_with_floating(&mut engine, p0, mournful_zombie());
    pass_until(&mut engine, stack_is_empty);
    let zombie = on_battlefield(&engine, p0, mournful_zombie()).expect("the Zombie resolved");
    assert_eq!(pt(&engine, zombie), (2, 1), "the printed 2/1 body");

    // Its price is its own tap, and a creature cast this turn is summoning
    // sick (CR 302.6), so the activation belongs to the next turn.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, zombie), "it untapped on the way round");

    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        (
            pool.available(ManaColor::Black),
            pool.available(ManaColor::White)
        ),
        (3, 1),
        "the three Swamps and the one Plains that can pay the {{W}}"
    );

    activate(&mut engine, p0, mournful_zombie(), 0);
    // "target player" is any player at the table, which is what makes the
    // answer below a choice rather than a rule — and it arrives as one of two
    // questions, so both are read rather than one assumed.
    let (chooser, answer) = match engine.pending().clone() {
        Pending::ChooseTargets {
            player,
            player_options,
            ..
        } => {
            assert!(
                player_options.contains(&p0) && player_options.contains(&p1),
                "either seat may be named: {player_options:?}"
            );
            (
                player,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
        }
        Pending::ChoosePlayer { player, options } => {
            assert!(
                options.contains(&p0) && options.contains(&p1),
                "either seat may be named: {options:?}"
            );
            (player, PlayerAction::ChoosePlayer(p1))
        }
        other => panic!("`target player` is a choice of player, got {other:?}"),
    };
    assert_eq!(chooser, p0, "the activating seat is the one that names it");
    assert!(
        !is_tapped(&engine, zombie),
        "the target is answered before the price is paid (CR 601.2c, then CR 601.2h)"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "and the {{W}} is still in the pool for the same reason"
    );
    engine
        .apply(chooser, answer)
        .expect("the seat the question offered is a legal answer");

    assert!(
        is_tapped(&engine, zombie),
        "{{T}} is the other half of the price"
    );
    assert!(
        !stack_is_empty(&engine),
        "gaining life is no mana ability, so the ability waits on the stack"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 0, "the {{W}} was paid");
    assert_eq!(
        pool.available(ManaColor::Black),
        3,
        "and it was the white that paid it: the Swamps' mana is untouched"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "nothing has been gained yet — the ability resolves off the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        21,
        "\"target player gains 1 life\" — the seat that was named, and one life"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the seat that paid the price and the tap gains nothing"
    );
}
