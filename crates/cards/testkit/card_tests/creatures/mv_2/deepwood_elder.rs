//! `cards/creatures/mv_2/deepwood_elder.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn subject_card() -> CardIndex {
    card_index("ffb62e4a-802d-4f1e-b92e-ec0919baafd5")
}

/// Deepwood Elder — {G}{G} 2/2 Creature — Dryad Spellshaper:
/// "{X}{G}{G}, {T}, Discard a card: X target lands become Forests until end of turn."
#[test]
fn deepwood_elder_turns_x_target_lands_into_forests_until_end_of_turn() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[subject_card(), mountain()])
        .battlefield(1, &[island()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let mtn = on_battlefield(&engine, P0, mountain()).unwrap();
    let isl = on_battlefield(&engine, P1, island()).unwrap();
    let discard = in_hand(&engine, P0, forest()).unwrap();

    engine.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Green, 4);
    engine.refresh_offer();

    activate(&mut engine, P0, subject_card(), 0);
    for _ in 0..5 {
        match engine.pending().clone() {
            Pending::ChooseNumber { .. } => {
                engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
            }
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        P0,
                        PlayerAction::ChooseTargets {
                            objects: vec![mtn, isl],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards { .. } => {
                engine
                    .apply(
                        P0,
                        PlayerAction::ChooseObjects {
                            objects: vec![discard],
                        },
                    )
                    .unwrap();
            }
            _ => break,
        }
    }
    pass_until(&mut engine, stack_is_empty);

    for land in [mtn, isl] {
        let sub = &engine
            .state()
            .object(land)
            .unwrap()
            .characteristics()
            .subtypes;
        assert!(sub.contains(subtypes::land::FOREST));
        assert!(!sub.contains(subtypes::land::MOUNTAIN));
        assert!(!sub.contains(subtypes::land::ISLAND));
    }

    reach_their_main_phase(&mut engine, P1);
    assert!(
        engine
            .state()
            .object(mtn)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::land::MOUNTAIN)
    );
    assert!(
        engine
            .state()
            .object(isl)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::land::ISLAND)
    );
}
