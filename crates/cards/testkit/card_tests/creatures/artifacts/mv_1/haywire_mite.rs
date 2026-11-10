//! `cards/creatures/artifacts/mv_1/haywire_mite.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn haywire_mite() -> CardIndex {
    card_index("749d2994-44e7-40d3-8630-7bebed239e9e")
}

/// "When this creature dies, you gain 2 life." Dying to damage-free
/// destruction gains exactly two for its controller.
#[test]
fn haywire_mite_gains_two_life_when_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[haywire_mite()], &[]);
    let me = on_battlefield(&engine, p0, haywire_mite()).expect("seated");
    let life = engine.state().players[0].life;
    kill(&mut engine, me);
    assert!(in_graveyard(&engine, p0, haywire_mite()).is_some());
    assert_eq!(engine.state().players[0].life, life + 2);
}

/// "{G}, Sacrifice this creature: Exile target noncreature artifact or
/// noncreature enchantment." The menu holds the Sol Ring and nothing that is
/// a creature, the Mite included; the sacrifice is the cost, so it dies and
/// its own trigger pays two life as well.
#[test]
fn haywire_mite_sacrifices_to_exile_a_noncreature_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = main_with(&[haywire_mite()], &[sol_ring(), gray_ogre()]);
    let me = on_battlefield(&engine, p0, haywire_mite()).expect("seated");
    let ring = on_battlefield(&engine, p1, sol_ring()).expect("ring");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("ogre");
    let life = engine.state().players[0].life;

    assert!(!is_offered(&engine, me, 1), "no {{G}} yet");
    float_mana(&mut engine, ManaColor::Green, 1);
    assert!(is_offered(&engine, me, 1));
    activate(&mut engine, p0, haywire_mite(), 1);
    let options = aim_at(&mut engine, p0, ring);
    assert!(options.contains(&ring));
    assert!(!options.contains(&ogre), "a creature is not a target");
    assert!(!options.contains(&me), "an artifact creature is not either");
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, sol_ring()).is_none(), "exiled");
    assert!(
        in_graveyard(&engine, p1, sol_ring()).is_none(),
        "not killed"
    );
    assert!(
        in_graveyard(&engine, p0, haywire_mite()).is_some(),
        "the sacrifice was the cost"
    );
    assert_eq!(
        engine.state().players[0].life,
        life + 2,
        "sacrificed means dead: the first ability paid out"
    );
}
