//! When an instant is cast (`timing`, [`AIProfile::INSTANT_TIMING`]): at
//! the moment its purpose can happen, for the levels that manage their open
//! mana, and at the first chance for the levels that tap out and for the
//! frozen reference.

use super::*;
use crate::timing::{Boon, Purpose, purpose};
use baylee_core::ids::Defender;
use baylee_engine::choice::{LegalActions, TargetPrompt};
use baylee_view::{AttackerView, BlockerView, Phase, Step};

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

/// The levels that time their instants.
const TIMING: [(&str, AIProfile); 3] = [
    ("STEADY", AIProfile::STEADY),
    ("SHARP", AIProfile::SHARP),
    ("EXPERT", AIProfile::EXPERT),
];

/// The levels that cast at the first chance, and the frozen reference.
const FIRST_CHANCE: [(&str, AIProfile); 3] = [
    ("NOVICE", AIProfile::NOVICE),
    ("CASUAL", AIProfile::CASUAL),
    ("SHARP_2026_11", AIProfile::SHARP_2026_11),
];

fn cast_effects(name: &str) -> &'static [baylee_cards_dsl::Effect] {
    let def = baylee_cards::by_index(baylee_cards::decks::by_name(name).unwrap()).unwrap();
    crate::timing::cast_effects(def.abilities_for_face(0)).unwrap()
}

/// Priority with `castable` paid for already, and passing allowed.
fn priority(castable: Vec<ObjectId>) -> Pending {
    Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            castable,
            can_pass: true,
            ..Default::default()
        }),
    }
}

/// This seat's 2/2 attacks and is blocked by a 3/3; its 5/5 attacks
/// unblocked into twenty life. Giant Growth is in hand, its mana floating.
fn blocked_bear() -> PlayerView {
    let mut v = view(
        0,
        &[20, 20],
        vec![
            permanent(obj(1), ME, 2),
            permanent(obj(2), THEM, 3),
            permanent(obj(4), ME, 5),
        ],
    );
    v.phase = Phase::Combat;
    v.step = Step::DeclareBlockers;
    v.combat.attackers = vec![
        AttackerView {
            creature: obj(1),
            defending: Defender::Player(THEM),
            blocked: true,
        },
        AttackerView {
            creature: obj(4),
            defending: Defender::Player(THEM),
            blocked: false,
        },
    ];
    v.combat.blockers = vec![BlockerView {
        blocker: obj(2),
        attacker: obj(1),
    }];
    v.hand = vec![hand_card(3, "Giant Growth")];
    v
}

#[test]
fn an_instant_is_read_by_what_it_is_for() {
    assert_eq!(
        purpose(cast_effects("Giant Growth"), 0),
        Purpose::Trick(Boon {
            power: 3,
            toughness: 3,
            keywords: 0
        })
    );
    assert_eq!(
        purpose(cast_effects("Titanic Growth"), 0),
        Purpose::Trick(Boon {
            power: 4,
            toughness: 4,
            keywords: 0
        })
    );
    assert_eq!(purpose(cast_effects("Counterspell"), 0), Purpose::Counter);
    for name in ["Plummet", "Lightning Bolt", "Terror", "Opt"] {
        assert_eq!(
            purpose(cast_effects(name), 0),
            Purpose::Lasting,
            "{name}: what it does stays done"
        );
    }
}

/// **A trick is cast once blocks are declared, where it turns the
/// exchange, and on the creature it turns.** Giant Growth makes the
/// blocked 2/2 a 5/5 that kills the 3/3 and lives; on the unblocked 5/5 it
/// would be three points into twenty, which is not worth a card.
#[test]
fn a_trick_waits_for_blocks_and_goes_where_it_turns_the_exchange() {
    let blocked = blocked_bear();
    for (name, profile) in TIMING {
        let agent = HeuristicAgent::new(profile);
        assert_eq!(
            agent.act(&blocked, &priority(vec![obj(3)])),
            PlayerAction::CastSpell { card: obj(3) },
            "{name}"
        );
        let context = baylee_engine::engine::DecisionContext {
            effects: cast_effects("Giant Growth"),
            ..Default::default()
        };
        let aim = Pending::ChooseTargets {
            player: ME,
            options: vec![obj(4), obj(1), obj(2)],
            player_options: vec![],
            min: 1,
            max: 1,
            reason: TargetPrompt::Targets,
        };
        assert_eq!(
            HeuristicAgent::new(profile).act_with_context(&blocked, &aim, &context),
            PlayerAction::ChooseTargets {
                objects: vec![obj(1)],
                players: vec![]
            },
            "{name}: the 2/2 it saves, not the 5/5 ranked first"
        );

        // Before blocks, nothing is turned yet.
        let mut before = blocked.clone();
        before.step = Step::DeclareAttackers;
        before.combat.blockers.clear();
        assert_eq!(
            HeuristicAgent::new(profile).act(&before, &priority(vec![obj(3)])),
            PlayerAction::PassPriority,
            "{name}"
        );
        // Nor in a main phase, where it would only waste the card.
        let mut main = view(0, &[20, 20], vec![permanent(obj(1), ME, 2)]);
        main.phase = Phase::FirstMain;
        main.step = Step::Main;
        main.hand = blocked.hand.clone();
        assert_eq!(
            HeuristicAgent::new(profile).act(&main, &priority(vec![obj(3)])),
            PlayerAction::PassPriority,
            "{name}"
        );
    }
}

