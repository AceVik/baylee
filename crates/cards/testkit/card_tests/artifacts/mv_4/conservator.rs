//! `cards/artifacts/mv_4/conservator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Conservator: "{3}, {T}: Prevent the next 2 damage that would be dealt to
/// you this turn." A shield of exactly 2, read off a burn spell for 3: one
/// point gets through, proving the shield absorbs its printed amount and
/// nothing more (`prevention::ShieldKind::Next`).
#[test]
fn conservator_prevents_the_next_2_damage_dealt_to_its_controller() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[conservator(), mountain(), forest(), forest(), forest()],
        )
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let red = on_battlefield(&engine, p0, mountain()).expect("the Mountain is out");
    tap_mana_except(&mut engine, p0, red);
    activate(&mut engine, p0, conservator(), 0);
    pass_until(&mut engine, stack_is_empty);

    let before = life_of(&engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("p0 is any target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life_of(&engine, p0),
        before - 1,
        "3 dealt, 2 prevented by the shield"
    );
}

/// Conservator: "{3}, {T}: Prevent the next 2 damage that would be dealt to
/// you this turn." The cost taps it; the shield covers its controller and
/// not the opponent (a Bolt at them is dealt in full); it is spent by the
/// first Bolt at its controller (1 of 3 gets through) and stops there: the
/// next Bolt is dealt in full.
#[test]
fn conservator_taps_covers_only_its_controller_and_is_spent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                conservator(),
                forest(),
                forest(),
                forest(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[lightning_bolt(), lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forests = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| forests.contains(&id));
    let rod = on_battlefield(&engine, p0, conservator()).expect("Conservator");
    assert!(!is_tapped(&engine, rod));
    activate(&mut engine, p0, conservator(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, rod), "{{T}} is part of the cost");

    tap_all_mana(&mut engine, p0);
    let bolt_at = |engine: &mut Engine<RegistryLookup>, target: PlayerId| {
        cast_with_floating(engine, p0, lightning_bolt());
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![target],
                },
            )
            .expect("a player is any target");
        pass_until(engine, stack_is_empty);
    };
    bolt_at(&mut engine, p1);
    assert_eq!(
        life_of(&engine, p1),
        17,
        "\"dealt to you\": the opponent's damage is not prevented"
    );
    assert_eq!(life_of(&engine, p0), 20, "and the shield was not touched");
    bolt_at(&mut engine, p0);
    assert_eq!(life_of(&engine, p0), 19, "3 dealt, 2 prevented");
    bolt_at(&mut engine, p0);
    assert_eq!(
        life_of(&engine, p0),
        16,
        "the shield is spent: the next Bolt is dealt in full"
    );
}

/// Conservator: "… this turn". A shield raised in its controller's own turn
/// with nothing dealt is gone when the opponent's turn comes, and their
/// Bolt is dealt in full.
#[test]
fn conservators_shield_ends_with_the_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[conservator(), forest(), forest(), forest()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, conservator(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !engine.state().shields.is_empty(),
        "the shield stands in the turn that made it"
    );

    reach_their_main_phase(&mut engine, p1);
    assert!(engine.state().shields.is_empty(), "gone with that turn");
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("a player is any target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), 17, "dealt in full");
}
