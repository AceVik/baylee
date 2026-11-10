//! `cards/sorceries/mv_6/devils_playground.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "ab9e2045-860a-40ab-bec1-0dffced003ef"
fn devils_playground() -> CardIndex {
    card_index("ab9e2045-860a-40ab-bec1-0dffced003ef")
}

/// Devils' Playground: "Create four 1/1 red Devil creature tokens. They have
/// 'When this token dies, it deals 1 damage to any target.'"
///
/// Four 1/1 creatures; then one is killed and its trigger asks for a target:
/// either player or any creature is on the menu, and the one named takes one.
#[test]
fn devils_playground_makes_four_devils_whose_death_pings() {
    let mut engine = table(
        955,
        &[devils_playground()],
        &[mountain(); 6],
        &[],
        &[gray_ogre()],
    );
    cast_plain(&mut engine, devils_playground());
    let devils = my_tokens(&engine);
    assert_eq!(devils.len(), 4, "four tokens");
    for devil in &devils {
        assert_eq!(pt(&engine, *devil), (1, 1));
        assert!(types(&engine, *devil).contains(TypeSet::CREATURE));
    }

    bury(&mut engine, &devils[..1]);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    let options = pass_until_targets(&mut engine, me());
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        unreachable!("pass_until_targets stopped on exactly this")
    };
    assert!(
        player_options.contains(&me()) && player_options.contains(&them()),
        "\"any target\": {player_options:?}"
    );
    assert!(!options.is_empty(), "and creatures too");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .expect("a player was on the menu");
    settle(&mut engine);
    assert_eq!(life_of(&engine, them()), 19, "one damage");
    assert_eq!(my_tokens(&engine).len(), 3, "the other three stand");
}
