//! `cards/creatures/mv_4/desolation_giant.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn desolation_giant() -> CardIndex {
    card_index("c23aaf6d-151e-481a-a345-ca00c14940d1")
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

/// Desolation Giant (c23aaf6d-151e-481a-a345-ca00c14940d1) — `{2}{R}{R}` 3/3 Creature — Giant:
/// - Kicker `{W}{W}`
/// - When this creature enters, destroy all other creatures you control.
///   If it was kicked, destroy all other creatures instead.
#[test]
fn desolation_giant_unkicked_destroys_only_other_creatures_you_control() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[hill_giant()])
        .hand(0, &[desolation_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, desolation_giant());
    answer_kicker(&mut engine, false);
    pass_until(&mut engine, stack_is_empty);

    let giant = on_battlefield(&engine, P0, desolation_giant()).expect("Giant entered");
    assert_eq!(pt(&engine, giant), (3, 3));
    assert!(in_graveyard(&engine, P0, grizzly_bears()).is_some());
    assert!(
        on_battlefield(&engine, P1, hill_giant()).is_some(),
        "unkicked Desolation Giant spares opponent's creatures"
    );
}

#[test]
fn desolation_giant_kicked_destroys_all_other_creatures_on_both_sides() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                plains(),
                plains(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[hill_giant()])
        .hand(0, &[desolation_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, desolation_giant());
    answer_kicker(&mut engine, true);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, desolation_giant()).is_some(),
        "Desolation Giant spares itself"
    );
    assert!(in_graveyard(&engine, P0, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, P1, hill_giant()).is_some());
}
