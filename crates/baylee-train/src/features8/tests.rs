use super::*;
use baylee_core::ids::{PlayerId, PrintRef};
use baylee_core::mana::ManaColor;
use baylee_core::preset::{AIProfile, DeckEntry, FormatId};
use baylee_engine::choice::PlayerAction;
use baylee_gamehost::Session;
use baylee_view::Phase;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

#[test]
fn experimental_announcement_x_does_not_change_encoder7_or8_or_legacy_hashes() {
    let (session, _) = at_x_choice();
    let (pending, view) = session.view_for_with_payment_context(P0).unwrap();
    let mut experimental = view.clone();
    experimental.announced_x = Some(7);
    for (before, after) in [
        (
            crate::features7::encode_state7(&view, Some(&pending), None, &[]),
            crate::features7::encode_state7(&experimental, Some(&pending), None, &[]),
        ),
        (
            encode_state8(&view, Some(&pending), None, &[]),
            encode_state8(&experimental, Some(&pending), None, &[]),
        ),
    ] {
        assert_eq!(
            serde_json::to_value(before).unwrap(),
            serde_json::to_value(after).unwrap()
        );
    }
    assert_eq!(
        board_observation_hash8(&view),
        board_observation_hash8(&experimental)
    );
    assert_eq!(
        view_only_state_hash8(&view, &pending),
        view_only_state_hash8(&experimental, &pending)
    );
    assert_eq!(
        crate::features4::board_observation_hash(&view),
        crate::features4::board_observation_hash(&experimental)
    );
    assert_eq!(
        crate::features4::view_only_state_hash(&view, &pending),
        crate::features4::view_only_state_hash(&experimental, &pending)
    );
}

fn card(name: &str) -> DeckEntry {
    let card = baylee_cards::all()
        .find(|c| c.name() == name)
        .unwrap()
        .index;
    DeckEntry {
        card,
        print: PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    }
}

fn fixture() -> Session {
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    let mut preset = crate::selfplay::table_for(12, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    preset.format = FormatId::Freeform;
    preset.modifiers.clear();
    for seat in &mut preset.seats {
        seat.deck = vec![card("Forest"); 32];
        seat.commanders.clear();
        seat.sideboard.clear();
        seat.starting_life = Some(20);
        seat.starting_hand = Some(Vec::new());
        seat.starting_battlefield = Vec::new();
    }
    preset.seats[0].starting_hand = Some(vec![card("Vitalizing Cascade"), card("Fireball")]);
    preset.seats[0].starting_battlefield = ["Blossoming Sands", "Forest", "Plains"]
        .iter()
        .map(|name| card(name))
        .collect();
    let mut session = Session::new(&preset).unwrap();
    assert!(session.take_over(P0));
    assert!(session.take_over(P1));
    for _ in 0..64 {
        if session.observe_view(P0).phase == Phase::FirstMain {
            return session;
        }
        let player = session.pending().asked().unwrap();
        let action = match session.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("unexpected opening {other:?}"),
        };
        session.act(player, action).unwrap();
    }
    panic!("main phase not reached");
}

fn at_x_choice() -> (Session, baylee_core::ids::ObjectId) {
    let mut session = fixture();
    let view = session.observe_view_with_payment_context(P0);
    let source = view
        .hand
        .iter()
        .find(|c| c.name == "Vitalizing Cascade")
        .unwrap()
        .id;
    session
        .act(P0, PlayerAction::CastSpell { card: source })
        .unwrap();
    while matches!(session.pending(), Pending::ChooseCastMode { .. }) {
        session.act(P0, PlayerAction::ChooseMode(0)).unwrap();
    }
    assert!(matches!(
        session.pending(),
        Pending::ChooseNumber {
            reason: NumberPrompt::X,
            ..
        }
    ));
    (session, source)
}

#[test]
fn hand_spell_identity_is_explicit_without_changing_legacy_encoding() {
    let (session, source) = at_x_choice();
    let (pending, view) = session.view_for_with_payment_context(P0).unwrap();
    assert_eq!(view.casting, Some(source));
    assert_eq!(view.announced_cast, Some(source));
    assert!(view.hand.iter().any(|c| c.id == source));
    let other = view.hand.iter().find(|c| c.name == "Fireball").unwrap().id;
    let mut switched = view.clone();
    switched.casting = Some(other);
    switched.announced_cast = Some(other);
    let old = crate::features7::encode_state7(&view, Some(&pending), None, &[]);
    let old_other = crate::features7::encode_state7(&switched, Some(&pending), None, &[]);
    assert_eq!(
        serde_json::to_value(&old).unwrap(),
        serde_json::to_value(&old_other).unwrap()
    );
    let new = encode_state8(&view, Some(&pending), None, &[]);
    let new_other = encode_state8(&switched, Some(&pending), None, &[]);
    let first = new.find_entity(source).unwrap();
    let second = new.find_entity(other).unwrap();
    assert_eq!(
        (new.entity_feat[first][33], new.entity_feat[second][33]),
        (1.0, 0.0)
    );
    assert_eq!(
        (
            new_other.entity_feat[first][33],
            new_other.entity_feat[second][33]
        ),
        (0.0, 1.0)
    );
    assert_eq!(new.global_feat[29], 1.0);
    assert_eq!(new.global_feat[36], 1.0);
    assert_eq!(new.global_feat[39], 1.0);
    assert_eq!(old.entity_feat[first][33], 0.0);
    assert_eq!(old.global_feat[36], 0.0);
}

