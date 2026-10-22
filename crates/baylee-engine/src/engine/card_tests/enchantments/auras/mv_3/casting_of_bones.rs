//! `cards/enchantments/auras/mv_3/casting_of_bones.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn casting_of_bones() -> CardIndex {
    card_index("5a747256-4215-4334-98ab-0c2e4ed92e47")
}

/// `Casting of Bones` ({2}{B}, Enchantment — Aura, `Coverage::Partial`)
/// attaches to target creature and when enchanted creature dies, draws three
/// cards and then discards a card.
#[test]
fn casting_of_bones_draws_three_and_discards_one_when_enchanted_creature_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), mountain(), grizzly_bears()])
        .hand(0, &[casting_of_bones(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, casting_of_bones());
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

    // Destroy Grizzly Bears with Lightning Bolt.
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options.len(), 3);
    let discard = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![discard],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(discard).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 2);
}
