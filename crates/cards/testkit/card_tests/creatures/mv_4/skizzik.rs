//! `cards/creatures/mv_4/skizzik.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn skizzik() -> CardIndex {
    card_index("0c77d37e-791c-415e-b369-55e93147ec34")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
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
            let idx = options
                .iter()
                .find(|o| o.kind == want)
                .expect("kicker cast mode offered")
                .index;
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

/// Skizzik (0c77d37e-791c-415e-b369-55e93147ec34) — `{3}{R}` 5/3 Creature — Elemental:
/// - Kicker `{R}`
/// - Trample, haste
/// - At the beginning of the end step, if this creature wasn't kicked, sacrifice it.
#[test]
fn skizzik_unkicked_attacks_immediately_with_trample_and_sacrifices_at_end_step() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[skizzik()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, skizzik());
    answer_kicker(&mut engine, false);
    pass_until(&mut engine, stack_is_empty);

    let skiz = on_battlefield(&engine, P0, skizzik()).expect("Skizzik entered");
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, skiz), (5, 3));
    let kw = keywords(&engine, skiz);
    assert!(kw.contains(KeywordSet::HASTE) && kw.contains(KeywordSet::TRAMPLE));

    // Attacks immediately with haste and tramples 3 damage over 2/2 Grizzly Bears.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(skiz, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, skiz)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 17, "3 trample damage dealt");

    reach_their_main_phase(&mut engine, P1);
    assert!(on_battlefield(&engine, P0, skizzik()).is_none());
    assert!(in_graveyard(&engine, P0, skizzik()).is_some());
}

#[test]
fn skizzik_kicked_stays_on_battlefield_past_end_steps() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[skizzik()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, skizzik());
    answer_kicker(&mut engine, true);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, skizzik()).is_some());

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(
        on_battlefield(&engine, P0, skizzik()).is_some(),
        "kicked Skizzik is not sacrificed at end steps"
    );
}
