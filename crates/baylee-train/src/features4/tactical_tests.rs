//! Deterministic plumbing tests, not claims about a learned checkpoint's choices.
use super::*;
use crate::housedeck::HouseDeck;
use baylee_core::ids::PrintRef;
use baylee_core::preset::{AIProfile, DeckEntry, FormatId};
use baylee_gamehost::Session;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn card(name: &str) -> CardIndex {
    baylee_cards::all()
        .find(|card| card.name() == name)
        .expect("registered tactical card")
        .index
}

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        card: card(name),
        print: PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    }
}

fn game(hands: [&[&str]; 2], boards: [&[&str]; 2]) -> Session {
    let deck = HouseDeck::named("victory").unwrap();
    let mut preset = crate::selfplay::table_for(12, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    preset.format = FormatId::Freeform;
    preset.modifiers.clear();
    for (index, seat) in preset.seats.iter_mut().enumerate() {
        seat.deck = vec![entry("Forest"); 32];
        seat.commanders.clear();
        seat.sideboard.clear();
        seat.starting_hand = Some(hands[index].iter().map(|name| entry(name)).collect());
        seat.starting_battlefield = boards[index].iter().map(|name| entry(name)).collect();
        seat.starting_life = Some(20);
    }
    let mut session = Session::new(&preset).unwrap();
    assert!(session.take_over(P0));
    assert!(session.take_over(P1));
    session
}

/// Choose a legal encoded answer explicitly. No heuristic or engine-state read
/// enters encoding/decoding, and no synthetic Pending replaces the real one.
fn apply_encoded(session: &mut Session, wanted: &PlayerAction) {
    let player = session
        .pending()
        .asked()
        .expect("an actual offered question");
    let (pending, view) = session.view_for(player).expect("authorized deciding view");
    let state = encode_state4(&view, Some(&pending), None, &[]);
    let cands = encode_candidates4(&state, &view, &pending);
    if matches!(wanted, PlayerAction::CastSpell { .. }) {
        assert!(
            cands
                .candidates
                .iter()
                .any(|candidate| candidate.action == Some(PlayerAction::PassPriority)),
            "casting competes with an encoded pass, not a forced test choice"
        );
    }
    let index = cands
        .candidates
        .iter()
        .position(|candidate| candidate.action.as_ref() == Some(wanted))
        .unwrap_or_else(|| panic!("missing encoded {wanted:?} for {pending:?}"));
    let target = TeacherTarget4 {
        single_idx: u16::try_from(index).unwrap(),
        ..TeacherTarget4::default()
    };
    let decoded = decode_action4(&cands, &view, &pending, &target).expect("decode offered action");
    assert_eq!(&decoded, wanted);
    assert!(pending.answer_fault(&decoded).is_none());
    session
        .act(player, decoded)
        .expect("engine accepts decoded candidate");
}

fn pass(session: &mut Session) {
    apply_encoded(session, &PlayerAction::PassPriority);
}

fn main_phase(session: &mut Session) {
    for _ in 0..64 {
        let view = session.observe_view(P0);
        if view.active == P0
            && view.phase == Phase::FirstMain
            && matches!(session.pending(), Pending::Priority { player, .. } if *player == P0)
        {
            return;
        }
        match session.pending() {
            Pending::Mulligan { .. } => apply_encoded(session, &PlayerAction::MulliganKeep),
            Pending::Priority { .. } => pass(session),
            other => panic!("unexpected opening question {other:?}"),
        }
    }
    panic!("first main phase not reached");
}

fn tap_lands(session: &mut Session) {
    let Pending::Priority { legal, .. } = session.pending() else {
        panic!("mana priority")
    };
    let sources = legal.mana_abilities.clone();
    assert!(!sources.is_empty());
    let player = session.pending().asked().unwrap();
    let view = session.observe_view(player);
    for source in sources {
        if view
            .object(source)
            .is_some_and(|object| object.types.contains(TypeSet::LAND))
        {
            apply_encoded(session, &PlayerAction::ActivateManaAbility { source });
        }
    }
}

fn hand_object(session: &Session, player: PlayerId, name: &str) -> ObjectId {
    session
        .observe_view(player)
        .hand
        .iter()
        .find(|object| object.card.index == card(name))
        .expect("own hand card")
        .id
}

fn battlefield_object(session: &Session, name: &str) -> ObjectId {
    session
        .observe_view(P0)
        .battlefield
        .iter()
        .find(|object| {
            object
                .card
                .is_some_and(|identity| identity.index == card(name))
        })
        .expect("public battlefield card")
        .id
}

fn in_graveyard(session: &Session, player: PlayerId, name: &str) -> bool {
    session.observe_view(P0).graveyards[usize::from(player.get())]
        .iter()
        .any(|object| {
            object
                .card
                .is_some_and(|identity| identity.index == card(name))
        })
}

fn cast(session: &mut Session, player: PlayerId, name: &str) -> ObjectId {
    let object = hand_object(session, player, name);
    let Pending::Priority {
        player: asked,
        legal,
    } = session.pending()
    else {
        panic!("cast priority")
    };
    assert_eq!(*asked, player);
    assert!(
        legal.can_pass,
        "passing remains a competing legal policy action"
    );
    assert!(legal.castable.contains(&object), "engine offers this spell");
    apply_encoded(session, &PlayerAction::CastSpell { card: object });
    while matches!(session.pending(), Pending::ChooseCastMode { .. }) {
        apply_encoded(session, &PlayerAction::ChooseMode(0));
    }
    object
}

fn pass_until(session: &mut Session, condition: impl Fn(&Session) -> bool) {
    for _ in 0..16 {
        if condition(session) {
            return;
        }
        pass(session);
    }
    assert!(condition(session), "resolution condition not reached");
}

#[test]
fn instant_is_encoded_cast_and_targeted_on_the_opponents_turn_and_resolves() {
    let mut session = game([&[], &["Shock"]], [&[], &["Mountain"]]);
    main_phase(&mut session);
    pass(&mut session);
    assert_eq!(session.observe_view(P1).active, P0);
    tap_lands(&mut session);
    cast(&mut session, P1, "Shock");
    apply_encoded(
        &mut session,
        &PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![P0],
        },
    );
    assert_eq!(
        session.observe_view(P1).seats[1].mana_pool.total(),
        0,
        "Shock paid for its cast"
    );
    pass_until(&mut session, |session| {
        session.observe_view(P0).stack.is_empty()
    });
    assert_eq!(session.observe_view(P0).seats[0].life, 18);
    assert!(in_graveyard(&session, P1, "Shock"));
}

