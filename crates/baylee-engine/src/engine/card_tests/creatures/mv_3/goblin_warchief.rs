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
