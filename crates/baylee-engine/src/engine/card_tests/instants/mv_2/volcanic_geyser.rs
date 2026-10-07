//! `cards/instants/mv_2/volcanic_geyser.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Volcanic Geyser — {X}{R}{R} instant: "Volcanic Geyser deals X damage to any
/// target."
///
/// X is the whole card, so the amount has to be read off a number only one
/// value produces: five Mountains pay {3}{R}{R} exactly, and the opponent's
/// twenty life becomes seventeen — not the five a card reading the mana it ate
/// would deal, and not the zero a card that never asked for X would. The other
/// printed word is "any target" (CR 115.4), so the question is read while it
/// stands: one choice carrying the Elf across the table and both seats, aimed at
/// the player, with the Elf still standing afterwards.
#[test]
#[allow(clippy::too_many_lines)]
fn volcanic_geyser_deals_three_damage_for_x_three_to_the_seat_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        // A creature across the table, so "any target" has an object to offer
        // beside the two seats and the damage has somewhere to *not* go.
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[volcanic_geyser()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Mountains, five red — the board has no other source on it"
    );

    // `can_afford` reads the pool and not the untapped lands, so the claim
    // about the offer is made with the mana already floating.
    let card = in_hand(&engine, p0, volcanic_geyser()).expect("the Geyser is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "five red pays {{3}}{{R}}{{R}}: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, volcanic_geyser());

    // X at CR 601.2b, the target at CR 601.2c, the mana at CR 601.2h. The
    // questions are answered in the order they arrive rather than in the order
    // they are expected.
    let mut asked_for_x = false;
    let mut aimed = false;
    let mut menu: Vec<ObjectId> = Vec::new();
    let mut seats: Vec<PlayerId> = Vec::new();
    for _ in 0..12 {
        match engine.pending().clone() {
            Pending::Priority { .. } => break,
            Pending::ChooseNumber { player, max, .. } => {
                assert_eq!(player, p0, "the caster of the spell names X");
                assert!(
                    max >= 3,
                    "the cost is {{X}}{{R}}{{R}} and five red is floating, so \
                     three has to be on the menu: max is {max}"
                );
                engine
                    .apply(p0, PlayerAction::ChooseNumber(3))
                    .expect("three came out of the range the question published");
                asked_for_x = true;
            }
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the caster aims it");
                assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
                assert_eq!(
                    options,
                    vec![elf],
                    "the only object \"any target\" names here is the creature \
                     across the table — the Mountains are lands: {options:?}"
                );
                assert!(
                    player_options.contains(&p0) && player_options.contains(&p1),
                    "CR 115.4: \"any target\" counts players in the same choice: \
                     {player_options:?}"
                );
                assert_eq!(
                    engine.state().players[0].mana_pool.total(),
                    5,
                    "CR 601.2c before CR 601.2h: the cost is the last step of \
                     the cast, so nothing is spent while the question stands"
                );
                menu = options;
                seats = player_options;
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p1],
                        },
                    )
                    .expect("the seat was one of the targets it enumerated");
                aimed = true;
            }
            other => panic!("unexpected while the Geyser is cast: {other:?}"),
        }
    }
    assert!(
        asked_for_x,
        "X is a choice: a spell that skipped it would deal a fixed amount and \
         leave the pool untouched"
    );
    assert!(aimed, "\"any target\" is a target choice");
    assert_eq!(menu, vec![elf], "the menu the target question published");
    assert!(seats.contains(&p1), "and it carried both seats");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{3}} and {{R}}{{R}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the Geyser is waiting on the stack"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and nothing has happened while it is still there"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        17,
        "\"deals X damage\" with X = 3 — five would mean the mana spent was \
         read instead of the number chosen"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the damage belongs to the target, not to the seat that paid for it"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell did not name never moved, so the three points \
         went to the target and not to the board"
    );
    assert!(
        in_graveyard(&engine, p0, volcanic_geyser()).is_some(),
        "and the instant resolved into its owner's graveyard"
    );
}
