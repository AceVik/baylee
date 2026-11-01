//! `cards/instants/mv_5/contagion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn contagion() -> CardIndex {
    card_index("433e86ce-df04-4182-bb63-ad2607d67dbd")
}

fn drudge_skeletons() -> CardIndex {
    card_index("c180ed02-07f4-4538-9bea-8c249234a8e2")
}

const MINUS: CounterKind = CounterKind::Minus {
    power: 2,
    toughness: 1,
};

/// Casts Contagion from `p0`'s hand at `target`, answering what is asked: the cast mode (the
/// alternative cost when `alternative`, the printed one otherwise) and the card to exile for it.
/// Returns the cards the exile question offered (empty when none was asked).
fn cast_contagion(
    engine: &mut Engine<RegistryLookup>,
    p0: PlayerId,
    target: ObjectId,
    alternative: bool,
    pitch: ObjectId,
) -> Vec<ObjectId> {
    let spell = in_hand(engine, p0, contagion()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    let mut offered = Vec::new();
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseCastMode { options, .. } => {
                let slot = options
                    .iter()
                    .position(|o| {
                        if alternative {
                            matches!(o.kind, CastModeKind::Alternative(_))
                        } else {
                            matches!(o.kind, CastModeKind::Normal)
                        }
                    })
                    .expect("the cast mode is on offer");
                engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
            }
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![target],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards { options, .. } => {
                offered = options;
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
            other => panic!("unexpected pending during Contagion's cast: {other:?}"),
        }
    }
    offered
}

/// `Contagion` ({3}{B}{B}, Instant, `Coverage::Partial`): "You may pay 1 life and exile a black
/// card from your hand rather than pay this spell's mana cost." Only a black card is offered, only
/// that one is exiled, and one life is paid. Both counters go on the one target (a legal way to
/// distribute them); nothing else is touched.
#[test]
fn contagion_pitches_a_black_card_and_one_life_for_two_counters_on_the_target() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(1, &[serra_angel(), grizzly_bears()])
        .hand(0, &[contagion(), drudge_skeletons(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let angel = on_battlefield(&engine, p1, serra_angel()).unwrap();
    let their_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let pitch = in_hand(&engine, p0, drudge_skeletons()).unwrap();
    let green = in_hand(&engine, p0, grizzly_bears()).unwrap();

    let offered = cast_contagion(&mut engine, p0, angel, true, pitch);
    assert!(offered.contains(&pitch), "a black card is offered");
    assert!(!offered.contains(&green), "a green card is not");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 19, "one life paid");
    assert_eq!(engine.state().object(pitch).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().object(green).unwrap().zone, Zone::Hand);
    assert_eq!(counters_on(&engine, angel, MINUS), 2);
    assert_eq!(pt(&engine, angel), (0, 2));
    assert_eq!(pt(&engine, their_bears), (2, 2), "only the target");
    assert_eq!(counters_on(&engine, their_bears, MINUS), 0);
}

/// "May": paying the printed mana cost costs no life and exiles nothing.
#[test]
fn contagion_paid_with_mana_costs_no_life_and_keeps_the_black_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 5])
        .battlefield(1, &[serra_angel()])
        .hand(0, &[contagion(), drudge_skeletons()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let angel = on_battlefield(&engine, p1, serra_angel()).unwrap();
    let kept = in_hand(&engine, p0, drudge_skeletons()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_contagion(&mut engine, p0, angel, false, kept);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().object(kept).unwrap().zone, Zone::Hand);
    assert_eq!(counters_on(&engine, angel, MINUS), 2);
}

/// Without a black card to exile and without the mana, Contagion cannot be cast for free.
#[test]
fn contagion_cannot_be_cast_for_free_without_a_black_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(1, &[serra_angel()])
        .hand(0, &[contagion(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spell = in_hand(&engine, p0, contagion()).unwrap();
    let result = engine.apply(p0, PlayerAction::CastSpell { card: spell });
    let gets_free_cast = match result {
        Err(_) => false,
        Ok(()) => matches!(
            engine.pending(),
            Pending::ChooseCastMode { options, .. }
                if options.iter().any(|o| matches!(o.kind, CastModeKind::Alternative(_)))
        ),
    };
    assert!(!gets_free_cast, "a green card is not a black card");
    assert_eq!(engine.state().players[0].life, 20);
}
