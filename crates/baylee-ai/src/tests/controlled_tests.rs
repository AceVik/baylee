//! Deciding for a player whose turn this seat controls (Mindslaver,
//! CR 723.5): every answer is made against that player, and this seat's own
//! answers in that turn are the ones it always gave (CR 723.8).
//!
//! The house is seat 0 and decides for seat 1 in seat 1's turn: the view is
//! the house's, awaits the house, and names seat 1 as the player the
//! question's resources belong to, as `Session::agent_view` builds it.
use super::*;
use baylee_cards_dsl::Effect;
use baylee_engine::choice::{LegalActions, TargetPrompt};
use baylee_engine::engine::DecisionContext;

const HOUSE: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

/// The house's view of seat 1's second main phase, deciding for seat 1,
/// whose hand it is entitled to see (CR 723.4).
fn controlling(battlefield: Vec<PublicObject>, hand: Vec<baylee_view::HandObject>) -> PlayerView {
    let mut v = view(0, &[20, 20], battlefield);
    v.active = THEM;
    v.phase = baylee_view::Phase::SecondMain;
    v.step = baylee_view::Step::Main;
    v.awaiting = Some(HOUSE);
    v.decision_player = Some(THEM);
    v.seats[1].hand_count = u32::try_from(hand.len()).unwrap();
    v.controlled_hands.push(baylee_view::SharedHand {
        public: false,
        player: THEM,
        cards: hand,
    });
    v
}

/// Seat 1's priority with `castable` already paid for.
fn their_priority(castable: Vec<ObjectId>) -> Pending {
    Pending::Priority {
        player: THEM,
        legal: Box::new(LegalActions {
            can_pass: true,
            castable,
            ..Default::default()
        }),
    }
}

/// The effects of `name`'s spell, as a target question explains them.
fn spell_effects(name: &str) -> &'static [Effect] {
    let card = baylee_cards::by_index(baylee_cards::decks::by_name(name).unwrap()).unwrap();
    card.abilities_for_face(0)
        .iter()
        .find_map(|a| match a {
            AbilityDef::Spell { effects, .. } => Some(*effects),
            _ => None,
        })
        .expect("a spell")
}

/// `name` cast by seat 1, on the stack, as the target question finds it.
fn their_spell(slot: u32, name: &str) -> PublicObject {
    let mut o = permanent(obj(slot), THEM, 0);
    o.card = Some(hand_card(slot, name).card);
    o.rules = o.card.map(baylee_view::RulesFace::from);
    o.name = name.into();
    o.types = TypeSet::INSTANT;
    o.power = None;
    o.toughness = None;
    o.stack_item = Some(baylee_view::StackItem::Spell);
    o
}

fn sized(id: ObjectId, controller: PlayerId, power: i16, toughness: i16) -> PublicObject {
    PublicObject {
        toughness: Some(toughness),
        ..permanent(id, controller, power)
    }
}

#[test]
fn only_a_question_asked_of_another_player_is_decided_for_them() {
    let mut v = controlling(vec![], vec![]);
    let theirs = their_priority(vec![]);
    assert_eq!(crate::controlled::subject(&v, &theirs), Some(THEM));
    let ours = Pending::Priority {
        player: HOUSE,
        legal: Box::default(),
    };
    assert_eq!(crate::controlled::subject(&v, &ours), None, "CR 723.8");
    v.awaiting = Some(THEM);
    assert_eq!(
        crate::controlled::subject(&v, &theirs),
        None,
        "a question this seat is not awaited for is not its to answer"
    );
}

/// Their Lightning Bolt, paid with their red, goes at their own creature:
/// the Bolt is spent and their Bears are gone. Before their combat it is
/// held, so nothing is spent on a creature this seat's block would kill.
#[test]
fn their_burn_is_cast_after_combat_at_their_own_creature() {
    let bears = sized(obj(2), THEM, 2, 2);
    let giant = sized(obj(3), HOUSE, 3, 3);
    let mut v = controlling(vec![bears, giant], vec![hand_card(1, "Lightning Bolt")]);
    v.seats[1].mana_pool.red = 1;
    assert_eq!(
        agent().act(&v, &their_priority(vec![obj(1)])),
        PlayerAction::CastSpell { card: obj(1) }
    );
    let mut before = v.clone();
    before.phase = baylee_view::Phase::FirstMain;
    assert_eq!(
        agent().act(&before, &their_priority(vec![obj(1)])),
        PlayerAction::PassPriority,
        "nothing before their combat is over"
    );

    v.controlled_hands[0].cards.clear();
    v.stack.push(their_spell(1, "Lightning Bolt"));
    let pending = Pending::ChooseTargets {
        player: THEM,
        options: vec![obj(2), obj(3)],
        player_options: vec![HOUSE, THEM],
        min: 1,
        max: 1,
        reason: TargetPrompt::Targets,
    };
    let context = DecisionContext {
        source: Some(obj(1)),
        effects: spell_effects("Lightning Bolt"),
        ..Default::default()
    };
    let agent = agent();
    assert_eq!(
        agent.act_with_context(&v, &pending, &context),
        PlayerAction::ChooseTargets {
            objects: vec![obj(2)],
            players: vec![],
        }
    );
    assert_eq!(agent.fallbacks(), 0);
}

