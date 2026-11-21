//! `cards/artifacts/mv_3/chronatog_totem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn chronatog_totem() -> CardIndex {
    card_index("24542d66-e67a-4403-91a8-bb4231ea1f50")
}

fn activate_totem(engine: &mut Engine<RegistryLookup>, p0: PlayerId, totem: ObjectId, idx: u32) {
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: totem,
                ability_index: idx,
            },
        )
        .expect("activate Chronatog Totem");
    pass_until(engine, stack_is_empty);
}

/// Chronatog Totem (24542d66-e67a-4403-91a8-bb4231ea1f50) — {3} Artifact:
/// "{T}: Add {U}."
/// "{1}{U}: This artifact becomes a 1/2 blue Atog artifact creature until end
/// of turn."
/// "{0}: This creature gets +3/+3 until end of turn. You skip your next turn.
/// Activate only once each turn and only if this permanent is a creature."
#[test]
fn chronatog_totem_mana_animation_once_per_turn_pump_and_skip_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[chronatog_totem(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let totem = on_battlefield(&engine, p0, chronatog_totem()).unwrap();
    assert!(!types(&engine, totem).contains(TypeSet::CREATURE));
    assert!(
        !priority_offer(&engine).abilities.contains(&(totem, 2)),
        "{{0}} pump requires Chronatog Totem to be a creature"
    );

    // Animate with {1}{U} (ability 1) -> 1/2 blue Atog artifact creature.
    tap_all_mana_but(&mut engine, p0, Some(chronatog_totem()));
    activate_totem(&mut engine, p0, totem, 1);

    let chars = engine.state().object(totem).unwrap().characteristics();
    assert_eq!(pt(&engine, totem), (1, 2));
    assert!(types(&engine, totem).contains(TypeSet::ARTIFACT.union(TypeSet::CREATURE)));
    assert_eq!(
        chars.colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Blue])
    );
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::ATOG)
    );

    // Activate {0} (ability 2): +3/+3 -> 4/5, and cannot activate twice this turn.
    assert!(priority_offer(&engine).abilities.contains(&(totem, 2)));
    activate_totem(&mut engine, p0, totem, 2);
    assert_eq!(pt(&engine, totem), (4, 5), "1/2 + 3/+3 = 4/5");
    assert!(
        !priority_offer(&engine).abilities.contains(&(totem, 2)),
        "activate only once each turn"
    );

    // On p1's turn, animation wears off (noncreature, colorless).
    reach_their_main_phase(&mut engine, p1);
    let p1_first_turn = engine.state().turn.number;
    assert!(!types(&engine, totem).contains(TypeSet::CREATURE));
    assert_eq!(
        engine
            .state()
            .object(totem)
            .unwrap()
            .characteristics()
            .colors,
        baylee_core::color::ColorSet::EMPTY
    );

    // Next turn after p1's turn ALSO belongs to p1 because p0 skipped their next turn!
    pass_until(&mut engine, |e| e.state().turn.number > p1_first_turn);
    assert_eq!(
        engine.state().turn.active,
        p1,
        "p0 skipped their next turn, so p1 takes two turns in a row"
    );
    let p1_second_turn = engine.state().turn.number;
    pass_until(&mut engine, |e| e.state().turn.number > p1_second_turn);
    assert_eq!(engine.state().turn.active, p0);
}

/// {T}: Add {U} produces one blue mana.
#[test]
fn chronatog_totem_taps_for_blue_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[chronatog_totem()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let totem = on_battlefield(&engine, p0, chronatog_totem()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: totem,
                ability_index: 0,
            },
        )
        .expect("tap Chronatog Totem for {U}");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
}
