//! `cards/artifacts/mv_1/iron_star.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Iron Star: "Whenever a player casts a red spell, you may pay {1}. If you
/// do, you gain 1 life." The flagship of the five color rocks: a blue spell
/// (Unsummon) first, to show the filter withholds the question on the wrong
/// color, then a red one (Lightning Bolt) to show it asks on the right one.
#[test]
fn iron_star_offers_to_pay_and_gain_life_off_a_red_spell_and_not_a_blue_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[iron_star(), mountain(), mountain(), island()])
        .hand(0, &[unsummon(), lightning_bolt()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is seated");
    let before = life_of(&engine, p0);
    let blue_land = on_battlefield(&engine, p0, island()).expect("the Island is out");
    tap_mana_where(&mut engine, p0, |id| id == blue_land);
    cast_with_floating(&mut engine, p0, unsummon());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("the Elf is a legal target");
    // A blue spell must not ask Iron Star's question at all: were it to,
    // `pass_until` has no arm for `YesNoPrompt::PayTax` and panics here,
    // which is itself a finding.
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life_of(&engine, p0),
        before,
        "no tax asked for a blue spell"
    );
    assert!(
        in_hand(&engine, p1, quiet_creature()).is_some(),
        "Unsummon resolved"
    );

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("p0 is any target");
    pays_the_tax_and_gains_a_life(&mut engine, p0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life_of(&engine, p0),
        before + 1 - 3,
        "gained 1 from the tax, then took 3 from its own Bolt"
    );
}

/// Iron Star: "you may pay {1}" is a choice. Declined, it gains no life and
/// spends nothing of the mana floating in the pool.
#[test]
fn iron_star_declined_gains_nothing_and_spends_nothing() {
    a_rock_declined(iron_star(), mountain(), lightning_bolt());
}

/// Iron Star: "Whenever *a player* casts a red spell, *you* may pay {1}":
/// the opponent's red spell asks the controller of the Iron Star, who pays
/// and gains the life.
#[test]
fn iron_star_pays_its_controller_off_an_opponents_red_spell() {
    a_rock_pays_off_an_opponents_spell(iron_star(), mountain(), mountain(), lightning_bolt());
}
