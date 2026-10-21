//! `cards/instants/mv_3/spoils_of_evil.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn spoils_of_evil() -> CardIndex {
    card_index("a40c52ee-4b45-4b14-8fbc-6529b42bfc9d")
}

/// Spoils of Evil (a40c52ee-4b45-4b14-8fbc-6529b42bfc9d) — {2}{B} Instant:
/// "For each artifact or creature card in target opponent's graveyard, add {C} and you gain 1 life."
#[test]
fn spoils_of_evil_adds_colorless_mana_and_life_per_artifact_or_creature_in_opponent_graveyard() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(
            0,
            &[
                spoils_of_evil(),
                grizzly_bears(),
                gray_ogre(), // in your own graveyard (excluded)
            ],
        )
        .hand(
            1,
            &[
                jayemdae_tome(), // artifact (counted)
                grizzly_bears(), // creature (counted)
                gray_ogre(),     // creature (counted)
                stone_rain(),    // sorcery (excluded)
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    hand_to_graveyard(&mut engine, P0, grizzly_bears());
    hand_to_graveyard(&mut engine, P0, gray_ogre());
    hand_to_graveyard(&mut engine, P1, jayemdae_tome());
    hand_to_graveyard(&mut engine, P1, grizzly_bears());
    hand_to_graveyard(&mut engine, P1, gray_ogre());
    hand_to_graveyard(&mut engine, P1, stone_rain());

    cast_from_hand(&mut engine, P0, spoils_of_evil());
    if matches!(engine.pending(), Pending::ChoosePlayer { .. }) {
        engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    } else if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![P1],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        3,
        "adds 3 {{C}} for the 1 artifact and 2 creature cards in opponent's graveyard"
    );
    assert_eq!(
        engine.state().players[0].life,
        23,
        "gains 3 life for the 1 artifact and 2 creature cards in opponent's graveyard"
    );
}
