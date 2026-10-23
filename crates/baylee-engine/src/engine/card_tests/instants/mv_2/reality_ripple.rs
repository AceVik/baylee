//! `cards/instants/mv_2/reality_ripple.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "504d5c29-7c37-4c31-8549-2a49eeef74c8"
fn reality_ripple() -> CardIndex {
    card_index("504d5c29-7c37-4c31-8549-2a49eeef74c8")
}

#[test]
fn reality_ripple_phases_out_target_creature() {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[reality_ripple()], &[]);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
    );
    lk::cast_at(&mut engine, P0, reality_ripple(), &[bears], &[]);

    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .status
            .contains(crate::object::Status::PHASED_OUT)
    );
}

/// "Phases in before its controller untaps during their next untap step": the
/// opponent's Bears are back at the opponent's untap step, not at the caster's.
#[test]
fn reality_ripple_phases_the_creature_in_again_at_its_controllers_untap_step() {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[reality_ripple()], &[]);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
    );
    lk::cast_at(&mut engine, P0, reality_ripple(), &[bears], &[]);
    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .status
            .contains(crate::object::Status::PHASED_OUT)
    );

    reach_their_main_phase(&mut engine, P1);
    assert!(
        !engine
            .state()
            .object(bears)
            .unwrap()
            .status
            .contains(crate::object::Status::PHASED_OUT),
        "phased in at its controller's next untap step"
    );
}

/// "Target artifact, creature, or land": the menu holds those three and no
/// enchantment.
#[test]
fn reality_ripple_offers_artifacts_creatures_and_lands_but_not_enchantments() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[
            index::GRIZZLY_BEARS,
            index::JAYEMDAE_TOME,
            plains(),
            index::BAD_MOON,
        ],
        &[reality_ripple()],
        &[],
    );
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let tome = lk::obj(&engine, P1, index::JAYEMDAE_TOME);
    let land = lk::obj(&engine, P1, plains());
    let moon = lk::obj(&engine, P1, index::BAD_MOON);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
    );
    cast_with_floating(&mut engine, P0, reality_ripple());
    let menu = aim_at(&mut engine, P0, tome);
    assert!(menu.contains(&bears) && menu.contains(&tome) && menu.contains(&land));
    assert!(
        !menu.contains(&moon),
        "an enchantment is not a legal target"
    );
}
