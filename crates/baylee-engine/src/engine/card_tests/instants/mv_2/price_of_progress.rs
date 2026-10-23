use super::*;

fn price_of_progress() -> CardIndex {
    card_index("e9da499c-fa43-4e94-8395-5c030ff39502")
}

fn city_of_traitors() -> CardIndex {
    card_index("f161111d-9747-47b3-bb10-3c8bded32e21")
}

#[test]
fn price_of_progress_deals_2_damage_per_nonbasic_land_to_each_player() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), city_of_traitors()])
        .battlefield(1, &[city_of_traitors(), city_of_traitors()])
        .hand(0, &[price_of_progress()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, price_of_progress());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 18);
    assert_eq!(engine.state().players[1].life, 16);
}
