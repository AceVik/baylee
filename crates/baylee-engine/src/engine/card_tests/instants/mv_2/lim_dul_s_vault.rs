//! `cards/instants/mv_2/lim_dul_s_vault.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn lim_dul_s_vault() -> CardIndex {
    card_index("3f8e7a45-4c6e-4ee6-93d0-b7de9715ec97")
}

/// Lim-Dûl's Vault ({U}{B} Instant, Partial):
/// Looks at the top five cards of your library and puts them back on top in any order.
#[test]
fn lim_dul_s_vault_reorders_top_five_cards_of_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), swamp()])
        .hand(0, &[lim_dul_s_vault()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, lim_dul_s_vault());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });

    let Pending::Arrange { cards, prompt, .. } = engine.pending().clone() else {
        panic!("expected Arrange");
    };
    assert_eq!(prompt, ArrangePrompt::Order);
    assert_eq!(cards.len(), 5);

    let mut reversed = cards.clone();
    reversed.reverse();
    engine
        .apply(
            p0,
            PlayerAction::Arrange {
                piles: vec![reversed.clone()],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let after = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let new_top: Vec<ObjectId> = after.iter().rev().take(5).copied().collect();
    assert_eq!(new_top, reversed);
}
