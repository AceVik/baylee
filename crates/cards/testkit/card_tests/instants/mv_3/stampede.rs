//! `cards/instants/mv_3/stampede.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn stampede() -> CardIndex {
    card_index("49c997d5-7b41-4faf-b898-58db19055671")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn merfolk_of_the_pearl_trident() -> CardIndex {
    card_index("218d9277-c179-4de3-9c7f-79b5a6d4fa38")
}

/// Stampede (49c997d5-7b41-4faf-b898-58db19055671) — {1}{G}{G} Instant:
/// "Attacking creatures get +1/+0 and gain trample until end of turn."
#[test]
fn stampede_pumps_and_grants_trample_only_to_attacking_creatures() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                hill_giant(),
                grizzly_bears(), // stays back (not attacking)
            ],
        )
        .battlefield(1, &[merfolk_of_the_pearl_trident()])
        .hand(0, &[stampede()])
        .start();
    keep_mulligans(&mut engine);

    let giant = on_battlefield(&engine, P0, hill_giant()).unwrap();
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let blocker = on_battlefield(&engine, P1, merfolk_of_the_pearl_trident()).unwrap();

    // P0 attacks with Hill Giant (3/3) only, leaving Grizzly Bears (2/2) back; P1 blocks with 1/1 Merfolk.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, giant)],
            },
        )
        .unwrap();

    // P0 casts Stampede after blockers are declared.
    cast_from_hand(&mut engine, P0, stampede());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, giant), (4, 3));
    assert!(
        keywords(&engine, giant).contains(KeywordSet::TRAMPLE),
        "attacking Hill Giant gets +1/+0 and gains trample"
    );
    assert_eq!(
        pt(&engine, bears),
        (2, 2),
        "non-attacking Grizzly Bears is not pumped"
    );
    assert!(
        !keywords(&engine, bears).contains(KeywordSet::TRAMPLE),
        "non-attacking Grizzly Bears does not gain trample"
    );

    // Combat damage: 4/3 trampling Hill Giant assigns 1 lethal damage to the 1/1 blocker and 3 trample damage to P1.
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, P1, merfolk_of_the_pearl_trident()).is_none()
            && in_graveyard(&engine, P1, merfolk_of_the_pearl_trident()).is_some()
    );
    assert_eq!(
        engine.state().players[1].life,
        17,
        "3 excess combat damage trampled over the 1/1 blocker to P1"
    );
}
