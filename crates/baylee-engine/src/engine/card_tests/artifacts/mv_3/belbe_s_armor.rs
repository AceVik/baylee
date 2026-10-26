//! `cards/artifacts/mv_3/belbe_s_armor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Belbe's Armor ({3} Artifact):
/// "{X}, {T}: Target creature gets -X/+X until end of turn."
#[test]
fn belbes_armor_gives_target_creature_minus_x_plus_x_until_end_of_turn() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                card_index("1650d85f-4c65-429d-8952-f68b62935a55"),
                index::HILL_GIANT,
            ],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    assert_eq!(pt(&engine, giant), (3, 3));

    engine.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 2);
    engine.refresh_offer();

    activate(
        &mut engine,
        P0,
        card_index("1650d85f-4c65-429d-8952-f68b62935a55"),
        0,
    );
    for _ in 0..4 {
        match engine.pending().clone() {
            Pending::ChooseNumber { .. } => {
                engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
            }
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        P0,
                        PlayerAction::ChooseTargets {
                            objects: vec![giant],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            _ => break,
        }
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, giant), (1, 5));

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, giant), (3, 3));
}
