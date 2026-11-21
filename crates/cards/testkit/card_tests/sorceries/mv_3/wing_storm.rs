//! `cards/sorceries/mv_3/wing_storm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn wing_storm() -> CardIndex {
    card_index("c593f66d-0e9a-4d46-a9c6-ac7a70bf17ea")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn mahamoti_djinn() -> CardIndex {
    card_index("c39ea5f9-6ec0-4697-897b-779e326754a7")
}

/// Wing Storm (c593f66d-0e9a-4d46-a9c6-ac7a70bf17ea) — {2}{G} Sorcery:
/// "Wing Storm deals damage to each player equal to twice the number of
/// creatures that player controls with flying."
#[test]
fn wing_storm_deals_twice_each_players_own_flying_creature_count() {
    let mut engine = lk::game(
        SEED,
        &[forest(), forest(), forest(), serra_angel(), aurochs()],
        &[serra_angel(), mahamoti_djinn(), fangren_hunter()],
        &[wing_storm()],
        &[],
    );

    cast_from_hand(&mut engine, P0, wing_storm());
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P0), 18, "P0 controlled 1 flyer -> 2");
    assert_eq!(lk::life(&engine, P1), 16, "P1 controlled 2 flyers -> 4");
}
