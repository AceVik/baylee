//! `cards/enchantments/auras/mv_2/relic_ward.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index::{JAYEMDAE_TOME, SHATTER};

fn relic_ward() -> CardIndex {
    card_index("c6321de2-80b0-4679-93cf-97df75344e97")
}

/// `Relic Ward` ({1}{W}, Enchantment — Aura, `Coverage::Partial`) enchants an
/// artifact, which has shroud: an opponent's Shatter is not offered the
/// enchanted artifact, and is offered the artifact beside it.
#[test]
fn relic_ward_gives_enchanted_artifact_shroud() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), JAYEMDAE_TOME, sol_ring()])
        .battlefield(1, &[mountain(), mountain()])
        .hand(0, &[relic_ward()])
        .hand(1, &[SHATTER])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tome = on_battlefield(&engine, p0, JAYEMDAE_TOME).unwrap();
    let ring = on_battlefield(&engine, p0, sol_ring()).unwrap();
    cast_from_hand(&mut engine, p0, relic_ward());
    aim_at(&mut engine, p0, tome);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, relic_ward()).is_some());
    assert!(keywords(&engine, tome).contains(KeywordSet::SHROUD));
    assert!(!keywords(&engine, ring).contains(KeywordSet::SHROUD));

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, SHATTER);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&ring),
        "the unenchanted artifact is a target"
    );
    assert!(
        !options.contains(&tome),
        "the enchanted artifact has shroud: it is no target"
    );
}

/// Flash is left off because the sacrifice rider that goes with it is not
/// supported: the Aura is castable in the main phase and not in the end step.
#[test]
fn relic_ward_is_castable_only_at_sorcery_speed() {
    let p0 = PlayerId::new(0);
    let mut engine = lk::game(
        SEED,
        &[plains(), plains(), JAYEMDAE_TOME],
        &[],
        &[relic_ward()],
        &[],
    );
    lk::float(
        &mut engine,
        p0,
        &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
    );
    assert!(
        lk::castable(&engine, p0, relic_ward()),
        "castable in the main phase with its mana floating"
    );

    lk::to_step(&mut engine, p0, Step::End);
    lk::float(
        &mut engine,
        p0,
        &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
    );
    assert!(
        !lk::castable(&engine, p0, relic_ward()),
        "not castable in the end step without flash"
    );
}
