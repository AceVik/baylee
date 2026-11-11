//! `cards/creatures/mv_3/nighthawk_scavenger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn nighthawk_scavenger() -> CardIndex {
    card_index("379ed4ec-8f2e-448b-9ed2-a3181e2f877e")
}

#[track_caller]
fn refresh(e: &mut Engine<RegistryLookup>) {
    e.dev_state_mut(P0)
        .expect("the harness may refresh after preparing a graveyard")
        .refresh_characteristics();
}

/// Only opposing graveyards count, and each card type is counted once there.
#[test]
fn nighthawk_scavenger_counts_types_only_in_opponents_graveyards() {
    let mut e = Duel::new(SEED, steadfast_guard())
        .battlefield(0, &[nighthawk_scavenger()])
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let nighthawk = on_battlefield(&e, P0, nighthawk_scavenger()).unwrap();
    assert_eq!(pt(&e, nighthawk), (1, 3));

    // The backing decks contain only creature cards. Seeding a graveyard is a
    // harness move, so refresh the CDA exactly as for the other Goyf fixtures.
    seed_graveyard(&mut e, P0, 1);
    refresh(&mut e);
    assert_eq!(
        pt(&e, nighthawk),
        (1, 3),
        "your creature card does not count"
    );
    seed_graveyard(&mut e, P1, 1);
    refresh(&mut e);
    assert_eq!(pt(&e, nighthawk), (2, 3), "one opposing card type");
    seed_graveyard(&mut e, P0, 1);
    seed_graveyard(&mut e, P1, 1);
    refresh(&mut e);
    assert_eq!(
        pt(&e, nighthawk),
        (2, 3),
        "a second creature is no new type"
    );

    let land = on_battlefield(&e, P1, mountain()).unwrap();
    bury(&mut e, &[land]);
    refresh(&mut e);
    assert_eq!(
        pt(&e, nighthawk),
        (3, 3),
        "opposing land adds a second type"
    );
}

/// Flying excludes the ordinary Bears as a blocker but not the Spider's reach.
/// One damage with deathtouch kills the 2/4 Spider, and lifelink gains that one.
#[test]
fn nighthawk_scavenger_plays_flying_deathtouch_and_lifelink_in_combat() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[nighthawk_scavenger()])
        .battlefield(1, &[giant_spider(), grizzly_bears()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let nighthawk = on_battlefield(&e, P0, nighthawk_scavenger()).unwrap();
    let spider = on_battlefield(&e, P1, giant_spider()).unwrap();
    let bears = on_battlefield(&e, P1, grizzly_bears()).unwrap();
    let keywords = keywords(&e, nighthawk);
    assert!(keywords.contains(KeywordSet::FLYING));
    assert!(keywords.contains(KeywordSet::DEATHTOUCH));
    assert!(keywords.contains(KeywordSet::LIFELINK));

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(nighthawk, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert!(
        blockers
            .iter()
            .any(|b| b.blocker == spider && b.attackers.contains(&nighthawk))
    );
    assert!(
        !blockers
            .iter()
            .any(|b| b.blocker == bears && b.attackers.contains(&nighthawk))
    );
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(spider, nighthawk)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(in_graveyard(&e, P1, giant_spider()).is_some());
    assert_eq!(e.state().object(nighthawk).unwrap().damage, 2);
    assert_eq!(e.state().players[0].life, 21);
    assert_eq!(e.state().players[1].life, 20);
    assert_eq!(
        pt(&e, nighthawk),
        (2, 3),
        "the Spider is now an opposing creature card"
    );
}
