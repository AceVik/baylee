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
}
