//! `cards/enchantments/auras/mv_2/nature_s_kiss.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, NATURE_S_KISS};

/// `Nature's Kiss` ({1}{G}, Enchantment — Aura, Partial) — `{1}`, Exile a card
/// from your graveyard: Enchanted creature gets +1/+1 until end of turn.
#[test]
fn nature_s_kiss_exiles_card_from_graveyard_to_pump_enchanted_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[GRIZZLY_BEARS, forest(), forest(), forest()])
        .hand(0, &[NATURE_S_KISS, forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let gy_card = hand_to_graveyard(&mut engine, p0, forest());
    cast_from_hand(&mut engine, p0, NATURE_S_KISS);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, NATURE_S_KISS, 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![gy_card],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (3, 3));
}
