//! `cards/instants/mv_2/withering_boon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn withering_boon() -> CardIndex {
    card_index("6d87415b-0618-48e1-9510-320c4611ecda")
}

/// Withering Boon (`{1}{B}`, `Coverage::Implemented`): Instant that costs an
/// additional 3 life to cast and counters target creature spell.
#[test]
fn withering_boon_pays_3_life_and_counters_creature_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[grizzly_bears()])
        .battlefield(1, &[swamp(), swamp()])
        .hand(1, &[withering_boon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, grizzly_bears());
    let bear_spell = on_stack(&engine, grizzly_bears()).unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, withering_boon());
    aim_at(&mut engine, p1, bear_spell);
    assert_eq!(engine.state().players[1].life, 17);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
}

/// "Counter target creature spell": a noncreature spell is no target, so with
/// only a Lightning Bolt on the stack the Boon cannot be cast at all and its
/// 3 life are not paid; the Bolt resolves and is the only life lost.
#[test]
fn withering_boon_cannot_be_cast_when_only_a_noncreature_spell_is_on_the_stack() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[lightning_bolt()])
        .battlefield(1, &[swamp(), swamp()])
        .hand(1, &[withering_boon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let boon = in_hand(&engine, p1, withering_boon()).expect("in hand");
    assert!(
        engine
            .apply(p1, PlayerAction::CastSpell { card: boon })
            .is_err(),
        "an instant spell is not a creature spell"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 17, "only the Bolt's 3");
    assert!(in_hand(&engine, p1, withering_boon()).is_some());
}

/// "Pay 3 life" is a cost: with 2 life the Boon cannot be cast, and the
/// creature spell it would have countered resolves.
#[test]
fn withering_boon_cannot_be_cast_without_3_life_to_pay() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[grizzly_bears()])
        .battlefield(1, &[swamp(), swamp()])
        .hand(1, &[withering_boon()])
        .life(1, 2)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, grizzly_bears());
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let boon = in_hand(&engine, p1, withering_boon()).expect("in hand");
    assert!(
        engine
            .apply(p1, PlayerAction::CastSpell { card: boon })
            .is_err(),
        "3 life cannot be paid from 2"
    );
    assert_eq!(engine.state().players[1].life, 2, "nothing was paid");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "the Bears was not countered"
    );
}
