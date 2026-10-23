//! `cards/creatures/mv_4/auspicious_ancestor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{DARK_BANISHING, SAVANNAH_LIONS};

fn auspicious_ancestor() -> CardIndex {
    card_index("82fb9834-de65-41a5-b908-f49668dee3ec")
}

#[test]
fn auspicious_ancestor_gains_life_on_white_spell_and_on_death() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[SAVANNAH_LIONS, DARK_BANISHING])
        .battlefield(
            0,
            &[
                auspicious_ancestor(),
                plains(),
                plains(),
                swamp(),
                plains(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ancestor = on_battlefield(&engine, p0, auspicious_ancestor()).unwrap();
    let lions = in_hand(&engine, p0, SAVANNAH_LIONS).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: lions })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 21);

    // Destroy Auspicious Ancestor to trigger its death ability (+3 life).
    let banishing = in_hand(&engine, p0, DARK_BANISHING).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: banishing })
        .unwrap();
    aim_at(&mut engine, p0, ancestor);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 24);
}

#[test]
fn auspicious_ancestor_gains_nothing_when_the_payment_is_declined() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[SAVANNAH_LIONS])
        .battlefield(0, &[auspicious_ancestor(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, SAVANNAH_LIONS);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 20);
}

#[test]
fn auspicious_ancestor_also_answers_the_opponents_white_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(1, &[SAVANNAH_LIONS])
        .battlefield(0, &[auspicious_ancestor(), plains()])
        .battlefield(1, &[plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    // p0 has {1} floating to pay with when the trigger asks.
    engine.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::White, 1);
    engine.refresh_offer();
    cast_from_hand(&mut engine, p1, SAVANNAH_LIONS);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        panic!("expected a question, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "the Ancestor's controller decides and pays");
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 21);
    assert_eq!(engine.state().players[1].life, 20);
}
