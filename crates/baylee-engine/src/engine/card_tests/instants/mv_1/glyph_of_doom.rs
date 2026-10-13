//! `cards/instants/mv_1/glyph_of_doom.rs`, played.
//!
//! "Choose target Wall creature. At this turn's next end of combat, destroy all
//! creatures that were blocked by that creature this turn."
//!
//! The Walls are Walls of Stone (0/8), so nothing a Wall blocks dies in combat
//! damage: every creature that is gone afterwards was destroyed by the delayed
//! trigger.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w2c_kit as kit;
use crate::turn::Step;
use baylee_core::generated::index;

// oracle_id = "a3b3e02f-c75d-4cd6-8c57-d85ce00975fb"
fn glyph_of_doom() -> CardIndex {
    card_index("a3b3e02f-c75d-4cd6-8c57-d85ce00975fb")
}

/// "Choose target Wall creature. At this turn's next end of combat, destroy all
/// creatures that were blocked by that creature this turn." Hill Giant and
/// Savannah Lions attack; one Wall of Stone blocks each. Glyph of Doom is cast
/// on the Lions' Wall once the blockers are down. The menu holds the two Walls
/// and neither attacker: only a Wall creature is a legal target. After combat
/// damage the Lions are still there (the trigger waits for the end of combat);
/// in the second main phase they are gone, and the Giant, blocked by the other
/// Wall, is not.
#[test]
fn glyph_of_doom_destroys_the_creatures_its_wall_blocked_at_end_of_combat_and_no_other() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kit::hill_giant(), kit::savannah_lions(), swamp()])
        .battlefield(1, &[kit::wall_of_stone(), kit::wall_of_stone()])
        .hand(0, &[glyph_of_doom()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let giant = on_battlefield(&engine, kit::P0, kit::hill_giant()).expect("the Giant is out");
    let lions = on_battlefield(&engine, kit::P0, kit::savannah_lions()).expect("the Lions are out");
    let walls = kit::objects_of(&engine, kit::P1, kit::wall_of_stone());
    assert_eq!(walls.len(), 2, "two Walls");
    let (lions_wall, giant_wall) = (walls[0], walls[1]);

    kit::attack(&mut engine, kit::P0, &[giant, lions], kit::P1);
    kit::block(
        &mut engine,
        kit::P1,
        &[(lions_wall, lions), (giant_wall, giant)],
    );

    let mut answers = kit::Answers::new().aim(&[lions_wall]);
    kit::walk_to_quiet_priority(&mut engine, &mut answers, kit::P0, Step::DeclareBlockers);
    tap_all_mana(&mut engine, kit::P0);
    kit::cast(&mut engine, kit::P0, glyph_of_doom(), &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);

    let menu = answers
        .menus
        .first()
        .expect("Glyph of Doom asked for its target");
    assert!(
        menu.contains(&lions_wall) && menu.contains(&giant_wall),
        "both Walls are legal targets: {menu:?}"
    );
    assert!(
        !menu.contains(&lions) && !menu.contains(&giant),
        "a creature that is not a Wall is not: {menu:?}"
    );

    // Combat damage is dealt; the end of combat has not come.
    let mut quiet = kit::Answers::new();
    kit::walk_to_quiet_priority(&mut engine, &mut quiet, kit::P0, Step::CombatDamage);
    assert!(
        kit::on_the_table(&engine, lions),
        "\"at this turn's next end of combat\": not yet, the Lions are still there"
    );

    kit::walk_to_second_main(&mut engine, &mut quiet, kit::P0);
    assert_eq!(
        kit::zone_of(&engine, lions),
        Zone::Graveyard,
        "the creature the chosen Wall blocked is destroyed at end of combat"
    );
    assert!(
        kit::on_the_table(&engine, giant),
        "the creature blocked by the other Wall is not"
    );
    assert!(
        kit::on_the_table(&engine, lions_wall) && kit::on_the_table(&engine, giant_wall),
        "and neither Wall is touched"
    );
}

/// "Destroy", not "destroy … can't be regenerated": a regeneration shield
/// saves the creature. A Lotleth Troll attacks and a Wall blocks it; Glyph of
/// Doom on the Wall and a `{B}` shield on the Troll before the end of combat.
/// The shield is used up (the destroy event did happen, so this is not a test
/// of a trigger that never fired) and the Troll is still on the table. The
/// first test above is the control for the destroy itself.
#[test]
fn glyph_of_doom_can_be_regenerated_from() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kit::troll(), swamp(), swamp()])
        .battlefield(1, &[kit::wall_of_stone()])
        .hand(0, &[glyph_of_doom()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let troll = on_battlefield(&engine, kit::P0, kit::troll()).expect("the Troll is out");
    let wall = on_battlefield(&engine, kit::P1, kit::wall_of_stone()).expect("the Wall is out");

    kit::attack(&mut engine, kit::P0, &[troll], kit::P1);
    kit::block(&mut engine, kit::P1, &[(wall, troll)]);

    let mut answers = kit::Answers::new().aim(&[wall]).shield(true);
    kit::walk_to_quiet_priority(&mut engine, &mut answers, kit::P0, Step::DeclareBlockers);
    tap_all_mana(&mut engine, kit::P0);
    kit::cast(&mut engine, kit::P0, glyph_of_doom(), &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);
    raise_a_shield(&mut engine, kit::P0, troll, kit::TROLL_REGENERATES);

    kit::walk_to_second_main(&mut engine, &mut answers, kit::P0);
    assert_eq!(
        engine
            .state()
            .object(troll)
            .expect("the Troll")
            .regeneration_shields,
        0,
        "the shield was used: something tried to destroy the Troll"
    );
    assert!(
        kit::on_the_table(&engine, troll),
        "and it regenerated: \"destroy\" without \"can't be regenerated\""
    );
}

/// "At this turn's next end of combat": cast in the second main phase there is
/// no next end of combat this turn, and the delayed trigger does not wait for
/// another turn's. Savannah Lions attack, a Wall blocks them, and the combat
/// ends. In the second main phase p0 casts Glyph of Doom on the Wall and then
/// Time Walk, so that the next turn is p0's own: the Lions attack and the same
/// Wall blocks them again, which is the situation in which a trigger that
/// lapsed late would destroy them. They survive that end of combat (the Wall
/// is damaged by them in that turn, so the block did happen).
#[test]
fn glyph_of_doom_cast_after_combat_does_not_wait_for_a_later_end_of_combat() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kit::savannah_lions(), swamp(), island(), mountain()])
        .battlefield(1, &[kit::wall_of_stone()])
        .hand(0, &[glyph_of_doom(), index::TIME_WALK])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );
    let first_turn = engine.state().turn.number;

    let lions = on_battlefield(&engine, kit::P0, kit::savannah_lions()).expect("the Lions are out");
    let wall = on_battlefield(&engine, kit::P1, kit::wall_of_stone()).expect("the Wall is out");

    kit::attack(&mut engine, kit::P0, &[lions], kit::P1);
    kit::block(&mut engine, kit::P1, &[(wall, lions)]);
    let mut answers = kit::Answers::new().aim(&[wall]);
    kit::walk_to_second_main(&mut engine, &mut answers, kit::P0);
    assert_eq!(
        kit::damage_on(&engine, wall),
        2,
        "the Lions and the Wall fought"
    );

    tap_all_mana(&mut engine, kit::P0);
    kit::cast(&mut engine, kit::P0, glyph_of_doom(), &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);
    kit::cast(&mut engine, kit::P0, index::TIME_WALK, &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);
    assert!(
        kit::on_the_table(&engine, lions),
        "cast after combat, Glyph of Doom destroys nothing now"
    );

    // The extra turn: the same attack, the same block, and an end of combat.
    kit::walk_to_main_of_turn(&mut engine, kit::P0, first_turn + 1);
    kit::attack(&mut engine, kit::P0, &[lions], kit::P1);
    kit::block(&mut engine, kit::P1, &[(wall, lions)]);
    kit::walk_to_second_main(&mut engine, &mut answers, kit::P0);
    assert_eq!(
        kit::damage_on(&engine, wall),
        2,
        "in the extra turn the Lions were blocked by the same Wall again"
    );
    assert!(
        kit::on_the_table(&engine, lions),
        "and the Glyph cast a turn earlier does not destroy them at this end of combat"
    );
}

