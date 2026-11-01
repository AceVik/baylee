//! `cards/enchantments/mv_2/chill.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn chill() -> CardIndex {
    card_index("70a93d0f-9a02-4e60-a2b8-827fd2410604")
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Aims the Bolt that was just cast at p1's face.
#[track_caller]
fn bolt_the_opponent(engine: &mut Engine<RegistryLookup>) {
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
}

/// Chill (70a93d0f-9a02-4e60-a2b8-827fd2410604) — {1}{U} Enchantment:
/// "Red spells cost {2} more to cast."
#[test]
fn chill_makes_a_red_spell_cost_two_more_than_its_printed_cost() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[chill(), mountain(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    let bolt = in_hand(&engine, P0, lightning_bolt()).unwrap();
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: bolt })
            .is_err(),
        "with only two red mana available, Lightning Bolt cannot pay {{R}} plus {{2}} under Chill"
    );
    assert!(in_hand(&engine, P0, lightning_bolt()).is_some());
}

#[test]
fn chill_lets_a_red_spell_be_cast_for_exactly_its_cost_plus_two() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[chill(), mountain(), mountain(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, lightning_bolt());
    bolt_the_opponent(&mut engine);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 17);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "all three Mountains were spent: {{R}} plus the tax of {{2}}"
    );
}

#[test]
fn chill_does_not_tax_a_spell_that_is_not_red() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[chill(), forest(), forest()])
        .hand(0, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, grizzly_bears()).is_some(),
        "Grizzly Bears still costs {{1}}{{G}}"
    );
}

#[test]
fn chill_taxes_the_opponents_red_spells_too() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[chill()])
        .battlefield(1, &[mountain(), mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    tap_all_mana(&mut engine, P1);
    let bolt = in_hand(&engine, P1, lightning_bolt()).unwrap();
    assert!(
        engine
            .apply(P1, PlayerAction::CastSpell { card: bolt })
            .is_err(),
        "\"Red spells\" is every player's red spells"
    );
}
