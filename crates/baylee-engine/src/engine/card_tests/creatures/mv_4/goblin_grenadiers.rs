//! `cards/creatures/mv_4/goblin_grenadiers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn goblin_grenadiers() -> CardIndex {
    card_index("be83a518-cfab-46b1-9fe9-7e3d54311c37")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Goblin Grenadiers keeps its 2/2 body while the entire optional sacrifice
/// trigger is omitted. With no land on either side it must not offer a
/// creature-only destruction that its printed two-target ability cannot do.
#[test]
fn goblin_grenadiers_attacks_without_a_creature_only_destruction_trigger() {
    let mut e = lk::game(SEED, &[], &[grizzly_bears()], &[goblin_grenadiers()], &[]);
    lk::float(
        &mut e,
        P0,
        &[(ManaColor::Red, 1), (ManaColor::Colorless, 3)],
    );
    cast_with_floating(&mut e, P0, goblin_grenadiers());
    lk::settle(&mut e);
    reach_their_main_phase(&mut e, P1);
    reach_their_main_phase(&mut e, P0);
    let grenadiers = lk::obj(&e, P0, goblin_grenadiers());
    lk::attack(&mut e, &[grenadiers]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, P1), 18, "the body deals its two combat damage");
    assert!(
        on_battlefield(&e, P0, goblin_grenadiers()).is_some(),
        "not sacrificed"
    );
    assert!(
        on_battlefield(&e, P1, grizzly_bears()).is_some(),
        "no creature-only destruction"
    );
}
