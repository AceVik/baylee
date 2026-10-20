//! `cards/enchantments/mv_2/freyalise_s_charm.rs`, played.
//!
//! Oracle:
//! - "Whenever an opponent casts a black spell, you may pay {G}{G}. If you do, you draw a card."
//! - "{G}{G}: Return this enchantment to its owner's hand."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn freyalises_charm() -> CardIndex {
    card_index("e0d638d2-2c5b-415c-95e7-74bf8d88794f")
}

#[test]
fn freyalises_charm_draws_on_opponent_black_spell_for_gg_and_bounces_self() {
    let mut engine = lk::game(SEED, &[freyalises_charm()], &[], &[], &[index::DARK_RITUAL]);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();

    lk::float(&mut engine, P0, &[(ManaColor::Green, 4)]);
    lk::float(&mut engine, P1, &[(ManaColor::Black, 1)]);
    lk::pass_once(&mut engine);
    cast_with_floating(&mut engine, P1, index::DARK_RITUAL);

    for _ in 0..20 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            other => panic!("unexpected pending {other:?}"),
        }
    }

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before + 1,
        "paying {{G}}{{G}} when opponent casts a black spell draws a card"
    );

    activate(&mut engine, P0, freyalises_charm(), 1);
    lk::settle(&mut engine);

    assert!(in_hand(&engine, P0, freyalises_charm()).is_some());
}
