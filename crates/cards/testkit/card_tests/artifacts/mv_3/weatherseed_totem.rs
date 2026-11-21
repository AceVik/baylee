//! `cards/artifacts/mv_3/weatherseed_totem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn weatherseed_totem() -> CardIndex {
    card_index("5d19c68d-6eb8-4b34-9f31-c26931e59670")
}

fn control_magic() -> CardIndex {
    card_index("cd0d7141-46d2-4aa3-bc77-6b3b4513803e")
}

fn life(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

/// Weatherseed Totem (5d19c68d-6eb8-4b34-9f31-c26931e59670) — {3} Artifact:
/// "{T}: Add {G}."
/// "{2}{G}{G}{G}: This artifact becomes a 5/3 green Treefolk artifact creature
/// with trample until end of turn."
/// "When this artifact is put into a graveyard from the battlefield, if it was
/// a creature, return this card to its owner's hand."
#[test]
fn weatherseed_totem_animates_tramples_and_returns_to_hand_when_killed_as_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                weatherseed_totem(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let totem = on_battlefield(&engine, p0, weatherseed_totem()).unwrap();
    tap_all_mana_but(&mut engine, p0, Some(weatherseed_totem()));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: totem,
                ability_index: 1,
            },
        )
        .expect("animate Weatherseed Totem");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, totem), (5, 3));
    assert!(keywords(&engine, totem).contains(KeywordSet::TRAMPLE));
    assert_eq!(
        engine
            .state()
            .object(totem)
            .unwrap()
            .characteristics()
            .colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Green])
    );
    assert!(
        engine
            .state()
            .object(totem)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::TREEFOLK)
    );

    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(totem, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, totem)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        life(&engine, p1),
        18,
        "5/3 trample dealt 2 excess damage over 3/3"
    );
    assert!(in_hand(&engine, p0, weatherseed_totem()).is_some());
    assert!(in_graveyard(&engine, p0, weatherseed_totem()).is_none());
}

/// Tapping adds {G}, and dying while NOT a creature leaves Weatherseed Totem in
/// the graveyard.
#[test]
fn weatherseed_totem_taps_for_green_and_noncreature_death_stays_in_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[weatherseed_totem(), plains(), plains()])
        .hand(0, &[disenchant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let totem = on_battlefield(&engine, p0, weatherseed_totem()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: totem,
                ability_index: 0,
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );

    tap_all_mana(&mut engine, p0);
    let dis = in_hand(&engine, p0, disenchant()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: dis })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![totem],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, weatherseed_totem()).is_some());
    assert!(in_hand(&engine, p0, weatherseed_totem()).is_none());
}

/// Dying as a creature while controlled by an opponent still returns Weatherseed
/// Totem to its owner's hand.
#[test]
fn weatherseed_totem_stolen_creature_death_returns_to_owners_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                weatherseed_totem(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[island(), island(), island(), island(), mountain()])
        .hand(1, &[control_magic(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    let totem = on_battlefield(&engine, p0, weatherseed_totem()).unwrap();
    tap_all_mana_but(&mut engine, p0, Some(weatherseed_totem()));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: totem,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    tap_all_mana(&mut engine, p1);
    let cm = in_hand(&engine, p1, control_magic()).unwrap();
    engine
        .apply(p1, PlayerAction::CastSpell { card: cm })
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![totem],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(totem).unwrap().controller, p1);

    let bolt = in_hand(&engine, p1, lightning_bolt()).unwrap();
    engine
        .apply(p1, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![totem],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, weatherseed_totem()).is_some(),
        "Weatherseed Totem returns to its owner's (p0) hand"
    );
    assert!(in_hand(&engine, p1, weatherseed_totem()).is_none());
}
