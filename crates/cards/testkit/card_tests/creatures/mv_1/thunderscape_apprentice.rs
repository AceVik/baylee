//! `cards/creatures/mv_1/thunderscape_apprentice.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thunderscape Apprentice — {R}, a 1/1 Human Wizard — prints two activated
/// abilities, one black and one green, and each pays for itself with {T}:
/// "{B}, {T}: Target player loses 1 life" and "{G}, {T}: Target creature gets
/// +1/+1 until end of turn."
///
/// Two Apprentices stand on the board because the tap symbol is half of each
/// price, so one body can pay for one line and no more — a test that asserted
/// both off the same card would be proving that a tapped creature activates.
/// The pump's filter is `Filter::CREATURE` and says nothing about control, so
/// the Elf across the table is a legal target the option list has to name,
/// while the one beside it is the creature that actually takes the +1/+1 —
/// which is what makes the Elf that stays a printed 1/1 a reading and not an
/// assumption.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn thunderscape_apprentice_drains_a_player_and_pumps_a_creature_with_two_bodies() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(4711, forest())
        .battlefield(
            0,
            &[
                thunderscape_apprentice(),
                thunderscape_apprentice(),
                swamp(),
                forest(),
                quiet_creature(),
            ],
        )
        .battlefield(1, &[quiet_creature()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, quiet_creature()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and so is the one across the table"
    );

    // The Swamp and the Forest, with the Elf named as the one thing kept
    // back: a creature's own `{T}: Add {G}` is a mana route like any other
    // (#159), and tapping it would have put a third mana in the pool that
    // this scenario never counted.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "{{B}} off the Swamp and {{G}} off the Forest, which is exactly both \
         prices and nothing from the Elf"
    );

    // Ability 0: "{B}, {T}: Target player loses 1 life." A player is not an
    // object, so the question can arrive either way; both are answered with
    // p1, the seat "target player" reaches across the table.
    activate(&mut engine, p0, thunderscape_apprentice(), 0);
    match engine.pending().clone() {
        Pending::ChooseTargets {
            player,
            player_options,
            ..
        } => {
            assert_eq!(player, p0, "the activating seat chooses");
            assert!(
                player_options.contains(&p1),
                "\"target player\" reaches the other seat: {player_options:?}"
            );
            engine
                .apply(
                    player,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .unwrap();
        }
        Pending::ChoosePlayer { player, options } => {
            assert!(
                options.contains(&p1),
                "\"target player\" reaches the other seat: {options:?}"
            );
            engine
                .apply(player, PlayerAction::ChoosePlayer(p1))
                .unwrap();
        }
        other => panic!("expected a player target, got {other:?}"),
    }
    pass_until(&mut engine, |e| e.state().players[1].life == 19);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "the seat that was named lost the life"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the seat that activated lost nothing, which is the half a drain \
         pointed at the wrong player would have passed too"
    );

    // Ability 1: "{G}, {T}: Target creature gets +1/+1 until end of turn."
    activate(&mut engine, p0, thunderscape_apprentice(), 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the pump targets a creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat chooses");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| pt(e, mine) == (2, 2));

    assert_eq!(
        pt(&engine, mine),
        (2, 2),
        "+1/+1 until end of turn on the creature that was named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the creature it did not name is still the 1/1 it was printed as"
    );
    assert!(
        all_on_battlefield(&engine, p0, thunderscape_apprentice())
            .iter()
            .all(|id| is_tapped(&engine, *id)),
        "both lines were paid for with {{T}}, so both bodies are tapped"
    );
}
