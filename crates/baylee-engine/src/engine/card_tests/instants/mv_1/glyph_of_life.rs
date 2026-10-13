//! `cards/instants/mv_1/glyph_of_life.rs`, played.
//!
//! "Choose target Wall creature. Whenever that creature is dealt damage by an
//! attacking creature this turn, you gain that much life."
//!
//! Rulings (2004-10-04): "You can cast this targeting your opponent's walls to
//! good effect." and "This is not redirection of damage. The wall still takes
//! the damage."
//!
//! The Walls are Walls of Stone (0/8), which take everything these boards
//! throw at them and stay.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w2c_kit as kit;
use crate::turn::Step;

// oracle_id = "ec450179-35e0-4d72-b42f-b507cbce03ad"
fn glyph_of_life() -> CardIndex {
    card_index("ec450179-35e0-4d72-b42f-b507cbce03ad")
}

/// "Whenever that creature is dealt damage by an attacking creature this turn,
/// you gain that much life", on the opponent's Wall (ruling: "to good effect").
/// Hill Giant attacks into p1's Wall of Stone. p0 casts Glyph of Life on the
/// Wall and, with a second spell, Lightning Bolt on the Wall as well: the Bolt
/// is not an attacking creature and gains nothing, the Giant's 3 combat damage
/// gains 3. p0 ends at 23, not 20 (no gain) and not 26 (the Bolt counted), and
/// the Wall carries 6 damage: the damage is not redirected.
#[test]
fn glyph_of_life_gains_the_damage_an_attacker_deals_to_the_wall_and_only_that() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kit::hill_giant(), plains(), mountain()])
        .battlefield(1, &[kit::wall_of_stone()])
        .hand(0, &[glyph_of_life(), kit::bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let giant = on_battlefield(&engine, kit::P0, kit::hill_giant()).expect("the Giant is out");
    let wall = on_battlefield(&engine, kit::P1, kit::wall_of_stone()).expect("the Wall is out");

    kit::attack(&mut engine, kit::P0, &[giant], kit::P1);
    kit::block(&mut engine, kit::P1, &[(wall, giant)]);

    let mut answers = kit::Answers::new().aim(&[wall]).aim(&[wall]);
    kit::walk_to_quiet_priority(&mut engine, &mut answers, kit::P0, Step::DeclareBlockers);
    tap_all_mana(&mut engine, kit::P0);
    kit::cast(&mut engine, kit::P0, glyph_of_life(), &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);
    kit::cast(&mut engine, kit::P0, kit::bolt(), &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);
    assert_eq!(
        kit::damage_on(&engine, wall),
        3,
        "the Bolt dealt the Wall 3"
    );
    assert_eq!(
        kit::life(&engine, kit::P0),
        20,
        "damage from a spell is not damage from an attacking creature: nothing gained yet"
    );

    kit::walk_to_second_main(&mut engine, &mut answers, kit::P0);
    assert_eq!(
        kit::damage_on(&engine, wall),
        6,
        "the Wall still took the Giant's 3 as well: the damage is not redirected"
    );
    assert_eq!(
        kit::life(&engine, kit::P0),
        23,
        "the attacking Giant dealt it 3, and p0 gained that much"
    );
}

/// The same sentence on the caster's own Wall: p1's Hill Giant attacks p0 and
/// p0's Wall of Stone blocks. p0 holds Glyph of Life with a Plains untapped
/// and casts it on its own Wall in p1's declare blockers step; the Giant's 3
/// damage to the Wall gains p0 3 life. The attacker belongs to the opponent
/// and "an attacking creature" does not mean "your attacking creature".
#[test]
fn glyph_of_life_gains_life_for_damage_an_opponents_attacker_deals_to_the_casters_wall() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kit::wall_of_stone(), plains()])
        .battlefield(1, &[kit::hill_giant()])
        .hand(0, &[glyph_of_life()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let wall = on_battlefield(&engine, kit::P0, kit::wall_of_stone()).expect("the Wall is out");
    let giant = on_battlefield(&engine, kit::P1, kit::hill_giant()).expect("the Giant is out");

    kit::attack(&mut engine, kit::P1, &[giant], kit::P0);
    kit::block(&mut engine, kit::P0, &[(wall, giant)]);

    // p1 is the active player and passes first; p0 gets priority in the step.
    let mut answers = kit::Answers::new().aim(&[wall]);
    kit::walk_to_quiet_priority(&mut engine, &mut answers, kit::P0, Step::DeclareBlockers);
    tap_all_mana(&mut engine, kit::P0);
    kit::cast(&mut engine, kit::P0, glyph_of_life(), &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);
    kit::walk_to_second_main(&mut engine, &mut answers, kit::P0);

    assert_eq!(
        kit::damage_on(&engine, wall),
        3,
        "the Wall took the Giant's 3"
    );
    assert_eq!(
        kit::life(&engine, kit::P0),
        23,
        "and its controller, who cast the Glyph, gained 3"
    );
}

/// "This turn": the Glyph does not outlast it. p0 casts Glyph of Life on its
/// own Wall in its own main phase; nothing is dealt to the Wall that turn. In
/// p1's turn Hill Giant attacks and the Wall blocks it and takes 3, and p0
/// gains nothing. The test above is the control: the same block while the
/// Glyph is still in force gains 3.
#[test]
fn glyph_of_life_does_not_last_past_the_turn_it_was_cast_in() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kit::wall_of_stone(), plains()])
        .battlefield(1, &[kit::hill_giant()])
        .hand(0, &[glyph_of_life()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );
    let first_turn = engine.state().turn.number;

    let wall = on_battlefield(&engine, kit::P0, kit::wall_of_stone()).expect("the Wall is out");
    let giant = on_battlefield(&engine, kit::P1, kit::hill_giant()).expect("the Giant is out");

    tap_all_mana(&mut engine, kit::P0);
    let mut answers = kit::Answers::new().aim(&[wall]);
    kit::cast(&mut engine, kit::P0, glyph_of_life(), &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);

    kit::walk_to_main_of_turn(&mut engine, kit::P1, first_turn + 1);
    kit::attack(&mut engine, kit::P1, &[giant], kit::P0);
    kit::block(&mut engine, kit::P0, &[(wall, giant)]);
    kit::walk_to_second_main(&mut engine, &mut answers, kit::P1);

    assert_eq!(
        kit::damage_on(&engine, wall),
        3,
        "the Wall took the Giant's damage in the next turn"
    );
    assert_eq!(
        kit::life(&engine, kit::P0),
        20,
        "and p0 gained nothing: the Glyph ended with the turn it was cast in"
    );
}
