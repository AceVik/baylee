use super::*;
use baylee_client_core::board::{BoardModel, Lane, Provenance, SeatPod};
use baylee_client_core::interaction::Interaction;
use baylee_client_core::layout::LaneKind;
use baylee_core::ids::Defender;
use baylee_engine::choice::Pending;

fn obj(slot: u32) -> ObjectId {
    ObjectId::new(slot, 0)
}

/// One creature, standing for itself.
fn creature(slot: u32) -> CardGroup {
    CardGroup {
        representative: obj(slot),
        members: vec![obj(slot)],
        name: format!("Creature {slot}"),
        power: Some(2),
        toughness: Some(2),
        damage: 0,
        loyalty: None,
        status: baylee_view::ObjectStatus::NONE,
        counters: Vec::new(),
        badges: Vec::new(),
        art: None,
        provenance: Provenance::Token,
        original: None,
        summoning_sick: false,
        activatable: false,
        commander: false,
        individual: None,
    }
}

/// A one-seat board holding `groups` in the creature lane.
fn board(groups: Vec<CardGroup>) -> BoardModel {
    BoardModel {
        seq: 1,
        local: PlayerId::new(0),
        turn: 1,
        step: baylee_view::Step::DeclareAttackers,
        pods: vec![SeatPod {
            player: PlayerId::new(0),
            life: 20,
            poison: 0,
            energy: 0,
            hand_count: 0,
            library_count: 40,
            graveyard_count: 0,
            has_lost: false,
            is_local: true,
            is_active: true,
            has_priority: true,
            lanes: vec![Lane {
                kind: LaneKind::Creatures,
                groups,
                overflowing: false,
            }],
            piles: baylee_client_core::PileKind::ALL
                .into_iter()
                .map(baylee_client_core::ZonePile::empty)
                .collect(),
            tokens: Vec::new(),
            threat: baylee_client_core::ThreatSummary::default(),
        }],
        stack: Vec::new(),
        hand: Vec::new(),
    }
}

fn duel(board: BoardModel, interaction: Option<Interaction>) -> Duel {
    Duel {
        board: Some(board),
        layout: Some(TableLayout::new(&[PlayerId::new(0)], 1.78, None)),
        interaction,
        ..Duel::default()
    }
}

/// The defect `docs/design.md` §2.2 calls "unreadable": both combat
/// prompts were fully operable and drew nothing, because the sync loop
/// asked `selected()` — a list that stays empty in the two combat modes,
/// where the answer being built is a list of *pairs*.
#[test]
fn a_declared_attacker_is_marked_chosen_and_an_undeclared_one_is_not() {
    let mut i = Interaction::new(
        Pending::ChooseAttackers {
            player: PlayerId::new(0),
            attackers: vec![obj(1), obj(2)],
            defenders: vec![Defender::Player(PlayerId::new(1))],
        },
        PlayerId::new(0),
    );
    assert!(
        i.declare_attacker(obj(1), Defender::Player(PlayerId::new(1))),
        "the choice offered this attacker"
    );

    let duel = duel(board(vec![creature(1), creature(2)]), Some(i));
    let placed = placements(&duel);
    let chosen = |id: ObjectId| {
        placed
            .iter()
            .find(|p| p.object == id)
            .unwrap_or_else(|| panic!("{id:?} is on the table"))
            .selected
    };
    assert!(chosen(obj(1)), "the declared attacker lies flat");
    assert!(
        !chosen(obj(2)),
        "a creature held back is drawn as attacking"
    );
}

/// A card standing for four is chosen when *any* of the four is: the
/// representative is a drawing decision, and a plan or a declaration
/// names one particular permanent.
#[test]
fn a_stack_of_identical_creatures_is_chosen_by_any_of_its_members() {
    let mut group = creature(1);
    group.members = vec![obj(1), obj(2), obj(3)];
    let mut i = Interaction::new(
        Pending::ChooseAttackers {
            player: PlayerId::new(0),
            attackers: vec![obj(1), obj(2), obj(3)],
            defenders: vec![Defender::Player(PlayerId::new(1))],
        },
        PlayerId::new(0),
    );
    // Not the representative — that is the whole point.
    assert!(i.declare_attacker(obj(3), Defender::Player(PlayerId::new(1))));

    let duel = duel(board(vec![group]), Some(i));
    let placed = placements(&duel);
    assert_eq!(placed.len(), 1, "one card stands for the three");
    assert!(
        placed[0].selected,
        "the card drawn for the stack ignores a declaration by a member \
         that is not its representative"
    );
}

