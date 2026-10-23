#[allow(clippy::wildcard_imports)]
use super::*;

fn lotus_blossom() -> CardIndex {
    card_index("32e66ea0-5f35-4df8-8d00-580403c51bb5")
}

#[test]
fn lotus_blossom_adds_petal_counter_on_upkeep_and_sacrifices_for_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[lotus_blossom()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    let blossom = on_battlefield(&engine, p0, lotus_blossom()).expect("seated");
    assert_eq!(
        counters_on(&engine, blossom, baylee_cards_dsl::counters::PETAL),
        1
    );
    activate(&mut engine, p0, lotus_blossom(), 1);
    if matches!(engine.pending(), Pending::ChooseColor { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseColor(baylee_core::mana::ManaColor::Green),
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, lotus_blossom()).is_none());
    assert!(in_graveyard(&engine, p0, lotus_blossom()).is_some());
}
