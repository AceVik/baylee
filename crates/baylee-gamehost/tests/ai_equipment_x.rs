//! An Equipment read whatever its filter's shape, and no X spell cast at
//! X = 0 (`AIProfile::EQUIPMENT_AND_X`, generation 12), on tables a real
//! engine dealt, each beside the level at generation 11.

use baylee_ai::{AIProfile, HeuristicAgent};
use baylee_engine::choice::{Pending, PlayerAction, timeout_answer};

use crate::house_table::{ME, Table};

const BEFORE: u16 = AIProfile::TARGET_SIDES;

/// Plays seat 0's main phase with `profile` until it passes with the stack
/// empty.
fn play_main(table: &mut Table, profile: AIProfile) {
    let agent = HeuristicAgent::new(profile);
    for _ in 0..60 {
        let pending = table.engine.pending().clone();
        let player = pending.asked().expect("somebody is asked");
        let action = if player == ME {
            agent.act_with_context(&table.view(), &pending, &table.engine.decision_context())
        } else {
            timeout_answer(&pending).expect("a default answer")
        };
        let passes = player == ME
            && matches!(pending, Pending::Priority { .. })
            && action == PlayerAction::PassPriority
            && table.engine.state().zones.stack_is_empty();
        if passes {
            return;
        }
        table.apply(action);
    }
    panic!("the main phase never ended");
}

/// Bonesplitter ("Equipped creature gets +2/+0", written as an `And` of
/// creature and attached-to-source) beside a Grizzly Bears and a Mountain:
/// a level that makes mana for an ability (sharp, expert) pays the `{1}` and
/// equips it. Read as no static, the equip was worth nothing.
#[test]
fn an_equipment_is_worth_its_equip() {
    for (name, profile) in [("sharp", AIProfile::SHARP), ("expert", AIProfile::EXPERT)] {
        for (generation, equips) in [(AIProfile::GENERATION, true), (BEFORE, false)] {
            let mut table = Table::open(
                "Mountain",
                &[],
                &["Mountain", "Bonesplitter", "Grizzly Bears"],
                &[],
            );
            play_main(&mut table, profile.at_generation(generation));
            let view = table.view();
            let splitter = view
                .battlefield
                .iter()
                .find(|o| o.name.as_ref() == "Bonesplitter")
                .expect("on the table");
            assert_eq!(
                splitter.attached_to.is_some(),
                equips,
                "{name} at generation {generation}"
            );
        }
    }
}

/// Stream of Life (`{X}{G}`: target player gains X life) beside one Forest
/// is not cast: X can only be 0.
#[test]
fn an_x_spell_waits_for_more_than_nothing() {
    for (name, profile) in AIProfile::NAMED {
        if profile.peeks() {
            continue;
        }
        let mut table = Table::open("Forest", &["Stream of Life"], &["Forest"], &[]);
        let stream = table.card("Stream of Life");
        assert!(!table.house_casts(profile, stream), "{name}");
    }
    let mut table = Table::open("Forest", &["Stream of Life"], &["Forest"], &[]);
    let stream = table.card("Stream of Life");
    assert!(
        table.house_casts(AIProfile::STEADY.at_generation(BEFORE), stream),
        "the premise: at generation 11 it was cast for nothing"
    );
}
