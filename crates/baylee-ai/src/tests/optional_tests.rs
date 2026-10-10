//! The beta.6 findings `optional` answers: a "may" weighed, Camouflage
//! cast, False Orders' re-block taken, and the piles split by strength.

use super::*;
use baylee_cards_dsl::{Amount, Effect, PlayerRel};
use baylee_core::ids::AbilityRef;
use baylee_engine::engine::DecisionContext;
use baylee_view::AttackerView;

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

fn creature(slot: u32, controller: PlayerId, power: i16, toughness: i16) -> PublicObject {
    let mut o = permanent(obj(slot), controller, power);
    o.toughness = Some(toughness);
    o
}

fn may(prompt_source: Option<AbilityRef>) -> Pending {
    Pending::YesNo {
        player: ME,
        prompt: YesNoPrompt::MayDo,
        source: prompt_source,
    }
}

fn sanctuary() -> AbilityRef {
    AbilityRef::new(baylee_cards::decks::by_name("Island Sanctuary").unwrap(), 0)
}

#[test]
fn island_sanctuary_draws_unless_the_ground_threatens_the_seats_life() {
    // A lone bear against twenty life: the card is worth more.
    let calm = view(0, &[20, 20], vec![creature(5, THEM, 2, 2)]);
    assert_eq!(
        agent().act(&calm, &may(Some(sanctuary()))),
        PlayerAction::YesNo(false)
    );
    // Eight power on the ground against ten life: skip and stay alive.
    let pressed = view(
        0,
        &[10, 20],
        vec![creature(5, THEM, 4, 4), creature(6, THEM, 4, 4)],
    );
    assert_eq!(
        agent().act(&pressed, &may(Some(sanctuary()))),
        PlayerAction::YesNo(true)
    );
    // The same power in the air is not stopped by the Sanctuary.
    let mut fliers = pressed.clone();
    for o in &mut fliers.battlefield {
        o.keywords |= baylee_cards_dsl::KeywordSet::FLYING.bits();
    }
    assert_eq!(
        agent().act(&fliers, &may(Some(sanctuary()))),
        PlayerAction::YesNo(false)
    );
}

#[test]
fn a_may_that_costs_the_seat_is_declined_and_a_free_gain_is_taken() {
    static LOSE: [Effect; 1] = [Effect::MayDo {
        effects: &[Effect::LoseLife {
            amount: Amount::Fixed(4),
            target: PlayerRel::You,
        }],
    }];
    static GAIN: [Effect; 1] = [Effect::MayDo {
        effects: &[Effect::GainLife {
            amount: Amount::Fixed(4),
        }],
    }];
    let v = view(0, &[20, 20], vec![creature(7, ME, 1, 1)]);
    let ask = |effects: &'static [Effect]| {
        agent().act_with_context(
            &v,
            &may(None),
            &DecisionContext {
                source: Some(obj(7)),
                effects,
                ..Default::default()
            },
        )
    };
    assert_eq!(ask(&LOSE), PlayerAction::YesNo(false));
    assert_eq!(ask(&GAIN), PlayerAction::YesNo(true));
    // Nothing to read: the old default, yes.
    assert_eq!(agent().act(&v, &may(None)), PlayerAction::YesNo(true));
}

#[test]
fn camouflage_is_worth_casting_only_once_attacking_into_a_blocker() {
    let hostile = |p: PlayerId| p != ME;
    let mut v = view(
        0,
        &[20, 20],
        vec![creature(1, ME, 2, 2), creature(5, THEM, 2, 2)],
    );
    v.active = ME;
    v.step = baylee_view::Step::DeclareAttackers;
    assert!(
        !optional::camouflage_worth(&v, hostile),
        "nothing attacks yet"
    );
    v.combat.attackers.push(AttackerView {
        creature: obj(1),
        defending: Defender::Player(THEM),
        blocked: false,
    });
    assert!(optional::camouflage_worth(&v, hostile));
    // No untapped blocker: nothing to scramble.
    v.battlefield[1].status = ObjectStatus::TAPPED;
    assert!(!optional::camouflage_worth(&v, hostile));
    // The main phase is not its step.
    v.battlefield[1].status = ObjectStatus::default();
    v.step = baylee_view::Step::Main;
    assert!(!optional::camouflage_worth(&v, hostile));
}

#[test]
fn camouflage_is_not_tapped_for_outside_its_step() {
    let spells = baylee_cards::by_index(baylee_cards::decks::by_name("Camouflage").unwrap())
        .unwrap()
        .abilities_for_face(0);
    let mut v = view(0, &[20, 20], vec![]);
    v.active = ME;
    v.step = baylee_view::Step::Main;
    assert!(optional::timing_closed(&v, spells));
    v.step = baylee_view::Step::DeclareAttackers;
    assert!(!optional::timing_closed(&v, spells));
    v.active = THEM;
    assert!(optional::timing_closed(&v, spells));
}

