//! `cards/creatures/artifacts/mv_2/myr_sire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn myr_sire() -> CardIndex {
    card_index("25794033-bd17-4056-b116-68d3acf52569")
}

/// Myr Sire: a 1/1; "when this creature dies, create a 1/1 colorless
/// Phyrexian Myr artifact creature token". No token while it lives; one 1/1
/// colorless artifact creature Phyrexian Myr after.
#[test]
fn myr_sire_leaves_a_one_one_phyrexian_myr_token_when_it_dies() {
    use baylee_core::generated::subtypes::creature;
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[myr_sire()], &[]);
    let sire = on_battlefield(&engine, p0, myr_sire()).expect("seated");
    assert_eq!(pt(&engine, sire), (1, 1));
    assert!(tokens_of(&engine, p0).is_empty());
    kill(&mut engine, sire);
    assert!(in_graveyard(&engine, p0, myr_sire()).is_some());
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "exactly one token");
    let token = tokens[0];
    assert_eq!(pt(&engine, token), (1, 1));
    let chars = engine.state().object(token).unwrap().characteristics();
    assert!(chars.types.contains(TypeSet::ARTIFACT));
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(chars.colors.is_empty(), "colorless");
    assert!(chars.subtypes.contains(creature::PHYREXIAN));
    assert!(chars.subtypes.contains(creature::MYR));
}

/// The token is a creature in its own right: it attacks next turn for one.
#[test]
fn myr_sire_token_is_a_real_creature_that_attacks() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[myr_sire()], &[]);
    let sire = on_battlefield(&engine, p0, myr_sire()).expect("seated");
    kill(&mut engine, sire);
    let token = tokens_of(&engine, p0)[0];
    pass_until(&mut engine, |e| {
        e.state().turn.number > 2
            && e.state().turn.active == p0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { .. })
    });
    swing_unblocked(&mut engine, token);
    assert_eq!(life_of(&engine, 1), 19);
}
