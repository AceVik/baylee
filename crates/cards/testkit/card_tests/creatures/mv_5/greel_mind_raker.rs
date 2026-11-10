//! `cards/creatures/mv_5/greel_mind_raker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn greel_mind_raker() -> CardIndex {
    card_index("2b84a10e-4c5c-407f-8b5b-a23d402f0529")
}

/// Greel, Mind Raker ({3}{B}{B} 3/3 Legendary Creature — Horror Spellshaper):
/// "{X}{B}, {T}, Discard two cards: Target player discards X cards at random."
#[test]
fn greel_mind_raker_discards_two_cards_to_make_target_player_discard_x_at_random() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[greel_mind_raker(), swamp(), swamp(), swamp()])
        .hand(0, &[grizzly_bears(), index::HILL_GIANT])
        .hand(1, &[plains(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, greel_mind_raker(), 0);
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::ChooseNumber { .. } => {
                engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
            }
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p1],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards { options, .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .unwrap();
            }
            _ => break,
        }
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, p0, index::HILL_GIANT).is_some());
    assert!(in_graveyard(&engine, p1, plains()).is_some());
    assert!(in_graveyard(&engine, p1, forest()).is_some());
}

/// Activating Greel, Mind Raker with X = 0 still pays {B}, {T}, and discards two
/// cards, while target player discards 0 cards.
#[test]
fn greel_mind_raker_with_x_zero_discards_two_and_target_discards_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[greel_mind_raker(), swamp()])
        .hand(0, &[grizzly_bears(), index::HILL_GIANT])
        .hand(1, &[plains(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, greel_mind_raker(), 0);
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::ChooseNumber { .. } => {
                engine.apply(p0, PlayerAction::ChooseNumber(0)).unwrap();
            }
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p1],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards { options, .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .unwrap();
            }
            _ => break,
        }
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, p0, index::HILL_GIANT).is_some());
    assert!(in_hand(&engine, p1, plains()).is_some());
    assert!(in_hand(&engine, p1, forest()).is_some());
}
