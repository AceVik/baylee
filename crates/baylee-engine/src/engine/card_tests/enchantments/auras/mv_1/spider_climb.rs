//! `cards/enchantments/auras/mv_1/spider_climb.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT, SERRA_ANGEL};

fn spider_climb() -> CardIndex {
    card_index("2636aafa-83f9-4b72-b729-04e1c3aa0eec")
}

/// `Spider Climb` ({G}, Enchantment — Aura, `Coverage::Partial`) enchants a
/// creature and gives it +0/+3 and reach: it can block a creature with flying,
/// and the unenchanted creature beside it cannot.
#[test]
fn spider_climb_grants_plus_0_plus_3_and_reach_to_enchanted_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), GRIZZLY_BEARS, HILL_GIANT])
        .battlefield(1, &[SERRA_ANGEL])
        .hand(0, &[spider_climb()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let other = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    let angel = on_battlefield(&engine, p1, SERRA_ANGEL).unwrap();

    cast_from_hand(&mut engine, p0, spider_climb());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (2, 5));
    assert!(keywords(&engine, bear).contains(KeywordSet::REACH));
    assert_eq!(pt(&engine, other), (3, 3));
    assert!(!keywords(&engine, other).contains(KeywordSet::REACH));

    reach_their_main_phase(&mut engine, p1);
    let offer = lk::attack(&mut engine, &[angel]);
    assert!(
        lk::may_block(&offer, bear, angel),
        "reach lets the enchanted creature block the flier"
    );
    assert!(
        !lk::may_block(&offer, other, angel),
        "the unenchanted creature has no reach"
    );
}

/// Flash is left off because the sacrifice rider that goes with it is not
/// supported: the Aura is castable in the main phase and not in the end step.
#[test]
fn spider_climb_is_castable_only_at_sorcery_speed() {
    let p0 = PlayerId::new(0);
    let mut engine = lk::game(
        SEED,
        &[forest(), GRIZZLY_BEARS],
        &[],
        &[spider_climb()],
        &[],
    );
    lk::float(&mut engine, p0, &[(ManaColor::Green, 1)]);
    assert!(
        lk::castable(&engine, p0, spider_climb()),
        "castable in the main phase with its mana floating"
    );

    lk::to_step(&mut engine, p0, Step::End);
    lk::float(&mut engine, p0, &[(ManaColor::Green, 1)]);
    assert!(
        !lk::castable(&engine, p0, spider_climb()),
        "not castable in the end step without flash"
    );
}
