//! Coral Reef — `{U}{U}` Enchantment (Homelands)
//!
//! Coral Reef enters with four polyp counters on it.
//! Sacrifice an Island: Put two polyp counters on Coral Reef.
//! `{U}`, Tap an untapped blue creature you control, Remove a polyp counter
//! from Coral Reef: Put a +0/+1 counter on target creature.
//!
//! (`Coverage::Partial`: polyp counter type is not yet in the
//! `CounterKind::Custom` registry; the sacrifice-an-Island ability resolves
//! with no effect, and the `{U}`, tap an untapped blue creature you control
//! ability puts a +0/+1 counter on target creature without removing a polyp
//! counter.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn coral_reef() -> CardIndex {
    card_index("37a40a53-2668-4521-a786-c5747eedeeb2")
}

fn merfolk_of_the_pearl_trident() -> CardIndex {
    card_index("218d9277-c179-4de3-9c7f-79b5a6d4fa38")
}

#[test]
fn coral_reef_taps_blue_creature_to_put_plus_zero_plus_one_counter_on_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[coral_reef()])
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                merfolk_of_the_pearl_trident(), // blue creature to tap
                grizzly_bears(),                // target for +0/+1 counter
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let merfolk = on_battlefield(&engine, p0, merfolk_of_the_pearl_trident()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let reef_hand = in_hand(&engine, p0, coral_reef()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: reef_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let reef = on_battlefield(&engine, p0, coral_reef()).unwrap();

    // Activate ability 1 ({U}, Tap an untapped blue creature you control): target Grizzly Bears.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: reef,
                ability_index: 1,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards for TapOther");
    };
    assert!(
        options.contains(&merfolk),
        "untapped blue creature is offered"
    );
    assert!(
        !options.contains(&bears),
        "green creature is not a legal blue creature to tap"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![merfolk],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, merfolk));
    assert_eq!(pt(&engine, bears), (2, 3));
}
