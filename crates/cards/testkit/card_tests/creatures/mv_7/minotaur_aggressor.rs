//! `cards/creatures/mv_7/minotaur_aggressor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn minotaur_aggressor() -> CardIndex {
    card_index("7576bfba-eb4f-4d3e-aec5-52daee84d1d6")
}

/// Minotaur Aggressor — a 6/2 with first strike and haste for {6}{R}: cast in
/// the main phase it is offered as an attacker at once.
#[test]
fn minotaur_aggressor_has_haste() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&lots(mountain(), 7), &[minotaur_aggressor()], &[]);
    let minotaur = cast_and_resolve(&mut engine, minotaur_aggressor());
    assert_body(
        &engine,
        minotaur,
        (6, 2),
        KeywordSet::FIRST_STRIKE.union(KeywordSet::HASTE),
    );
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the pass stopped here")
    };
    assert!(attackers.contains(&minotaur));
}

/// First strike: blocked by a 2/2 Ogre, which would kill a toughness-2
/// creature, the Minotaur kills it first and takes no damage.
#[test]
fn minotaur_aggressor_kills_a_blocker_before_it_hits_back() {
    let (mut engine, minotaur, ogre) = duel_vs_ogre(minotaur_aggressor(), &[]);
    fight(&mut engine, &[minotaur], &[(ogre, minotaur)]);
    assert!(in_graveyard(&engine, PlayerId::new(1), gray_ogre()).is_some());
    assert!(
        on_battlefield(&engine, PlayerId::new(0), minotaur_aggressor()).is_some(),
        "first strike: the Ogre dealt no damage"
    );
}
