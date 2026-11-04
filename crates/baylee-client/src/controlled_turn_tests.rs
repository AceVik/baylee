//! A turn this seat controls (Mindslaver, CR 723.5) played through the
//! client against a real engine: the question names the controlled player,
//! the view waits for this seat, and every decision of that turn is this
//! seat's to make with that player's cards, mana and creatures, including
//! the ones against that player's interest. Its own decisions in that turn
//! (CR 723.8) stay its own.
//!
//! The views are built the way the gamehost builds a seat's
//! (`awaiting_for`, `decision_player_for`, `controlled_players`), so what is
//! asserted is the join between the engine's routing and the client's
//! reading of it rather than a hand-made view either side could agree with.

use super::*;
use baylee_core::ids::Defender;
use baylee_core::preset::DeckEntry;
use baylee_engine::choice::LegalActions;
use baylee_engine::{engine::Engine, turn::Step};
use baylee_gamehost::view::{awaiting_for, decision_player_for};
use baylee_gamehost::{RegistryLookup, SeatContext, owed_payment, player_view};

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("no {name}")),
        print: baylee_core::ids::PrintRef::new(0),
    }
}

/// The view this seat is sent, as the gamehost builds it.
fn sync(duel: &mut Duel, engine: &Engine<RegistryLookup>) {
    let seq = duel.view.as_ref().map_or(1, |v| v.seq + 1);
    let mut view = player_view(
        engine.state(),
        ME,
        seq,
        engine.information_pending_for(ME),
        &SeatContext {
            awaiting: awaiting_for(engine, ME),
            decision_player: decision_player_for(engine, ME),
            controlled_players: engine.controlled_players(ME),
            owed: owed_payment(engine),
            ..SeatContext::default()
        },
        &[],
    );
    view.casting = engine.cancellable_cast(ME);
    duel.receive_view(view);
    if let Some(pending) = engine.pending_for(ME) {
        duel.receive_choice(pending.clone());
    }
    rebuild_board(duel);
}

/// The least answer to whatever is asked, by whoever answers it.
fn least(engine: &mut Engine<RegistryLookup>) {
    let pending = engine.pending().clone();
    let actor = engine.decision_actor().expect("somebody is asked");
    let action = match pending {
        Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
        Pending::Priority { .. } => PlayerAction::PassPriority,
        Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers { attackers: vec![] },
        Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers { blockers: vec![] },
        other => panic!("unexpected fixture question: {other:?}"),
    };
    engine.apply(actor, action).expect("the least answer");
}

fn on_battlefield(engine: &Engine<RegistryLookup>, owner: PlayerId, name: &str) -> Vec<ObjectId> {
    let view = player_view(engine.state(), ME, 0, None, &SeatContext::default(), &[]);
    view.battlefield
        .iter()
        .filter(|o| o.controller == owner && o.name == name)
        .map(|o| o.id)
        .collect()
}

/// This seat activated Mindslaver on its own turn, and the controlled
/// player's next turn has reached its first main phase: the controlled
/// player holds priority and this seat answers for them.
///
/// Mine: Mindslaver, four Forests to pay for it and `mine`. Theirs:
/// `board` and `hand` (their library is Forests, so they draw one).
fn their_controlled_main(mine: &[&str], board: &[&str], hand: &[&str]) -> Engine<RegistryLookup> {
    let mut preset = baylee_cards::decks::probe_preset(41, entry("Forest").card).unwrap();
    for seat in &mut preset.seats {
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_battlefield = ["Mindslaver", "Forest", "Forest", "Forest", "Forest"]
        .iter()
        .chain(mine)
        .map(|name| entry(name))
        .collect();
    preset.seats[1].starting_battlefield = board.iter().map(|name| entry(name)).collect();
    preset.seats[1].starting_hand = Some(hand.iter().map(|name| entry(name)).collect());
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let my_main = |e: &Engine<RegistryLookup>| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == ME)
            && e.state().turn.active == ME
            && e.state().turn.step == Step::Main
    };
    for _ in 0..200 {
        if my_main(&engine) {
            break;
        }
        least(&mut engine);
    }
    assert!(my_main(&engine), "no main phase of my own");
    let mindslaver = on_battlefield(&engine, ME, "Mindslaver")[0];
    for forest in on_battlefield(&engine, ME, "Forest").into_iter().take(4) {
        engine
            .apply(ME, PlayerAction::ActivateManaAbility { source: forest })
            .unwrap();
    }
    engine
        .apply(
            ME,
            PlayerAction::ActivateAbility {
                source: mindslaver,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            ME,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![THEM],
            },
        )
        .unwrap();
    let controlled_main = |e: &Engine<RegistryLookup>| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == THEM)
            && e.state().turn.active == THEM
            && e.state().turn.step == Step::Main
    };
    for _ in 0..400 {
        if controlled_main(&engine) {
            break;
        }
        least(&mut engine);
    }
    assert!(controlled_main(&engine), "no controlled main phase");
    assert_eq!(
        engine.decision_actor(),
        Some(ME),
        "the controlled player's priority is answered by its controller"
    );
    engine
}

