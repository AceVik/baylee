//! `cards/sorceries/mv_5/mana_geyser.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mana Geyser — {3}{R}{R} sorcery: "Add {R} for each tapped land your
/// opponents control."
///
/// Every word of that sentence needs its own witness, so the board carries
/// both of them. Seat 1 taps three of its four lands for mana in its own main
/// phase, which is enough to leave them tapped through seat 0's next turn
/// (CR 502.3 untaps only the active player's permanents) while the fourth
/// land stays standing — that one is what says the word "tapped" was read.
/// Seat 0 then taps its own five Mountains to pay the {3}{R}{R}, which is the
/// other half: eight tapped lands lie on the battlefield when the spell
/// resolves and the printed sentence counts three.
#[test]
fn mana_geyser_adds_one_red_for_each_tapped_land_an_opponent_controls() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[mana_geyser()])
        // Three lands to tap and one to leave standing, so the count below
        // has something to decline rather than merely something to count.
        .battlefield(1, &[mountain(), mountain(), mountain(), island()])
        .start();
    keep_mulligans(&mut engine);

    // Seat 1's own main phase: three of its four lands are tapped for mana and
    // the Island is kept back. Nothing untaps them again before seat 0's turn,
    // because an untap step only ever belongs to the turn's own player.
    reach_their_main_phase(&mut engine, p1);
    let island = on_battlefield(&engine, p1, island()).expect("seat 1 has an Island");
    tap_mana_except(&mut engine, p1, island);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        3,
        "three Mountains tapped for mana and the Island left untapped"
    );

    reach_their_main_phase(&mut engine, p0);
    let mine = all_on_battlefield(&engine, p0, mountain());
    assert_eq!(mine.len(), 5, "seat 0's five Mountains are the whole board");
    assert!(
        mine.iter().all(|id| !is_tapped(&engine, *id)),
        "its own untap step stood them back up"
    );
    assert_eq!(
        lands_of(&engine, p1)
            .iter()
            .filter(|id| is_tapped(&engine, **id))
            .count(),
        3,
        "and seat 1's three tapped lands are still tapped, which is what the \
         spell counts"
    );

    let card = in_hand(&engine, p0, mana_geyser()).expect("the Geyser is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{3}}{{R}}{{R}}, and `can_afford` reads the pool \
         rather than the five untapped Mountains: {:?}",
        legal.castable
    );

    cast_from_hand(&mut engine, p0, mana_geyser());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, mana_geyser()).is_some(),
        "the sorcery resolved rather than being countered, and went to its \
         owner's graveyard"
    );
    assert_eq!(
        lands_of(&engine, p0)
            .iter()
            .filter(|id| is_tapped(&engine, **id))
            .count(),
        5,
        "seat 0's own five Mountains were tapped to pay the cost"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        3,
        "one red for each of the three tapped lands the opponent controls — \
         eight tapped lands stand on the battlefield and the other five \
         belong to the caster"
    );
    assert_eq!(
        pool.total(),
        3,
        "the five Mountains paid the {{3}}{{R}}{{R}} down to an empty pool, so \
         the three red are the whole of what the spell added"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "seat 1's floating mana emptied with the step it was made in (CR 500.5)"
    );
}
