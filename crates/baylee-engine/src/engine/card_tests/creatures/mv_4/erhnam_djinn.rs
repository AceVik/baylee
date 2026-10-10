//! `cards/creatures/mv_4/erhnam_djinn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn erhnam_djinn() -> CardIndex {
    card_index("d48a38c9-3dcd-4c18-8840-1b057ede3ff0")
}

/// Passes priority until `seat`'s upkeep trigger asks for its target, and
/// returns the menu. Counts only the stops on the way: the game starts
/// before any upkeep, so the first one reached is the first turn's.
fn upkeep_targets(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseTargets { player, .. } if *player == seat),
    );
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until waited for exactly this")
    };
    options
}

/// Erhnam Djinn: "At the beginning of your upkeep, target non-Wall creature
/// an opponent controls gains forestwalk until your next upkeep."
///
/// The menu is "non-Wall creature an opponent controls": the opponent's
/// Grizzly Bears is on it, their Wall of Wood and the Djinn's controller's
/// own Llanowar Elves are not. The Bears gain forestwalk, which holds through
/// the opponent's whole turn and ends as the Djinn's controller's next upkeep
/// begins (CR 611.2b), when a new trigger asks for a target again.
#[test]
fn erhnam_djinn_gives_forestwalk_until_its_controllers_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[erhnam_djinn(), llanowar_elves(), forest()])
        .battlefield(1, &[grizzly_bears(), wall_of_wood()])
        .start();
    keep_mulligans(&mut engine);
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("the Bears are out");
    let wall = on_battlefield(&engine, p1, wall_of_wood()).expect("the Wall is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let djinn = on_battlefield(&engine, p0, erhnam_djinn()).expect("the Djinn is out");
    assert!(!keywords(&engine, bears).contains(KeywordSet::FORESTWALK));

    let options = upkeep_targets(&mut engine, p0);
    assert!(
        options.contains(&bears),
        "an opposing non-Wall: {options:?}"
    );
    assert!(!options.contains(&wall), "a Wall is not: {options:?}");
    assert!(
        !options.contains(&elves) && !options.contains(&djinn),
        "your own creatures are not: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, bears).contains(KeywordSet::FORESTWALK),
        "forestwalk granted"
    );
    assert!(!keywords(&engine, wall).contains(KeywordSet::FORESTWALK));

    // Through the opponent's turn.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        keywords(&engine, bears).contains(KeywordSet::FORESTWALK),
        "still granted during the opponent's turn"
    );

    // Our next upkeep: the grant ends as it begins, and the new trigger asks
    // again.
    let options = upkeep_targets(&mut engine, p0);
    assert!(
        !keywords(&engine, bears).contains(KeywordSet::FORESTWALK),
        "forestwalk ended as the next upkeep began"
    );
    assert!(
        options.contains(&bears),
        "and the Bears may be chosen again"
    );
}