/// Sends what the client queued, as this seat, and hands back the view.
fn send(duel: &mut Duel, engine: &mut Engine<RegistryLookup>) {
    for action in std::mem::take(&mut duel.outbox) {
        engine
            .apply(ME, action.clone())
            .unwrap_or_else(|e| panic!("{action:?} refused: {e:?}"));
    }
    duel.interaction = None;
    sync(duel, engine);
}

/// Spends a run the way the frame loop does, until it is over or waits for
/// a question of the cast's own (its targets) that the player answers.
fn run(duel: &mut Duel, engine: &mut Engine<RegistryLookup>) {
    for _ in 0..20 {
        if duel.mana_run.is_none() {
            return;
        }
        if !matches!(
            duel.interaction.as_ref().map(Interaction::pending),
            Some(Pending::Priority { .. })
        ) {
            return;
        }
        advance_mana_run(duel);
        send(duel, engine);
    }
    panic!("the run did not finish");
}

/// Everybody passes until the stack is empty.
fn resolve(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..10 {
        let empty = player_view(engine.state(), ME, 0, None, &SeatContext::default(), &[])
            .stack
            .is_empty();
        if empty || !matches!(engine.pending(), Pending::Priority { .. }) {
            return;
        }
        let actor = engine.decision_actor().unwrap();
        engine.apply(actor, PlayerAction::PassPriority).unwrap();
    }
}

fn their_hand_card(duel: &Duel, name: &str) -> ObjectId {
    baylee_client_core::decision::hand(duel.view.as_ref().unwrap())
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("no {name} in the hand this seat plays from"))
        .id
}

fn tapped(engine: &Engine<RegistryLookup>, owner: PlayerId, name: &str) -> usize {
    let view = player_view(engine.state(), ME, 0, None, &SeatContext::default(), &[]);
    view.battlefield
        .iter()
        .filter(|o| o.controller == owner && o.name == name && o.status.is_tapped())
        .count()
}

/// The controlled player's creature spell, cast from their hand with their
/// lands: the hand this seat plays from is theirs, the card is offered on
/// the click that taps their Forests, the armed deed survives the question
/// being asked again, and none of this seat's own lands is touched.
#[test]
fn their_spell_is_cast_from_their_hand_with_their_lands() {
    let mut engine = their_controlled_main(
        &["Forest", "Forest"],
        &["Forest", "Forest"],
        &["Grizzly Bears"],
    );
    let mut duel = Duel::default();
    sync(&mut duel, &engine);
    assert_eq!(duel.actor(), Some(THEM));
    assert_eq!(duel.deciding_for(), Some(THEM));
    assert!(duel.interaction.as_ref().unwrap().is_mine());
    let bears = their_hand_card(&duel, "Grizzly Bears");
    assert!(
        duel.reachable.contains(&bears),
        "their bears are reached for with their lands: {:?}",
        duel.reachable
    );
    let mine_tapped = tapped(&engine, ME, "Forest");

    input::activate_card(&mut duel, bears);
    assert!(
        matches!(duel.armed, Some(Armed { object, .. }) if object == bears),
        "the first click arms"
    );
    // Anybody saying anything re-sends the question; the deed stands.
    sync(&mut duel, &engine);
    assert!(
        matches!(duel.armed, Some(Armed { object, .. }) if object == bears),
        "a re-sent controlled priority took the armed deed"
    );
    input::activate_card(&mut duel, bears);
    assert!(duel.mana_run.is_some(), "the second click starts the run");
    run(&mut duel, &mut engine);
    assert!(duel.mana_run.is_none(), "the run is spent");
    resolve(&mut engine);
    assert_eq!(
        on_battlefield(&engine, THEM, "Grizzly Bears").len(),
        1,
        "the bears entered under their owner's control (CR 723.3)"
    );
    assert_eq!(tapped(&engine, THEM, "Forest"), 2, "their two Forests paid");
    assert_eq!(
        tapped(&engine, ME, "Forest"),
        mine_tapped,
        "and none of mine did (CR 723.5a)"
    );
}