/// Their Giant Growth is spent on this seat's creature, which is the one
/// place it does nothing for them.
#[test]
fn their_pump_is_spent_on_this_seats_creature() {
    let mut v = controlling(
        vec![sized(obj(2), THEM, 2, 2), sized(obj(3), HOUSE, 3, 3)],
        vec![hand_card(1, "Giant Growth")],
    );
    v.seats[1].mana_pool.green = 1;
    assert_eq!(
        agent().act(&v, &their_priority(vec![obj(1)])),
        PlayerAction::CastSpell { card: obj(1) }
    );
    v.controlled_hands[0].cards.clear();
    v.stack.push(their_spell(1, "Giant Growth"));
    let pending = Pending::ChooseTargets {
        player: THEM,
        options: vec![obj(2), obj(3)],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: TargetPrompt::Targets,
    };
    let context = DecisionContext {
        source: Some(obj(1)),
        effects: spell_effects("Giant Growth"),
        ..Default::default()
    };
    assert_eq!(
        agent().act_with_context(&v, &pending, &context),
        PlayerAction::ChooseTargets {
            objects: vec![obj(3)],
            players: vec![],
        }
    );
}

/// Their creature, their land and their Divination stay where they are:
/// casting any of them builds their board or does for them only what they
/// would have done themselves. Their Healing Salve does not: its three life
/// can be this seat's.
#[test]
fn nothing_is_cast_or_played_that_helps_them() {
    let mut v = controlling(
        vec![sized(obj(3), HOUSE, 3, 3)],
        vec![
            hand_card(1, "Grizzly Bears"),
            hand_card(2, "Divination"),
            hand_card(4, "Mountain"),
            hand_card(5, "Healing Salve"),
        ],
    );
    v.seats[1].mana_pool.green = 2;
    v.seats[1].mana_pool.blue = 3;
    let priority = |castable| Pending::Priority {
        player: THEM,
        legal: Box::new(LegalActions {
            can_pass: true,
            castable,
            lands: vec![obj(4)],
            ..Default::default()
        }),
    };
    assert_eq!(
        agent().act(&v, &priority(vec![obj(1), obj(2)])),
        PlayerAction::PassPriority
    );
    v.seats[1].mana_pool.white = 1;
    assert_eq!(
        agent().act(&v, &priority(vec![obj(1), obj(2), obj(5)])),
        PlayerAction::CastSpell { card: obj(5) }
    );
}

/// In their end step their lands are tapped, so they stay tapped through
/// this seat's turn (CR 502.3); before it, nothing is.
#[test]
fn their_end_step_taps_out_their_lands() {
    let mut mountain = permanent(obj(4), THEM, 0);
    mountain.types = TypeSet::LAND;
    mountain
        .subtypes
        .insert(baylee_core::generated::subtypes::land::MOUNTAIN);
    let mut v = controlling(vec![mountain], vec![]);
    let pending = Pending::Priority {
        player: THEM,
        legal: Box::new(LegalActions {
            can_pass: true,
            mana_abilities: vec![obj(4)],
            ..Default::default()
        }),
    };
    assert_eq!(agent().act(&v, &pending), PlayerAction::PassPriority);
    v.phase = baylee_view::Phase::Ending;
    v.step = baylee_view::Step::End;
    assert_eq!(
        agent().act(&v, &pending),
        PlayerAction::ActivateManaAbility { source: obj(4) }
    );
}

/// The house's own block in their turn is the block it always makes
/// (CR 723.8): the control fields change nothing about its own question.
#[test]
fn this_seats_own_block_in_their_turn_is_unchanged() {
    let mut v = controlling(
        vec![sized(obj(1), THEM, 2, 2), sized(obj(3), HOUSE, 4, 4)],
        vec![],
    );
    v.phase = baylee_view::Phase::Combat;
    v.step = baylee_view::Step::DeclareBlockers;
    v.decision_player = Some(HOUSE);
    v.combat.attackers = vec![baylee_view::AttackerView {
        creature: obj(1),
        defending: Defender::Player(HOUSE),
        blocked: false,
    }];
    let pending = Pending::ChooseBlockers {
        demands: Vec::new(),
        player: HOUSE,
        attacker: THEM,
        blockers: vec![baylee_engine::choice::BlockOption {
            blocker: obj(3),
            attackers: vec![obj(1)],
        }],
        capacity: Vec::new(),
        obeying: Vec::new(),
        bounds: Vec::new(),
    };
    let mut plain = v.clone();
    plain.controlled_hands.clear();
    plain.decision_player = None;
    let controlled = agent().act(&v, &pending);
    assert_eq!(controlled, agent().act(&plain, &pending));
    assert_eq!(
        controlled,
        PlayerAction::DeclareBlockers {
            blockers: vec![(obj(3), obj(1))]
        }
    );
}
