//! `cards/creatures/mv_4/reveille_squad.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn reveille_squad() -> CardIndex {
    card_index("2531eb1a-ae24-450a-b8e8-dbbd65eaad64")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn is_tapped(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .status
        .contains(Status::TAPPED)
}

/// Reveille Squad (2531eb1a-ae24-450a-b8e8-dbbd65eaad64) — `{2}{W}{W}` 3/3
/// Creature — Human Rebel:
/// "Whenever one or more creatures attack you, if this creature is untapped,
/// you may untap all creatures you control."
#[test]
fn reveille_squad_untapped_triggers_once_for_multiple_attackers_and_untaps_your_creatures() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), llanowar_elves()],
        )
        .battlefield(1, &[grizzly_bears(), grizzly_bears()])
        .hand(0, &[reveille_squad()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana_but(&mut engine, P0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, P0, reveille_squad());
    pass_until(&mut engine, stack_is_empty);

    let squad = on_battlefield(&engine, P0, reveille_squad()).expect("Reveille Squad entered");
    let elf = on_battlefield(&engine, P0, llanowar_elves()).unwrap();
    let p0_plains = on_battlefield(&engine, P0, plains()).unwrap();
    assert_eq!(pt(&engine, squad), (3, 3));

    // Tap Llanowar Elves for mana so P0 has a tapped creature and tapped lands.
    tap_all_mana(&mut engine, P0);
    assert!(is_tapped(&engine, elf) && is_tapped(&engine, p0_plains));

    // P1 attacks P0 with TWO creatures while Reveille Squad is untapped.
    reach_their_main_phase(&mut engine, P1);
    let p1_bears: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| engine.state().object(id).unwrap().controller == P1)
        .collect();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (p1_bears[0], Defender::Player(P0)),
                    (p1_bears[1], Defender::Player(P0)),
                ],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    assert!(!is_tapped(&engine, elf), "creatures P0 controls untap");
    assert!(
        is_tapped(&engine, p0_plains),
        "noncreature lands stay tapped"
    );
}

#[test]
fn reveille_squad_does_not_trigger_when_tapped() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[reveille_squad(), llanowar_elves()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);

    let squad = on_battlefield(&engine, P0, reveille_squad()).unwrap();
    let elf = on_battlefield(&engine, P0, llanowar_elves()).unwrap();
    let p1_bear = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    // Squad attacks on P0's turn, becoming tapped; Elf taps for mana.
    tap_all_mana(&mut engine, P0);
    let _ = attack_and_collect_blocks(&mut engine, squad, P1);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(is_tapped(&engine, squad) && is_tapped(&engine, elf));

    // P1 attacks P0 while Squad is tapped -> intervening-if prevents trigger; reaches blockers.
    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(p1_bear, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(is_tapped(&engine, elf));
}
