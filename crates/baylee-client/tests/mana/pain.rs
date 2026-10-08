//! A cast paid by a land that hurts: a painland's coloured tap deals its
//! controller 1 damage beside the mana, City of Brass deals 1 whenever it
//! becomes tapped. The owner's report (08.10.2026): "whenever the player
//! casts something and this land gets tapped, the cast is interrupted and the
//! mana is auto-tapped".
//!
//! Every test clicks the card the way a player does — arm, confirm — and lets
//! the run spend its plan against a real `LocalHost`.

use super::*;

/// Adarkar Wastes: `{T}: Add {C}` and `{T}: Add {W} or {U}`, 1 damage to you.
const ADARKAR: &str = "d5ad26cc-2bdb-46b7-b8bf-dd099d5fa09b";
/// City of Brass: any colour, and 1 damage whenever it becomes tapped.
const BRASS: &str = "f25351e3-539b-4bbc-b92d-6480acf4d722";
/// Serra Zealot, `{W}`.
const ZEALOT: &str = "989a7353-8d3d-4ea2-ab5e-8535d95dddae";
/// Crystalline Sliver, `{W}{U}`: a creature, so cast only with an empty stack.
const SLIVER: &str = "ba3aa1eb-722a-47d3-83be-96daddb50265";

/// Seat 0 with `hand` and these lands, in its first main phase.
fn table_with(hand: &[&str], lands: &[&str]) -> Table {
    let mut preset = white_preset(hand, 0);
    preset.seats[0].starting_battlefield = lands.iter().map(|l| entry(l)).collect();
    let mut table = Table::open_with(&preset);
    table.walk_to_main();
    table
}

fn life(table: &Table) -> i32 {
    table.view().seat(PlayerId::new(0)).expect("own seat").life
}

fn on_stack_or_battlefield(table: &Table, name: &str) -> bool {
    table
        .view()
        .battlefield
        .iter()
        .chain(table.view().stack.iter())
        .any(|o| o.name == name)
}

/// Clicks `name` in hand twice (arm, confirm) and plays the run out, one
/// engine round trip per step. Returns the run's refusal, if it had one.
fn click_to_cast(table: &mut Table, name: &str) -> Option<String> {
    use baylee_client::Duel;
    use baylee_client::input::activate_card;
    let card = id_in_hand(table, name);
    let mut duel = Duel::default();
    refresh(&mut duel, table);
    activate_card(&mut duel, card);
    activate_card(&mut duel, card);
    for _ in 0..16 {
        for action in duel.take_outbox() {
            table.submit(action);
        }
        refresh(&mut duel, table);
        if duel.mana_run.is_none() {
            break;
        }
        baylee_client::advance_mana_run(&mut duel);
    }
    for action in duel.take_outbox() {
        table.submit(action);
    }
    duel.last_error.map(|e| format!("{e:?}"))
}

/// One white pip, a Plains and a painland: the Plains pays, nobody is hurt.
#[test]
fn a_white_pip_is_paid_by_the_plains_and_not_the_painland() {
    let mut table = table_with(&[ZEALOT], &[PLAINS, ADARKAR]);
    let before = life(&table);
    assert_eq!(click_to_cast(&mut table, "Serra Zealot"), None);
    assert!(on_stack_or_battlefield(&table, "Serra Zealot"));
    assert_eq!(life(&table), before, "the Plains paid");
}

/// `{W}{U}` from a Plains and Adarkar Wastes: the Wastes taps for blue, deals
/// its damage, and the spell is cast.
#[test]
fn white_and_blue_from_a_plains_and_a_painland_casts_the_spell() {
    let mut table = table_with(&[SLIVER], &[PLAINS, ADARKAR]);
    let before = life(&table);
    let refusal = click_to_cast(&mut table, "Crystalline Sliver");
    assert_eq!(refusal, None);
    assert!(on_stack_or_battlefield(&table, "Crystalline Sliver"));
    assert_eq!(life(&table), before - 1, "the Wastes' blue cost a life");
}

/// The colour tapped by hand first, then the card clicked: the run spends
/// the blue already floating and taps the Plains, and the cast goes through.
#[test]
fn a_painland_tapped_by_hand_for_its_colour_does_not_stop_the_cast() {
    let mut table = table_with(&[SLIVER], &[PLAINS, ADARKAR]);
    let wastes = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Adarkar Wastes")
        .expect("the Wastes")
        .id;
    let before = life(&table);
    table.submit(PlayerAction::ActivateAbility {
        source: wastes,
        ability_index: 1,
    });
    table.submit(PlayerAction::ChooseColor(
        baylee_core::mana::ManaColor::Blue,
    ));
    assert_eq!(life(&table), before - 1);
    assert_eq!(click_to_cast(&mut table, "Crystalline Sliver"), None);
    assert!(on_stack_or_battlefield(&table, "Crystalline Sliver"));
}

/// With a clean land for the colour, City of Brass is left untapped. A guard,
/// not the proof of its price: the planner's breadth order already passes
/// over a five-colour land here, and `city_of_brass_is_priced_by_its_own_tap_trigger`
/// is the test that reads the price.
#[test]
fn city_of_brass_is_passed_over_when_a_clean_land_makes_the_colour() {
    let mut table = table_with(&[SLIVER], &[ISLAND, PLAINS, BRASS]);
    let before = life(&table);
    assert_eq!(click_to_cast(&mut table, "Crystalline Sliver"), None);
    assert!(on_stack_or_battlefield(&table, "Crystalline Sliver"));
    assert_eq!(life(&table), before, "the City was never tapped");
}

/// **A pinned limitation**, the half of the owner's report a planner cannot
/// reach. City of Brass is the only white here, and its damage is a
/// triggered ability: tapped before the spell, as this client pays (the mana
/// floats first, then the spell is cast), the trigger goes on the stack the
/// next time a player would receive priority (CR 603.3), which is right
/// then, and a creature can no longer be cast. The run stops with the City
/// tapped and the mana floating. Any painland does the same the moment
/// something on the board triggers on damage or life loss.
///
/// The rules answer is to tap inside the cast (CR 601.2g), where a trigger
/// waits until the spell has been cast (CR 601.2i). That is a change to how
/// this client casts, and this test goes red the day it lands.
#[test]
fn city_of_brass_as_the_only_colour_still_stops_a_creature_cast() {
    let mut table = table_with(&[SLIVER], &[ISLAND, BRASS]);
    let refusal = click_to_cast(&mut table, "Crystalline Sliver");
    assert_eq!(refusal.as_deref(), Some("Said(PlanSpellRefused)"));
    assert!(!on_stack_or_battlefield(&table, "Crystalline Sliver"));
}
