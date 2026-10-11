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
