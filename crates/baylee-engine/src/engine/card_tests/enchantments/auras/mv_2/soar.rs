//! `cards/enchantments/auras/mv_2/soar.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;

fn soar() -> CardIndex {
    card_index("acee6f4f-8b66-495d-bb0e-71c339e7dc49")
}

/// Soar (`{1}{U}`, `Coverage::Partial`): Aura that enchants a creature and
/// gives it +0/+1 and flying, cast at sorcery speed (flash is left off, see
/// the card).
#[test]
fn soar_enchants_creature_and_grants_0_1_and_flying() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), grizzly_bears()])
        .hand(0, &[soar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, soar());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (2, 3));
    assert!(keywords(&engine, bear).contains(KeywordSet::FLYING));
}

/// Flash is left off because the sacrifice rider that goes with it is not
/// supported: the Aura is castable in the main phase and not in the end step.
#[test]
fn soar_is_castable_only_at_sorcery_speed() {
    let p0 = PlayerId::new(0);
    let mut engine = lk::game(
        SEED,
        &[island(), island(), grizzly_bears()],
        &[],
        &[soar()],
        &[],
    );
    lk::float(
        &mut engine,
        p0,
        &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
    );
    assert!(
        lk::castable(&engine, p0, soar()),
        "castable in the main phase with its mana floating"
    );

    lk::to_step(&mut engine, p0, Step::End);
    lk::float(
        &mut engine,
        p0,
        &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
    );
    assert!(
        !lk::castable(&engine, p0, soar()),
        "not castable in the end step without flash"
    );
}
