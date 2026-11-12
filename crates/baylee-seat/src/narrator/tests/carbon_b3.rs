//! What the Carbon seats of self-play block 3 (12.11.2026) were offered
//! wrong or told wrong, on a real engine where the table can be built:
//! a suspend from an untapped board, a division on the stack.

use super::*;
use crate::wake::tests::played::position;
use baylee_engine::choice::PlayerAction;
use baylee_gamehost::Session;
use baylee_view::LogTail;

fn asked(session: &Session, seat: PlayerId) -> Request {
    let (pending, view) = session.view_for(seat).expect("the seat is asked");
    request(view, pending, LogTail::default())
}

/// Attempt 30, seat 1, q322: Ancestral Vision in hand beside untapped
/// Islands, and no suspend among the options. On the engine itself: the
/// option taps the Island and suspends, and the card is exiled with its
/// four time counters (CR 702.62a).
#[test]
fn ancestral_vision_is_suspended_from_an_untapped_island() {
    let (mut session, view, legal) = position(&["Island", "Island"], &["Ancestral Vision"], &[]);
    let vision = view.hand[0].id;
    assert!(legal.suspendable.is_empty(), "the {{U}} is in the Island");
    let request = asked(&session, ME);
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    let line = wake
        .text
        .lines()
        .find(|line| {
            line.contains(&format!(
                "Suspend Ancestral Vision {} for {{U}}",
                tag(vision)
            ))
        })
        .unwrap_or_else(|| panic!("the suspend is offered:\n{}", wake.text));
    assert!(line.contains("4 time counters"), "{line}");
    let pick = line.split_whitespace().next().unwrap().to_string();
    let answer = wake
        .menu
        .resolve(&Decision {
            ask: Some("q12".into()),
            pick: vec![pick],
            ..Default::default()
        })
        .unwrap();
    let Act::Taps { steps, then } = answer.act else {
        panic!("taps, then the suspend: {:?}", answer.act);
    };
    assert_eq!(then, PlayerAction::Suspend { card: vision });
    for step in &steps {
        session
            .act_slice(ME, super::menu::tap(step), 0)
            .expect("the planned tap");
    }
    session
        .act_slice(ME, then, 0)
        .expect("the {U} floats, and the suspend is taken");
    let exiled = session
        .observe_view(ME)
        .exile
        .iter()
        .flatten()
        .find(|o| o.name.as_ref() == "Ancestral Vision")
        .cloned()
        .expect("the card is in exile");
    assert!(exiled.suspended, "{exiled:?}");
}

/// Attempt 18, seat 0, q210-q216: Fire Covenant's 5 divided among two
/// Zombies and Bloodflow Connoisseur read "targets Zombie #124, Zombie #125,
/// Bloodflow Connoisseur #9" to the player it was cast against. The
/// division is announced as the spell is cast (CR 601.2d), and the line
/// says each share. Arc Lightning's 3 here: 1 to the Bears, 2 to the Giant.
#[test]
fn a_divided_spell_on_the_stack_says_each_share_to_every_seat() {
    let (mut session, view, _) = position(
        &["Mountain", "Mountain", "Mountain"],
        &["Arc Lightning"],
        &["Grizzly Bears", "Hill Giant"],
    );
    for mountain in view
        .battlefield
        .iter()
        .filter(|o| o.controller == ME)
        .map(|o| o.id)
        .collect::<Vec<_>>()
    {
        session
            .act_slice(
                ME,
                PlayerAction::ActivateManaAbility { source: mountain },
                0,
            )
            .expect("a Mountain's red");
    }
    let theirs = |name: &str| {
        view.battlefield
            .iter()
            .find(|o| o.name.as_ref() == name)
            .unwrap_or_else(|| panic!("{name}"))
            .id
    };
    let (bears, giant) = (theirs("Grizzly Bears"), theirs("Hill Giant"));
    session
        .act_slice(
            ME,
            PlayerAction::CastSpell {
                card: view.hand[0].id,
            },
            0,
        )
        .expect("three red pay Arc Lightning");
    session
        .act_slice(
            ME,
            PlayerAction::ChooseTargets {
                objects: vec![bears, giant],
                players: Vec::new(),
            },
            0,
        )
        .expect("two targets");
    session
        .act_slice(ME, PlayerAction::ChooseNumber(1), 0)
        .expect("1 to the Bears, the rest to the Giant");
    session
        .act_slice(ME, PlayerAction::PassPriority, 0)
        .expect("the caster passes");
    for seat in [THEM, ME] {
        let request = if seat == THEM {
            asked(&session, THEM)
        } else {
            let view = session.observe_view(ME);
            request(view, priority(&board().0), LogTail::default())
        };
        let text = Narrator::new(&request.context)
            .wake(&request, &[], None)
            .text;
        let line = text
            .lines()
            .find(|line| line.contains("Arc Lightning") && line.contains(" · targets "))
            .unwrap_or_else(|| panic!("the spell on the stack:\n{text}"));
        let at = |name: &str| line.find(name).unwrap_or_else(|| panic!("{name}: {line}"));
        let bears_at = at(&format!("Grizzly Bears {}", tag(bears)));
        let giant_at = at(&format!("Hill Giant {}", tag(giant)));
        assert!(bears_at < giant_at, "{line}");
        assert!(line[bears_at..giant_at].contains("(1 damage)"), "{line}");
        assert!(line[giant_at..].contains("(2 damage)"), "{line}");
    }
}

