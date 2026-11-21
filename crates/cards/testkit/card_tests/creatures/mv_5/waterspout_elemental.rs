//! `cards/creatures/mv_5/waterspout_elemental.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn waterspout_elemental() -> CardIndex {
    card_index("f86b8c03-9f1e-4d42-ac45-907c46e9e1c0")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
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

/// Waterspout Elemental (f86b8c03-9f1e-4d42-ac45-907c46e9e1c0) — `{3}{U}{U}` 3/4
/// Creature — Elemental:
/// - Kicker `{U}`
/// - Flying
/// - When this creature enters, if it was kicked, return all other creatures to
///   their owners' hands and you skip your next turn.
#[test]
fn waterspout_elemental_kicked_bounces_all_other_creatures_and_skips_your_next_turn() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[hill_giant()])
        .hand(0, &[waterspout_elemental()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, waterspout_elemental());
    answer_kicker(&mut engine, true);
    pass_until(&mut engine, stack_is_empty);

    let elem = on_battlefield(&engine, P0, waterspout_elemental()).expect("Elemental entered");
    assert_eq!(pt(&engine, elem), (3, 4));
    assert!(keywords(&engine, elem).contains(KeywordSet::FLYING));
    assert!(in_hand(&engine, P0, grizzly_bears()).is_some());
    assert!(in_hand(&engine, P1, hill_giant()).is_some());

    // End P0's Turn 1 -> P1 gets Turn 2 -> P0's next turn is skipped, so P1 immediately gets Turn 3!
    reach_their_main_phase(&mut engine, P1);
    let p1_first_turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > p1_first_turn && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        engine.state().turn.active,
        P1,
        "P0 skipped their next turn, giving P1 two consecutive turns"
    );
}

#[test]
fn waterspout_elemental_unkicked_neither_bounces_creatures_nor_skips_turn() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[hill_giant()])
        .hand(0, &[waterspout_elemental()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, waterspout_elemental());
    answer_kicker(&mut engine, false);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, P1, hill_giant()).is_some());

    reach_their_main_phase(&mut engine, P1);
    let p1_turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > p1_turn && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(engine.state().turn.active, P0);
}
