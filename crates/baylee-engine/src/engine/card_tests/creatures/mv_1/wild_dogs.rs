#[allow(clippy::wildcard_imports)]
use super::*;

fn wild_dogs() -> CardIndex {
    card_index("711cda97-bf57-4b22-b9c1-854ef448d27a")
}

#[test]
fn wild_dogs_cycles_to_draw_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[wild_dogs()])
        .battlefield(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dogs_id = in_hand(&engine, p0, wild_dogs()).expect("wild dogs in hand");
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: dogs_id,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, wild_dogs()).is_some());
}
