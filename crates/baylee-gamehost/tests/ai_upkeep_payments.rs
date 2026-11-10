//! Echo and a pact's price across the real engine boundary: the view a seat
//! is shown with one on the stack, and the house AI playing through it.
//!
//! The price is a triggered ability on the stack (`docs/engine-internals.md`
//! §"An upkeep payment is a triggered ability on the stack"), so nothing short
//! of a game proves that the view names it, that the agent holds its mana over
//! it, answers the question it asks as it resolves, and pays in the window a
//! yes opens.

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_ai::{AIProfile, HeuristicAgent, pending_player};
use baylee_cards_pool as _;
use baylee_core::ids::{CardIndex, PlayerId};
use baylee_core::preset::{DeckEntry, GamePreset};
use baylee_engine::choice::{Pending, PlayerAction, YesNoPrompt};
use baylee_engine::engine::Engine;
use baylee_engine::zone::ZoneLocation;
use baylee_gamehost::{PlayerView, RegistryLookup, SeatContext, owed_payment, player_view};
use baylee_view::StackItem;

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: baylee_cards::decks::by_name(name).expect("registered fixture"),
        print: baylee_core::ids::PrintRef::new(0),
    }
}

fn card(name: &str) -> CardIndex {
    entry(name).card
}

/// Seat 0 with `hand` and `mine`, seat 1 with nothing but `theirs`.
fn table(hand: &[&str], mine: &[&str], theirs: &[&str]) -> GamePreset {
    let mut preset = baylee_cards::decks::probe_preset(43, card("Plains")).unwrap();
    for seat in &mut preset.seats {
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_hand = Some(hand.iter().map(|name| entry(name)).collect());
    preset.seats[0].starting_battlefield = mine.iter().map(|name| entry(name)).collect();
    preset.seats[1].starting_battlefield = theirs.iter().map(|name| entry(name)).collect();
    preset
}

fn view_of(engine: &Engine<RegistryLookup>, seat: PlayerId, seq: u64) -> PlayerView {
    let ctx = SeatContext {
        awaiting: Some(seat),
        owed: owed_payment(engine),
        ..Default::default()
    };
    player_view(engine.state(), seat, seq, Some(engine.pending()), &ctx, &[])
}

fn on_the_battlefield(engine: &Engine<RegistryLookup>, card: CardIndex) -> bool {
    let state = engine.state();
    state
        .battlefield_view()
        .into_iter()
        .filter_map(|id| state.object(id))
        .any(|object| object.card.is_some_and(|c| c.index == card))
}

/// The first card of that name in `zone`.
fn named(
    engine: &Engine<RegistryLookup>,
    zone: ZoneLocation,
    name: &str,
) -> baylee_core::ids::ObjectId {
    let state = engine.state();
    state
        .zones
        .list(zone)
        .iter()
        .copied()
        .find(|id| {
            state
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card(name)))
        })
        .unwrap_or_else(|| panic!("{name} is not there"))
}

/// Past the first turn, with seat 0 active and something on the stack: the
/// moment its upkeep's payment has gone on.
fn on_stack_in_my_upkeep(engine: &Engine<RegistryLookup>) -> bool {
    let state = engine.state();
    state.turn.active == ME
        && state.turn.number > 1
        && !state.zones.list(ZoneLocation::Stack).is_empty()
}

/// What the view says of an ability on the stack: its printed sentence.
fn sentence(object: &baylee_view::PublicObject) -> &'static str {
    let Some(StackItem::Ability {
        text: Some(text),
        rules: Some(rules),
        ability: Some(ability),
        ..
    }) = object.stack_item
    else {
        panic!("an ability with a sentence: {:?}", object.stack_item)
    };
    assert_eq!(ability.card, rules.card, "of the card it is printed on");
    assert!(ability.is_listed_ability(), "by its place in the list");
    baylee_cards::oracle::sentence(rules.card, usize::from(text.face), text.line)
        .expect("the host points at a printed sentence")
}

/// Plays on with seat 1 passing and the agent answering seat 0, until `stop`.
/// Records what the agent answered the question `asked` picks out.
fn play(
    engine: &mut Engine<RegistryLookup>,
    agent: &HeuristicAgent,
    stop: impl Fn(&Engine<RegistryLookup>) -> bool,
    mut asked: impl FnMut(&Pending, &PlayerView, &PlayerAction),
) {
    for seq in 0..600 {
        if stop(engine) {
            return;
        }
        let pending = engine.pending();
        let Some(seat) = pending_player(pending) else {
            return;
        };
        let view = view_of(engine, seat, seq);
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } if seat == THEM => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, pending, &engine.decision_context()),
        };
        asked(pending, &view, &action);
        engine
            .apply(seat, action)
            .expect("every answer comes from the engine's own offer");
    }
    panic!("the game did not get there: {:?}", engine.pending());
}

