//! `cards/artifacts/mv_0/dark_sphere.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn dark_sphere() -> CardIndex {
    card_index("397f53f7-f801-4442-a778-2f26ac246b62")
}

fn bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

fn stack_ids(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    engine.state().zones.list(ZoneLocation::Stack).clone()
}

/// Casts a Lightning Bolt off floating mana at `victim` and returns the
/// spell's object.
#[track_caller]
fn bolt_at(engine: &mut Engine<RegistryLookup>, seat: PlayerId, victim: PlayerId) -> ObjectId {
    let before = stack_ids(engine);
    cast_with_floating(engine, seat, bolt());
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![victim],
            },
        )
        .expect("a player is a legal target for the Bolt");
    stack_ids(engine)
        .into_iter()
        .find(|id| !before.contains(id))
        .expect("the Bolt is on the stack")
}

/// Answers the Sphere's "source of your choice" with `source`.
#[track_caller]
fn choose_source(engine: &mut Engine<RegistryLookup>, seat: PlayerId, source: ObjectId) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        player,
        choice,
        options,
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the source choice")
    };
    assert_eq!(player, seat, "the Sphere's controller chooses the source");
    let chosen = options
        .iter()
        .copied()
        .find(|candidate| candidate.object == source)
        .expect("the Bolt is a source that can be chosen");
    engine
        .apply(
            seat,
            PlayerAction::ChooseDamageSource {
                choice,
                source: chosen,
            },
        )
        .expect("the choice came off the menu");
}

fn life(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[seat.get() as usize].life
}

/// Dark Sphere (397f53f7-f801-4442-a778-2f26ac246b62) — "{T}, Sacrifice this
/// artifact: The next time a source of your choice would deal damage to you
/// this turn, prevent half that damage, rounded down."
///
/// p0 Bolts themselves (3), answers with the Sphere in response and names the
/// Bolt: half of 3 rounded down is 1 prevented, so 2 is dealt.
#[test]
fn dark_sphere_prevents_half_of_the_chosen_sources_damage_rounded_down() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dark_sphere(), mountain()])
        .hand(0, &[bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    let spell = bolt_at(&mut engine, p0, p0);
    assert_eq!(life(&engine, p0), 20);

    activate(&mut engine, p0, dark_sphere(), 0);
    assert!(
        on_battlefield(&engine, p0, dark_sphere()).is_none(),
        "tapping and sacrificing are the price: the Sphere is gone before it resolves"
    );
    assert!(in_graveyard(&engine, p0, dark_sphere()).is_some());
    choose_source(&mut engine, p0, spell);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p0), 18, "3 damage, 1 prevented");
}

/// "A source of your choice": the shield waits for the named source only. Two
/// Bolts are on the stack, the Sphere names the lower one; the upper one
/// resolves first and is dealt in full, then the named one is halved.
#[test]
fn dark_sphere_leaves_other_sources_alone() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dark_sphere(), mountain(), mountain()])
        .hand(0, &[bolt(), bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    let named = bolt_at(&mut engine, p0, p0);
    let other = bolt_at(&mut engine, p0, p0);
    assert_ne!(named, other);

    activate(&mut engine, p0, dark_sphere(), 0);
    choose_source(&mut engine, p0, named);
    pass_until(&mut engine, |e| life(e, p0) < 20);
    assert_eq!(
        life(&engine, p0),
        17,
        "the Bolt cast last resolves first, and it is not the named source"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p0), 15, "the named Bolt: 3 less 1 prevented");
}

/// "Damage to you": the shield guards the Sphere's controller. A named Bolt
/// aimed at the opponent is dealt in full.
#[test]
fn dark_sphere_does_not_shield_the_opponent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dark_sphere(), mountain()])
        .hand(0, &[bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    let spell = bolt_at(&mut engine, p0, p1);
    activate(&mut engine, p0, dark_sphere(), 0);
    choose_source(&mut engine, p0, spell);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p1), 17, "all 3 reached the opponent");
    assert_eq!(life(&engine, p0), 20);
}
