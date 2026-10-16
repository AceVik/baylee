//! `cards/instants/mv_5/breath_of_malfegor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "81e6ade9-a38f-4655-b09c-9ca41a11face"
fn breath_of_malfegor() -> CardIndex {
    card_index("81e6ade9-a38f-4655-b09c-9ca41a11face")
}

/// Breath of Malfegor: "deals 5 damage to each opponent."
///
/// No target is named (nothing is asked), the opponent loses five, the caster
/// nothing.
#[test]
fn breath_of_malfegor_deals_five_to_each_opponent_and_asks_nothing() {
    let mut engine = table(
        904,
        &[breath_of_malfegor()],
        &[swamp(), swamp(), swamp(), swamp(), mountain()],
        &[],
        &[],
    );
    let (mine, theirs) = (life_of(&engine, me()), life_of(&engine, them()));
    cast_from_hand(&mut engine, me(), breath_of_malfegor());
    assert!(
        !matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "\"each opponent\" is not a target: {:?}",
        engine.pending()
    );
    settle(&mut engine);
    assert_eq!(life_of(&engine, them()), theirs - 5);
    assert_eq!(life_of(&engine, me()), mine, "the caster is unharmed");
}
