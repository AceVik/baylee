//! `cards/instants/mv_1/glyph_of_delusion.rs`, played.
//!
//! "Put X glyph counters on target creature that target Wall blocked this turn,
//! where X is the power of that blocked creature. The creature gains 'This
//! creature doesn't untap during your untap step if it has a glyph counter on
//! it' and 'At the beginning of your upkeep, remove a glyph counter from this
//! creature.'"
//!
//! The Walls are Walls of Stone (0/8), which survive every attacker here, so
//! the creature a Wall blocked is the creature a Wall was *in combat with*,
//! not one it killed.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w2c_kit as kit;
use crate::turn::Step;

// oracle_id = "e23ba169-4eb5-4083-a199-ac7e3026e67f"
fn glyph_of_delusion() -> CardIndex {
    card_index("e23ba169-4eb5-4083-a199-ac7e3026e67f")
}

/// Casts Glyph of Delusion as p0 once the blockers are down, aimed at `pair`
/// (a creature and the Wall that blocked it, in whichever order the card asks
/// for them), and lets the stack empty. Returns the target menus it showed.
#[track_caller]
fn glyph_the_pair(engine: &mut Engine<RegistryLookup>, pair: [ObjectId; 2]) -> Vec<Vec<ObjectId>> {
    let mut answers = kit::Answers::new().spread(&pair);
    kit::walk_to_quiet_priority(engine, &mut answers, kit::P0, Step::DeclareBlockers);
    tap_all_mana(engine, kit::P0);
    kit::cast(engine, kit::P0, glyph_of_delusion(), &mut answers);
    kit::settle(engine, kit::P0, &mut answers);
    answers.menus
}

