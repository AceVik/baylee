//! `cards/artifacts/mv_1/crystal_rod.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Crystal Rod: "Whenever a player casts a blue spell, you may pay {1}. If
/// you do, you gain 1 life." Unsummon bouncing an opponent's creature is the
/// stimulus; the trigger asks before Unsummon itself resolves.
#[test]
fn crystal_rod_offers_to_pay_and_gain_life_off_a_blue_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[crystal_rod(), island(), island()])
        .hand(0, &[unsummon()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is seated");
    tap_all_mana(&mut engine, p0);
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

    pays_the_tax_and_gains_a_life(&mut engine, p0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_hand(&engine, p1, quiet_creature()).is_some(),
        "Unsummon still resolved and bounced the Elf"
    );
}

/// Crystal Rod: "you may pay {1}" is a choice. Declined, it gains no life and
/// spends nothing of the mana floating in the pool.
#[test]
fn crystal_rod_declined_gains_nothing_and_spends_nothing() {
    a_rock_declined(crystal_rod(), island(), flying_men());
}

/// Crystal Rod: "Whenever *a player* casts a blue spell, *you* may pay {1}":
/// the opponent's blue spell asks the controller of the Crystal Rod, who pays
/// and gains the life.
#[test]
fn crystal_rod_pays_its_controller_off_an_opponents_blue_spell() {
    a_rock_pays_off_an_opponents_spell(crystal_rod(), island(), island(), flying_men());
}

/// Crystal Rod: only a blue spell is asked about. A spell of another color
/// resolves without a question and the life total stands.
#[test]
fn crystal_rod_ignores_a_spell_of_another_color() {
    a_rock_ignores_another_color(crystal_rod(), forest(), quiet_creature());
}
