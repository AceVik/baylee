//! `cards/enchantments/auras/mv_2/ward_of_lights.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn ward_of_lights() -> CardIndex {
    card_index("edc05b33-2bfa-4692-bb3b-26b7cf111ce8")
}

/// Ward of Lights (`{W}{W}`, `Coverage::Partial`): Aura with flash that asks
/// for a color as it enters and grants enchanted creature protection from the
/// chosen color (without removing Ward of Lights itself).
#[test]
fn ward_of_lights_grants_enchanted_creature_protection_from_chosen_color() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), grizzly_bears()])
        .hand(0, &[ward_of_lights()])
        .battlefield(1, &[mountain(), grizzly_bears()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, p0, ward_of_lights());
    aim_at(&mut engine, p0, my_bear);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // On p1's turn, red Lightning Bolt cannot target my_bear.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, lightning_bolt());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert!(!options.contains(&my_bear));
    assert!(options.contains(&their_bear));
}
