//! `cards/creatures/mv_1/stormscape_apprentice.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Stormscape Apprentice — {U}, 1/1 Human Wizard — prints two lines, and both
/// of them are gated on `{T}`: "{W}, {T}: Tap target creature" and
/// "{B}, {T}: Target player loses 1 life". One copy can pay for at most one of
/// them a turn, so the board carries two, one per line, over a Plains, a Swamp
/// and an Island — the three lands float exactly `{W}`, `{B}` and `{U}`, so
/// neither activation is refused for mana and the leftover `{U}` then buys the
/// third copy, cast *last* because a creature that arrived this turn could not
/// have paid either of the taps above (CR 302.6). "Target creature" is the
/// whole table and not just the far side of it, and the black line's target is
/// a player, so the question it raises is the player question and not a
/// permanent one.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn stormscape_apprentice_taps_a_creature_for_white_and_a_player_for_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                swamp(),
                island(),
                stormscape_apprentice(),
                stormscape_apprentice(),
            ],
        )
        .hand(0, &[stormscape_apprentice()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let copies = all_on_battlefield(&engine, p0, stormscape_apprentice());
    assert_eq!(
        copies.len(),
        2,
        "two Apprentices stand, neither summoning sick"
    );
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");

    // {W}{B}{U} off the three lands, and nothing else on this board taps for
    // mana: the pool is the whole of what the three activations below spend.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "a Plains, a Swamp and an Island, and no mana creature beside them"
    );

    // The white line: "{W}, {T}: Tap target creature."
    activate(&mut engine, p0, stormscape_apprentice(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "`tap target creature` asks for a target, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elf) && options.iter().all(|id| copies.contains(id) || *id == elf),
        "\"target creature\" is every creature at the table — the Elf across \
         it and both of this seat's own Apprentices, and nothing else: {options:?}"
    );
    assert_eq!(options.len(), 3, "those three are the whole menu");

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        0,
        "the {{W}} is paid on announcement (CR 601.2h), before the ability \
         has done anything"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_tapped(&engine, elf),
        "and the creature it named is tapped"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "{{B}} and {{U}} are left, and the {{W}} is gone"
    );

    // The black line, on the copy that has not yet paid a tap. The {B} is
    // already in the pool, so what keeps the tapped copy out of the offer is
    // CR 118.3 and not a price this board cannot pay.
    let tapped: Vec<ObjectId> = copies
        .iter()
        .copied()
        .filter(|id| is_tapped(&engine, *id))
        .collect();
    assert_eq!(tapped.len(), 1, "{{T}} tapped exactly one Apprentice");
    let standing = copies
        .iter()
        .copied()
        .find(|id| !is_tapped(&engine, *id))
        .expect("and left the other one standing");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(standing, 1)),
        "the standing Apprentice is offered the black line off the {{B}} \
         already floating: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == tapped[0]),
        "while the copy that tapped for the white line offers nothing at all"
    );

    activate(&mut engine, p0, stormscape_apprentice(), 1);
    match engine.pending().clone() {
        Pending::ChoosePlayer { player, options } => {
            assert!(
                options.contains(&p0) && options.contains(&p1),
                "\"target player\" is either seat, and which one is the \
                 choice: {options:?}"
            );
            engine
                .apply(player, PlayerAction::ChoosePlayer(p1))
                .expect("the seat the question enumerated");
        }
        Pending::ChooseTargets {
            player,
            player_options,
            ..
        } => {
            assert!(
                player_options.contains(&p0) && player_options.contains(&p1),
                "\"target player\" is either seat, and which one is the \
                 choice: {player_options:?}"
            );
            engine
                .apply(
                    player,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .expect("the seat the question enumerated");
        }
        other => panic!("\"target player\" asks for a player, got {other:?}"),
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"target player loses 1 life\" — the seat that was named"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and not the seat that activated the Apprentice"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{B}} is spent and the {{U}} is still floating"
    );

    // And the {U} has a buyer: a third copy, cast last, which resolves as the
    // printed 1/1 and empties the pool the three costs drew on.
    cast_with_floating(&mut engine, p0, stormscape_apprentice());
    pass_until(&mut engine, stack_is_empty);
    let arrived = all_on_battlefield(&engine, p0, stormscape_apprentice())
        .into_iter()
        .find(|id| !copies.contains(id))
        .expect("the cast copy is a new object on the battlefield");
    assert_eq!(pt(&engine, arrived), (1, 1), "a printed 1/1 for {{U}}");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool the three activations drew on is empty"
    );
}
