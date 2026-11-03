//! `cards/creatures/mv_3/goblin_warchief.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn goblin_warchief_reduces_goblin_spell_cost_and_grants_goblins_haste() {
    let warchief = card_index("39882df0-c20f-469d-94ba-1617224e71a1");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[warchief, index::GOBLIN_SKY_RAIDER])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Spend 3 Mountains on Goblin Warchief ({1}{R}{R}); 2 Mountains remain for Goblin Sky Raider ({2}{R} -> {1}{R}).
    let mountains = lands_of(&engine, p0);
    for &m in &mountains[..3] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: m })
            .unwrap();
    }
    cast_with_floating(&mut engine, p0, warchief);
    pass_until(&mut engine, stack_is_empty);

    let warchief_id = on_battlefield(&engine, p0, warchief).unwrap();
    assert!(
        keywords(&engine, warchief_id).contains(KeywordSet::HASTE),
        "Goblin Warchief grants itself haste"
    );

    for &m in &mountains[3..5] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: m })
            .unwrap();
    }
    cast_with_floating(&mut engine, p0, index::GOBLIN_SKY_RAIDER);
    pass_until(&mut engine, stack_is_empty);

    let raider_id = on_battlefield(&engine, p0, index::GOBLIN_SKY_RAIDER).unwrap();
    assert!(
        keywords(&engine, raider_id).contains(KeywordSet::HASTE),
        "Goblin Sky Raider enters with haste from Goblin Warchief"
    );
}

/// The reduction is in the offer before a land is tapped, which is when the
/// planners price a cast (NEW-03, 3.11.2026). The wizard always charged
/// `{1}{R}` for the Sky Raider under the Warchief; `LegalActions` published
/// only increases, so every planner priced the printed `{2}{R}`: a Goblin two
/// Mountains could pay was not offered, and one that was tapped three.
#[test]
fn goblin_warchief_publishes_its_reduction_to_the_planners() {
    let warchief = card_index("39882df0-c20f-469d-94ba-1617224e71a1");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[warchief, mountain(), mountain()])
        .hand(0, &[index::GOBLIN_SKY_RAIDER, index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let raider = in_hand(&engine, p0, index::GOBLIN_SKY_RAIDER).unwrap();
    let bolt = in_hand(&engine, p0, index::LIGHTNING_BOLT).unwrap();
    let offer = priority_offer(&engine);
    assert_eq!(offer.spell_reduction(raider, CastModeKind::Normal), 1);
    assert_eq!(
        offer.spell_increase(raider, CastModeKind::Normal),
        0,
        "a reduction is never folded into the increases (protocol 11)"
    );
    assert_eq!(
        offer.spell_reduction(bolt, CastModeKind::Normal),
        0,
        "only Goblin spells cost less"
    );
    assert!(
        offer.payable.contains(&raider),
        "two Mountains pay the reduced {{1}}{{R}}"
    );
    assert!(
        !engine
            .compute_legal(p1)
            .spell_reductions
            .iter()
            .any(|(id, _, _)| *id == raider),
        "the other seat's offer names none of this hand"
    );

    // And the price is the one the wizard charges: both Mountains, nothing left.
    for m in lands_of(&engine, p0) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: m })
            .unwrap();
    }
    cast_with_floating(&mut engine, p0, index::GOBLIN_SKY_RAIDER);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
