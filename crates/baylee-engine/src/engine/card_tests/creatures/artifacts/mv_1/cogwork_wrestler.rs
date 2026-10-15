//! `cards/creatures/artifacts/mv_1/cogwork_wrestler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn cogwork_wrestler() -> CardIndex {
    card_index("db27e3f0-5750-4c4b-9da9-70c3ab627f9c")
}

/// "Flash. When this creature enters, target creature an opponent controls
/// gets -2/-0 until end of turn." Cast on the opponent's turn, it asks for a
/// target among the opponent's creatures only.
#[test]
fn cogwork_wrestler_flashes_in_and_shrinks_an_opposing_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), grizzly_bears()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[cogwork_wrestler()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("ogre");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("bears");
    let before = pt(&engine, ogre);

    cast_from_hand(&mut engine, p0, cogwork_wrestler());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let options = aim_at(&mut engine, p0, ogre);
    assert!(options.contains(&ogre));
    assert!(!options.contains(&bears), "only opponents' creatures");
    pass_until(&mut engine, stack_is_empty);
    let wrestler = on_battlefield(&engine, p0, cogwork_wrestler()).expect("wrestler");
    assert!(!options.contains(&wrestler), "it is not on the menu");
    assert_eq!(pt(&engine, ogre), (before.0 - 2, before.1), "-2/-0");
    assert_eq!(pt(&engine, bears), (2, 2), "own creatures untouched");
    assert_eq!(pt(&engine, wrestler), (1, 2), "the printed body");
}
