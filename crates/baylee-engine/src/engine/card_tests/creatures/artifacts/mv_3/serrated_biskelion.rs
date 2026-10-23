//! `cards/creatures/artifacts/mv_3/serrated_biskelion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn serrated_biskelion() -> CardIndex {
    card_index("c8552706-83f2-41d7-8068-75891abb8bde")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Serrated Biskelion (c8552706-83f2-41d7-8068-75891abb8bde) — "{T}: Put a
/// -1/-1 counter on this creature and a -1/-1 counter on target creature."
/// One counter each (2/2 to 1/1 for both), only on the target (the other
/// opposing creature is untouched), the Biskelion is tapped by the cost, and
/// a land is not a legal target.
#[test]
fn serrated_biskelion_puts_a_counter_on_itself_and_one_on_its_target_only() {
    let mut e = lk::game(
        SEED,
        &[serrated_biskelion()],
        &[grizzly_bears(), hill_giant(), forest()],
        &[],
        &[],
    );
    let biskelion = lk::obj(&e, P0, serrated_biskelion());
    let bears = lk::obj(&e, P1, grizzly_bears());
    let giant = lk::obj(&e, P1, hill_giant());
    let land = lk::obj(&e, P1, forest());

    activate(&mut e, P0, serrated_biskelion(), 0);
    let menu = aim_at(&mut e, P0, bears);
    assert!(menu.contains(&giant), "any creature is a target: {menu:?}");
    assert!(!menu.contains(&land), "a land is no creature: {menu:?}");
    pass_until(&mut e, stack_is_empty);

    assert_eq!(pt(&e, biskelion), (1, 1), "a -1/-1 counter on itself");
    assert_eq!(pt(&e, bears), (1, 1), "a -1/-1 counter on the target");
    assert_eq!(pt(&e, giant), (3, 3), "the other creature is untouched");
    assert!(lk::tapped(&e, biskelion), "{{T}} is the cost");
    assert!(
        !lk::offered(&e, serrated_biskelion(), P0, 0),
        "tapped, it cannot do it a second time this turn"
    );
}

/// Serrated Biskelion — the counters stay: untapped on the next turn, a
/// second use takes the 1/1 Biskelion to 0/0 and it dies (the target Bears,
/// also at 1/1, dies with it).
#[test]
fn serrated_biskelion_kills_itself_on_its_second_use() {
    let mut e = lk::game(SEED, &[serrated_biskelion()], &[grizzly_bears()], &[], &[]);
    let bears = lk::obj(&e, P1, grizzly_bears());

    activate(&mut e, P0, serrated_biskelion(), 0);
    aim_at(&mut e, P0, bears);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, serrated_biskelion()).is_some(),
        "it survives the first use"
    );

    pass_until(&mut e, |e| {
        e.state().turn.number == 3
            && e.state().turn.active == P0
            && e.state().turn.step == Step::Main
            && matches!(e.pending(), Pending::Priority { .. })
    });
    activate(&mut e, P0, serrated_biskelion(), 0);
    aim_at(&mut e, P0, bears);
    pass_until(&mut e, stack_is_empty);

    assert!(
        in_graveyard(&e, P0, serrated_biskelion()).is_some(),
        "1/1 plus a counter is 0/0"
    );
    assert!(
        in_graveyard(&e, P1, grizzly_bears()).is_some(),
        "the target's second counter kills it"
    );
}
