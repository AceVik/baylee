//! `cards/instants/mv_3/spoils_of_evil.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn spoils_of_evil() -> CardIndex {
    card_index("a40c52ee-4b45-4b14-8fbc-6529b42bfc9d")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

fn sol_ring() -> CardIndex {
    card_index("6ad8011d-3471-4369-9d68-b264cc027487")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

#[test]
fn spoils_of_evil_counts_only_opponent_artifact_and_creature_graveyard_cards() {
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), sol_ring()])
        .battlefield(1, &[sol_ring(), grizzly_bears()])
        .hand(0, &[spoils_of_evil(), grizzly_bears()])
        .hand(1, &[lightning_bolt(), swamp()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    // Put 1 artifact + 1 creature + 1 instant + 1 land in P1's graveyard, and 1 creature + 1 artifact in P0's.
    let p0_ring = on_battlefield(&e, P0, sol_ring()).unwrap();
    let p1_ring = on_battlefield(&e, P1, sol_ring()).unwrap();
    let p1_bears = on_battlefield(&e, P1, grizzly_bears()).unwrap();
    bury(&mut e, &[p0_ring, p1_ring, p1_bears]);
    hand_to_graveyard(&mut e, P0, grizzly_bears());
    hand_to_graveyard(&mut e, P1, lightning_bolt());
    hand_to_graveyard(&mut e, P1, swamp());

    let life_before = e.state().players[0].life;
    cast_from_hand(&mut e, P0, spoils_of_evil());
    e.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        e.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2
    );
    assert_eq!(e.state().players[0].life, life_before + 2);
}
