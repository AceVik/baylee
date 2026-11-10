//! `cards/creatures/mv_4/burning_palm_efreet.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, SERRA_ANGEL};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn burning_palm_efreet() -> CardIndex {
    card_index("55cbc0e8-eeec-433a-b70c-3501238b658b")
}

/// Burning Palm Efreet (55cbc0e8-eeec-433a-b70c-3501238b658b) — {2}{R}{R} 2/2:
/// "{1}{R}{R}: This creature deals 2 damage to target creature with flying and
/// that creature loses flying until end of turn."
#[test]
fn burning_palm_efreet_damages_flyer_and_strips_flying_until_eot() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[burning_palm_efreet(), mountain(), mountain(), mountain()],
        )
        .battlefield(1, &[SERRA_ANGEL, GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let angel = on_battlefield(&engine, P1, SERRA_ANGEL).unwrap();
    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, burning_palm_efreet(), 0);
    let options = aim_at(&mut engine, P0, angel);
    assert!(options.contains(&angel));
    assert!(!options.contains(&bears));
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(angel).unwrap().damage, 2);
    assert!(!keywords(&engine, angel).contains(KeywordSet::FLYING));
}

/// "until end of turn": on its controller's next turn the Angel flies again
/// (and the damage is gone).
#[test]
fn burning_palm_efreet_flying_returns_on_the_next_turn() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[burning_palm_efreet(), mountain(), mountain(), mountain()],
        )
        .battlefield(1, &[SERRA_ANGEL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let angel = on_battlefield(&engine, P1, SERRA_ANGEL).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, burning_palm_efreet(), 0);
    aim_at(&mut engine, P0, angel);
    pass_until(&mut engine, stack_is_empty);
    assert!(!keywords(&engine, angel).contains(KeywordSet::FLYING));

    reach_their_main_phase(&mut engine, P1);
    assert!(keywords(&engine, angel).contains(KeywordSet::FLYING));
    assert_eq!(engine.state().object(angel).unwrap().damage, 0);
}
