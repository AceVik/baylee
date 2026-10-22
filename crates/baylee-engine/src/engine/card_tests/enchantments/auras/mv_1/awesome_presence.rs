//! `cards/enchantments/auras/mv_1/awesome_presence.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn awesome_presence() -> CardIndex {
    card_index("d6efff61-2087-4f48-a8a8-6c465190dad9")
}

/// `Awesome Presence` ({U}, Enchantment — Aura, `Coverage::Partial`) attaches
/// to target creature.
#[test]
fn awesome_presence_attaches_to_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), grizzly_bears()])
        .hand(0, &[awesome_presence()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, awesome_presence());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, awesome_presence()).expect("Awesome Presence enters");
    assert_eq!(
        engine.state().object(aura).unwrap().attached_to,
        Some(bears)
    );
}
