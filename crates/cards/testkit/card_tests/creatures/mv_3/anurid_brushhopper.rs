//! `cards/creatures/mv_3/anurid_brushhopper.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn anurid_brushhopper() -> CardIndex {
    card_index("bdecf59c-f8bd-4003-9058-685b70c08837")
}

/// Anurid Brushhopper cannot be activated when you have fewer than two cards in hand.
#[test]
fn anurid_brushhopper_requires_two_cards_in_hand() {
    let e = lk::game(SEED, &[anurid_brushhopper()], &[], &[forest()], &[]);
    assert!(
        !lk::offered(&e, anurid_brushhopper(), lk::P0, 0),
        "Anurid Brushhopper ability must not be offered with only 1 card in hand"
    );
}

/// Anurid Brushhopper discards two cards to exile itself and returns to the
/// battlefield under its owner's control at the beginning of the next end step.
#[test]
fn anurid_brushhopper_discards_two_to_exile_until_next_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[anurid_brushhopper()])
        .hand(0, &[forest(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hopper = on_battlefield(&engine, p0, anurid_brushhopper()).unwrap();
    let f = in_hand(&engine, p0, forest()).unwrap();
    let p = in_hand(&engine, p0, plains()).unwrap();

    activate(&mut engine, p0, anurid_brushhopper(), 0);
    for _ in 0..4 {
        match engine.pending().clone() {
            Pending::ChooseCards {
                player, options, ..
            } => {
                let pick = if options.contains(&f) {
                    f
                } else if options.contains(&p) {
                    p
                } else {
                    options[0]
                };
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![pick],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(on_battlefield(&engine, p0, anurid_brushhopper()).is_none());
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 0);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&hopper)
    );

    advance_next_main(&mut engine, p1);
    assert!(on_battlefield(&engine, p0, anurid_brushhopper()).is_some());
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&hopper)
    );
}

fn advance_next_main(engine: &mut Engine<RegistryLookup>, target: PlayerId) {
    let start_turn = engine.state().turn.number;
    loop {
        if engine.state().turn.number > start_turn
            && engine.state().turn.active == target
            && engine.state().turn.phase == Phase::FirstMain
            && at_rest(engine, target)
        {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected pending while advancing: {other:?}"),
        }
    }
}
