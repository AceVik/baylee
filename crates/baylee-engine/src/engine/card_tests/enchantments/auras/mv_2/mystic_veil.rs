//! `cards/enchantments/auras/mv_2/mystic_veil.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn mystic_veil() -> CardIndex {
    card_index("43c88fcd-aac9-4741-8fd6-31ac81c5bb6f")
}

/// `Mystic Veil` ({1}{U}, Enchantment — Aura, `Coverage::Partial`) enchants a
/// creature, which has shroud: an opponent's Lightning Bolt is not offered the
/// enchanted creature as a target, and is offered the creature beside it.
#[test]
fn mystic_veil_gives_enchanted_creature_shroud() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), GRIZZLY_BEARS, HILL_GIANT])
        .battlefield(1, &[mountain()])
        .hand(0, &[mystic_veil()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    cast_from_hand(&mut engine, p0, mystic_veil());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, mystic_veil()).is_some());
    assert!(keywords(&engine, bear).contains(KeywordSet::SHROUD));
    assert!(!keywords(&engine, giant).contains(KeywordSet::SHROUD));

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, lightning_bolt());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&giant),
        "the unenchanted creature is a target"
    );
    assert!(
        !options.contains(&bear),
        "the enchanted creature has shroud: it is no target"
    );
}

/// Flash is left off because the sacrifice rider that goes with it is not
/// supported: the Aura is castable in the main phase and not in the end step.
#[test]
fn mystic_veil_is_castable_only_at_sorcery_speed() {
    let p0 = PlayerId::new(0);
    let mut engine = lk::game(
        SEED,
        &[island(), island(), GRIZZLY_BEARS],
        &[],
        &[mystic_veil()],
        &[],
    );
    lk::float(
        &mut engine,
        p0,
        &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
    );
    assert!(
        lk::castable(&engine, p0, mystic_veil()),
        "castable in the main phase with its mana floating"
    );

    lk::to_step(&mut engine, p0, Step::End);
    lk::float(
        &mut engine,
        p0,
        &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
    );
    assert!(
        !lk::castable(&engine, p0, mystic_veil()),
        "not castable in the end step without flash"
    );
}
