//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn mana_matrix_reduces_enchantments_but_not_sorceries_or_colored_requirements() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::MANA_MATRIX])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[index::CONTROL_MAGIC, index::COUNSEL_OF_THE_SORATAMI])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let bear = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Blue, 2);
    e.refresh_offer();
    let sorcery = in_hand(&e, p0, index::COUNSEL_OF_THE_SORATAMI).unwrap();
    let aura = in_hand(&e, p0, index::CONTROL_MAGIC).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(legal.castable.contains(&aura));
    assert!(!legal.castable.contains(&sorcery));
    cast_with_floating(&mut e, p0, index::CONTROL_MAGIC);
    aim_at(&mut e, p0, bear);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(bear).unwrap().controller, p0);
}
#[test]
fn mana_matrix_prices_spree_modes_as_one_total_cost() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::MANA_MATRIX, index::GRIZZLY_BEARS])
        .hand(0, &[index::FINAL_SHOWDOWN])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::White, 1);
    e.refresh_offer();
    cast_with_floating(&mut e, p0, index::FINAL_SHOWDOWN);
    let Pending::ChooseCastMode { options, .. } = e.pending().clone() else {
        panic!("modes")
    };
    let option = options
        .iter()
        .find(|option| option.kind == crate::choice::CastModeKind::Modes(3))
        .unwrap();
    assert_eq!(option.cost, baylee_core::mana!("{W}"));
    e.apply(
        p0,
        PlayerAction::ChooseMode(usize::try_from(option.index).unwrap()),
    )
    .unwrap();
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}

/// "**Instant** and enchantment spells you cast cost {2} less": an instant is
/// reduced too. Disenchant is {1}{W}; with the Matrix out, one {W} floating is
/// the whole price, and with nothing floating it is still not castable.
#[test]
fn mana_matrix_reduces_the_instants_its_controller_casts() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::MANA_MATRIX])
        .hand(0, &[index::DISENCHANT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let matrix = on_battlefield(&e, p0, index::MANA_MATRIX).unwrap();
    let instant = in_hand(&e, p0, index::DISENCHANT).unwrap();
    assert!(
        crate::casting::can_cast(e.state(), &RegistryLookup, p0, instant).is_err(),
        "with nothing floating the instant is not castable"
    );
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::White, 1);
    e.refresh_offer();
    assert!(
        crate::casting::can_cast(e.state(), &RegistryLookup, p0, instant).is_ok(),
        "{{1}}{{W}} less {{2}} is {{W}}: one white mana casts the instant"
    );
    cast_with_floating(&mut e, p0, index::DISENCHANT);
    aim_at(&mut e, p0, matrix);
    assert_eq!(
        e.state().players[0].mana_pool.total(),
        0,
        "the single {{W}} paid for the whole spell"
    );
}

/// "...spells **you** cast": an opponent's instant is not reduced by my Matrix.
/// Disenchant with only {W} floating for the other seat is one mana short.
#[test]
fn mana_matrix_does_not_reduce_an_opponents_instant() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::MANA_MATRIX])
        .hand(1, &[index::DISENCHANT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    reach_their_main_phase(&mut e, p1);
    let instant = in_hand(&e, p1, index::DISENCHANT).unwrap();
    e.dev_state_mut(p1).unwrap().players[1]
        .mana_pool
        .add(ManaColor::White, 1);
    e.refresh_offer();
    assert!(
        crate::casting::can_cast(e.state(), &RegistryLookup, p1, instant).is_err(),
        "the opponent pays the full {{1}}{{W}}: my Matrix does not discount it, so {{W}} alone is one short"
    );
    e.dev_state_mut(p1).unwrap().players[1]
        .mana_pool
        .add(ManaColor::Colorless, 1);
    e.refresh_offer();
    assert!(
        crate::casting::can_cast(e.state(), &RegistryLookup, p1, instant).is_ok(),
        "with the full {{1}}{{W}} the same instant is castable (control: only the price was short)"
    );
}
