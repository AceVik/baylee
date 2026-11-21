//! `cards/creatures/mv_5/desolation_angel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn desolation_angel() -> CardIndex {
    card_index("87f82107-eaba-484a-800d-e61a0cc5e1f2")
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

/// Desolation Angel (87f82107-eaba-484a-800d-e61a0cc5e1f2) — `{3}{B}{B}` 5/4 Creature — Angel:
/// - Kicker `{W}{W}`
/// - Flying
/// - When this creature enters, destroy all lands you control.
///   If it was kicked, destroy all lands instead.
#[test]
fn desolation_angel_unkicked_destroys_only_your_lands() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[forest(), forest()])
        .hand(0, &[desolation_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, desolation_angel());
    answer_kicker(&mut engine, false);
    pass_until(&mut engine, stack_is_empty);

    let angel = on_battlefield(&engine, P0, desolation_angel()).expect("Angel entered");
    assert_eq!(pt(&engine, angel), (5, 4));
    assert!(keywords(&engine, angel).contains(KeywordSet::FLYING));
    assert!(on_battlefield(&engine, P0, swamp()).is_none());
    assert!(
        on_battlefield(&engine, P1, forest()).is_some(),
        "unkicked Desolation Angel spares opponent's lands"
    );
}

#[test]
fn desolation_angel_kicked_destroys_all_lands_on_both_sides() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                plains(),
                plains(),
                sol_ring(),
            ],
        )
        .battlefield(1, &[forest(), island()])
        .hand(0, &[desolation_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, desolation_angel());
    answer_kicker(&mut engine, true);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, desolation_angel()).is_some());
    assert!(on_battlefield(&engine, P0, swamp()).is_none());
    assert!(on_battlefield(&engine, P0, plains()).is_none());
    assert!(on_battlefield(&engine, P1, forest()).is_none());
    assert!(on_battlefield(&engine, P1, island()).is_none());
    assert!(
        on_battlefield(&engine, P0, sol_ring()).is_some(),
        "nonland permanents survive"
    );
}
