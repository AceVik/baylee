//! Oracle checklist: pay exactly {3}{U}; draw one card on resolution, not activation; the ability does not tap the Mage.
#[allow(clippy::wildcard_imports)]
use super::*;

fn azure_mage() -> CardIndex {
    card_index("bbf9fd47-10e1-4749-b06f-4fe72200afc7")
}

#[test]
fn mage_pays_three_and_blue_then_draws_only_when_its_ability_resolves() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[azure_mage()])
        .hand(0, &[island()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let top = hand_to_library_top(&mut e, p0, island());
    let before = library_size(&e, p0);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Blue, 1);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    e.refresh_offer();

    activate(&mut e, p0, azure_mage(), 0);
    assert_eq!(library_size(&e, p0), before, "drawing waits for resolution");
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(library_size(&e, p0), before - 1);
    assert_eq!(in_hand(&e, p0, island()), Some(top));
    assert!(on_battlefield(&e, p0, azure_mage()).is_some());
    assert!(!is_tapped(
        &e,
        on_battlefield(&e, p0, azure_mage()).unwrap()
    ));
}

#[test]
fn mage_cannot_pay_four_colorless_in_place_of_blue() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[azure_mage()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let mage = on_battlefield(&e, p0, azure_mage()).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 4);
    e.refresh_offer();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(
        legal.abilities.iter().all(|(source, _)| *source != mage),
        "four colorless mana cannot pay the {{3}}{{U}} cost"
    );
}
