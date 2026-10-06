//! `cards/enchantments/auras/mv_3/farmstead.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Farmstead — {W}{W}{W}, Aura: "Enchant land. Enchanted land has 'At the
/// beginning of your upkeep, you may pay {W}{W}. If you do, you gain 1
/// life.'" The land has the ability, so the question comes in its
/// controller's upkeep and in nobody else's, the price is white, and a
/// player who says yes with an empty pool makes the mana then (CR 605.3a).
#[test]
fn farmstead_asks_its_lands_controller_for_two_white_and_sells_one_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let farmstead = card_index("0d71b157-09a0-4fd4-beb9-103117a784ad");
    let mut engine = Duel::new(9, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[farmstead])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, farmstead);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Enchant land asks for a land, got {:?}", engine.pending())
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(on_battlefield(&engine, p0, farmstead).expect("resolved"))
            .and_then(|o| o.attached_to),
        Some(options[0])
    );

    // The opponent's upkeep passes without a question (`pass_until` would
    // stop on one); the next is p0's own.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(engine.state().turn.active, p0, "p1's upkeep asked nothing");
    assert_ne!(player, p1);
    assert_eq!(
        prompt,
        YesNoPrompt::PayMana {
            cost: baylee_core::mana!("{W}{W}")
        }
    );
    let life = engine.state().players[0].life;
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert_eq!(engine.state().players[0].life, life + 1, "paid, and bought");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::White),
        2,
        "four Plains made four white, and two of them paid"
    );
}
