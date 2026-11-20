//! `cards/instants/mv_3/astral_steel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;
use baylee_engine::choice::TargetPrompt;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Astral Steel — {2}{W} Instant:
/// "Target creature gets +1/+2 until end of turn.
/// Storm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)"
#[test]
fn astral_steel_counts_both_players_casts_retargets_copies_and_wears_off() {
    let steel = card_index("ef8ac291-3789-4147-a11f-c62fd8566dbe");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[index::GRIZZLY_BEARS, index::LLANOWAR_ELVES])
        .hand(0, &[index::DARK_RITUAL, steel])
        .hand(1, &[index::DARK_RITUAL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let elves = on_battlefield(&engine, P0, index::LLANOWAR_ELVES).unwrap();

    // Seat 0 casts Dark Ritual (spell 1) and Seat 1 casts Dark Ritual (spell 2).
    engine.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Black, 1);
    engine.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::White, 3);
    engine.dev_state_mut(P1).unwrap().players[1]
        .mana_pool
        .add(ManaColor::Black, 1);
    engine.refresh_offer();

    cast_with_floating(&mut engine, P0, index::DARK_RITUAL);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_with_floating(&mut engine, P1, index::DARK_RITUAL);
    pass_until(&mut engine, stack_is_empty);

    // Cast Astral Steel targeting Bears -> Storm puts 2 copies on the stack.
    cast_with_floating(&mut engine, P0, steel);
    aim_at(&mut engine, P0, bears);

    // Resolve Storm trigger: retarget copy 1 to Elves, keep copy 2 on Bears.
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
                assert_eq!(player, P0);
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
            other => panic!("unexpected pending during Astral Steel: {other:?}"),
        }
    }
    assert_eq!(
        retargeted, 2,
        "Storm created two copies for two prior casts"
    );
    assert_eq!(
        engine.state().per_turn.spells_cast,
        vec![2, 1],
        "Storm copies do not increment spells_cast"
    );
    // Bears got original + 1 copy (+2/+4 -> 4/6); Elves got 1 retargeted copy (+1/+2 -> 2/3).
    assert_eq!(pt(&engine, bears), (4, 6));
    assert_eq!(pt(&engine, elves), (2, 3));

    // "until end of turn": next turn both creatures return to printed stats.
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, bears), (2, 2));
    assert_eq!(pt(&engine, elves), (1, 1));
}
