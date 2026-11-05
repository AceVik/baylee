//! Buyback and multikicker (CR 702.27a, 702.33c): the two optional costs the
//! house answers out of what it floated, and the planner's half of each.

use super::*;
use baylee_core::types::SubtypeSet;
use baylee_engine::choice::{BuybackParts, LegalActions, NumberPrompt};
use baylee_engine::engine::DecisionContext;

const ME: PlayerId = PlayerId::new(0);

fn the_question(mana: &str, parts: BuybackParts) -> Pending {
    Pending::YesNo {
        player: ME,
        prompt: YesNoPrompt::Buyback {
            cost: mana.parse().unwrap(),
            parts,
        },
        source: None,
    }
}

fn answer(v: &PlayerView, question: &Pending, cost: &str) -> PlayerAction {
    let context = DecisionContext {
        source: Some(obj(1)),
        cost: Some(cost.parse().unwrap()),
        ..Default::default()
    };
    agent().act_with_context(v, question, &context)
}

fn land(slot: u32, subtype: baylee_core::ids::SubtypeId) -> PublicObject {
    let mut object = permanent(obj(slot), ME, 0);
    object.types = TypeSet::LAND;
    object.subtypes = {
        let mut set = SubtypeSet::EMPTY;
        set.insert(subtype);
        set
    };
    object.power = None;
    object.toughness = None;
    object
}

fn swamps(slots: std::ops::Range<u32>) -> Vec<PublicObject> {
    slots
        .map(|slot| land(slot, baylee_core::generated::subtypes::land::SWAMP))
        .collect()
}

/// Mana: a yes is the pool covering both prices, because what the planner
/// floated is what it decided to spend.
#[test]
fn a_mana_buyback_is_paid_out_of_what_is_floating() {
    let mut v = view(0, &[20, 20], vec![]);
    v.hand = vec![hand_card(1, "Capsize")];
    let question = the_question("{3}", BuybackParts::default());
    v.seats[0].mana_pool.blue = 6;
    assert_eq!(
        answer(&v, &question, "{1}{U}{U}"),
        PlayerAction::YesNo(true)
    );
    v.seats[0].mana_pool.blue = 5;
    assert_eq!(
        answer(&v, &question, "{1}{U}{U}"),
        PlayerAction::YesNo(false),
        "one short, a yes is a cast the engine has to reverse"
    );
}

/// Life goes while plenty stays.
#[test]
fn life_is_paid_only_while_plenty_stays() {
    let parts = BuybackParts {
        life: 4,
        ..BuybackParts::default()
    };
    let question = the_question("{0}", parts);
    let mut v = view(0, &[14, 20], vec![]);
    v.hand = vec![hand_card(1, "Slaughter")];
    v.seats[0].mana_pool.black = 4;
    assert_eq!(
        answer(&v, &question, "{2}{B}{B}"),
        PlayerAction::YesNo(true)
    );
    let mut v = view(0, &[13, 20], vec![]);
    v.hand = vec![hand_card(1, "Slaughter")];
    v.seats[0].mana_pool.black = 4;
    assert_eq!(
        answer(&v, &question, "{2}{B}{B}"),
        PlayerAction::YesNo(false),
        "thirteen life does not buy a Slaughter back at nine"
    );
}

/// A land is a price only once there are more than the seat needs.
#[test]
fn a_land_is_sacrificed_only_from_a_surplus() {
    let parts = BuybackParts {
        sacrifices: 1,
        ..BuybackParts::default()
    };
    let question = the_question("{0}", parts);
    let mut v = view(0, &[20, 20], swamps(20..25));
    v.hand = vec![hand_card(1, "Reaping the Rewards")];
    v.seats[0].mana_pool.white = 1;
    assert_eq!(answer(&v, &question, "{W}"), PlayerAction::YesNo(false));
    let mut v = view(0, &[20, 20], swamps(20..26));
    v.hand = vec![hand_card(1, "Reaping the Rewards")];
    v.seats[0].mana_pool.white = 1;
    assert_eq!(answer(&v, &question, "{W}"), PlayerAction::YesNo(true));
}

/// Cards go only out of the lands the seat will not play this turn.
#[test]
fn a_discard_takes_spare_lands_and_nothing_else() {
    let parts = BuybackParts {
        discards: 2,
        ..BuybackParts::default()
    };
    let question = the_question("{0}", parts);
    let mut v = view(0, &[20, 20], vec![]);
    v.seats[0].mana_pool.blue = 3;
    v.hand = vec![
        hand_card(1, "Forbid"),
        hand_card(2, "Island"),
        hand_card(3, "Island"),
    ];
    assert_eq!(
        answer(&v, &question, "{1}{U}{U}"),
        PlayerAction::YesNo(false),
        "two lands is one to play and one to spare"
    );
    v.hand.push(hand_card(4, "Island"));
    assert_eq!(
        answer(&v, &question, "{1}{U}{U}"),
        PlayerAction::YesNo(true)
    );
}