/// Keldon Champion, echo {2}{R}{R}, on four Mountains: cast on the first turn,
/// on the stack with its printed line at the next upkeep, and paid from the
/// same four Mountains, which the agent held over the trigger.
#[test]
fn the_agent_holds_its_mana_over_its_echo_and_pays_it() {
    let preset = table(
        &["Keldon Champion"],
        &["Mountain", "Mountain", "Mountain", "Mountain"],
        &[],
    );
    let champion = card("Keldon Champion");
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);

    play(&mut engine, &agent, on_stack_in_my_upkeep, |_, _, _| {});
    assert!(on_the_battlefield(&engine, champion), "the agent cast it");
    for seat in [ME, THEM] {
        let view = view_of(&engine, seat, 0);
        let [echo] = &view.stack[..] else {
            panic!("the echo, for every seat: {:?}", view.stack)
        };
        assert_eq!(echo.controller, ME);
        assert!(
            sentence(echo).starts_with("Echo {2}{R}{R}"),
            "{}",
            sentence(echo)
        );
    }

    let mut answered = None;
    let turn = engine.state().turn.number;
    play(
        &mut engine,
        &agent,
        |e| e.state().turn.number > turn,
        |pending, _, action| {
            if matches!(
                pending,
                Pending::YesNo {
                    prompt: YesNoPrompt::PayMana { .. },
                    ..
                }
            ) {
                answered = Some(action.clone());
            }
        },
    );
    assert_eq!(answered, Some(PlayerAction::YesNo(true)));
    assert!(
        on_the_battlefield(&engine, champion),
        "paid, from the lands it did not spend over the trigger"
    );
}

/// A replay is the same answers on a fresh engine (`record`): through the
/// trigger, the question and the window, they reach the same game.
#[test]
fn a_game_through_its_echo_and_window_replays_answer_for_answer() {
    let preset = table(
        &["Keldon Champion"],
        &["Mountain", "Mountain", "Mountain", "Mountain"],
        &[],
    );
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let mut answers = Vec::new();
    let mut echo_asked = false;
    play(
        &mut engine,
        &agent,
        |e| e.state().turn.number >= 4,
        |pending, _, action| {
            echo_asked |= matches!(
                pending,
                Pending::YesNo {
                    prompt: YesNoPrompt::PayMana { .. },
                    ..
                }
            );
            answers.push((
                pending_player(pending).expect("a seat is asked"),
                action.clone(),
            ));
        },
    );
    assert!(echo_asked, "the game went through the echo");

    let mut again = Engine::new(&preset, RegistryLookup).unwrap();
    for (seat, action) in answers {
        again
            .apply(seat, action)
            .expect("the same game asks the same questions");
    }
    assert_eq!(again.snapshot_hash(), engine.snapshot_hash());
}

/// With nothing to pay it with there is no question and no window: the
/// Champion goes as the ability resolves. An opening board has been through
/// no upkeep of its controller's, so it owes at the first.
#[test]
fn the_agent_is_not_asked_for_an_echo_nothing_can_pay() {
    let preset = table(&[], &["Keldon Champion"], &[]);
    let champion = card("Keldon Champion");
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let mut questions = 0;
    play(
        &mut engine,
        &agent,
        |e| e.state().turn.number >= 2,
        |pending, _, _| {
            questions += usize::from(matches!(pending, Pending::YesNo { .. }));
        },
    );
    assert_eq!(questions, 0);
    assert!(!on_the_battlefield(&engine, champion));
}

/// Pact of Negation, played by hand on the first turn; its price is the
/// agent's to pay from five Islands at the next upkeep, over a stack that
/// names it, in the window a yes opens.
#[test]
fn the_agent_pays_a_pacts_price_from_the_lands_it_held() {
    let preset = table(
        &["Llanowar Elves", "Pact of Negation"],
        &["Forest", "Island", "Island", "Island", "Island", "Island"],
        &[],
    );
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    // Mulligans, then to the first main with priority.
    for _ in 0..40 {
        if engine.state().turn.step == baylee_engine::turn::Step::Main
            && matches!(engine.pending(), Pending::Priority { .. })
        {
            break;
        }
        let seat = pending_player(engine.pending()).expect("a seat is asked");
        let action = match engine.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            _ => PlayerAction::PassPriority,
        };
        engine.apply(seat, action).unwrap();
    }
    let source = named(&engine, ZoneLocation::Battlefield, "Forest");
    engine
        .apply(ME, PlayerAction::ActivateManaAbility { source })
        .unwrap();
    let elves = named(&engine, ZoneLocation::Hand(ME), "Llanowar Elves");
    engine
        .apply(ME, PlayerAction::CastSpell { card: elves })
        .unwrap();
    let pact = named(&engine, ZoneLocation::Hand(ME), "Pact of Negation");
    engine
        .apply(ME, PlayerAction::CastSpell { card: pact })
        .unwrap();
    engine
        .apply(
            ME,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    play(&mut engine, &agent, on_stack_in_my_upkeep, |_, _, _| {});
    let view = view_of(&engine, THEM, 0);
    let [price] = &view.stack[..] else {
        panic!("the price, for every seat: {:?}", view.stack)
    };
    assert_eq!(
        sentence(price),
        "At the beginning of your next upkeep, pay {3}{U}{U}. If you don't, you lose the game."
    );

    let turn = engine.state().turn.number;
    let mut answered = None;
    play(
        &mut engine,
        &agent,
        |e| e.state().turn.number > turn,
        |pending, _, action| {
            if matches!(
                pending,
                Pending::YesNo {
                    prompt: YesNoPrompt::PayPact { .. },
                    ..
                }
            ) {
                answered = Some(action.clone());
            }
        },
    );
    assert_eq!(answered, Some(PlayerAction::YesNo(true)));
    assert!(!engine.state().players[0].has_lost(), "and it was paid");
}