#[test]
fn counterspell_is_encoded_targets_the_stack_and_prevents_creature_resolution() {
    let mut session = game(
        [&["Llanowar Elves"], &["Counterspell"]],
        [&["Forest"], &["Island", "Island"]],
    );
    main_phase(&mut session);
    tap_lands(&mut session);
    let spell = cast(&mut session, P0, "Llanowar Elves");
    pass(&mut session);
    tap_lands(&mut session);
    cast(&mut session, P1, "Counterspell");
    apply_encoded(
        &mut session,
        &PlayerAction::ChooseTargets {
            objects: vec![spell],
            players: vec![],
        },
    );
    assert_eq!(
        session.observe_view(P1).seats[1].mana_pool.total(),
        0,
        "Counterspell paid both blue mana"
    );
    pass_until(&mut session, |session| {
        session.observe_view(P0).stack.is_empty()
    });
    assert!(in_graveyard(&session, P0, "Llanowar Elves"));
    assert!(in_graveyard(&session, P1, "Counterspell"));
    assert!(
        !session
            .observe_view(P0)
            .battlefield
            .iter()
            .any(|object| object.id == spell)
    );
}

#[test]
fn uncounterable_spell_is_a_legal_counterspell_target_but_still_resolves() {
    let mut session = game(
        [&["Abrupt Decay"], &["Counterspell"]],
        [
            &["Swamp", "Forest"],
            &["Island", "Island", "Llanowar Elves"],
        ],
    );
    main_phase(&mut session);
    let victim = battlefield_object(&session, "Llanowar Elves");
    tap_lands(&mut session);
    let decay = cast(&mut session, P0, "Abrupt Decay");
    apply_encoded(
        &mut session,
        &PlayerAction::ChooseTargets {
            objects: vec![victim],
            players: vec![],
        },
    );
    pass(&mut session);
    tap_lands(&mut session);
    cast(&mut session, P1, "Counterspell");
    apply_encoded(
        &mut session,
        &PlayerAction::ChooseTargets {
            objects: vec![decay],
            players: vec![],
        },
    );
    pass_until(&mut session, |session| {
        in_graveyard(session, P1, "Counterspell")
    });
    assert!(
        session
            .observe_view(P0)
            .stack
            .iter()
            .any(|object| object.id == decay)
    );
    assert!(
        session
            .observe_view(P0)
            .battlefield
            .iter()
            .any(|object| object.id == victim)
    );
    pass_until(&mut session, |session| {
        session.observe_view(P0).stack.is_empty()
    });
    assert!(in_graveyard(&session, P0, "Abrupt Decay"));
    assert!(in_graveyard(&session, P1, "Llanowar Elves"));
}
