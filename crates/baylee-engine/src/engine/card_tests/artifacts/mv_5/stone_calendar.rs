//! `cards/artifacts/mv_5/stone_calendar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn stone_calendar() -> CardIndex {
    card_index("301a59c1-e372-4c31-a2a8-88c5ba7d822a")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

fn castable(engine: &Engine<RegistryLookup>, spell: ObjectId) -> bool {
    priority_offer(engine).castable.contains(&spell)
}

/// p0's main phase: `board` on p0's side, Hill Giant ({3}{R}) in p0's hand.
fn a_giant_in_hand(board: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, board)
        .hand(0, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    engine
}

/// Stone Calendar (301a59c1-e372-4c31-a2a8-88c5ba7d822a) — "Spells you cast
/// cost {1} less to cast."
///
/// Hill Giant costs {3}{R}; with the Calendar three mana (a Mountain and two
/// Forests) cast it, and exactly those three are spent.
#[test]
fn stone_calendar_makes_a_four_mana_spell_cost_three() {
    let p0 = PlayerId::new(0);
    let mut engine = a_giant_in_hand(&[stone_calendar(), mountain(), forest(), forest()]);
    assert_eq!(tap_all_mana(&mut engine, p0), 3);
    let giant = in_hand(&engine, p0, hill_giant()).expect("Giant in hand");
    assert!(castable(&engine, giant), "{{2}}{{R}} is enough");
    cast_with_floating(&mut engine, p0, hill_giant());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "all three floating mana paid for it"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, hill_giant()).is_some());
}

/// Control: the same three lands without the Calendar cannot cast it.
#[test]
fn stone_calendar_is_what_makes_the_difference() {
    let p0 = PlayerId::new(0);
    let mut engine = a_giant_in_hand(&[mountain(), forest(), forest()]);
    assert_eq!(tap_all_mana(&mut engine, p0), 3);
    let giant = in_hand(&engine, p0, hill_giant()).expect("Giant in hand");
    assert!(!castable(&engine, giant), "three mana is one short");
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: giant })
            .is_err(),
        "the engine refuses the cast"
    );
}

/// "{1} less" is generic mana only: three Forests make no red, so the Giant
/// stays out of reach even with the Calendar.
#[test]
fn stone_calendar_does_not_reduce_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = a_giant_in_hand(&[stone_calendar(), forest(), forest(), forest()]);
    assert_eq!(tap_all_mana(&mut engine, p0), 3);
    let giant = in_hand(&engine, p0, hill_giant()).expect("Giant in hand");
    assert!(!castable(&engine, giant), "{{R}} is still owed");
}

/// Nor does it make a one-mana spell free: Lightning Bolt ({R}) with the
/// Calendar and no mana is not castable.
#[test]
fn stone_calendar_does_not_make_a_colored_spell_free() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[stone_calendar()])
        .hand(0, &[bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let spell = in_hand(&engine, p0, bolt()).expect("Bolt in hand");
    assert!(!castable(&engine, spell));
}

/// "Spells you cast": the opponent's spells cost what they cost. p1, with the
/// same three lands, cannot cast the Giant on p1's own turn.
#[test]
fn stone_calendar_only_helps_its_controller() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[stone_calendar()])
        .battlefield(1, &[mountain(), forest(), forest()])
        .hand(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(tap_all_mana(&mut engine, p1), 3);
    let giant = in_hand(&engine, p1, hill_giant()).expect("their Giant");
    assert!(!castable(&engine, giant), "no discount across the table");
}