/// A seat with `swamps` untapped Swamps asked to pay `prompt`, and the
/// option ids it is offered.
fn asked_to_pay(
    lands: &[&str],
    prompt: baylee_engine::choice::YesNoPrompt,
) -> (String, Vec<String>) {
    let mine: Vec<PublicObject> = lands
        .iter()
        .enumerate()
        .map(|(i, name)| card(70 + u32::try_from(i).expect("a few"), name))
        .collect();
    let view = ViewBuilder::new(2).with_battlefield(0, mine).build();
    let pending = Pending::YesNo {
        player: ME,
        prompt,
        source: None,
    };
    let request = request(view, pending, LogTail::default());
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    let ids = wake.menu.offered().iter().map(|o| o.id.clone()).collect();
    (wake.text, ids)
}

/// Attempt 23, seat 1, q760: Circular Logic's "Pay {21}?" with seven
/// untapped Swamps and nothing floating, and "y Yes: pay {21}" offered. A
/// cost cannot be paid without the resources to pay it fully (CR 118.3),
/// and an impossible option cannot be chosen (CR 608.2d): the yes is not
/// offered, and the question says why.
#[test]
fn a_tax_no_untapped_source_can_reach_offers_no_yes() {
    use baylee_engine::choice::YesNoPrompt;
    let swamps = ["Swamp"; 7];
    let (text, ids) = asked_to_pay(&swamps, YesNoPrompt::PayTax { mana: 21 });
    assert_eq!(ids, vec!["n".to_string()], "{text}");
    assert!(text.contains("You cannot pay {21}"), "{text}");
    assert!(text.contains("at most 7"), "{text}");
    // Within reach, the question is the player's.
    let (text, ids) = asked_to_pay(&swamps, YesNoPrompt::PayTax { mana: 3 });
    assert_eq!(ids, vec!["y".to_string(), "n".to_string()], "{text}");
}

/// Attempt 25, seat 0, q160: Force of Nature's "Pay {G}{G}{G}{G}?" beside
/// two Forests offered the yes too.
#[test]
fn a_coloured_price_two_forests_cannot_reach_offers_no_yes() {
    use baylee_engine::choice::YesNoPrompt;
    let (text, ids) = asked_to_pay(
        &["Forest", "Forest"],
        YesNoPrompt::PayMana {
            cost: baylee_core::mana::ManaCost::parse("{G}{G}{G}{G}"),
        },
    );
    assert_eq!(ids, vec!["n".to_string()], "{text}");
    let (text, ids) = asked_to_pay(
        &["Forest", "Forest", "Forest", "Forest"],
        YesNoPrompt::PayMana {
            cost: baylee_core::mana::ManaCost::parse("{G}{G}{G}{G}"),
        },
    );
    assert_eq!(ids, vec!["y".to_string(), "n".to_string()], "{text}");
}

/// Attempt 32, seat 0, q105: Wild Mongrel's combat damage divided between
/// Ravenous Rats and Zodiac Snake under the header "turn 9, yours ·
/// blockers". The engine asks as declare blockers hands over to the damage
/// step; the damage is that step's (CR 510.1), and the header and the
/// question say so.
#[test]
fn a_combat_damage_division_is_headed_by_the_damage_step() {
    let (mut view, _) = board();
    view.step = baylee_view::Step::DeclareBlockers;
    view.phase = baylee_view::Phase::Combat;
    view.combat.attackers = vec![baylee_view::AttackerView {
        creature: id(30),
        defending: baylee_core::ids::Defender::Player(THEM),
        blocked: true,
    }];
    let pending = Pending::ChooseNumber {
        player: ME,
        min: 0,
        max: 2,
        reason: baylee_engine::choice::NumberPrompt::CombatDamage {
            source: id(30),
            recipient: id(45),
            index: 0,
            of: 2,
            left: 2,
            lethal: None,
        },
    };
    let request = request(view, pending, LogTail::default());
    let text = Narrator::new(&request.context)
        .wake(&request, &[], None)
        .text;
    let header = text.lines().next().expect("a header");
    assert!(header.contains("· damage ·"), "{header}");
    assert!(!header.contains("blockers"), "{header}");
    assert!(
        text.contains("As the combat damage step begins, before any damage is dealt"),
        "{text}"
    );
}

