//! `cards/artifacts/mv_3/chariot_of_the_sun.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

const P1: PlayerId = PlayerId::new(1);

const P0: PlayerId = PlayerId::new(0);

fn chariot_of_the_sun() -> CardIndex {
    card_index("b0af249d-9255-42f5-bafc-ac61ee281930")
}

/// Chariot of the Sun (b0af249d-9255-42f5-bafc-ac61ee281930) — {3} Artifact:
/// "{2}, {T}: Until end of turn, target creature you control gains flying and
/// has base toughness 1."
#[test]
fn chariot_of_the_sun_grants_flying_and_sets_base_toughness_to_one() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[chariot_of_the_sun(), plains(), plains(), HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P0, HILL_GIANT).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, chariot_of_the_sun(), 0);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, giant).contains(KeywordSet::FLYING));
    assert_eq!(pt(&engine, giant), (3, 1));
}

/// "target creature you control": the opponent's creature is not on the menu.
#[test]
fn chariot_of_the_sun_cannot_aim_at_an_opponents_creature() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[chariot_of_the_sun(), plains(), plains(), HILL_GIANT])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P0, HILL_GIANT).unwrap();
    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, chariot_of_the_sun(), 0);
    let options = aim_at(&mut engine, P0, giant);

    assert!(options.contains(&giant));
    assert!(!options.contains(&bears));
}

/// "Until end of turn": on the opponent's next turn the Giant is a plain 3/3
/// without flying again.
#[test]
fn chariot_of_the_sun_effect_ends_with_the_turn() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[chariot_of_the_sun(), plains(), plains(), HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P0, HILL_GIANT).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, chariot_of_the_sun(), 0);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (3, 1));

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, giant), (3, 3));
    assert!(!keywords(&engine, giant).contains(KeywordSet::FLYING));
}
