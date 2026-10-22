//! `cards/instants/mv_5/contagion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn contagion() -> CardIndex {
    card_index("433e86ce-df04-4182-bb63-ad2607d67dbd")
}

fn drudge_skeletons() -> CardIndex {
    card_index("c180ed02-07f4-4538-9bea-8c249234a8e2")
}

/// `Contagion` ({3}{B}{B}, Instant, `Coverage::Partial`) may be cast by paying
/// 1 life and exiling a black card from your hand rather than paying its mana
/// cost, and puts two -2/-1 counters on target creature.
#[test]
fn contagion_pitches_black_card_and_one_life_to_put_two_m2m1_counters_on_target_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(1, &[serra_angel()])
        .hand(0, &[contagion(), drudge_skeletons()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let angel = on_battlefield(&engine, p1, serra_angel()).unwrap();
    let pitch = in_hand(&engine, p0, drudge_skeletons()).unwrap();
    let spell = in_hand(&engine, p0, contagion()).unwrap();

    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseCastMode { options, .. } => {
                let slot = options
                    .iter()
                    .position(|o| matches!(o.kind, CastModeKind::Alternative(_)))
                    .unwrap();
                engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
            }
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![angel],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards { .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![pitch],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending during Contagion pitch cast: {other:?}"),
        }
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(engine.state().object(pitch).unwrap().zone, Zone::Exile);
    assert_eq!(
        counters_on(
            &engine,
            angel,
            CounterKind::Minus {
                power: 2,
                toughness: 1
            }
        ),
        2
    );
    assert_eq!(pt(&engine, angel), (0, 2));
}
