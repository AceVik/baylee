use super::*;

fn pegasus_stampede() -> CardIndex {
    card_index("16b4a48a-fec2-433c-948d-e0b16c961172")
}

#[test]
fn pegasus_stampede_creates_1_1_white_pegasus_token_with_flying() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[pegasus_stampede()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, pegasus_stampede());
    pass_until(&mut engine, stack_is_empty);

    let token = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| engine.state().object(id).unwrap().card.is_none())
        .expect("a Pegasus token was created");
    assert_eq!(pt(&engine, token), (1, 1));
    assert!(keywords(&engine, token).contains(KeywordSet::FLYING));
}
