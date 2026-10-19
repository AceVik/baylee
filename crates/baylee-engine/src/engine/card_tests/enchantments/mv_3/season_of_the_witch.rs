//! `cards/enchantments/mv_3/season_of_the_witch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::YesNoPrompt;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);

fn season_card() -> CardIndex {
    card_index("307813c5-c3a8-46c5-8ed4-3b244dd169dc")
}
fn bears_card() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}
fn hill_giant_card() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}
fn wall_of_air_card() -> CardIndex {
    // Wall of Air: defender, so it could not have attacked.
    card_index("b2d3da40-e2f7-4480-9da9-33019d6f4071")
}
fn raiders_card() -> CardIndex {
    card_index("a37159df-f6d7-4db6-85de-0ea77f425993")
}

fn pay_life_question(e: &Engine<RegistryLookup>) -> bool {
    matches!(
        e.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::PayLife { .. },
            ..
        }
    )
}

/// Season of the Witch is cast off three Swamps in the first main phase and
/// has resolved.
fn season_cast() -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, swamp())
        .hand(0, &[season_card()])
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    cast_from_hand(&mut e, P0, season_card());
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, season_card()).is_some());
    e
}

/// Season of the Witch (307813c5-c3a8-46c5-8ed4-3b244dd169dc) — "At the
/// beginning of your upkeep, sacrifice this enchantment unless you pay 2
/// life." Paying costs 2 and keeps it.
#[test]
fn season_of_the_witch_upkeep_paid_costs_two_life_and_stays() {
    let mut e = season_cast();
    let life_before = e.state().players[0].life;
    pass_until(&mut e, pay_life_question);
    assert_eq!(e.state().turn.active, P0, "our own upkeep");
    assert_eq!(e.state().turn.step, Step::Upkeep);
    let Pending::YesNo { player, prompt, .. } = e.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, P0);
    assert_eq!(prompt, YesNoPrompt::PayLife { amount: 2 });
    e.apply(P0, PlayerAction::YesNo(true)).expect("pay");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, life_before - 2);
    assert!(on_battlefield(&e, P0, season_card()).is_some(), "it stays");
}

/// Season of the Witch — declining the payment sacrifices it, and the life
/// is untouched.
#[test]
fn season_of_the_witch_upkeep_declined_sacrifices_it() {
    let mut e = season_cast();
    let life_before = e.state().players[0].life;
    pass_until(&mut e, pay_life_question);
    e.apply(P0, PlayerAction::YesNo(false)).expect("decline");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, life_before);
    assert!(on_battlefield(&e, P0, season_card()).is_none());
    assert!(in_graveyard(&e, P0, season_card()).is_some());
}

/// Season of the Witch — "At the beginning of the end step, destroy all
/// untapped creatures that didn't attack this turn, except for creatures that
/// couldn't attack." The idle Giant goes; the attacking Bears, the tapped
/// Goblin, the Wall (defender) and the Elves (summoning sick) stay.
#[test]
fn season_of_the_witch_destroys_the_idle_creatures_that_could_have_attacked() {
    let mut e = Duel::new(SEED, swamp())
        .hand(0, &[season_card(), llanowar_elves()])
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                forest(),
                bears_card(),
                hill_giant_card(),
                wall_of_air_card(),
                raiders_card(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let bears = on_battlefield(&e, P0, bears_card()).expect("the Bears");
    let giant = on_battlefield(&e, P0, hill_giant_card()).expect("the Giant");
    let wall = on_battlefield(&e, P0, wall_of_air_card()).expect("the Wall");
    let goblin = on_battlefield(&e, P0, raiders_card()).expect("the Goblin");

    // Season first, off the Swamps; the Elves come off the Forest's mana
    // and arrive summoning sick.
    tap_all_mana(&mut e, P0);
    cast_with_floating(&mut e, P0, season_card());
    pass_until(&mut e, stack_is_empty);
    cast_with_floating(&mut e, P0, llanowar_elves());
    pass_until(&mut e, stack_is_empty);
    let elves = on_battlefield(&e, P0, llanowar_elves()).expect("the Elves arrived");
    assert!(on_battlefield(&e, P0, season_card()).is_some());

    e.dev_state_mut(P0)
        .expect("the harness may set boards up")
        .set_tapped(goblin, true);
    e.refresh_offer();

    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareAttackers);
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bears, Defender::Player(PlayerId::new(1)))],
        },
    )
    .expect("the Bears attack");

    pass_until(&mut e, |e| e.state().turn.step == Step::End);
    pass_until(&mut e, stack_is_empty);

    let zone = |id| e.state().object(id).map(|o| o.zone);
    assert_eq!(
        zone(giant),
        Some(Zone::Graveyard),
        "untapped, able to attack, did not: destroyed"
    );
    assert_eq!(zone(bears), Some(Zone::Battlefield), "it attacked");
    assert_eq!(zone(goblin), Some(Zone::Battlefield), "it was tapped");
    assert_eq!(
        zone(wall),
        Some(Zone::Battlefield),
        "defender: could not attack"
    );
    assert_eq!(
        zone(elves),
        Some(Zone::Battlefield),
        "summoning sick: could not attack"
    );
}
