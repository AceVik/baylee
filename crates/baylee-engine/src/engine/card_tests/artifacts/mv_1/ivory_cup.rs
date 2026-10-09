//! `cards/artifacts/mv_1/ivory_cup.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ivory Cup: "Whenever a player casts a white spell, you may pay {1}. If
/// you do, you gain 1 life." Swords to Plowshares is the white stimulus, and
/// its own life gain (to the exiled creature's controller) is a second,
/// independent number in the same resolution.
#[test]
fn ivory_cup_offers_to_pay_and_gain_life_off_a_white_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ivory_cup(), plains(), plains()])
        .hand(0, &[swords_to_plowshares()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is seated");
    let their_life = life_of(&engine, p1);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, swords_to_plowshares());
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
    assert_eq!(
        life_of(&engine, p1),
        their_life + 1,
        "Swords' own life gain still happened, to the Elf's controller"
    );
}

/// Ivory Cup: "you may pay {1}" is a choice. Declined, it gains no life and
/// spends nothing of the mana floating in the pool.
#[test]
fn ivory_cup_declined_gains_nothing_and_spends_nothing() {
    a_rock_declined(ivory_cup(), plains(), camel());
}

/// Ivory Cup: "Whenever *a player* casts a white spell, *you* may pay {1}":
/// the opponent's white spell asks the controller of the Ivory Cup, who pays
/// and gains the life.
#[test]
fn ivory_cup_pays_its_controller_off_an_opponents_white_spell() {
    a_rock_pays_off_an_opponents_spell(ivory_cup(), plains(), plains(), camel());
}

/// Ivory Cup: only a white spell is asked about. A spell of another color
/// resolves without a question and the life total stands.
#[test]
fn ivory_cup_ignores_a_spell_of_another_color() {
    a_rock_ignores_another_color(ivory_cup(), mountain(), lightning_bolt());
}
