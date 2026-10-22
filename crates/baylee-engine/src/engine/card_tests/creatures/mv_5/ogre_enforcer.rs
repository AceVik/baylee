//! `cards/creatures/mv_5/ogre_enforcer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn ogre_enforcer() -> CardIndex {
    card_index("86cc83d8-4a70-4b97-aa4d-160afd4cd6cb")
}

/// `Ogre Enforcer` ({3}{R}{R}, 4/4 Creature — Ogre, `Coverage::Partial`) enters
/// the battlefield as a 4/4 Ogre.
#[test]
fn ogre_enforcer_enters_as_4_4_ogre() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[ogre_enforcer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, ogre_enforcer());
    pass_until(&mut engine, stack_is_empty);

    let ogre = on_battlefield(&engine, p0, ogre_enforcer()).unwrap();
    assert_eq!(pt(&engine, ogre), (4, 4));
}
