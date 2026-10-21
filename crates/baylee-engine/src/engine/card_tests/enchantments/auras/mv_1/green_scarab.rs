//! `cards/enchantments/auras/mv_1/green_scarab.rs`, played.
//!
//! Oracle:
//! - "Enchant creature"
//! - "Enchanted creature can't be blocked by green creatures."
//! - "Enchanted creature gets +2/+2 as long as an opponent controls a green permanent."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn green_scarab() -> CardIndex {
    card_index("c5ca2df3-6fd9-478d-9bc6-48695a4600fb")
}

#[test]
fn green_scarab_grants_green_evasion_and_conditional_stat_boost() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::GRIZZLY_BEARS],
        &[green_scarab()],
        &[],
    );
    let runner = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let green_blocker = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::White, 1)]);
    lk::cast_at(&mut engine, P0, green_scarab(), &[runner], &[]);
    assert_eq!(power_of(&engine, runner), Some(4));

    let offer = lk::attack(&mut engine, &[runner]);
    assert!(
        !lk::may_block(&offer, green_blocker, runner),
        "green creature cannot block a creature enchanted by Green Scarab"
    );
}

/// The other side of both sentences: against an opponent whose only
/// permanent is a red Hill Giant, the enchanted creature gets nothing (the
/// green Bears it enchants are its controller's own, not an opponent's), and
/// a creature that is not green may block it.
#[test]
fn green_scarab_gives_nothing_without_an_opposing_green_permanent() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[green_scarab()],
        &[],
    );
    let runner = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let red_blocker = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(&mut engine, P0, &[(ManaColor::White, 1)]);
    lk::cast_at(&mut engine, P0, green_scarab(), &[runner], &[]);
    let scarab = lk::obj(&engine, P0, green_scarab());
    assert_eq!(
        engine.state().object(scarab).and_then(|o| o.attached_to),
        Some(runner),
        "Green Scarab enchants the Bears"
    );
    assert_eq!(
        power_of(&engine, runner),
        Some(2),
        "no opponent controls a green permanent"
    );

    let offer = lk::attack(&mut engine, &[runner]);
    assert!(
        lk::may_block(&offer, red_blocker, runner),
        "a creature that is not green may block it"
    );
}