/// Attempt 32, seat 0, q136-q139: Wild Mongrel's "becomes the color of your
/// choice" was asked as "How do you cast Wild Mongrel #56?", and the plan's
/// `color B` stopped on it ("the table asked for how to cast it"). The
/// colour is chosen as the ability resolves (CR 608.2d, 105.1): the
/// question says so, its options are named by colour, and the plan's colour
/// step answers it.
#[test]
fn a_colour_chosen_as_an_ability_resolves_is_a_colour_question_a_plan_answers() {
    let (mut session, view, _) = position(&["Wild Mongrel"], &["Mountain", "Mountain"], &[]);
    let mongrel = view
        .battlefield
        .iter()
        .find(|o| o.name.as_ref() == "Wild Mongrel")
        .expect("the Mongrel")
        .id;
    session
        .act_slice(
            ME,
            PlayerAction::ActivateAbility {
                source: mongrel,
                ability_index: 0,
            },
            0,
        )
        .expect("discard a card: the Mongrel's cost");
    for _ in 0..8 {
        match session.pending().clone() {
            Pending::ChooseCastMode { .. } => break,
            Pending::Priority { player, .. } => {
                session
                    .act_slice(player, PlayerAction::PassPriority, 0)
                    .expect("a pass");
            }
            Pending::ChooseCards {
                player, options, ..
            } => {
                session
                    .act_slice(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                        0,
                    )
                    .expect("the card discarded");
            }
            Pending::DiscardChoice { player, .. } => {
                let card = session.observe_view(player).hand[0].id;
                session
                    .act_slice(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![card],
                        },
                        0,
                    )
                    .expect("the card discarded");
            }
            other => panic!("unexpected question: {other:?}"),
        }
    }
    let request = asked(&session, ME);
    assert!(
        matches!(request.pending, Pending::ChooseCastMode { .. }),
        "{:?}",
        request.pending
    );
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    assert!(!wake.text.contains("How do you cast"), "{}", wake.text);
    assert!(
        wake.text.contains(&format!(
            "QUESTION: Choose a colour for Wild Mongrel {}",
            tag(mongrel)
        )),
        "{}",
        wake.text
    );
    let decision = wake
        .menu
        .plan_decision(&super::Step::Color("B".into()), &request)
        .expect("the plan's colour step answers it");
    let answer = wake.menu.resolve(&decision).expect("an option");
    let Act::Now(action) = answer.act else {
        panic!("one answer: {:?}", answer.act);
    };
    session.act_slice(ME, action, 0).expect("black");
    let mongrel_now = session
        .observe_view(ME)
        .battlefield
        .iter()
        .find(|o| o.id == mongrel)
        .cloned()
        .expect("the Mongrel");
    assert_eq!(
        mongrel_now.colors,
        baylee_core::color::ColorSet::of(baylee_core::color::Color::Black),
        "{mongrel_now:?}"
    );
}

/// Attempt 28, seat 0, q331: Two-Headed Cerberus (double strike) and
/// Frostburn Weird. The list since the last decision printed both combat
/// damage steps as one run, the second step's attackers first, and the
/// model read the Weird as hitting in the first-strike step (CR 510.4,
/// 702.4b). The list marks the phase and each damage step.
#[test]
fn the_list_since_the_last_decision_marks_phases_and_damage_steps() {
    use baylee_view::{LogEntry, LogEvent, LogObject, LogTarget, Phase, Step};
    let (view, _) = board();
    let known = |slot: u32, name: &str| LogObject::Known {
        id: id(slot),
        card: None,
        token: None,
        name: name.to_string(),
    };
    let at = |(phase, step): (Phase, Step), event: LogEvent| LogEntry {
        turn: 7,
        repeat: 1,
        at: 0,
        event,
        phase: Some(phase),
        step: Some(step),
    };
    let attackers = (Phase::Combat, Step::DeclareAttackers);
    let first = (Phase::Combat, Step::CombatDamageFirst);
    let regular = (Phase::Combat, Step::CombatDamage);
    let main2 = (Phase::SecondMain, Step::Main);
    let hit = |slot: u32, name: &str, amount: u32| LogEvent::Damage {
        source: Some(known(slot, name)),
        target: LogTarget::Player(THEM),
        amount,
        combat: true,
    };
    let entries = vec![
        at(
            attackers,
            LogEvent::Attacked {
                attacker: known(30, "Llanowar Elves"),
                defending: baylee_core::ids::Defender::Player(THEM),
            },
        ),
        at(first, hit(31, "Prodigal Sorcerer", 1)),
        at(
            first,
            LogEvent::Life {
                player: THEM,
                old: 14,
                new: 13,
            },
        ),
        at(regular, hit(30, "Llanowar Elves", 1)),
        at(regular, hit(31, "Prodigal Sorcerer", 1)),
        at(
            regular,
            LogEvent::Life {
                player: THEM,
                old: 13,
                new: 11,
            },
        ),
        at(
            main2,
            LogEvent::LandPlayed {
                player: ME,
                land: known(23, "Mountain"),
                from: None,
            },
        ),
    ];
    let log = LogTail { from: 0, entries };
    let request = request(view, priority(&board().0), log.clone());
    let mut narrator = Narrator::new(&request.context);
    narrator.hear(&log);
    let text = narrator.wake(&request, &[], None).text;
    let Some((_, since)) = text.split_once("Since your last decision:\n") else {
        panic!("the list:\n{text}");
    };
    let marks: Vec<&str> = since
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("--"))
        .collect();
    assert_eq!(
        marks,
        vec![
            "-- combat --",
            "-- first-strike combat damage step --",
            "-- combat damage step --",
            "-- postcombat main phase --",
        ],
        "{since}"
    );
}