/// Their burn aimed at their own creature: the target question is theirs,
/// this seat answers it, and nothing in the client takes their own
/// creature off the list.
#[test]
fn their_burn_is_aimed_at_their_own_creature() {
    let mut engine =
        their_controlled_main(&[], &["Mountain", "Grizzly Bears"], &["Lightning Bolt"]);
    let mut duel = Duel::default();
    sync(&mut duel, &engine);
    let bolt = their_hand_card(&duel, "Lightning Bolt");
    let their_bears = on_battlefield(&engine, THEM, "Grizzly Bears")[0];
    assert!(duel.reachable.contains(&bolt), "{:?}", duel.reachable);
    input::activate_card(&mut duel, bolt);
    input::activate_card(&mut duel, bolt);
    run(&mut duel, &mut engine);
    let Some(Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    }) = duel.interaction.as_ref().map(Interaction::pending)
    else {
        panic!(
            "the bolt's target: {:?}",
            duel.interaction.as_ref().map(|i| i.pending().clone())
        );
    };
    assert_eq!(*player, THEM);
    assert!(
        options.contains(&their_bears) && player_options.contains(&THEM),
        "their own creature and their own face are offered"
    );
    assert!(duel.interaction.as_ref().unwrap().is_mine());
    assert_eq!(
        input::activate_card(&mut duel, their_bears),
        baylee_client_core::touch::Answer::Took
    );
    let action = duel
        .interaction
        .as_ref()
        .unwrap()
        .confirm()
        .expect("an answer");
    assert!(
        matches!(&action,
            PlayerAction::ChooseTargets { objects, .. } | PlayerAction::ChooseObjects { objects }
                if *objects == vec![their_bears]),
        "{action:?}"
    );
    duel.submit(action);
    send(&mut duel, &mut engine);
    run(&mut duel, &mut engine);
    resolve(&mut engine);
    assert!(
        on_battlefield(&engine, THEM, "Grizzly Bears").is_empty(),
        "their bolt killed their own bears"
    );
    assert_eq!(
        tapped(&engine, THEM, "Mountain"),
        1,
        "paid with their Mountain"
    );
}