fn reblock_question(blocker: ObjectId, options: Vec<ObjectId>) -> Pending {
    Pending::ChooseCards {
        player: ME,
        options,
        min: 0,
        max: 1,
        prompt: ChoicePrompt::BlockWith { blocker },
        total: None,
    }
}

#[test]
fn false_orders_reblocks_an_attacker_its_creature_kills_for_free() {
    // Defending: my 3/3 is moved; a 2/2 it kills and survives is offered
    // beside a 5/5 that would kill it.
    let v = view(
        0,
        &[20, 20],
        vec![
            creature(1, ME, 3, 3),
            creature(5, THEM, 2, 2),
            creature(6, THEM, 5, 5),
        ],
    );
    assert_eq!(
        agent().act(&v, &reblock_question(obj(1), vec![obj(6), obj(5)])),
        PlayerAction::ChooseObjects {
            objects: vec![obj(5)]
        }
    );
    // Only the 5/5: no block that gains, so none.
    assert_eq!(
        agent().act(&v, &reblock_question(obj(1), vec![obj(6)])),
        PlayerAction::ChooseObjects { objects: vec![] }
    );
}

#[test]
fn false_orders_hands_their_creature_an_attacker_that_kills_it_and_was_blocked_anyway() {
    // Attacking: their 2/2 is moved. My 4/4 was blocked by something else
    // and kills it; my 1/1 was unblocked and would not.
    let mut v = view(
        0,
        &[20, 20],
        vec![
            creature(1, ME, 4, 4),
            creature(2, ME, 1, 1),
            creature(5, THEM, 2, 2),
        ],
    );
    v.combat.attackers = vec![
        AttackerView {
            creature: obj(1),
            defending: Defender::Player(THEM),
            blocked: true,
        },
        AttackerView {
            creature: obj(2),
            defending: Defender::Player(THEM),
            blocked: false,
        },
    ];
    assert_eq!(
        agent().act(&v, &reblock_question(obj(5), vec![obj(2), obj(1)])),
        PlayerAction::ChooseObjects {
            objects: vec![obj(1)]
        }
    );
    // Unblocked, the 4/4's damage was getting through: declined.
    v.combat.attackers[0].blocked = false;
    assert_eq!(
        agent().act(&v, &reblock_question(obj(5), vec![obj(2), obj(1)])),
        PlayerAction::ChooseObjects { objects: vec![] }
    );
}

#[test]
fn raging_river_labels_the_side_that_stops_the_attacker_least() {
    // Two 1/1s (more creatures) against one 4/4 that kills my 3/3: the
    // side to name is the 1/1s', wherever it stands.
    let v = view(
        0,
        &[20, 20],
        vec![
            creature(1, ME, 3, 3),
            creature(5, THEM, 1, 1),
            creature(6, THEM, 1, 1),
            creature(7, THEM, 4, 4),
        ],
    );
    let ask = |piles: Vec<Vec<ObjectId>>| {
        agent().act(
            &v,
            &Pending::ChoosePile {
                player: ME,
                piles,
                label: Some(obj(1)),
            },
        )
    };
    assert_eq!(
        ask(vec![vec![obj(7)], vec![obj(5), obj(6)]]),
        PlayerAction::ChooseMode(1)
    );
    assert_eq!(
        ask(vec![vec![obj(5), obj(6)], vec![obj(7)]]),
        PlayerAction::ChooseMode(0)
    );
}

#[test]
fn raging_river_keeps_the_best_blockers_on_different_sides() {
    let v = view(
        1,
        &[20, 20],
        vec![
            creature(5, THEM, 1, 1),
            creature(6, THEM, 5, 5),
            creature(7, THEM, 1, 1),
            creature(8, THEM, 4, 4),
        ],
    );
    let PlayerAction::ChooseObjects { objects } = agent().act(
        &v,
        &Pending::ChooseCards {
            player: THEM,
            options: vec![obj(5), obj(6), obj(7), obj(8)],
            min: 0,
            max: 4,
            prompt: ChoicePrompt::LeftPile,
            total: None,
        },
    ) else {
        panic!("a pile");
    };
    assert!(
        objects.contains(&obj(6)) != objects.contains(&obj(8)),
        "the 5/5 and the 4/4 are split: {objects:?}"
    );
}

#[test]
fn camouflage_piles_each_get_one_of_the_best_blockers() {
    let v = view(
        1,
        &[20, 20],
        vec![
            creature(5, THEM, 1, 1),
            creature(6, THEM, 5, 5),
            creature(7, THEM, 1, 1),
            creature(8, THEM, 4, 4),
        ],
    );
    let PlayerAction::ChooseObjects { objects } = agent().act(
        &v,
        &Pending::ChooseCards {
            player: THEM,
            options: vec![obj(5), obj(6), obj(7), obj(8)],
            min: 0,
            max: 4,
            prompt: ChoicePrompt::CamouflagePile { pile: 1, of: 2 },
            total: None,
        },
    ) else {
        panic!("a pile");
    };
    assert_eq!(objects, vec![obj(6), obj(5)], "the best and a third");
}
