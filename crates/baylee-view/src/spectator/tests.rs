use super::*;
use crate::objects::{CardIdentity, DamageSourceView, HandObject, PublicObject};
use crate::seats::CommanderView;
use crate::seen::SeenIn;
use crate::tests::{obj, view};
use crate::{LogZone, SharedHand};
use baylee_core::color::ColorSet;
use baylee_core::ids::{CardIndex, DamageSourceRef, ObjectId, PrintRef};
use baylee_core::types::TypeSet;
use std::collections::BTreeSet;

const fn card(n: u16) -> CardIdentity {
    CardIdentity {
        index: CardIndex::new(n as u32 + 1000),
        print: PrintRef::new(n),
        face: 0,
    }
}

fn shown(n: u16) -> PublicObject {
    PublicObject {
        card: Some(card(n)),
        ..obj(n.into(), 1)
    }
}

fn in_hand(n: u16) -> HandObject {
    HandObject {
        id: ObjectId::new(n.into(), 0),
        card: card(n),
        name: format!("card {n}"),
        mana_value: 0,
        colors: ColorSet::default(),
        types: TypeSet::CREATURE,
        commander: false,
    }
}

fn described(n: u16) -> DamageSourceView {
    DamageSourceView {
        source: DamageSourceRef {
            object: ObjectId::new(n.into(), 0),
            version: 3,
        },
        name: format!("target {n}"),
        card: Some(card(n)),
        rules: None,
        token: None,
        controller: PlayerId::new(1),
        zone: LogZone::Graveyard,
        is_current: false,
        referenced_by: Vec::new(),
        colors: ColorSet::default(),
        types: TypeSet::CREATURE,
        power: None,
        toughness: None,
        keywords: 0,
    }
}

/// A public table with one card in every place a spectator is shown one, and
/// one blank beside it in each zone that can hold a blank.
fn table() -> SpectatorView {
    let seat = view(2);
    let mut seats = seat.seats;
    seats[1].commanders.push(CommanderView {
        object: ObjectId::new(8, 0),
        card: Some(card(8)),
        name: "commander".to_owned(),
        casts: 2,
    });
    let blank = || obj(99, 1);
    SpectatorView {
        seq: 41,
        turn: 7,
        phase: Phase::Combat,
        step: Step::DeclareBlockers,
        active: PlayerId::new(1),
        awaiting: Some(PlayerId::new(1)),
        deciding: SeatSet::new(),
        monarch: Some(PlayerId::new(1)),
        initiative: Some(PlayerId::new(0)),
        day_night: None,
        seats,
        battlefield: vec![shown(1), blank()],
        stack: vec![shown(2)],
        graveyards: vec![vec![], vec![shown(3)]],
        exile: vec![vec![shown(4), blank()], vec![]],
        command: vec![vec![], vec![shown(5)]],
        combat: CombatView::default(),
        library_tops: vec![shown(6)],
        revealed_hands: vec![SharedHand {
            player: PlayerId::new(1),
            public: true,
            cards: vec![in_hand(7)],
        }],
        ballots: Vec::new(),
        held_choices: Vec::new(),
        controls: Vec::new(),
        target_objects: vec![described(9)],
        divided: Vec::new(),
    }
}

#[test]
fn a_spectator_has_earned_the_printing_of_every_card_the_table_shows_and_no_other() {
    let prints: Vec<_> = table().prints().collect();
    let expected: BTreeSet<_> = (1..=9).map(PrintRef::new).collect();
    assert_eq!(prints.len(), expected.len(), "{prints:?}");
    assert_eq!(prints.into_iter().collect::<BTreeSet<_>>(), expected);
}

#[test]
fn the_drawn_view_shows_exactly_the_printings_the_spectator_earned() {
    // Two walks over one table: a zone added to one of them alone would
    // leave a hole on the screen, or hand out a printing nobody was shown.
    let earned: BTreeSet<_> = table().prints().collect();
    let drawn: BTreeSet<_> = table()
        .into_player_view(PlayerId::new(0))
        .prints()
        .collect();
    assert_eq!(earned, drawn);
}

