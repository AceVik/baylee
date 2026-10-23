//! `cards/creatures/mv_4/bounty_hunter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn bounty_hunter() -> CardIndex {
    card_index("2fd1cfc5-fbcd-4d31-ba8b-759d8c41b435")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// p0 has a Bounty Hunter and a Hill Giant; p1 has Grizzly Bears and a
/// Bounty Hunter of their own (a black creature). p0's first main phase.
fn a_hunter_among_creatures() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[bounty_hunter(), hill_giant()])
        .battlefield(1, &[grizzly_bears(), bounty_hunter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

fn bounty_counters(engine: &Engine<RegistryLookup>, id: ObjectId) -> u32 {
    u32::from(
        engine
            .state()
            .object(id)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::BOUNTY),
    )
}

/// Bounty Hunter (2fd1cfc5-fbcd-4d31-ba8b-759d8c41b435) — {2}{B}{B} Creature — Human Archer Minion:
/// "{T}: Put a bounty counter on target nonblack creature. {T}: Destroy target creature with a bounty counter on it."
#[test]
fn bounty_hunter_puts_a_bounty_counter_only_on_a_nonblack_creature() {
    let mut engine = a_hunter_among_creatures();
    let hunter = on_battlefield(&engine, P0, bounty_hunter()).unwrap();
    let giant = on_battlefield(&engine, P0, hill_giant()).unwrap();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let black_hunter = on_battlefield(&engine, P1, bounty_hunter()).unwrap();

    activate(&mut engine, P0, bounty_hunter(), 0);
    let menu = aim_at(&mut engine, P0, bears);
    assert!(menu.contains(&bears) && menu.contains(&giant));
    assert!(
        !menu.contains(&black_hunter) && !menu.contains(&hunter),
        "black creatures are not on the menu: {menu:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(bounty_counters(&engine, bears), 1);
    assert_eq!(bounty_counters(&engine, giant), 0);
    assert_eq!(bounty_counters(&engine, black_hunter), 0);
    assert!(is_tapped(&engine, hunter), "{{T}} was paid");
    assert!(
        on_battlefield(&engine, P1, grizzly_bears()).is_some(),
        "marking is not destroying"
    );
}

#[test]
fn bounty_hunter_destroys_only_a_creature_that_carries_a_bounty_counter() {
    let mut engine = a_hunter_among_creatures();
    let hunter = on_battlefield(&engine, P0, bounty_hunter()).unwrap();
    let giant = on_battlefield(&engine, P0, hill_giant()).unwrap();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let black_hunter = on_battlefield(&engine, P1, bounty_hunter()).unwrap();
    assert!(
        !ability_offered(&engine, hunter, 1),
        "no creature has a bounty counter yet, so there is nothing to destroy"
    );

    activate(&mut engine, P0, bounty_hunter(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(
        !is_tapped(&engine, hunter),
        "untapped for the second ability"
    );

    activate(&mut engine, P0, bounty_hunter(), 1);
    let menu = aim_at(&mut engine, P0, bears);
    assert!(menu.contains(&bears));
    assert!(
        !menu.contains(&giant) && !menu.contains(&black_hunter) && !menu.contains(&hunter),
        "only the creature with the counter is on the menu: {menu:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, P0, hill_giant()).is_some());
    assert!(on_battlefield(&engine, P1, bounty_hunter()).is_some());
}
