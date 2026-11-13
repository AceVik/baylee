//! The house aiming a spell at the side it helps or hurts
//! (`AIProfile::TARGET_SIDES`, generation 11), on tables a real engine
//! dealt. Each board is played by the level at generation 10 as well, which
//! puts a Holy Strength on the opponent's creature and casts a pump or a
//! burn spell with nothing on the right side to take it (Carbon's rank 1,
//! 11.11.2026).

use baylee_ai::AIProfile;
use baylee_core::ids::ObjectId;

use crate::house_table::{ME, Table};

const BEFORE: u16 = AIProfile::FULL_CASTS;

/// Every fair level, at generation 11 and at generation 10.
fn levels() -> impl Iterator<Item = (&'static str, AIProfile, AIProfile)> {
    AIProfile::NAMED
        .into_iter()
        .filter(|(_, profile)| !profile.peeks())
        .map(|(name, profile)| (name, profile, profile.at_generation(BEFORE)))
}

/// Whose creature the Aura `aura` went on, once cast: `Some(true)` for seat
/// 0's own.
fn enchanted_own(table: &Table, aura: ObjectId) -> Option<bool> {
    let view = table.view();
    let host = view
        .battlefield
        .iter()
        .find(|o| o.id == aura)?
        .attached_to?;
    view.object(host).map(|o| o.controller == ME)
}

/// Lets the house finish what it began: every question of seat 0's
/// answered by `profile` until seat 0 has priority with the stack empty.
fn settle(table: &mut Table, profile: AIProfile) {
    let agent = baylee_ai::HeuristicAgent::new(profile);
    for _ in 0..40 {
        let pending = table.engine.pending().clone();
        if table.engine.state().zones.stack_is_empty()
            && matches!(pending, baylee_engine::choice::Pending::Priority { player, .. } if player == ME)
        {
            return;
        }
        let player = pending.asked().expect("somebody is asked");
        let action = if player == ME {
            agent.act_with_context(&table.view(), &pending, &table.engine.decision_context())
        } else {
            baylee_engine::choice::timeout_answer(&pending).expect("a default answer")
        };
        table.apply(action);
    }
    panic!("the stack never emptied");
}

/// Holy Strength ("Enchanted creature gets +1/+2") with a Grizzly Bears on
/// each side goes on this seat's own Bears.
#[test]
fn a_helpful_aura_goes_on_its_own_side() {
    for (name, now, before) in levels() {
        let board = || {
            Table::open(
                "Plains",
                &["Holy Strength"],
                &["Plains", "Grizzly Bears"],
                &["Grizzly Bears"],
            )
        };
        let mut table = board();
        let aura = table.card("Holy Strength");
        assert!(table.house_casts(now, aura), "{name} casts it");
        settle(&mut table, now);
        assert_eq!(enchanted_own(&table, aura), Some(true), "{name}");
        let mut table = board();
        assert!(
            table.house_casts(before, aura),
            "{name} at generation 10 casts it"
        );
        settle(&mut table, before);
        assert_eq!(
            enchanted_own(&table, aura),
            Some(false),
            "{name} at generation 10 enchanted the opponent's creature"
        );
    }
}

/// Pacifism ("Enchanted creature can't attack or block") goes on the
/// opponent's creature, as it did.
#[test]
fn a_harmful_aura_goes_on_the_other_side() {
    for (name, now, _) in levels() {
        let mut table = Table::open(
            "Plains",
            &["Pacifism"],
            &["Plains", "Plains", "Grizzly Bears"],
            &["Grizzly Bears"],
        );
        let aura = table.card("Pacifism");
        assert!(table.house_casts(now, aura), "{name} casts it");
        settle(&mut table, now);
        assert_eq!(enchanted_own(&table, aura), Some(false), "{name}");
    }
}

/// Holy Strength with no creature of this seat's own is not cast: the only
/// creature it could enchant is the opponent's.
#[test]
fn a_helpful_aura_with_nothing_of_its_own_to_enchant_waits() {
    for (name, now, before) in levels() {
        let board = || {
            Table::open(
                "Plains",
                &["Holy Strength"],
                &["Plains"],
                &["Grizzly Bears"],
            )
        };
        let mut table = board();
        let aura = table.card("Holy Strength");
        assert!(!table.house_casts(now, aura), "{name}");
        // The premise, where a level's noise does not leave it in hand.
        if name == "steady" {
            let mut table = board();
            assert!(
                table.house_casts(before, aura),
                "{name} at generation 10 cast it on the opponent's creature"
            );
        }
    }
}

/// Giant Growth ("Target creature gets +3/+3") with no creature of this
/// seat's own is not cast.
#[test]
fn a_pump_with_nothing_of_its_own_to_pump_waits() {
    for (name, now, _) in levels() {
        let mut table = Table::open("Forest", &["Giant Growth"], &["Forest"], &["Grizzly Bears"]);
        let growth = table.card("Giant Growth");
        assert!(!table.house_casts(now, growth), "{name}");
    }
}

/// Magma Spray ("2 damage to target creature; exiled if it would die") with
/// only this seat's own creature on the table is not cast.
#[test]
fn a_burn_spell_with_nothing_of_theirs_to_burn_waits() {
    for (name, now, _) in levels() {
        let mut table = Table::open(
            "Mountain",
            &["Magma Spray"],
            &["Mountain", "Grizzly Bears"],
            &[],
        );
        let spray = table.card("Magma Spray");
        assert!(!table.house_casts(now, spray), "{name}");
    }
}