#[test]
fn a_spectator_drawn_as_a_seat_holds_no_hand_is_asked_nothing_and_decides_nothing() {
    for perspective in [0, 1] {
        let drawn = table().into_player_view(PlayerId::new(perspective));
        assert_eq!(drawn.seat, PlayerId::new(perspective));
        assert!(drawn.hand.is_empty());
        assert!(drawn.controlled_hands.is_empty());
        assert!(drawn.looking_at.is_empty());
        assert!(drawn.damage_sources.is_empty());
        assert!(drawn.clocks.is_empty());
        assert!(drawn.policy_acts.is_empty());
        assert!(drawn.trigger_order.is_empty());
        assert!(drawn.foreign_targets.is_empty());
        assert!(drawn.decision_player.is_none());
        assert!(drawn.decision_remaining_ms.is_none());
        assert!(drawn.owed.is_none());
        assert!(drawn.targeting.is_none());
        assert!(drawn.casting.is_none());
        assert!(drawn.announced_cast.is_none());
        assert!(drawn.announced_x.is_none());
        assert!(!drawn.priority_held);
        assert!(drawn.hand_shared_with.is_empty());
        assert!(drawn.hand_requests.is_empty());
        assert!(drawn.hand_requested.is_empty());
    }
}

#[test]
fn a_spectator_drawn_as_a_seat_sees_the_whole_public_table() {
    let public = table();
    let drawn = public.clone().into_player_view(PlayerId::new(0));
    assert_eq!(drawn.seq, public.seq);
    assert_eq!(drawn.turn, public.turn);
    assert_eq!(drawn.phase, public.phase);
    assert_eq!(drawn.step, public.step);
    assert_eq!(drawn.active, public.active);
    assert_eq!(drawn.awaiting, public.awaiting);
    assert_eq!(drawn.monarch, public.monarch);
    assert_eq!(drawn.initiative, public.initiative);
    assert_eq!(drawn.seats, public.seats);
    assert_eq!(drawn.battlefield, public.battlefield);
    assert_eq!(drawn.stack, public.stack);
    assert_eq!(drawn.graveyards, public.graveyards);
    assert_eq!(drawn.exile, public.exile);
    assert_eq!(drawn.command, public.command);
    assert_eq!(drawn.library_tops, public.library_tops);
    assert_eq!(drawn.shared_hands, public.revealed_hands);
    assert_eq!(drawn.target_objects, public.target_objects);
}

#[test]
fn each_card_is_seen_where_it_lies_and_a_blank_is_not_seen_at_all() {
    let drawn = table().into_player_view(PlayerId::new(0));
    let seen: Vec<_> = drawn
        .identities()
        .map(|s| (s.zone, s.card.print.get(), s.owner.map(PlayerId::get)))
        .collect();
    let mut expected = vec![
        (SeenIn::Battlefield, 1, Some(1)),
        (SeenIn::Stack, 2, Some(1)),
        (SeenIn::Graveyard, 3, Some(1)),
        (SeenIn::Exile, 4, Some(1)),
        (SeenIn::Command, 5, Some(1)),
        (SeenIn::LibraryTop, 6, Some(1)),
        (SeenIn::SharedHand, 7, Some(1)),
        (SeenIn::Commander, 8, Some(1)),
        (SeenIn::TargetObject, 9, None),
    ];
    expected.sort_by_key(|(_, print, _)| *print);
    let mut sorted = seen;
    sorted.sort_by_key(|(_, print, _)| *print);
    assert_eq!(sorted, expected);
}

#[test]
fn a_spectators_table_reads_back_whole() {
    let public = table();
    let json = serde_json::to_string(&public).unwrap();
    assert_eq!(
        serde_json::from_str::<SpectatorView>(&json).unwrap(),
        public
    );
}

#[test]
fn a_spectators_json_names_no_field_a_private_hand_could_travel_in() {
    let json = serde_json::to_value(table()).unwrap();
    let fields = json.as_object().unwrap();
    for private in [
        "hand",
        "controlled_hands",
        "looking_at",
        "damage_sources",
        "owed",
        "targeting",
        "casting",
        "announced_cast",
        "clocks",
        "hand_requests",
        "decision_player",
    ] {
        assert!(!fields.contains_key(private), "{private}");
    }
}
