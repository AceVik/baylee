//! `cards/instants/mv_1/songs_of_the_damned.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn songs_of_the_damned() -> CardIndex {
    card_index("39419d29-47e7-4d71-953b-96a2e14ae964")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Songs of the Damned (39419d29-47e7-4d71-953b-96a2e14ae964) — {B} Instant:
/// "Add {B} for each creature card in your graveyard."
#[test]
fn songs_of_the_damned_adds_black_mana_for_each_creature_card_in_your_graveyard_only() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp()])
        .hand(
            0,
            &[
                songs_of_the_damned(),
                grizzly_bears(),
                gray_ogre(),
                hill_giant(),
                lightning_bolt(), // non-creature in your graveyard (excluded)
            ],
        )
        .hand(
            1,
            &[
                grizzly_bears(),
                gray_ogre(), // creatures in opponent's graveyard (excluded)
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    hand_to_graveyard(&mut engine, P0, grizzly_bears());
    hand_to_graveyard(&mut engine, P0, gray_ogre());
    hand_to_graveyard(&mut engine, P0, hill_giant());
    hand_to_graveyard(&mut engine, P0, lightning_bolt());
    hand_to_graveyard(&mut engine, P1, grizzly_bears());
    hand_to_graveyard(&mut engine, P1, gray_ogre());

    cast_from_hand(&mut engine, P0, songs_of_the_damned());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        3,
        "adds {{B}}{{B}}{{B}} for the 3 creature cards in P0's graveyard, ignoring non-creatures and opponent's graveyard"
    );
}