/// The same rule outside combat, where `selected()` *is* the answer being
/// built. Reading the raw list was wrong here too, just less visibly: a
/// target chosen from a stack of four is one particular permanent, and
/// the card drawn for the stack is the only thing on the table that can
/// show it has been chosen.
#[test]
fn a_target_chosen_from_a_stack_lifts_the_card_that_stands_for_it() {
    let mut group = creature(1);
    group.members = vec![obj(1), obj(2), obj(3)];
    let mut i = Interaction::new(
        Pending::ChooseTargets {
            player: PlayerId::new(0),
            options: vec![obj(1), obj(2), obj(3)],
            player_options: vec![],
            min: 1,
            max: 1,
            reason: baylee_engine::choice::TargetPrompt::Targets,
        },
        PlayerId::new(0),
    );
    // Again not the representative.
    assert_eq!(
        i.toggle(obj(3)),
        baylee_client_core::interaction::SelectionOutcome::Added
    );

    let duel = duel(board(vec![group]), Some(i));
    assert!(
        placements(&duel)[0].selected,
        "the stack was targeted and the card drawn for it sits flat"
    );
}

/// Reported from a four-player game: the cards on the table flickered
/// against each other in bands.
///
/// A lane fans once it holds more than fits, and a fan is overlap by
/// definition — so the two quads sharing a patch of felt were at exactly
/// the same height, and which of them a pixel belongs to was decided by
/// the last bit of an interpolated depth. Every card on a lane got
/// `0.0`.
#[test]
fn cards_that_overlap_in_a_row_do_not_lie_at_the_same_height() {
    let seats: Vec<_> = (0..4).map(PlayerId::new).collect();
    let duel = Duel {
        // Enough of them that the row fans however wide a four-seat pod
        // is: twelve stopped fanning the day `MIN_POD_WIDTH` went up, and
        // a test that quietly stops testing is worse than one that fails
        // — which is what the assertion at the bottom is for.
        board: Some(board((1..=24).map(creature).collect())),
        layout: Some(TableLayout::new(&seats, 2.01, None)),
        ..Duel::default()
    };
    let placed = placements(&duel);

    let mut overlaps = 0;
    for pair in placed.windows(2) {
        if pair[0].position.distance(pair[1].position) >= CARD_WIDTH {
            continue;
        }
        overlaps += 1;
        assert!(
            pair[1].lift > pair[0].lift,
            "two cards {} apart — closer than a card is wide — both sit at {}",
            pair[0].position.distance(pair[1].position),
            pair[0].lift
        );
    }
    assert!(
        overlaps > 0,
        "the row did not fan, so nothing about overlapping cards was tested"
    );

    // And it stays a row: the whole rise is smaller than the gap a card
    // already floats above the felt, so this is order for the depth
    // buffer and not a staircase for the eye.
    let top = placed.iter().map(|p| p.lift).fold(0.0_f32, f32::max);
    assert!(
        top < CARD_LIFT,
        "a row of twelve climbs {top} off the table"
    );
}

/// What the pointer does to a pile.
mod fan {
    use super::*;

    /// A seat whose graveyard holds `count` cards, with a fan of at most
    /// seven of them and nothing else on the board.
    fn with_graveyard(count: u32) -> BoardModel {
        let mut model = board(Vec::new());
        let pile = model.pods[0]
            .piles
            .iter_mut()
            .find(|p| p.kind == baylee_client_core::PileKind::Graveyard)
            .expect("every seat has a graveyard");
        pile.count = count;
        pile.top = Some(obj(100));
        pile.fan = (0..count.min(7))
            .map(|i| baylee_client_core::FannedCard {
                // Top of the pile first, so the first is the top card.
                object: obj(100 - i),
                art: None,
                name: format!("card {i}"),
            })
            .collect();
        model
    }