/// "… destroy all creatures that were blocked by that creature this turn" is
/// read when the trigger resolves, and the Wall may be gone by then (CR 608.2h:
/// last known information). A Wall of Wood (0/3) blocks Hill Giant, Glyph of
/// Doom is cast on the Wall after blocks, and the Giant's 3 damage kills the
/// Wall in the combat damage step. The Giant is still there at that point, and
/// it is destroyed at end of combat.
#[test]
fn glyph_of_doom_still_destroys_what_its_wall_blocked_when_the_wall_died_in_combat() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kit::hill_giant(), swamp()])
        .battlefield(1, &[kit::wall_of_wood()])
        .hand(0, &[glyph_of_doom()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let giant = on_battlefield(&engine, kit::P0, kit::hill_giant()).expect("the Giant is out");
    let wall = on_battlefield(&engine, kit::P1, kit::wall_of_wood()).expect("the Wall is out");

    kit::attack(&mut engine, kit::P0, &[giant], kit::P1);
    kit::block(&mut engine, kit::P1, &[(wall, giant)]);
    let mut answers = kit::Answers::new().aim(&[wall]);
    kit::walk_to_quiet_priority(&mut engine, &mut answers, kit::P0, Step::DeclareBlockers);
    tap_all_mana(&mut engine, kit::P0);
    kit::cast(&mut engine, kit::P0, glyph_of_doom(), &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);

    kit::walk_to_quiet_priority(&mut engine, &mut answers, kit::P0, Step::CombatDamage);
    assert_eq!(
        kit::zone_of(&engine, wall),
        Zone::Graveyard,
        "the Giant's 3 killed the 0/3 Wall in the combat damage step"
    );
    assert!(
        kit::on_the_table(&engine, giant),
        "and the end of combat has not come: the Giant is still there"
    );

    kit::walk_to_second_main(&mut engine, &mut answers, kit::P0);
    assert_eq!(
        kit::zone_of(&engine, giant),
        Zone::Graveyard,
        "the Wall is gone, but what it blocked is destroyed at end of combat all the same"
    );
}

/// The creatures "blocked by that creature this turn" are the ones it blocks
/// after the Glyph is cast, too: the trigger reads the Wall's blocks when it
/// resolves at end of combat. Hill Giant attacks, p0 casts Glyph of Doom on
/// p1's Wall of Stone in the declare attackers step, before any block, and
/// then the Wall blocks the Giant. The Giant is destroyed at end of combat.
#[test]
fn glyph_of_doom_cast_before_blockers_destroys_the_creature_the_wall_blocks_afterwards() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kit::hill_giant(), swamp()])
        .battlefield(1, &[kit::wall_of_stone()])
        .hand(0, &[glyph_of_doom()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let giant = on_battlefield(&engine, kit::P0, kit::hill_giant()).expect("the Giant is out");
    let wall = on_battlefield(&engine, kit::P1, kit::wall_of_stone()).expect("the Wall is out");

    kit::attack(&mut engine, kit::P0, &[giant], kit::P1);
    let mut answers = kit::Answers::new().aim(&[wall]);
    kit::walk_to_quiet_priority(&mut engine, &mut answers, kit::P0, Step::DeclareAttackers);
    tap_all_mana(&mut engine, kit::P0);
    kit::cast(&mut engine, kit::P0, glyph_of_doom(), &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);
    assert!(
        kit::on_the_table(&engine, giant),
        "nothing has been blocked yet"
    );

    kit::block(&mut engine, kit::P1, &[(wall, giant)]);
    kit::walk_to_second_main(&mut engine, &mut answers, kit::P0);
    assert_eq!(
        kit::zone_of(&engine, giant),
        Zone::Graveyard,
        "the creature the Wall blocked afterwards is destroyed at end of combat"
    );
    assert!(
        kit::on_the_table(&engine, wall),
        "the Wall of Stone is untouched"
    );
}