/// A random discard is asked only where it mostly finds a spare land.
#[test]
fn a_random_discard_needs_spare_cards_to_land_on() {
    let parts = BuybackParts {
        life: 3,
        random_discards: 1,
        ..BuybackParts::default()
    };
    let question = the_question("{0}", parts);
    let mut v = view(0, &[20, 20], vec![]);
    v.seats[0].mana_pool.red = 4;
    v.hand = vec![
        hand_card(1, "Flowstone Flood"),
        hand_card(2, "Capsize"),
        hand_card(3, "Anoint"),
    ];
    assert_eq!(
        answer(&v, &question, "{3}{R}"),
        PlayerAction::YesNo(false),
        "either of those is a real card"
    );
    v.hand = vec![
        hand_card(1, "Flowstone Flood"),
        hand_card(2, "Mountain"),
        hand_card(3, "Mountain"),
    ];
    assert_eq!(
        answer(&v, &question, "{3}{R}"),
        PlayerAction::YesNo(true),
        "one land to play, one to spare, and two cards to hit"
    );
}

/// The planner's half, as for a kicker: Lab Rats is castable off the {B}
/// floating, and five Swamps untapped pay {4} too, so a land is tapped first.
/// With nothing else to spend the mana on the card is worth having back; with
/// a two-drop in hand it is not, and the Rats are cast as they are.
#[test]
fn buyback_is_floated_only_while_nothing_else_wants_the_mana() {
    let priority = |lands: std::ops::Range<u32>| Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            can_pass: true,
            castable: vec![obj(1)],
            mana_abilities: lands.map(obj).collect(),
            ..Default::default()
        }),
    };
    let mut v = view(0, &[20, 20], swamps(20..24));
    v.phase = baylee_view::Phase::FirstMain;
    v.hand = vec![hand_card(1, "Lab Rats")];
    v.seats[0].mana_pool.black = 1;
    for (name, profile) in PROFILES {
        let action = HeuristicAgent::new(profile).act(&v, &priority(20..24));
        assert!(
            matches!(action, PlayerAction::ActivateManaAbility { .. }),
            "{name} cast with the base cost floating and {{4}} untapped: {action:?}",
        );
    }
    v.hand.push(hand_card(2, "Grizzly Bears"));
    for (name, profile) in PROFILES {
        let action = HeuristicAgent::new(profile).act(&v, &priority(20..24));
        assert_eq!(
            action,
            PlayerAction::CastSpell { card: obj(1) },
            "{name} spent the Bears' mana on a buyback",
        );
    }
}

/// Memory Crystal's {2} comes off what the planner aims for: {4} less two is
/// what two Swamps can reach.
#[test]
fn the_planner_prices_buyback_as_the_engine_will() {
    let priority = |less: u32| Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            can_pass: true,
            castable: vec![obj(1)],
            mana_abilities: vec![obj(20), obj(21)],
            buyback_less: less,
            ..Default::default()
        }),
    };
    let mut v = view(0, &[20, 20], swamps(20..22));
    v.phase = baylee_view::Phase::FirstMain;
    v.hand = vec![hand_card(1, "Lab Rats")];
    v.seats[0].mana_pool.black = 1;
    assert_eq!(
        agent().act(&v, &priority(0)),
        PlayerAction::CastSpell { card: obj(1) },
        "two Swamps do not reach {{4}}"
    );
    assert!(matches!(
        agent().act(&v, &priority(2)),
        PlayerAction::ActivateManaAbility { .. }
    ));
}

/// The count is as far as the floating pool reaches, whatever the engine
/// leaves open: {1} and three {1}{G} are the seven floating, and a fourth
/// would be a cast reversed.
#[test]
fn multikicker_is_kicked_as_far_as_the_pool_reaches() {
    let question = Pending::ChooseNumber {
        player: ME,
        min: 0,
        max: 50,
        reason: NumberPrompt::Multikicker {
            cost: "{1}{G}".parse().unwrap(),
        },
    };
    let context = DecisionContext {
        source: Some(obj(1)),
        cost: Some("{1}".parse().unwrap()),
        ..Default::default()
    };
    let mut v = view(0, &[20, 20], vec![]);
    v.seats[0].mana_pool.green = 7;
    assert_eq!(
        agent().act_with_context(&v, &question, &context),
        PlayerAction::ChooseNumber(3)
    );
    v.seats[0].mana_pool.green = 0;
    assert_eq!(
        agent().act_with_context(&v, &question, &context),
        PlayerAction::ChooseNumber(0),
        "nothing floating kicks nothing"
    );
}