/// Their discard spell aimed at them, and its resolution choice: this seat
/// picks which of their cards they discard, from their hand.
#[test]
fn their_mind_rot_at_themselves_and_the_discard_it_asks_for() {
    let mut engine = their_controlled_main(
        &[],
        &["Swamp", "Swamp", "Swamp"],
        &["Mind Rot", "Grizzly Bears", "Lightning Bolt", "Mountain"],
    );
    let mut duel = Duel::default();
    sync(&mut duel, &engine);
    let rot = their_hand_card(&duel, "Mind Rot");
    assert!(duel.reachable.contains(&rot), "{:?}", duel.reachable);
    input::activate_card(&mut duel, rot);
    input::activate_card(&mut duel, rot);
    run(&mut duel, &mut engine);
    // "Target player", asked of them: themselves is one of the answers.
    let target = duel.interaction.as_ref().map(Interaction::pending).cloned();
    let Some(Pending::ChoosePlayer { player, options }) = &target else {
        panic!("the rot's target: {target:?}");
    };
    assert_eq!(*player, THEM);
    let at = options.iter().position(|p| *p == THEM).expect("themselves");
    assert!(duel.interaction.as_mut().unwrap().choose_index(at));
    let action = duel.interaction.as_ref().unwrap().confirm().unwrap();
    assert_eq!(action, PlayerAction::ChoosePlayer(THEM));
    duel.submit(action);
    send(&mut duel, &mut engine);
    run(&mut duel, &mut engine);
    resolve(&mut engine);
    sync(&mut duel, &engine);
    let asked = duel.interaction.as_ref().map(Interaction::pending).cloned();
    assert_eq!(
        asked.as_ref().and_then(baylee_ai::pending_player),
        Some(THEM),
        "the discard is theirs: {asked:?}"
    );
    assert_eq!(duel.actor(), Some(THEM));
    let bolt = their_hand_card(&duel, "Lightning Bolt");
    let bears = their_hand_card(&duel, "Grizzly Bears");
    input::activate_card(&mut duel, bolt);
    input::activate_card(&mut duel, bears);
    let action = duel
        .interaction
        .as_ref()
        .unwrap()
        .confirm()
        .expect("two picked");
    duel.submit(action);
    send(&mut duel, &mut engine);
    let view = player_view(engine.state(), THEM, 0, None, &SeatContext::default(), &[]);
    let kept: Vec<_> = view.hand.iter().map(|c| c.name.as_str()).collect();
    assert!(
        !kept.contains(&"Lightning Bolt") && !kept.contains(&"Grizzly Bears"),
        "the two this seat picked were discarded: {kept:?}"
    );
}

/// Their creature attacks, declared by this seat; this seat's own blocking
/// decision in that turn is its own again (CR 723.8), over its own
/// creatures.
#[test]
fn their_attack_is_declared_here_and_my_blocks_stay_mine() {
    let mut engine = their_controlled_main(&["Grizzly Bears"], &["Grizzly Bears"], &[]);
    // Their main phase passes into combat.
    let mut duel = Duel::default();
    for _ in 0..10 {
        if matches!(engine.pending(), Pending::ChooseAttackers { .. }) {
            break;
        }
        engine.apply(ME, PlayerAction::PassPriority).unwrap();
    }
    sync(&mut duel, &engine);
    let theirs = on_battlefield(&engine, THEM, "Grizzly Bears")[0];
    let mine = on_battlefield(&engine, ME, "Grizzly Bears")[0];
    assert!(
        matches!(duel.interaction.as_ref().map(Interaction::pending),
            Some(Pending::ChooseAttackers { player, .. }) if *player == THEM),
        "their attack is asked of them"
    );
    assert!(
        duel.activatable.contains(&theirs),
        "their creature is lit to declare: {:?}",
        duel.activatable
    );
    assert!(!duel.activatable.contains(&mine));
    input::activate_card(&mut duel, theirs);
    let action = duel
        .interaction
        .as_ref()
        .unwrap()
        .confirm()
        .expect("declared");
    assert_eq!(
        action,
        PlayerAction::DeclareAttackers {
            attackers: vec![(theirs, Defender::Player(ME))]
        }
    );
    duel.submit(action);
    send(&mut duel, &mut engine);
    for _ in 0..10 {
        if matches!(engine.pending(), Pending::ChooseBlockers { .. }) {
            break;
        }
        engine.apply(ME, PlayerAction::PassPriority).unwrap();
    }
    sync(&mut duel, &engine);
    assert!(
        matches!(duel.interaction.as_ref().map(Interaction::pending),
            Some(Pending::ChooseBlockers { player, .. }) if *player == ME),
        "the blocks are mine"
    );
    assert_eq!(duel.actor(), Some(ME), "my own decision, my own resources");
    assert_eq!(duel.deciding_for(), None);
    assert!(duel.activatable.contains(&mine), "{:?}", duel.activatable);
    assert!(!duel.activatable.contains(&theirs));
}

// The same edges with a view built by hand: what the client keeps between
// questions, which an engine-driven run passes over too quickly to see.

