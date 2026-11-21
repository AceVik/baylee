//! `cards/creatures/mv_3/urborg_emissary.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn urborg_emissary() -> CardIndex {
    card_index("f30943f1-473f-4fa4-b6a7-3dfbd1995ba9")
}

/// Urborg Emissary ({2}{B} 3/1 Human Wizard):
/// "Kicker {1}{U}; When this creature enters, if it was kicked, return target
/// permanent to its owner's hand."
#[test]
fn urborg_emissary_bounces_target_permanent_to_owners_hand_only_when_kicked() {
    let mut e_base = lk::game(SEED, &[], &[plains()], &[urborg_emissary()], &[]);
    lk::float(
        &mut e_base,
        lk::P0,
        &[(ManaColor::Black, 3), (ManaColor::Blue, 2)],
    );
    cast_with_floating(&mut e_base, lk::P0, urborg_emissary());
    lk::yes_no(&mut e_base, false);
    lk::settle(&mut e_base);
    assert!(on_battlefield(&e_base, lk::P1, plains()).is_some());

    // Kicked: bounces any permanent (including a stolen permanent back to its owner's hand).
    let mut e = lk::game(
        SEED,
        &[pearled_unicorn()],
        &[plains()],
        &[urborg_emissary()],
        &[],
    );
    let stolen = lk::obj(&e, lk::P0, pearled_unicorn());
    e.dev_state_mut(lk::P0)
        .unwrap()
        .object_mut(stolen)
        .unwrap()
        .set_controller(lk::P1);
    e.refresh_offer();

    lk::float(
        &mut e,
        lk::P0,
        &[(ManaColor::Black, 3), (ManaColor::Blue, 2)],
    );
    cast_with_floating(&mut e, lk::P0, urborg_emissary());
    lk::yes_no(&mut e, true);
    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[stolen], &[]);
    lk::settle(&mut e);

    assert!(in_hand(&e, lk::P0, pearled_unicorn()).is_some());
    assert!(in_hand(&e, lk::P1, pearled_unicorn()).is_none());
}
