//! `cards/instants/mv_5/command_the_storm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "1dc816b5-8918-4a0c-bb10-74abe579a98c"
fn command_the_storm() -> CardIndex {
    card_index("1dc816b5-8918-4a0c-bb10-74abe579a98c")
}

/// Command the Storm: "deals 5 damage to target creature."
///
/// Five is read at its edge: it kills a 4/5 Earth Elemental (toughness exactly
/// five), and a creature is the only kind of thing on the menu, so no player
/// is offered and none is hurt.
#[test]
fn command_the_storm_deals_five_damage_to_a_creature_only() {
    let mut engine = table(
        905,
        &[command_the_storm()],
        &[mountain(); 5],
        &[],
        &[earth_elemental()],
    );
    let big = on_battlefield(&engine, them(), earth_elemental()).expect("their Elemental");
    assert_eq!(pt(&engine, big), (4, 5), "toughness five, the exact edge");
    cast_from_hand(&mut engine, me(), command_the_storm());
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("a target choice, got {:?}", engine.pending())
    };
    assert!(
        player_options.is_empty(),
        "creatures only: {player_options:?}"
    );
    aim_at(&mut engine, me(), big);
    settle(&mut engine);
    assert!(
        in_graveyard(&engine, them(), earth_elemental()).is_some(),
        "five damage kills toughness five"
    );
    assert_eq!(life_of(&engine, them()), 20, "no damage to the player");
}