/// "Put X glyph counters on target creature that target Wall blocked this
/// turn, where X is the power of that blocked creature": three attackers, two
/// Walls. Hill Giant (power 3) is blocked by the second Wall, Savannah Lions
/// (power 2) by the first, and the Bear is not blocked at all. Glyph of
/// Delusion on the Giant and its Wall puts 3 counters on the Giant and none on
/// the Lions. The menus are the rest of the sentence: the Bear that no Wall
/// blocked is offered nowhere, and once one of the pair is chosen the other
/// question offers only the partner (the Giant for the Wall that blocked it,
/// the Wall for the Giant it blocked), not the other Wall or the Lions.
#[test]
fn glyph_of_delusion_puts_x_counters_on_the_creature_a_wall_blocked_x_being_its_power() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                kit::hill_giant(),
                kit::savannah_lions(),
                kit::bears(),
                island(),
            ],
        )
        .battlefield(1, &[kit::wall_of_stone(), kit::wall_of_stone()])
        .hand(0, &[glyph_of_delusion()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let giant = on_battlefield(&engine, kit::P0, kit::hill_giant()).expect("the Giant is out");
    let lions = on_battlefield(&engine, kit::P0, kit::savannah_lions()).expect("the Lions are out");
    let bear = on_battlefield(&engine, kit::P0, kit::bears()).expect("the Bear is out");
    let walls = kit::objects_of(&engine, kit::P1, kit::wall_of_stone());
    assert_eq!(walls.len(), 2, "two Walls");
    let (lions_wall, giant_wall) = (walls[0], walls[1]);

    kit::attack(&mut engine, kit::P0, &[giant, lions, bear], kit::P1);
    kit::block(
        &mut engine,
        kit::P1,
        &[(lions_wall, lions), (giant_wall, giant)],
    );
    let menus = glyph_the_pair(&mut engine, [giant, giant_wall]);

    assert_eq!(
        kit::glyph_counters(&engine, giant),
        3,
        "X is the power of the blocked creature: Hill Giant has 3"
    );
    assert_eq!(
        kit::glyph_counters(&engine, lions),
        0,
        "the other blocked creature, which was not the target, has none"
    );
    assert_eq!(
        kit::glyph_counters(&engine, bear),
        0,
        "nor has the unblocked one"
    );
    assert_eq!(
        kit::glyph_counters(&engine, giant_wall),
        0,
        "the Wall is a target, not the creature that gets the counters"
    );
    assert_eq!(
        kit::glyph_counters(&engine, lions_wall),
        0,
        "and the other Wall has none either"
    );

    assert_eq!(
        menus.len(),
        2,
        "one question for the creature, one for the Wall: {menus:?}"
    );
    for menu in &menus {
        assert!(
            !menu.contains(&bear),
            "a creature no Wall blocked is never offered: {menu:?}"
        );
    }
    let creature_first = menus[0].contains(&giant);
    let wall_first = menus[0].contains(&giant_wall);
    assert!(
        creature_first != wall_first,
        "the first question was about one of the pair, the second about the other: {menus:?}"
    );
    let first_has_the_other_blocked_pair = if creature_first {
        menus[0].contains(&lions)
    } else {
        menus[0].contains(&lions_wall)
    };
    assert!(
        first_has_the_other_blocked_pair,
        "the first question offered every creature a Wall blocked (or every Wall \
         that blocked): {menus:?}"
    );
    let partner = if creature_first { giant_wall } else { giant };
    assert_eq!(
        menus[1],
        vec![partner],
        "the second question offers only what goes with the first answer"
    );
}

/// "The creature gains 'This creature doesn't untap during your untap step if
/// it has a glyph counter on it' and 'At the beginning of your upkeep, remove a
/// glyph counter from this creature.'" Savannah Lions (power 2) and a Bear
/// attack; a Wall blocks the Lions and Glyph of Delusion puts 2 counters on
/// them. Every one of p0's untap steps afterwards leaves the Lions tapped while
/// they carry a counter, and every one of its upkeeps takes a counter off:
/// turn 3 ends the untap step with 2 counters (still tapped) and the upkeep
/// with 1, turn 5 has 1 at the untap step (still tapped) and 0 after the
/// upkeep, and in turn 7 the Lions untap. The Bear, which attacked in the same
/// turn with no counter, untaps in turn 3: untapping works in this game.
#[test]
fn glyph_of_delusion_keeps_the_creature_tapped_until_its_upkeeps_have_removed_the_counters() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kit::savannah_lions(), kit::bears(), island()])
        .battlefield(1, &[kit::wall_of_stone()])
        .hand(0, &[glyph_of_delusion()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );
    let first_turn = engine.state().turn.number;

    let lions = on_battlefield(&engine, kit::P0, kit::savannah_lions()).expect("the Lions are out");
    let bear = on_battlefield(&engine, kit::P0, kit::bears()).expect("the Bear is out");
    let wall = on_battlefield(&engine, kit::P1, kit::wall_of_stone()).expect("the Wall is out");

    kit::attack(&mut engine, kit::P0, &[lions, bear], kit::P1);
    kit::block(&mut engine, kit::P1, &[(wall, lions)]);
    glyph_the_pair(&mut engine, [lions, wall]);
    kit::drive(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && at_rest(e, kit::P0)
    });
    assert_eq!(
        kit::glyph_counters(&engine, lions),
        2,
        "X is the Lions' power, 2"
    );
    assert!(
        is_tapped(&engine, lions),
        "the Lions attacked and are tapped"
    );

    // p0's next turn: the untap step finds 2 counters; the upkeep removes one.
    kit::walk_to_main_of_turn(&mut engine, kit::P0, first_turn + 2);
    assert!(
        !is_tapped(&engine, bear),
        "the Bear, with no counter, untapped in its controller's untap step"
    );
    assert!(
        is_tapped(&engine, lions),
        "the Lions did not untap: they had glyph counters in the untap step"
    );
    assert_eq!(
        kit::glyph_counters(&engine, lions),
        1,
        "and the upkeep removed one of the two"
    );

    // The turn after: the untap step still finds 1; the upkeep removes the last.
    kit::walk_to_main_of_turn(&mut engine, kit::P0, first_turn + 4);
    assert!(
        is_tapped(&engine, lions),
        "they had a counter when the untap step began, so they stayed tapped"
    );
    assert_eq!(
        kit::glyph_counters(&engine, lions),
        0,
        "and the second upkeep took the last counter"
    );

    // And now the untap step finds none.
    kit::walk_to_main_of_turn(&mut engine, kit::P0, first_turn + 6);
    assert!(
        !is_tapped(&engine, lions),
        "once no counter remains the Lions untap again"
    );
}

