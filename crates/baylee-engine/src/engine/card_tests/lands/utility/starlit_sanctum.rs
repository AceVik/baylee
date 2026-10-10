//! `cards/lands/utility/starlit_sanctum.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// A 1/2 Phyrexian Human Cleric: power and toughness differ, and both
/// differ from the 4/5 Giant Growth makes of it.
fn priest_of_yawgmoth() -> CardIndex {
    card_index("cb6465f9-dcf8-4258-aa18-661ad252b58b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

/// A board with the Sanctum, a Plains for {W}, a Swamp for {B}, a Forest for
/// Giant Growth, a Cleric and a non-Cleric of the same controller.
fn sanctum_board() -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                starlit_sanctum(),
                plains(),
                swamp(),
                forest(),
                priest_of_yawgmoth(),
                grizzly_bears(),
            ],
        )
        .hand(0, &[giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let sanctum = on_battlefield(&engine, p0, starlit_sanctum()).expect("the Sanctum");
    let priest = on_battlefield(&engine, p0, priest_of_yawgmoth()).expect("the Cleric");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears");
    (engine, sanctum, priest, bears)
}

#[track_caller]
fn pump(engine: &mut Engine<RegistryLookup>, target: ObjectId) {
    let p0 = PlayerId::new(0);
    let forest_id = on_battlefield(engine, p0, forest()).expect("a Forest");
    tap_mana_where(engine, p0, |id| id == forest_id);
    cast_with_floating(engine, p0, giant_growth());
    aim_at(engine, p0, target);
    pass_until(engine, stack_is_empty);
}

/// Answers the activation's questions: the target player when asked, the
/// creature to sacrifice when asked. Returns the sacrifice options seen.
#[track_caller]
fn finish_activation(
    engine: &mut Engine<RegistryLookup>,
    victim: PlayerId,
    sacrifice: ObjectId,
) -> Vec<ObjectId> {
    let p0 = PlayerId::new(0);
    let mut offered = Vec::new();
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::ChooseTargets { player_options, .. } => {
                assert!(player_options.contains(&victim), "{player_options:?}");
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![victim],
                        },
                    )
                    .expect("name the player");
            }
            Pending::ChooseCards { options, .. } => {
                offered.clone_from(&options);
                assert!(options.contains(&sacrifice), "{options:?}");
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![sacrifice],
                        },
                    )
                    .expect("sacrifice it");
            }
            _ => break,
        }
    }
    pass_until(engine, stack_is_empty);
    offered
}

/// "{W}, {T}, Sacrifice a Cleric creature: You gain life equal to the
/// sacrificed creature's toughness." The Priest (1/2) is a 4/5 after Giant
/// Growth, so 5 is the toughness it last had, not 2 (printed) or 4 (power).
/// A Grizzly Bears of the same controller is not a Cleric and is not on the
/// offer.
#[test]
fn starlit_sanctum_white_gains_the_pumped_toughness_of_a_cleric() {
    let p0 = PlayerId::new(0);
    let (mut engine, sanctum, priest, bears) = sanctum_board();
    assert_eq!(pt(&engine, priest), (1, 2), "the printed body");
    pump(&mut engine, priest);
    assert_eq!(pt(&engine, priest), (4, 5), "Giant Growth: +3/+3");

    let plains_id = on_battlefield(&engine, p0, plains()).expect("a Plains");
    tap_mana_where(&mut engine, p0, |id| id == plains_id);
    let life = engine.state().players[0].life;
    activate(&mut engine, p0, starlit_sanctum(), 1);
    let offered = finish_activation(&mut engine, p0, priest);
    assert!(
        !offered.contains(&bears),
        "a Bear is no Cleric: {offered:?}"
    );
    assert!(is_tapped(&engine, sanctum), "{{T}} is part of the cost");
    assert!(in_graveyard(&engine, p0, priest_of_yawgmoth()).is_some());
    assert_eq!(engine.state().players[0].life, life + 5);
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the opponent is untouched"
    );
}

