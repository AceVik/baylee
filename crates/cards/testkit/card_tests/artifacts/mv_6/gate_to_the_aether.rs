//! Gate to the Aether — `{6}` Artifact (MRD).
//! Oracle: "At the beginning of each player's upkeep, that player reveals the top card of their library. If it's an artifact, creature, enchantment, or land card, the player may put it onto the battlefield."

use super::*;

fn gate_to_the_aether() -> CardIndex {
    card_index("a03ab63a-83f3-4dd6-926d-08d09ce2e696")
}

#[test]
fn gate_to_the_aether_lets_active_player_put_revealed_permanent_card_onto_battlefield_on_upkeep() {
    let p0 = PlayerId::new(0);
    let gate = gate_to_the_aether();
    let mut engine = Duel::new(SEED, forest()).battlefield(0, &[gate]).start();
    keep_mulligans(&mut engine);

    let p0_forests_before = all_on_battlefield(&engine, p0, forest()).len();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        p0_forests_before + 1
    );
}

#[test]
fn gate_to_the_aether_offers_the_reveal_to_the_player_whose_upkeep_it_is() {
    let p1 = PlayerId::new(1);
    let gate = gate_to_the_aether();
    let mut engine = Duel::new(SEED, forest()).battlefield(0, &[gate]).start();
    keep_mulligans(&mut engine);

    let before = all_on_battlefield(&engine, p1, forest()).len();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert_eq!(player, p1);
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(all_on_battlefield(&engine, p1, forest()).len(), before + 1);
}
