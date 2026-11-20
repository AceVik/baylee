//! `cards/sorceries/mv_1/illuminate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn illuminate() -> CardIndex {
    card_index("f944c245-08ff-41a8-bdfc-7d21a6a63eae")
}

fn hand_size(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

/// Casts Illuminate for `x` off floating mana, choosing dual kickers `{2}{R}`
/// (0) and `{3}{U}` (1), aimed at `target_creature`.
fn cast_illuminate(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    x: u32,
    kick_red: bool,
    kick_blue: bool,
    target_creature: ObjectId,
) {
    cast_with_floating(engine, seat, illuminate());
    let mut announced = false;
    let mut targeted = false;
    let mut saw_0 = false;
    let mut saw_1 = false;
    for _ in 0..16 {
        if announced && targeted && saw_0 && saw_1 {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseNumber {
                player, min, max, ..
            } => {
                assert_eq!(player, seat);
                assert!((min..=max).contains(&x));
                engine.apply(seat, PlayerAction::ChooseNumber(x)).unwrap();
                announced = true;
            }
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::KickerOption { index, cost, .. },
                ..
            } => {
                assert_eq!(player, seat);
                match index {
                    0 => {
                        assert!(!saw_0);
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{2}{R}"));
                        saw_0 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_red)).unwrap();
                    }
                    1 => {
                        assert!(saw_0);
                        assert!(!saw_1);
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{3}{U}"));
                        saw_1 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_blue)).unwrap();
                    }
                    other => panic!("unexpected kicker index {other}"),
                }
            }
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                ..
            } => {
                assert_eq!(player, seat);
                assert!(
                    player_options.is_empty(),
                    "Illuminate targets only a creature, never a player"
                );
                assert!(options.contains(&target_creature));
                engine
                    .apply(
                        seat,
                        PlayerAction::ChooseObjects {
                            objects: vec![target_creature],
                        },
                    )
                    .unwrap();
                targeted = true;
            }
            _ => break,
        }
    }
    assert!(
        announced && targeted && saw_0 && saw_1,
        "Illuminate asked X, targets, and both kickers; pending={:?}",
        engine.pending()
    );
}

#[test]
fn illuminate_unkicked_damages_creature_without_controller_damage_or_draws() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[illuminate()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let hand_before = hand_size(&engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_illuminate(&mut engine, p0, 2, false, false, bear);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(hand_size(&engine, p0), hand_before - 1);
}

#[test]
fn illuminate_first_kicker_damages_creature_and_its_controller_whether_creature_survives_or_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    // Lethal 2 damage to 2/2 Grizzly Bears also deals 2 to p1.
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[illuminate()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let hand_before = hand_size(&engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_illuminate(&mut engine, p0, 2, true, false, bear);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(hand_size(&engine, p0), hand_before - 1);

    // Nonlethal 2 damage to 3/3 Hill Giant also deals 2 to p1.
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[illuminate()])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_illuminate(&mut engine, p0, 2, true, false, giant);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(on_battlefield(&engine, p1, index::HILL_GIANT).is_some());
    assert_eq!(engine.state().players[1].life, 18);
}

#[test]
fn illuminate_second_kicker_draws_x_without_controller_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[illuminate()])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    let hand_before = hand_size(&engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_illuminate(&mut engine, p0, 2, false, true, giant);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(engine.state().object(giant).unwrap().damage, 2);
    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(hand_size(&engine, p0), hand_before - 1 + 2);
}

#[test]
fn illuminate_both_kickers_deal_x_to_creature_and_controller_and_draw_x() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[illuminate()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let hand_before = hand_size(&engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_illuminate(&mut engine, p0, 2, true, true, bear);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(hand_size(&engine, p0), hand_before - 1 + 2);
}
