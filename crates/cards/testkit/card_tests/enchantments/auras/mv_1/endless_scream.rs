//! `cards/enchantments/auras/mv_1/endless_scream.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::counters;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn endless_scream() -> CardIndex {
    card_index("cde3e29c-31fd-4dce-a008-283bbe90bfc6")
}

/// Endless Scream (`cde3e29c-31fd-4dce-a008-283bbe90bfc6`) — {X}{B} Enchantment — Aura (Partial):
/// "Enchant creature. This Aura enters with X scream counters on it. Enchanted creature gets +1/+0 for each scream counter on this Aura."
#[test]
fn endless_scream_enters_with_x_counters_and_retains_only_the_supported_pump() {
    for x in 0..=2 {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[swamp(), swamp(), swamp(), index::GRIZZLY_BEARS])
            .hand(0, &[endless_scream()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, P0);

        let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
        tap_all_mana(&mut engine, P0);
        let spell = in_hand(&engine, P0, endless_scream()).unwrap();
        engine
            .apply(P0, PlayerAction::CastSpell { card: spell })
            .unwrap();
        engine.apply(P0, PlayerAction::ChooseNumber(x)).unwrap();
        let land = on_battlefield(&engine, P0, swamp()).unwrap();
        let menu = aim_at(&mut engine, P0, bears);
        assert!(!menu.contains(&land));
        pass_until(&mut engine, stack_is_empty);

        let aura = on_battlefield(&engine, P0, endless_scream()).unwrap();
        assert_eq!(
            engine
                .state()
                .object(aura)
                .unwrap()
                .counters
                .get(counters::SCREAM),
            u16::try_from(x).unwrap()
        );
        assert_eq!(pt(&engine, bears), if x == 0 { (2, 2) } else { (3, 2) });
    }
}
