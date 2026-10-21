//! `cards/instants/mv_3/foxfire.rs`, played.
//!
//! Oracle:
//! - "Untap target attacking creature. Prevent all combat damage that would be dealt to and dealt by that creature this turn."
//! - "Draw a card at the beginning of the next turn's upkeep."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn foxfire() -> CardIndex {
    card_index("6c270e6a-12a7-4323-87ba-6d770986fb48")
}

#[test]
fn foxfire_untaps_attacker_prevents_its_combat_damage_and_draws_next_upkeep() {
    let mut engine = lk::game(SEED, &[index::GRIZZLY_BEARS], &[], &[foxfire()], &[]);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    let _offer = lk::attack(&mut engine, &[bears]);
    lk::block(&mut engine, &[]);
    assert!(lk::tapped(&engine, bears));

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
    );
    lk::cast_at(&mut engine, P0, foxfire(), &[bears], &[]);
    assert!(!lk::tapped(&engine, bears));

    lk::through_combat(&mut engine);
    assert_eq!(
        lk::life(&engine, P1),
        20,
        "combat damage from Grizzly Bears was prevented"
    );

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    lk::to_step(&mut engine, P0, Step::Upkeep);
    lk::settle(&mut engine);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before + 1
    );
    assert_eq!(
        engine.state().turn.active,
        P1,
        "\"the next turn's upkeep\" is the opponent's, not the caster's next one"
    );
}

/// "Prevent all combat damage that would be dealt **to and dealt by** that
/// creature": blocked by the opponent's Bears, neither creature is dealt any.
#[test]
fn foxfire_prevents_combat_damage_to_and_from_the_blocked_attacker() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::GRIZZLY_BEARS],
        &[foxfire()],
        &[],
    );
    let attacker = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let blocker = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    let offer = lk::attack(&mut engine, &[attacker]);
    assert!(lk::may_block(&offer, blocker, attacker));
    lk::block(&mut engine, &[(blocker, attacker)]);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
    );
    lk::cast_at(&mut engine, P0, foxfire(), &[attacker], &[]);
    lk::through_combat(&mut engine);

    assert_eq!(
        lk::damage_on(&engine, attacker),
        0,
        "the blocker's combat damage to it was prevented"
    );
    assert_eq!(
        lk::damage_on(&engine, blocker),
        0,
        "and its own combat damage to the blocker"
    );
    assert_eq!(lk::zone_of(&engine, attacker), Zone::Battlefield);
    assert_eq!(lk::zone_of(&engine, blocker), Zone::Battlefield);
}