fn obj(slot: u32) -> ObjectId {
    ObjectId::new(slot, 0)
}

/// A basic Forest under `controller`, a source the planner can read.
fn forest(slot: u32, controller: u8) -> baylee_view::PublicObject {
    let mut o = baylee_client_core::test_support::token(slot, controller, "Forest", 0, 0);
    o.types = baylee_core::types::TypeSet::LAND;
    o.power = None;
    o.toughness = None;
    o.subtypes
        .insert(baylee_core::generated::subtypes::land::FOREST);
    o
}

/// Their priority, as this seat is asked it.
fn their_priority(legal: LegalActions) -> Pending {
    Pending::Priority {
        player: THEM,
        legal: Box::new(legal),
    }
}

/// This seat holds its own card, controls theirs, and their Forest and two
/// of its own stand on the table.
fn controlling_view() -> PlayerView {
    let mut view = baylee_client_core::test_support::ViewBuilder::new(2)
        .with_hand(vec![("Own card", 1, 10)])
        .controlling(1, vec![("Their card", 1, 20)])
        .with_battlefield(1, [forest(31, 1)])
        .with_battlefield(0, [forest(41, 0), forest(42, 0)])
        .build();
    // Both are a card with one way to be cast, so a click arms it rather
    // than opening a chooser.
    let bears = baylee_cards::decks::by_name("Grizzly Bears").expect("Grizzly Bears");
    view.hand[0].card.index = bears;
    view.controlled_hands[0].cards[0].card.index = bears;
    view
}

#[test]
fn a_controlled_priority_counts_their_hand_and_keeps_the_armed_deed() {
    let mut duel = Duel::default();
    duel.receive_view(controlling_view());
    let legal = LegalActions {
        can_pass: true,
        castable: vec![obj(20)],
        mana_abilities: vec![obj(31)],
        ..LegalActions::default()
    };
    duel.receive_choice(their_priority(legal.clone()));
    assert_eq!(duel.actor(), Some(THEM));
    assert!(duel.a_spell_is_castable(), "their card, in their hand");
    assert_eq!(duel.castable_in_hand(), 1);

    input::activate_card(&mut duel, obj(20));
    assert!(matches!(duel.armed, Some(Armed { object, deed: Deed::Play }) if object == obj(20)));
    duel.receive_view(controlling_view());
    duel.receive_choice(their_priority(legal.clone()));
    assert!(
        duel.armed.is_some(),
        "the controlled priority asked again is the window the deed was armed in"
    );
    input::activate_card(&mut duel, obj(20));
    assert_eq!(duel.outbox(), [PlayerAction::CastSpell { card: obj(20) }]);

    // My own priority in their turn (CR 723.8): my hand, and an armed deed
    // of theirs does not cross over.
    duel.take_outbox();
    duel.interaction = None;
    duel.receive_choice(their_priority(legal));
    input::activate_card(&mut duel, obj(20));
    assert!(duel.armed.is_some());
    let mut mine = controlling_view();
    mine.decision_player = Some(ME);
    duel.receive_view(mine);
    duel.receive_choice(Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            can_pass: true,
            castable: vec![obj(10)],
            ..LegalActions::default()
        }),
    });
    assert_eq!(duel.actor(), Some(ME));
    assert!(
        duel.armed.is_none(),
        "their card's deed went with their decision"
    );
    assert_eq!(duel.castable_in_hand(), 1, "my own card now");
}

#[test]
fn a_mana_run_on_their_priority_taps_their_land() {
    let mut duel = Duel::default();
    duel.receive_view(controlling_view());
    let legal = LegalActions {
        can_pass: true,
        mana_abilities: vec![obj(31), obj(41), obj(42)],
        ..LegalActions::default()
    };
    duel.receive_choice(their_priority(legal.clone()));
    let view = duel.view.as_ref().unwrap();
    let plan = baylee_client_core::manaplan::plan(
        &baylee_core::mana::ManaCost::parse("{G}"),
        &view.seat(THEM).unwrap().mana_pool,
        &manasources::sources(
            view,
            &LegalActions {
                mana_abilities: vec![obj(31)],
                ..LegalActions::default()
            },
        ),
    )
    .expect("their Forest pays {G}");
    duel.mana_run = Some(ManaRun::new(plan, obj(20), RunEnd::Float));
    advance_mana_run(&mut duel);
    assert_eq!(
        duel.outbox(),
        [PlayerAction::ActivateManaAbility { source: obj(31) }],
        "their Forest is tapped on their priority, not the run aborted: {:?}",
        duel.last_error
    );
    assert!(duel.last_error.is_none());
}

