//! `cards/instants/mv_3/blood_of_the_martyr.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn blood_of_the_martyr() -> CardIndex {
    card_index("0e69e6c2-fdc5-4c1a-bd96-c4283b82743a")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// p0 casts Blood of the Martyr, then a Lightning Bolt at `target_card`'s
/// controller's creature, and stops at the question the Blood asks.
///
/// Returns the Bolt's target.
#[track_caller]
fn bolt_a_creature_under_blood(
    target_seat: PlayerId,
    target_card: CardIndex,
) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), mountain(), grizzly_bears()],
        )
        .hand(0, &[blood_of_the_martyr(), lightning_bolt()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let target = on_battlefield(&engine, target_seat, target_card).expect("the target is out");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, blood_of_the_martyr());
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, target);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("the pass stopped on a yes/no question")
    };
    assert_eq!(player, p0, "the caster of Blood decides whether to take it");
    (engine, target)
}

/// Blood of the Martyr (0e69e6c2-fdc5-4c1a-bd96-c4283b82743a) — "Until end
/// of turn, if damage would be dealt to any creature, you may have that
/// damage dealt to you instead."
///
/// Taken, the damage a Bolt would deal to my Bears is dealt to me.
#[test]
fn blood_of_the_martyr_redirects_damage_to_its_caster_when_taken() {
    let p0 = PlayerId::new(0);
    let (mut engine, bears) = bolt_a_creature_under_blood(p0, grizzly_bears());
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("the redirect is taken");
    pass_until(&mut engine, stack_is_empty);

    let object = engine
        .state()
        .object(bears)
        .expect("the Bears are still there");
    assert_eq!(
        object.zone,
        Zone::Battlefield,
        "no damage reached the Bears"
    );
    assert_eq!(object.damage, 0);
    assert_eq!(engine.state().players[0].life, 17, "I took the 3 instead");
}

/// "May": declined, the creature takes its damage as usual.
#[test]
fn blood_of_the_martyr_lets_the_damage_land_when_declined() {
    let p0 = PlayerId::new(0);
    let (mut engine, bears) = bolt_a_creature_under_blood(p0, grizzly_bears());
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("the redirect is declined");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, grizzly_bears()).is_some(),
        "3 damage kills the Bears"
    );
    assert_eq!(
        engine.state().object(bears).map(|o| o.zone),
        Some(Zone::Graveyard)
    );
    assert_eq!(engine.state().players[0].life, 20, "and I lose no life");
}

/// "Any creature": the opponent's Giant is saved from a Bolt the same way,
/// and I pay for it.
#[test]
fn blood_of_the_martyr_covers_an_opponents_creature_too() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, giant) = bolt_a_creature_under_blood(p1, hill_giant());
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("the redirect is taken");
    pass_until(&mut engine, stack_is_empty);

    let object = engine
        .state()
        .object(giant)
        .expect("the Giant is still there");
    assert_eq!(object.zone, Zone::Battlefield, "the 3/3 survives a Bolt");
    assert_eq!(object.damage, 0);
    assert_eq!(engine.state().players[0].life, 17, "I took the 3");
}
