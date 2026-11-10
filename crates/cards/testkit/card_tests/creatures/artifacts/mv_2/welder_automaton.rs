//! `cards/creatures/artifacts/mv_2/welder_automaton.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn welder_automaton() -> CardIndex {
    card_index("52eae443-fbe5-479f-a5a4-b5e4401d62d3")
}

/// Welder Automaton: a 2/1 with "{3}{R}: This creature deals 1 damage to each
/// opponent." Red is required; the damage goes to the opponent and not to
/// its controller.
#[test]
fn welder_automaton_deals_one_damage_to_each_opponent() {
    let p0 = PlayerId::new(0);
    let mut engine = table(
        &[],
        &[welder_automaton(), mountain(), forest(), forest(), forest()],
        &[],
    );
    let welder = on_battlefield(&engine, p0, welder_automaton()).expect("seated");
    assert_eq!(pt(&engine, welder), (2, 1));
    let forests = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| forests.contains(&id));
    assert!(
        !priority_offer(&engine).abilities.contains(&(welder, 0)),
        "green alone does not pay {{R}}"
    );
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, welder_automaton(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, 1), 19);
    assert_eq!(life_of(&engine, 0), 20, "not to its controller");
}

/// Three opponents each take one: the table has two more chairs.
#[test]
fn welder_automaton_hits_every_opponent_at_a_three_player_table() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(
            0,
            &[welder_automaton(), mountain(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, welder_automaton(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, 0), 20);
    assert_eq!(life_of(&engine, 1), 19);
    assert_eq!(life_of(&engine, 2), 19);
}
