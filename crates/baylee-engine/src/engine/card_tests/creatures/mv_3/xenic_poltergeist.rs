//! `cards/creatures/mv_3/xenic_poltergeist.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

use crate::turn::Step;

/// Xenic Poltergeist: "{T}: Until your next upkeep, target noncreature
/// artifact becomes an artifact creature with power and toughness each equal
/// to its mana value."
fn xenic_poltergeist() -> CardIndex {
    card_index("fb8f80cc-6214-4ab9-a9a9-1873ab9feb0c")
}

fn copper_myr() -> CardIndex {
    card_index("8b52f30c-5e38-4333-88ab-901b37105b36")
}

fn icy_manipulator() -> CardIndex {
    card_index("3608f1f7-8dc5-4dd1-ae91-c830e1de9529")
}

fn is_creature(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    types(engine, id).contains(baylee_core::types::TypeSet::CREATURE)
}

fn board() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                xenic_poltergeist(),
                sol_ring(),
                icy_manipulator(),
                copper_myr(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    engine
}

/// The menu holds noncreature artifacts only; the {4} artifact becomes a 4/4
/// artifact creature, and the animation holds through the opponent's whole
/// turn and ends as p0's next upkeep begins.
#[test]
fn xenic_poltergeist_animates_a_noncreature_artifact_until_your_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board();
    let icy = on_battlefield(&engine, p0, icy_manipulator()).expect("Icy");
    let ring = on_battlefield(&engine, p0, sol_ring()).expect("Sol Ring");
    let jug = on_battlefield(&engine, p0, copper_myr()).expect("Copper Myr");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("Bears");
    let poltergeist = on_battlefield(&engine, p0, xenic_poltergeist()).expect("Poltergeist");
    assert!(!is_creature(&engine, icy));

    activate(&mut engine, p0, xenic_poltergeist(), 0);
    let options = aim_at(&mut engine, p0, icy);
    assert!(
        options.contains(&icy) && options.contains(&ring),
        "noncreature artifacts: {options:?}"
    );
    assert!(
        !options.contains(&jug) && !options.contains(&bears) && !options.contains(&poltergeist),
        "no creatures, artifact or not: {options:?}"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, poltergeist), "{{T}} was paid");
    assert!(is_creature(&engine, icy), "now a creature");
    assert!(
        types(&engine, icy).contains(baylee_core::types::TypeSet::ARTIFACT),
        "and still an artifact"
    );
    assert_eq!(pt(&engine, icy), (4, 4), "its mana value, twice");
    assert!(
        !is_creature(&engine, ring),
        "the other artifact is unchanged"
    );

    // Through the opponent's turn, to the last step before our upkeep.
    reach_their_main_phase(&mut engine, p1);
    assert!(is_creature(&engine, icy), "still animated on their turn");
    assert_eq!(pt(&engine, icy), (4, 4));
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::End
    });
    assert!(
        is_creature(&engine, icy),
        "still animated at their end step"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::Upkeep
    });
    assert!(!is_creature(&engine, icy), "ended as our upkeep began");
}

/// Sol Ring (mana value 1) is a 1/1 creature.
#[test]
fn xenic_poltergeist_makes_sol_ring_a_one_one() {
    let p0 = PlayerId::new(0);
    let mut engine = board();
    let ring = on_battlefield(&engine, p0, sol_ring()).expect("Sol Ring");
    activate(&mut engine, p0, xenic_poltergeist(), 0);
    aim_at(&mut engine, p0, ring);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_creature(&engine, ring));
    assert_eq!(pt(&engine, ring), (1, 1));
}