/// "{B}, {T}, Sacrifice a Cleric creature: Target player loses life equal to
/// the sacrificed creature's power." The Priest is a 4/5 when sacrificed, so
/// the targeted opponent loses 4 (not printed 1, not toughness 5) and the
/// Sanctum's controller loses and gains nothing.
#[test]
fn starlit_sanctum_black_makes_the_target_lose_the_pumped_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, sanctum, priest, _bears) = sanctum_board();
    pump(&mut engine, priest);
    assert_eq!(pt(&engine, priest), (4, 5));

    let swamp_id = on_battlefield(&engine, p0, swamp()).expect("a Swamp");
    tap_mana_where(&mut engine, p0, |id| id == swamp_id);
    let mine = engine.state().players[0].life;
    activate(&mut engine, p0, starlit_sanctum(), 2);
    finish_activation(&mut engine, p1, priest);
    assert!(is_tapped(&engine, sanctum));
    assert!(in_graveyard(&engine, p0, priest_of_yawgmoth()).is_some());
    assert_eq!(
        engine.state().players[1].life,
        20 - 4,
        "power, as it last was"
    );
    assert_eq!(
        engine.state().players[0].life,
        mine,
        "no life for the Sanctum's owner"
    );
}

/// The target is any player: aimed at its own controller, the drain lands there.
#[test]
fn starlit_sanctum_black_may_target_its_own_controller() {
    let p0 = PlayerId::new(0);
    let (mut engine, _sanctum, priest, _bears) = sanctum_board();
    let swamp_id = on_battlefield(&engine, p0, swamp()).expect("a Swamp");
    tap_mana_where(&mut engine, p0, |id| id == swamp_id);
    activate(&mut engine, p0, starlit_sanctum(), 2);
    finish_activation(&mut engine, p0, priest);
    assert_eq!(
        engine.state().players[0].life,
        19,
        "the Priest's unpumped power 1"
    );
    assert_eq!(engine.state().players[1].life, 20);
}

/// A +1/+1 counter counts in both numbers: the Priest is a 2/3 when sacrificed.
#[test]
fn starlit_sanctum_counts_a_plus_one_counter() {
    let p0 = PlayerId::new(0);
    let (mut engine, _sanctum, priest, _bears) = sanctum_board();
    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    crate::replacement::put_counters(state, priest, baylee_cards_dsl::CounterKind::P1P1, 1);
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    assert_eq!(pt(&engine, priest), (2, 3), "a +1/+1 counter");

    let plains_id = on_battlefield(&engine, p0, plains()).expect("a Plains");
    tap_mana_where(&mut engine, p0, |id| id == plains_id);
    let life = engine.state().players[0].life;
    activate(&mut engine, p0, starlit_sanctum(), 1);
    finish_activation(&mut engine, p0, priest);
    assert_eq!(engine.state().players[0].life, life + 3);
}

/// With no Cleric to sacrifice neither sacrifice line is offered, though {W},
/// {B} and a creature are all there.
#[test]
fn starlit_sanctum_refuses_a_creature_that_is_not_a_cleric() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[starlit_sanctum(), plains(), swamp(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let sanctum = on_battlefield(&engine, p0, starlit_sanctum()).expect("the Sanctum");
    tap_mana_except(&mut engine, p0, sanctum);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "{{W}} and {{B}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("priority expected")
    };
    assert!(legal.abilities.contains(&(sanctum, 0)), "{{T}}: Add {{C}}");
    assert!(
        !legal.abilities.contains(&(sanctum, 1)),
        "{{W}} line needs a Cleric"
    );
    assert!(
        !legal.abilities.contains(&(sanctum, 2)),
        "{{B}} line needs a Cleric"
    );
}

/// "{T}: Add {C}."
#[test]
fn starlit_sanctum_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let (mut engine, sanctum, _priest, _bears) = sanctum_board();
    activate(&mut engine, p0, starlit_sanctum(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, sanctum));
}