#[test]
fn source_survives_nested_color_choice_without_granting_cancel_permission() {
    let (mut session, source) = at_x_choice();
    session.act(P0, PlayerAction::ChooseNumber(1)).unwrap();
    let (_, payment) = session.view_for_with_payment_context(P0).unwrap();
    let land = payment
        .battlefield
        .iter()
        .find(|c| c.name == "Blossoming Sands")
        .unwrap()
        .id;
    let Pending::Priority { legal, .. } = session.pending() else {
        panic!("payment priority")
    };
    let ability = legal
        .abilities
        .iter()
        .find(|(id, _)| *id == land)
        .unwrap()
        .1;
    session
        .act(
            P0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: ability,
            },
        )
        .unwrap();
    let (pending, view) = session.view_for_with_payment_context(P0).unwrap();
    assert!(matches!(pending, Pending::ChooseColor { .. }));
    assert_eq!(view.casting, None);
    assert_eq!(view.announced_cast, Some(source));
    assert_eq!(
        session.observe_view_with_payment_context(P1).announced_cast,
        None
    );
    let encoded = encode_state8(&view, Some(&pending), None, &[]);
    assert_eq!(
        encoded.entity_feat[encoded.find_entity(source).unwrap()][33],
        1.0
    );
    assert_eq!(encoded.global_feat[29], 0.0);
    assert_eq!(encoded.global_feat[36], 1.0);
    assert!(session.act(P0, PlayerAction::CancelCast).is_err());
    session
        .act(P0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    session.act(P0, PlayerAction::CancelCast).unwrap();
    let view = session.observe_view_with_payment_context(P0);
    assert_eq!(view.announced_cast, None);
    assert_eq!(encode_state8(&view, None, None, &[]).global_feat[36], 0.0);
}

#[test]
fn announcement_changes_tick_only_the_new_observation_clock() {
    let (session, source) = at_x_choice();
    let (pending, mut view) = session.view_for_with_payment_context(P0).unwrap();
    view.casting = None; // A nested choice may suppress cancellation permission.
    let other = view.hand.iter().find(|c| c.name == "Fireball").unwrap().id;
    let mut changed = view.clone();
    changed.announced_cast = Some(other);
    let mut old = SeatKnowledge4::empty(P0, 2);
    let mut new = SeatKnowledge4::empty(P0, 2);
    assert!(old.observe(&view));
    assert!(new.observe_with_payment_context(&view));
    assert!(!old.observe(&changed));
    assert!(new.observe_with_payment_context(&changed));
    assert!(!new.observe_with_payment_context(&changed));
    assert_ne!(
        view_only_state_hash8(&view, &pending),
        view_only_state_hash8(&changed, &pending)
    );
    assert_eq!(
        crate::features4::view_only_state_hash(&view, &pending),
        crate::features4::view_only_state_hash(&changed, &pending)
    );
    assert_eq!(view.announced_cast, Some(source));
}

#[test]
fn scalar_question_reason_and_legal_bounds_are_observable() {
    let session = fixture();
    let view = session.observe_view_with_payment_context(P0);
    let x = Pending::ChooseNumber {
        player: P0,
        min: 2,
        max: 12,
        reason: NumberPrompt::X,
    };
    let count = Pending::ChooseNumber {
        player: P0,
        min: 2,
        max: 7,
        reason: NumberPrompt::Sacrifices,
    };
    let a = encode_state8(&view, Some(&x), None, &[]);
    let b = encode_state8(&view, Some(&count), None, &[]);
    assert_eq!(&a.global_feat[37..40], &[2.0, 12.0, 1.0]);
    assert_eq!(&b.global_feat[37..40], &[2.0, 7.0, 8.0]);
    assert_eq!(
        &encode_state8(&view, None, None, &[]).global_feat[36..40],
        &[0.0; 4]
    );
}