/// "At the beginning of your upkeep, remove a glyph counter from this
/// creature": "your" is the controller of the creature that gained the
/// ability, not the player who cast the Glyph. p1 attacks with Savannah Lions
/// (power 2) and a Bear; p0's Wall of Stone blocks the Lions and p0 casts Glyph
/// of Delusion on the Lions and the Wall, so the caster and the creature's
/// controller are different players. The Lions get 2 counters and the Wall none.
/// p0's upkeep (turn 3) takes nothing off them; p1's (turn 4) takes one, with
/// the Lions kept tapped through p1's untap step while the Bear untaps; p0's
/// next upkeep (turn 5) takes nothing; p1's (turn 6) takes the last, the Lions
/// still tapped in its untap step; and in p1's turn 8 they untap.
#[test]
fn glyph_of_delusion_counts_down_in_its_creatures_controllers_upkeeps_not_the_casters() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kit::wall_of_stone(), island()])
        .battlefield(1, &[kit::savannah_lions(), kit::bears()])
        .hand(0, &[glyph_of_delusion()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );
    let first_turn = engine.state().turn.number;
    kit::walk_to_next_own_main(&mut engine, kit::P1);

    let lions = on_battlefield(&engine, kit::P1, kit::savannah_lions()).expect("the Lions are out");
    let bear = on_battlefield(&engine, kit::P1, kit::bears()).expect("the Bear is out");
    let wall = on_battlefield(&engine, kit::P0, kit::wall_of_stone()).expect("p0's Wall is out");

    kit::attack(&mut engine, kit::P1, &[lions, bear], kit::P0);
    kit::block(&mut engine, kit::P0, &[(wall, lions)]);
    glyph_the_pair(&mut engine, [lions, wall]);
    assert_eq!(
        kit::glyph_counters(&engine, lions),
        2,
        "X is the Lions' power, 2, though p0 cast the spell"
    );
    assert_eq!(
        kit::glyph_counters(&engine, wall),
        0,
        "and the Wall that blocked them gets no counter"
    );
    assert!(
        is_tapped(&engine, lions) && is_tapped(&engine, bear),
        "both of p1's attackers are tapped"
    );

    kit::walk_to_main_of_turn(&mut engine, kit::P0, first_turn + 2);
    assert_eq!(
        kit::glyph_counters(&engine, lions),
        2,
        "p0's upkeep removes nothing: it is not the upkeep of the Lions' controller"
    );

    kit::walk_to_main_of_turn(&mut engine, kit::P1, first_turn + 3);
    assert!(
        !is_tapped(&engine, bear),
        "p1's untap step untapped the Bear: untapping works in this game"
    );
    assert!(
        is_tapped(&engine, lions),
        "but the Lions had counters in it and stayed tapped"
    );
    assert_eq!(
        kit::glyph_counters(&engine, lions),
        1,
        "and p1's upkeep removed one of the two"
    );

    kit::walk_to_main_of_turn(&mut engine, kit::P0, first_turn + 4);
    assert_eq!(
        kit::glyph_counters(&engine, lions),
        1,
        "p0's second upkeep removes nothing either"
    );

    kit::walk_to_main_of_turn(&mut engine, kit::P1, first_turn + 5);
    assert!(
        is_tapped(&engine, lions),
        "one counter was left when p1's untap step began"
    );
    assert_eq!(
        kit::glyph_counters(&engine, lions),
        0,
        "and p1's second upkeep removed the last"
    );

    kit::walk_to_main_of_turn(&mut engine, kit::P1, first_turn + 7);
    assert!(
        !is_tapped(&engine, lions),
        "with no counter left p1's untap step untaps the Lions"
    );
}
