//! `cards/creatures/mv_4/painbringer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn painbringer() -> CardIndex {
    card_index("6ca49880-4e12-494b-b14e-e49385d0f7f5")
}

fn activate_painbringer(
    engine: &mut Engine<RegistryLookup>,
    p0: PlayerId,
    target: ObjectId,
    exile: &[ObjectId],
) {
    activate(engine, p0, painbringer(), 0);
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::ChooseTargets { player, .. } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![target],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseNumber { player, .. } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseNumber(u32::try_from(exile.len()).unwrap()),
                    )
                    .unwrap();
            }
            Pending::ChooseCards { player, .. } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: exile.to_vec(),
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    pass_until(engine, |e| at_rest(e, p0));
}

/// Painbringer taps and exiles X cards from your graveyard to give target creature
/// -X/-X until end of turn, shrinking or killing it cleanly and wearing off at cleanup.
#[test]
fn painbringer_exiles_x_graveyard_cards_for_minus_x_minus_x() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[painbringer(), swamp(), swamp(), swamp()])
        .battlefield(1, &[grizzly_bears(), pearled_unicorn()])
        .start();
    keep_mulligans(&mut engine);
    let sws = all_on_battlefield(&engine, p0, swamp());
    bury(&mut engine, &sws);
    reach_main_phase(&mut engine, p0);

    let gy_cards = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .clone();
    let unicorn = on_battlefield(&engine, p1, pearled_unicorn()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    activate_painbringer(&mut engine, p0, unicorn, &[gy_cards[0]]);
    assert_eq!(pt(&engine, unicorn), (1, 1));
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&gy_cards[0])
    );

    // Advance cleanly to P1's main phase then P0's turn 3.
    loop {
        if e_in_main(&engine, p1) {
            break;
        }
        step_one(&mut engine);
    }
    assert_eq!(pt(&engine, unicorn), (2, 2));

    loop {
        if e_in_main(&engine, p0) && engine.state().turn.number >= 3 {
            break;
        }
        step_one(&mut engine);
    }
    activate_painbringer(&mut engine, p0, bears, &[gy_cards[1], gy_cards[2]]);
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(p0)).len(), 3);
}

fn e_in_main(engine: &Engine<RegistryLookup>, seat: PlayerId) -> bool {
    engine.state().turn.active == seat
        && engine.state().turn.phase == Phase::FirstMain
        && at_rest(engine, seat)
}

fn step_one(engine: &mut Engine<RegistryLookup>) {
    match engine.pending().clone() {
        Pending::Priority { player, .. } => {
            engine.apply(player, PlayerAction::PassPriority).unwrap();
        }
        Pending::ChooseAttackers { player, .. } => engine
            .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
            .unwrap(),
        Pending::ChooseBlockers { player, .. } => engine
            .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
            .unwrap(),
        other => panic!("unexpected step: {other:?}"),
    }
}
