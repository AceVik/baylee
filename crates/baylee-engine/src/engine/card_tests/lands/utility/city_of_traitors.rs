//! `cards/lands/utility/city_of_traitors.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn city_of_traitors_taps_for_two_colorless_and_sacrifices_on_another_land() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(82, forest())
        .battlefield(0, &[city_of_traitors()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let city = on_battlefield(&engine, p0, city_of_traitors()).expect("City of Traitors deployed");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        0
    );

    activate(&mut engine, p0, city_of_traitors(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2,
        "City of Traitors adds {{C}}{{C}}"
    );
    assert!(is_tapped(&engine, city));

    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, city_of_traitors()).is_none(),
        "City of Traitors was sacrificed"
    );
    assert!(
        in_graveyard(&engine, p0, city_of_traitors()).is_some(),
        "City of Traitors is in graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, forest()).is_some(),
        "the played Forest remains on battlefield"
    );
}

#[test]
fn city_of_traitors_does_not_sacrifice_itself_for_its_own_play() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[city_of_traitors()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let city = play_land(&mut e, p0, city_of_traitors());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(city).unwrap().zone, Zone::Battlefield);
}