/// The X question of their cast is capped by what their lands pay, from the
/// priority this seat answered for them, and never by its own lands.
#[test]
fn their_x_is_capped_by_their_lands() {
    let endless = baylee_cards::decks::by_name("Endless One").expect("Endless One");
    let view = || {
        let mut view = controlling_view();
        view.controlled_hands[0].cards[0].card.index = endless;
        view.battlefield
            .extend([forest(32, 1), forest(33, 1)].into_iter().map(|mut o| {
                o.controller = THEM;
                o.owner = THEM;
                o
            }));
        view
    };
    let x_question = Pending::ChooseNumber {
        player: THEM,
        min: 0,
        max: u32::MAX,
        reason: baylee_engine::choice::NumberPrompt::X,
    };
    let theirs = LegalActions {
        can_pass: true,
        payable: vec![obj(20)],
        mana_abilities: vec![obj(31), obj(32), obj(33)],
        ..LegalActions::default()
    };
    let mut duel = Duel::default();
    duel.receive_view(view());
    duel.receive_choice(their_priority(theirs.clone()));
    duel.receive_choice(their_priority(theirs));
    let mut casting = view();
    casting.casting = Some(obj(20));
    duel.receive_view(casting.clone());
    duel.receive_choice(x_question.clone());
    assert_eq!(
        duel.interaction.as_mut().unwrap().set_number(100),
        3,
        "their three Forests pay X = 3"
    );

    // A priority of my own was the last one answered: my lands say nothing
    // about their X.
    let mut duel = Duel::default();
    let mut mine = view();
    mine.decision_player = Some(ME);
    duel.receive_view(mine);
    let my_priority = Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            can_pass: true,
            mana_abilities: vec![obj(41), obj(42)],
            ..LegalActions::default()
        }),
    };
    duel.receive_choice(my_priority.clone());
    duel.receive_choice(my_priority);
    duel.receive_view(casting);
    duel.receive_choice(x_question);
    assert_eq!(duel.interaction.as_mut().unwrap().set_number(100), 100);
}

/// The controlled turn ends while a question of theirs was never answered
/// here (a clock ran out): the next view no longer hands this seat their
/// decision, and what it held for them goes, with nothing said about it.
#[test]
fn a_controlled_question_left_standing_goes_when_control_ends() {
    let mut duel = Duel::default();
    duel.receive_view(controlling_view());
    duel.receive_choice(their_priority(LegalActions {
        can_pass: true,
        castable: vec![obj(20)],
        ..LegalActions::default()
    }));
    input::activate_card(&mut duel, obj(20));
    assert!(duel.armed.is_some());
    let mut after = controlling_view();
    after.awaiting = Some(THEM);
    after.decision_player = Some(THEM);
    after.controlled_hands.clear();
    after.active = THEM;
    duel.receive_view(after);
    assert!(
        duel.interaction.is_none(),
        "their question went with the control"
    );
    assert!(duel.armed.is_none());
    assert!(duel.mana_run.is_none() && duel.last_error.is_none());

    // My own question left standing is not this rule's to drop: a game with
    // nobody controlled behaves as before.
    let mut duel = Duel::default();
    let own = baylee_client_core::test_support::ViewBuilder::new(2).build();
    duel.receive_view(own.clone());
    duel.receive_choice(Pending::Priority {
        player: ME,
        legal: Box::default(),
    });
    let mut theirs = own;
    theirs.awaiting = Some(THEM);
    theirs.decision_player = Some(THEM);
    duel.receive_view(theirs);
    assert!(duel.interaction.is_some());
}
