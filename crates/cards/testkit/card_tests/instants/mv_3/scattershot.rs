//! `cards/instants/mv_3/scattershot.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;
use baylee_engine::choice::TargetPrompt;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Scattershot — {2}{R} Instant:
/// "Scattershot deals 1 damage to target creature.
/// Storm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)"
#[test]
fn scattershot_pings_creatures_only_and_retargets_storm_copies() {
    let scattershot = card_index("ed3652da-0127-4c9e-9913-f20becdd0ba9");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(1, &[index::GRIZZLY_BEARS, index::LLANOWAR_ELVES, forest()])
        .hand(0, &[index::DARK_RITUAL, scattershot])
        .hand(1, &[index::DARK_RITUAL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    let elves = on_battlefield(&engine, P1, index::LLANOWAR_ELVES).unwrap();
    {
        let state = engine.dev_state_mut(P0).unwrap();
        state.players[0].mana_pool.add(ManaColor::Black, 1);
        state.players[0].mana_pool.add(ManaColor::Red, 3);
        state.players[1].mana_pool.add(ManaColor::Black, 1);
    }
    engine.refresh_offer();

    // Two prior spells (one by each player) -> 2 Storm copies + 1 original = 3 pings.
    cast_with_floating(&mut engine, P0, index::DARK_RITUAL);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_with_floating(&mut engine, P1, index::DARK_RITUAL);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, scattershot);
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected creature target choice, got {:?}",
            engine.pending()
        );
    };
    assert!(
        player_options.is_empty(),
        "Scattershot targets creatures only, not players: {player_options:?}"
    );
    assert_eq!(options.len(), 2, "only the two creatures are legal targets");
    assert!(options.contains(&bears) && options.contains(&elves));
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();

    // Retarget first copy to Elves (1/1 -> dies), keep second copy + original on Bears (2/2 -> dies).
    let mut retargeted = 0usize;
    for _ in 0..40 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseTargets {
                player,
                reason: TargetPrompt::Retarget { .. },
                ..
            } => {
                let objects = if retargeted == 0 { vec![elves] } else { vec![] };
                retargeted += 1;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects,
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected pending during Scattershot: {other:?}"),
        }
    }
    assert_eq!(retargeted, 2);
    assert_eq!(engine.state().per_turn.spells_cast, vec![2, 1]);
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, P1, index::LLANOWAR_ELVES).is_some());
    assert_eq!(engine.state().players[1].life, 20);
}
