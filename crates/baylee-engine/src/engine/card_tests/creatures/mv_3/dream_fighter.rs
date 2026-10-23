//! `cards/creatures/mv_3/dream_fighter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn dream_fighter() -> CardIndex {
    card_index("ea8d1c55-e045-4a3f-92f0-4ecd86362b97")
}

fn phased_out(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .status
        .contains(Status::PHASED_OUT)
}

/// Dream Fighter (ea8d1c55-e045-4a3f-92f0-4ecd86362b97) — {2}{U} 1/1:
/// "Whenever this creature blocks or becomes blocked by a creature, this
/// creature and that creature phase out."
///
/// Dream Fighter attacks and the Bears block: both phase out, a bystander
/// (Hill Giant, in no combat) does not, and each comes back in its own
/// controller's untap step.
#[test]
fn dream_fighter_and_the_creature_blocking_it_phase_out() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[dream_fighter()])
        .battlefield(1, &[GRIZZLY_BEARS, HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);

    let fighter = on_battlefield(&engine, P0, dream_fighter()).unwrap();
    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(fighter, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, fighter)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(phased_out(&engine, fighter), "Dream Fighter phases out");
    assert!(phased_out(&engine, bears), "and so does the blocker");
    assert!(
        !phased_out(&engine, giant),
        "a creature in no combat stays in"
    );

    // P1's untap step (turn 2): the Bears phase in, with their controller.
    reach_their_main_phase(&mut engine, P1);
    assert!(
        !phased_out(&engine, bears),
        "the Bears phase in at P1's untap"
    );
    assert!(
        phased_out(&engine, fighter),
        "Dream Fighter waits for its own controller's untap step"
    );

    // P0's untap step (turn 3): Dream Fighter phases in.
    reach_their_main_phase(&mut engine, P0);
    assert!(!phased_out(&engine, fighter));
}

/// The same trigger from the other side: Dream Fighter blocks the attacking
/// Bears, and both phase out. Hill Giant, not in combat, stays in.
#[test]
fn dream_fighter_and_the_creature_it_blocks_phase_out() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[dream_fighter()])
        .battlefield(1, &[GRIZZLY_BEARS, HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);

    let fighter = on_battlefield(&engine, P0, dream_fighter()).unwrap();
    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(fighter, bears)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(phased_out(&engine, fighter), "Dream Fighter phases out");
    assert!(
        phased_out(&engine, bears),
        "and so does the creature it blocks"
    );
    assert!(!phased_out(&engine, giant));
}
