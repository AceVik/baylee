//! `cards/creatures/mv_3/coffin_queen.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn coffin_queen() -> CardIndex {
    card_index("c19ec6aa-aadd-4767-b695-384fd07b4ded")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// p0 has a Coffin Queen and three Swamps; p1 has Grizzly Bears. p0's first
/// main phase.
fn a_queen_with_mana() -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[coffin_queen(), swamp(), swamp(), swamp()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let queen = on_battlefield(&engine, P0, coffin_queen()).unwrap();
    (engine, queen)
}

/// Walks until the untap step asks which permanents stay tapped, handing the
/// question back unanswered (`answer_one` would untap).
#[track_caller]
fn to_the_untap_question(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    for _ in 0..200 {
        if let Pending::ChooseCards {
            options,
            prompt: ChoicePrompt::LeaveTapped,
            ..
        } = engine.pending().clone()
        {
            return options;
        }
        let (player, action) = answer_one(engine).expect("a rest on the way");
        engine.apply(player, action).expect("legal");
    }
    panic!("the untap step never asked");
}

/// Taps the Queen as if she had paid a cost, and walks to p0's next untap
/// step; returns what that step offered to leave tapped.
fn tapped_queen_at_the_untap_step(
    engine: &mut Engine<RegistryLookup>,
    queen: ObjectId,
) -> Vec<ObjectId> {
    engine.dev_state_mut(P0).unwrap().set_tapped(queen, true);
    engine.refresh_offer();
    assert!(is_tapped(engine, queen));
    reach_their_main_phase(engine, P1);
    to_the_untap_question(engine)
}

/// Coffin Queen (c19ec6aa-aadd-4767-b695-384fd07b4ded) — {2}{B} Creature — Zombie Wizard (Partial):
/// "You may choose not to untap this creature during your untap step."
#[test]
fn coffin_queen_stays_tapped_when_its_controller_chooses_not_to_untap_it() {
    let (mut engine, queen) = a_queen_with_mana();

    let options = tapped_queen_at_the_untap_step(&mut engine, queen);
    assert!(options.contains(&queen), "the Queen may stay down");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![queen],
            },
        )
        .expect("leave her tapped");
    reach_main_phase(&mut engine, P0);

    assert!(is_tapped(&engine, queen));
}

#[test]
fn coffin_queen_untaps_when_its_controller_does_not_choose_to_keep_it_tapped() {
    let (mut engine, queen) = a_queen_with_mana();

    let options = tapped_queen_at_the_untap_step(&mut engine, queen);
    assert!(options.contains(&queen));
    // `answer_one` untaps everything.
    let (player, action) = answer_one(&engine).expect("the question");
    assert_eq!(player, P0);
    engine.apply(player, action).expect("untap everything");
    reach_main_phase(&mut engine, P0);

    assert!(!is_tapped(&engine, queen));
}

/// The reanimation sentence is not played: without its exile clause it would
/// keep the creature for good, so no activation is offered at all.
#[test]
fn coffin_queen_offers_no_reanimation_ability() {
    let (mut engine, queen) = a_queen_with_mana();
    let theirs = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    bury(&mut engine, &[theirs]);
    engine.refresh_offer();
    assert!(in_graveyard(&engine, P1, grizzly_bears()).is_some());

    tap_all_mana(&mut engine, P0);
    let offer = priority_offer(&engine);
    assert!(
        offer.abilities.iter().all(|&(id, _)| id != queen),
        "no activation of the Queen with {{2}}{{B}} floating and a creature card to take"
    );
    assert!(offer.unpaid_abilities.iter().all(|&(id, _, _)| id != queen));
}
