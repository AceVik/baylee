//! `cards/enchantments/mv_3/dueling_grounds.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn dueling_grounds() -> CardIndex {
    card_index("eb2df4a1-6b63-4f6a-a830-e9487afe59f8")
}

fn objs(e: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> Vec<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            e.state()
                .object(id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// No more than one creature can attack each combat, and no more than one
/// creature can block each combat — checked for both players.
#[test]
fn dueling_grounds_limits_each_combat_to_one_attacker_and_one_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[dueling_grounds(), grizzly_bears(), grizzly_bears()],
        &[llanowar_elves(), llanowar_elves()],
        &[],
        &[],
    );
    let my_bears = objs(&e, p0, grizzly_bears());
    let their_elves = objs(&e, p1, llanowar_elves());

    // Only one attacker allowed for P0.
    lk::attackers_offered(&mut e);
    assert!(
        e.apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (my_bears[0], Defender::Player(p1)),
                    (my_bears[1], Defender::Player(p1)),
                ],
            },
        )
        .is_err()
    );
    lk::declare(&mut e, &[my_bears[0]]);

    // Only one blocker allowed for P1 against that single attacker.
    assert!(
        e.apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(their_elves[0], my_bears[0]), (their_elves[1], my_bears[0]),],
            },
        )
        .is_err()
    );
    lk::block(&mut e, &[(their_elves[0], my_bears[0])]);
    lk::through_combat(&mut e);
    lk::settle(&mut e);

    // Next turn P1 is likewise capped at one attacker and P0 at one blocker.
    let mut extra_game = lk::game(
        SEED,
        &[dueling_grounds(), grizzly_bears(), grizzly_bears()],
        &[llanowar_elves(), llanowar_elves()],
        &[],
        &[],
    );
    let g_p1 = objs(&extra_game, p1, llanowar_elves());
    let g_p0 = objs(&extra_game, p0, grizzly_bears());
    reach_their_main_phase(&mut extra_game, p1);
    lk::attackers_offered(&mut extra_game);
    assert!(
        extra_game
            .apply(
                p1,
                PlayerAction::DeclareAttackers {
                    attackers: vec![
                        (g_p1[0], Defender::Player(p0)),
                        (g_p1[1], Defender::Player(p0)),
                    ],
                },
            )
            .is_err()
    );
    lk::declare(&mut extra_game, &[g_p1[0]]);
    assert!(
        extra_game
            .apply(
                p0,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(g_p0[0], g_p1[0]), (g_p0[1], g_p1[0])],
                },
            )
            .is_err()
    );
    lk::block(&mut extra_game, &[(g_p0[0], g_p1[0])]);
    lk::through_combat(&mut extra_game);
    lk::settle(&mut extra_game);
    assert_eq!(lk::zone_of(&extra_game, g_p1[0]), Zone::Graveyard);
}
