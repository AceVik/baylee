//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn elder_counts_distinct_own_colors_for_a_layer_seven_bonus_and_exact_mana() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[faeburrow_elder(), baleful_strix(), baleful_strix()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let elder = on_battlefield(&e, p0, faeburrow_elder()).unwrap();
    assert_eq!(pt(&e, elder), (4, 4));
    assert!(keywords(&e, elder).contains(KeywordSet::VIGILANCE));
    activate(&mut e, p0, faeburrow_elder(), 1);
    assert!(stack_is_empty(&e));
    let pool = &e.state().players[0].mana_pool;
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Green,
    ] {
        assert_eq!(pool.available(color), 1);
    }
    assert_eq!(pool.total(), 4);
    let strixes: Vec<_> = e
        .state()
        .battlefield_view()
        .iter()
        .copied()
        .filter(|id| {
            e.state()
                .object(*id)
                .unwrap()
                .card
                .is_some_and(|card| card.index == baleful_strix())
        })
        .collect();
    bury(&mut e, &strixes);
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    assert_eq!(pt(&e, elder), (2, 2));
    let state = e.dev_state_mut(p0).unwrap();
    let base = state.object_mut(elder).unwrap().base_mut();
    base.power = Some(5);
    base.toughness = Some(5);
    state.refresh_characteristics();
    assert_eq!(pt(&e, elder), (7, 7), "bonus, not a base P/T setting");
}
#[test]
fn elder_cast_from_hand_survives_with_its_own_two_colors() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), plains()])
        .hand(0, &[faeburrow_elder()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, faeburrow_elder());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        pt(&e, on_battlefield(&e, p0, faeburrow_elder()).unwrap()),
        (2, 2)
    );
}
