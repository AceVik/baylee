//! `cards/enchantments/mv_2/icy_prison.rs`, played.
//!
//! Oracle:
//! - "When this enchantment enters, exile target creature."
//! - "At the beginning of your upkeep, sacrifice this enchantment unless any player pays {3}."
//! - "When this enchantment leaves the battlefield, return the exiled card to the battlefield under its owner's control."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn icy_prison() -> CardIndex {
    card_index("d4e2cc1a-4212-4284-9ebe-d3c40931de50")
}

#[test]
fn icy_prison_exiles_creature_on_etb_and_returns_it_when_sacrificed_at_upkeep() {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[icy_prison()], &[]);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    lk::float(&mut engine, P0, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut engine, P0, icy_prison());
    lk::settle(&mut engine);
    // Aim the ETB trigger at bears and resolve.
    lk::aim(&mut engine, &[bears], &[]);
    lk::settle(&mut engine);
    let prison = lk::obj(&engine, P0, icy_prison());
    assert_eq!(lk::zone_of(&engine, bears), Zone::Exile);

    // Advance to P0's next upkeep and decline the {3} upkeep payment: Icy Prison is sacrificed and bears returns.
    for _ in 0..80 {
        if engine.state().turn.active == P0
            && engine.state().turn.number > 1
            && matches!(engine.state().turn.phase, Phase::FirstMain)
            && matches!(engine.pending(), Pending::Priority { .. })
        {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo { .. } => {
                lk::yes_no(&mut engine, false);
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected pending {other:?}"),
        }
    }
    assert_eq!(lk::zone_of(&engine, prison), Zone::Graveyard);
    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