    fn hovering(board: BoardModel, hovered: Option<ObjectId>) -> Duel {
        Duel {
            board: Some(board),
            layout: Some(TableLayout::new(&[PlayerId::new(0)], 1.78, None)),
            hovered,
            ..Duel::default()
        }
    }

    /// A pointer on a graveyard card lifts the whole fan out of the pile,
    /// and a pointer anywhere else leaves one card lying on it.
    ///
    /// The counter-arm is the half that matters: a fan that was always
    /// out would pass the first assertion on its own.
    #[test]
    fn the_pile_opens_under_the_pointer_and_shuts_when_it_leaves() {
        let shut = placements(&hovering(with_graveyard(10), None));
        assert_eq!(shut.len(), 1, "a pile nobody is pointing at is one card");
        assert!(shut[0].fan.is_none());
        assert_eq!(shut[0].object, obj(100));

        let open = placements(&hovering(with_graveyard(10), Some(obj(100))));
        assert_eq!(open.len(), 7, "the fan is not seven cards");
        assert!(
            open.iter().all(|p| p.fan.is_some()),
            "a card of the fan is lying on the table"
        );
        assert_eq!(
            open.iter().filter(|p| p.object == obj(100)).count(),
            1,
            "the top card is drawn twice — once as the pile and once as \
             the fan, which is two entities for one id"
        );
    }

    /// The fan stays open while the pointer travels along it.
    ///
    /// This is the case that makes a fan usable at all: once it is out,
    /// the pointer is no longer over the pile — it is over a card that
    /// was not on the table a moment ago — and a reading that knew only
    /// the pile's top card would shut on the first pixel of travel.
    #[test]
    fn the_pointer_may_travel_along_the_fan() {
        for card in [obj(100), obj(97), obj(94)] {
            let open = placements(&hovering(with_graveyard(10), Some(card)));
            assert_eq!(open.len(), 7, "the fan shut under the pointer at {card:?}");
        }
    }

    /// The pile's thickness stays on the bottom of the fan.
    ///
    /// The deck hangs under the card that carries the count, so putting
    /// the ten on the card the fan raised highest would lift the whole
    /// graveyard a card's height into the air with it. The bottom card is
    /// the one still standing over the pile's own place, and it is the
    /// rest of the cards.
    #[test]
    fn the_deck_stays_under_the_bottom_of_the_fan() {
        let open = placements(&hovering(with_graveyard(10), Some(obj(100))));
        let counts: Vec<usize> = open.iter().map(|p| p.count).collect();
        assert_eq!(
            counts,
            [1, 1, 1, 1, 1, 1, 4],
            "the deck is on the wrong card"
        );

        // And a pile with nothing under the fan still stands on one card.
        let shallow = placements(&hovering(with_graveyard(3), Some(obj(100))));
        assert_eq!(
            shallow.iter().map(|p| p.count).collect::<Vec<_>>(),
            [1, 1, 1]
        );
    }

    /// Every card of the fan is tipped towards the camera.
    ///
    /// Measured as the quad's own normal after the rotation: world `+z`
    /// is where the camera is, so the face leans into it when the third
    /// component is positive. A card lying flat answers zero, which is
    /// what the counter-arm at the bottom holds.
    #[test]
    fn the_fan_leans_into_the_screen() {
        let duel = hovering(with_graveyard(7), Some(obj(100)));
        let placed = placements(&duel);
        let slot = placed[0].slot;
        for card in &placed {
            let (pose, _) = card.fan.expect("every card here is fanned");
            let normal = fan_rotation(&card.slot, pose) * Vec3::Z;
            assert!(
                normal.z > 0.3,
                "a fanned card faces {normal:?}, which is not the camera"
            );
        }

        let flat = card_transform(&slot, Vec2::ZERO, false, 0.0).rotation * Vec3::Z;
        assert!(
            flat.z.abs() < 1e-6,
            "a card on the table already leans, so leaning proves nothing"
        );
    }
}
