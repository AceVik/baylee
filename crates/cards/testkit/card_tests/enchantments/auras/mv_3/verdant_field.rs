//! `cards/enchantments/auras/mv_3/verdant_field.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn verdant_field() -> CardIndex {
    card_index("4d59fd2b-7fc6-49a7-8938-d3f08902a5a8")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Enchanted land gains `{T}: Target creature gets +1/+1 until end of turn`,
/// leaving other creatures untouched and wearing off by next turn.
#[test]
fn verdant_field_grants_temporary_plus_one_pump_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[forest(), grizzly_bears(), savannah_lions()],
        &[],
        &[verdant_field()],
        &[],
    );
    let land = lk::obj(&e, p0, forest());
    let bear = lk::obj(&e, p0, grizzly_bears());
    let lions = lk::obj(&e, p0, savannah_lions());

    lk::float(&mut e, p0, &[(ManaColor::Green, 3)]);
    cast_with_floating(&mut e, p0, verdant_field());
    let opts = pass_until_targets(&mut e, p0);
    assert!(opts.contains(&land));
    assert!(!opts.contains(&bear));
    lk::aim(&mut e, &[land], &[]);
    lk::settle(&mut e);

    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: land,
            ability_index: baylee_engine::choice::GRANTED_ABILITY,
        },
    )
    .unwrap();
    lk::aim(&mut e, &[bear], &[]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, bear), (3, 3));
    assert_eq!(pt(&e, lions), (2, 1));
    assert!(lk::tapped(&e, land));

    reach_their_main_phase(&mut e, p1);
    assert_eq!(pt(&e, bear), (2, 2));
}
