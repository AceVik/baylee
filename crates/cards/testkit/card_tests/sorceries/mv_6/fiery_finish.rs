//! `cards/sorceries/mv_6/fiery_finish.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "afa3eb61-624c-4abe-bf1e-58fa3b4c14a6"
fn fiery_finish() -> CardIndex {
    card_index("afa3eb61-624c-4abe-bf1e-58fa3b4c14a6")
}

/// Fiery Finish: "deals 7 damage to target creature."
#[test]
fn fiery_finish_deals_seven_to_a_creature() {
    let mut engine = table(
        958,
        &[fiery_finish()],
        &[mountain(); 6],
        &[],
        &[earth_elemental()],
    );
    let big = on_battlefield(&engine, them(), earth_elemental()).expect("their Elemental");
    let journal = engine.journal().entries().len();
    cast_from_hand(&mut engine, me(), fiery_finish());
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("a target choice, got {:?}", engine.pending())
    };
    assert!(
        player_options.is_empty(),
        "creatures only: {player_options:?}"
    );
    aim_at(&mut engine, me(), big);
    settle(&mut engine);
    let hits: Vec<_> = damage_events(&engine, journal)
        .into_iter()
        .filter(|(_, _, amount, _)| *amount > 0)
        .collect();
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].2, 7, "seven damage");
    assert!(in_graveyard(&engine, them(), earth_elemental()).is_some());
}
