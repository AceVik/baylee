//! `cards/creatures/mv_3/shieldmage_advocate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn shieldmage_advocate() -> CardIndex {
    card_index("57188b3d-567c-4dff-8b93-7cc7a47894be")
}

/// Shieldmage Advocate returns target card from an opponent's graveyard to their hand
/// and prevents ALL damage that would be dealt to any target this turn by a chosen source,
/// leaving damage from unchosen sources unprevented.
#[test]
fn shieldmage_advocate_returns_opponent_graveyard_card_and_prevents_all_source_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[shieldmage_advocate()])
        .battlefield(
            1,
            &[prodigal_sorcerer(), savannah_lions(), island(), forest()],
        )
        .hand(1, &[index::TWIDDLE])
        .start();
    keep_mulligans(&mut engine);
    let p1_forest = on_battlefield(&engine, p1, forest()).unwrap();
    bury(&mut engine, &[p1_forest]);
    reach_their_main_phase(&mut engine, p1);

    let sorcerer = on_battlefield(&engine, p1, prodigal_sorcerer()).unwrap();
    let lions = on_battlefield(&engine, p1, savannah_lions()).unwrap();
    let gy_forest = in_graveyard(&engine, p1, forest()).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(lions, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    activate(&mut engine, p0, shieldmage_advocate(), 0);
    for _ in 0..4 {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                if options.contains(&gy_forest) {
                    engine
                        .apply(
                            player,
                            PlayerAction::ChooseTargets {
                                objects: vec![gy_forest],
                                players: vec![],
                            },
                        )
                        .unwrap();
                } else {
                    engine
                        .apply(
                            player,
                            PlayerAction::ChooseTargets {
                                objects: vec![],
                                players: vec![p0],
                            },
                        )
                        .unwrap();
                }
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    let sorcerer_source = options
        .iter()
        .copied()
        .find(|s| s.object == sorcerer)
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseDamageSource {
                choice,
                source: sorcerer_source,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(in_hand(&engine, p1, forest()).is_some());
    assert!(in_graveyard(&engine, p1, forest()).is_none());
    assert_eq!(
        lk::life(&engine, p0),
        18,
        "damage from unchosen Savannah Lions (2 power) was NOT prevented"
    );

    // First damage instance from the chosen Prodigal Sorcerer this turn is prevented.
    activate(&mut engine, p1, prodigal_sorcerer(), 0);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| stack_is_empty(e) && at_rest(e, p1));
    assert_eq!(
        lk::life(&engine, p0),
        18,
        "first damage from chosen Prodigal Sorcerer was prevented"
    );

    // Untap Prodigal Sorcerer and deal damage from it a SECOND time this turn:
    // "prevent ALL damage ... this turn" must still prevent the second instance.
    let isl = on_battlefield(&engine, p1, island()).unwrap();
    tap_mana_where(&mut engine, p1, |id| id == isl);
    cast_with_floating(&mut engine, p1, index::TWIDDLE);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![sorcerer],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| stack_is_empty(e) && at_rest(e, p1));

    activate(&mut engine, p1, prodigal_sorcerer(), 0);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| stack_is_empty(e) && at_rest(e, p1));
    assert_eq!(
        lk::life(&engine, p0),
        18,
        "second damage instance this turn from chosen source was also prevented"
    );
}
