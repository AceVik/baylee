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
