//! `cards/enchantments/mv_4/aluren.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn aluren() -> CardIndex {
    card_index("44e02e70-07ab-4795-8037-55185bd11d54")
}

/// Aluren (44e02e70-07ab-4795-8037-55185bd11d54) — {2}{G}{G} Enchantment (Partial):
/// Reduces generic mana cost of creature spells with mana value 3 or less by {3} and grants creature spells flash.
#[test]
fn aluren_reduces_generic_cost_and_grants_flash_to_creature_spells() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[aluren(), forest()])
        .hand(0, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    tap_all_mana(&mut engine, P0);
    let bears = in_hand(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    engine
        .apply(P0, PlayerAction::CastSpell { card: bears })
        .expect("Aluren grants flash and reduces {1}{G} to {G}");
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());
}
