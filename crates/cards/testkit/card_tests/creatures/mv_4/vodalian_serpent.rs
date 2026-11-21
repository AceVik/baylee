//! `cards/creatures/mv_4/vodalian_serpent.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn vodalian_serpent() -> CardIndex {
    card_index("c39c1604-3bae-454d-9985-85101e51ec6e")
}

fn answer_kicker(engine: &mut Engine<RegistryLookup>, kicked: bool) {
    match engine.pending().clone() {
        Pending::YesNo {
            prompt: YesNoPrompt::Kicker,
            ..
        } => {
            engine.apply(P0, PlayerAction::YesNo(kicked)).unwrap();
        }
        Pending::ChooseCastMode { options, .. } => {
            let want = if kicked {
                CastModeKind::Kicked
            } else {
                CastModeKind::Normal
            };
            let idx = options.iter().find(|o| o.kind == want).unwrap().index;
            engine
                .apply(P0, PlayerAction::ChooseMode(usize::try_from(idx).unwrap()))
                .unwrap();
        }
        _ => assert!(
            !kicked,
            "expected kicker prompt, got {:?}",
            engine.pending()
        ),
    }
}

/// Vodalian Serpent (c39c1604-3bae-454d-9985-85101e51ec6e) — `{3}{U}` 2/2 Creature — Serpent:
/// - Kicker `{2}`
/// - This creature can't attack unless defending player controls an Island.
/// - If this creature was kicked, it enters with four +1/+1 counters on it.
#[test]
fn vodalian_serpent_unkicked_is_two_two_and_requires_defending_island_to_attack() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[forest()])
        .hand(0, &[vodalian_serpent()])
        .hand(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, vodalian_serpent());
    answer_kicker(&mut engine, false);
    pass_until(&mut engine, stack_is_empty);

    let serpent = on_battlefield(&engine, P0, vodalian_serpent()).expect("Serpent entered");
    assert_eq!(pt(&engine, serpent), (2, 2));
    assert_eq!(
        engine
            .state()
            .object(serpent)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );

    // P1 controls only a Forest -> Vodalian Serpent cannot attack P1 on Turn 3.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(serpent, Defender::Player(P1))],
                },
            )
            .is_err(),
        "Vodalian Serpent cannot attack unless defending player controls an Island"
    );
    engine
        .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();

    // P1 plays an Island -> Vodalian Serpent can now attack P1.
    reach_their_main_phase(&mut engine, P1);
    let p1_island = in_hand(&engine, P1, island()).unwrap();
    engine
        .apply(P1, PlayerAction::PlayLand { card: p1_island })
        .unwrap();
    reach_their_main_phase(&mut engine, P0);
    let _ = attack_and_collect_blocks(&mut engine, serpent, P1);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 18);
}

#[test]
fn vodalian_serpent_kicked_enters_with_four_plus_one_plus_one_counters() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), island()],
        )
        .hand(0, &[vodalian_serpent()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, vodalian_serpent());
    answer_kicker(&mut engine, true);
    pass_until(&mut engine, stack_is_empty);

    let serpent = on_battlefield(&engine, P0, vodalian_serpent()).unwrap();
    assert_eq!(
        engine
            .state()
            .object(serpent)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        4
    );
    assert_eq!(pt(&engine, serpent), (6, 6));
}