/// An unblocked attacker is pumped only where the pump ends the game.
#[test]
fn a_trick_on_an_unblocked_attacker_is_cast_for_lethal_only() {
    let mut lethal = blocked_bear();
    lethal.seats[1].life = 8;
    for (name, profile) in TIMING {
        let context = baylee_engine::engine::DecisionContext {
            effects: cast_effects("Giant Growth"),
            ..Default::default()
        };
        let aim = Pending::ChooseTargets {
            player: ME,
            options: vec![obj(1), obj(4)],
            player_options: vec![],
            min: 1,
            max: 1,
            reason: TargetPrompt::Targets,
        };
        assert_eq!(
            HeuristicAgent::new(profile).act_with_context(&lethal, &aim, &context),
            PlayerAction::ChooseTargets {
                objects: vec![obj(4)],
                players: vec![]
            },
            "{name}: five and three is eight"
        );
    }
}

/// **Something lasting waits out the upkeep and the draw step**: a
/// Lightning Bolt for the opponent's creature is cast in their end step or
/// once they attack, not in their upkeep, and not in this seat's own draw
/// step. The levels that tap out, and the frozen reference, cast it at the
/// first chance as before.
#[test]
fn a_lasting_instant_waits_for_an_end_step_or_an_attack() {
    let mut theirs = view(0, &[20, 20], vec![permanent(obj(2), THEM, 3)]);
    theirs.active = THEM;
    theirs.phase = Phase::Beginning;
    theirs.step = Step::Upkeep;
    theirs.hand = vec![hand_card(3, "Lightning Bolt")];
    let mut end = theirs.clone();
    end.phase = Phase::Ending;
    end.step = Step::End;
    let mut own_draw = theirs.clone();
    own_draw.active = ME;
    own_draw.step = Step::Draw;
    let bolt = priority(vec![obj(3)]);
    for (name, profile) in TIMING {
        let agent = HeuristicAgent::new(profile);
        assert_eq!(
            agent.act(&theirs, &bolt),
            PlayerAction::PassPriority,
            "{name}"
        );
        assert_eq!(
            HeuristicAgent::new(profile).act(&own_draw, &bolt),
            PlayerAction::PassPriority,
            "{name}"
        );
        assert_eq!(
            HeuristicAgent::new(profile).act(&end, &bolt),
            PlayerAction::CastSpell { card: obj(3) },
            "{name}"
        );
    }
    for (name, profile) in FIRST_CHANCE {
        assert_eq!(
            HeuristicAgent::new(profile).act(&theirs, &bolt),
            PlayerAction::CastSpell { card: obj(3) },
            "{name}"
        );
    }
}

/// **Mana is kept up for a trick only until combat, and never for a
/// trick after it.** With five floating, a creature that can attack and
/// Titanic Growth in hand, the Hill Giant waits for the second main phase,
/// where it is cast. Generation 2's hold-up kept the trick's two up after
/// combat too, and spent them on the trick itself, which pumped a creature
/// that had nothing left to fight.
#[test]
fn trick_mana_is_kept_up_through_combat_only() {
    let mut first = view(0, &[20, 20], vec![permanent(obj(1), ME, 2)]);
    first.phase = Phase::FirstMain;
    first.step = Step::Main;
    first.seats[1].hand_count = 3;
    first.seats[0].mana_pool.green = 5;
    first.hand = vec![hand_card(3, "Titanic Growth"), hand_card(5, "Hill Giant")];
    let mut second = first.clone();
    second.phase = Phase::SecondMain;
    let both = priority(vec![obj(3), obj(5)]);
    let old = AIProfile {
        generation: AIProfile::EXACT_SPELL_X,
        ..AIProfile::STEADY
    };
    for (name, profile) in TIMING {
        assert_eq!(
            HeuristicAgent::new(profile).act(&first, &both),
            PlayerAction::PassPriority,
            "{name}: the trick's mana stays up through combat"
        );
        assert_eq!(
            HeuristicAgent::new(profile).act(&second, &both),
            PlayerAction::CastSpell { card: obj(5) },
            "{name}"
        );
    }
    assert_eq!(
        HeuristicAgent::new(old).act(&second, &both),
        PlayerAction::CastSpell { card: obj(3) },
        "generation 2 kept the trick's mana up, then cast the trick"
    );
}

/// A counterspell's mana is kept up in a main phase while an opponent holds
/// a card; a removal spell's is not, since it can be cast now.
#[test]
fn mana_is_kept_up_for_a_counterspell_and_not_for_removal() {
    let mut main = view(0, &[20, 20], vec![permanent(obj(1), ME, 2)]);
    main.phase = Phase::SecondMain;
    main.step = Step::Main;
    main.seats[1].hand_count = 3;
    main.seats[0].mana_pool.blue = 5;
    main.hand = vec![hand_card(3, "Counterspell"), hand_card(5, "Hill Giant")];
    let giant = priority(vec![obj(5)]);
    for (name, profile) in TIMING {
        assert_eq!(
            HeuristicAgent::new(profile).act(&main, &giant),
            PlayerAction::PassPriority,
            "{name}: four for the Giant would leave one, and Counterspell needs two"
        );
    }
    let mut removal = main.clone();
    removal.hand = vec![hand_card(3, "Terror"), hand_card(5, "Hill Giant")];
    for (name, profile) in TIMING {
        assert_eq!(
            HeuristicAgent::new(profile).act(&removal, &giant),
            PlayerAction::CastSpell { card: obj(5) },
            "{name}"
        );
    }
}
