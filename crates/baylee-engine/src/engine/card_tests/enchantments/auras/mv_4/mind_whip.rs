//! `cards/enchantments/auras/mv_4/mind_whip.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mind_whip() -> CardIndex {
    card_index("e26f7a58-18ae-432b-bc3f-c410512eb6bf")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Mind Whip (e26f7a58-18ae-432b-bc3f-c410512eb6bf) — {2}{B}{B} Enchantment — Aura:
/// - "Enchant creature"
/// - "At the beginning of the upkeep of enchanted creature's controller, that player may pay {3}. If they don't, this Aura deals 2 damage to that player and you tap that creature."
#[test]
fn mind_whip_does_nothing_when_three_is_paid_and_deals_two_and_taps_creature_when_declined() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[grizzly_bears(), swamp(), swamp(), swamp()])
        .hand(0, &[mind_whip()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, P0, mind_whip());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    // 1. P1's upkeep (turn 2), paid branch: P1 taps 3 Swamps and pays {3}.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana_but(&mut engine, P1, None);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[1].life,
        20,
        "paying {{3}} avoids the 2 damage"
    );
    assert!(
        !entered_tapped(&engine, bears),
        "paying {{3}} keeps enchanted creature untapped"
    );

    // 2. P1's next upkeep (turn 4), unpaid branch: P1 declines to pay {3}.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 4 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 4 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[1].life,
        18,
        "declining {{3}} deals 2 damage to enchanted creature's controller"
    );
    assert!(
        entered_tapped(&engine, bears),
        "declining {{3}} taps enchanted creature"
    );
}
