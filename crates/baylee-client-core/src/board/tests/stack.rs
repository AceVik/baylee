//! The stack, and what a renderer can draw of an entry it is handed. Order is the first claim — the item that resolves next is first, and its depth says how far down the rest are — and the remainder is the lookup only this model can make: a `TargetRef` is a handle, so the name and the face drawn beside a spell are resolved here and kept resident like anything else. An ability has no card of its own and borrows one, from the face its *text* came from rather than the face its source happens to be showing; a source that has left the battlefield is a missing picture and never a missing entry (CR 113.7a). A targeted player has neither name nor face here, because seat names live in `GameStatic` and the renderer spells them out.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn the_stack_is_ordered_with_the_next_resolving_item_first() {
    let bottom = token(50, 0, "Counterspell", 0, 0);
    let top = token(51, 1, "Lightning Bolt", 0, 0);
    let view = ViewBuilder::new(2).with_stack(vec![bottom, top]).build();
    let m = model(&view);
    assert_eq!(m.stack.len(), 2);
    assert_eq!(m.stack[0].name, "Lightning Bolt");
    assert_eq!(m.stack[0].depth, 0);
    assert_eq!(m.stack[1].depth, 1);
}

#[test]
fn a_spell_on_the_stack_says_what_it_points_at() {
    let view = bolt_at_bears();
    let m = model(&view);
    let item = &m.stack[0];
    assert_eq!(item.kind, StackKind::Spell);
    assert!(item.art.is_some(), "a spell shows its own card");
    assert_eq!(item.targets.len(), 1);
    // The whole point: a handle is not drawable, a name and a face are.
    assert_eq!(item.targets[0].name.as_deref(), Some("Grizzly Bears"));
    assert!(item.targets[0].art.is_some());
    assert_eq!(item.targets[0].object(), Some(ObjectId::new(1, 0)));
}

#[test]
fn a_targets_picture_is_kept_resident_too() {
    let view = bolt_at_bears();
    let m = model(&view);
    let art = m.stack[0].targets[0].art.expect("the target has a face");
    assert!(
        m.required_images().contains(&art),
        "a target drawn beside the spell has to be loaded like anything else"
    );
}

#[test]
fn an_ability_on_the_stack_borrows_its_sources_picture() {
    let source = printed(1, 0, "Llanowar Elves", 33);
    let source_art = ImageKey::new(PrintRef::new(33), 0, ArtSize::Small);
    let mut ability = token(2, 0, "Llanowar Elves", 0, 0);
    ability.card = None;
    ability.types = TypeSet::EMPTY;
    ability.power = None;
    ability.toughness = None;
    ability.stack_item = Some(StackItem::Ability {
        token: None,
        source: ObjectId::new(1, 0),
        ability: Some(AbilityRef::new(CardIndex::new(33), 0)),
        text: None,
        rules: None,
    });
    let view = ViewBuilder::new(2)
        .with_battlefield(0, [source])
        .with_stack(vec![ability])
        .build();

    let m = model(&view);
    assert_eq!(
        m.stack[0].kind,
        StackKind::Ability {
            token: None,
            source: ObjectId::new(1, 0),
            text: None,
            rules: None,
            ability: Some(AbilityRef::new(CardIndex::new(33), 0)),
        }
    );
    assert_eq!(
        m.stack[0].art,
        Some(source_art),
        "an ability has no card, so it wears the picture of whatever made it"
    );
}

/// The picture and the sentence are two halves of one card, so the face
/// the host named for the *text* is the face the borrowed picture is
/// taken from — not the face the source happens to be showing.
///
/// A Sheoldred who has turned back over while her chapter ability waits
/// on the stack is the shape of it (CR 113.7a): the permanent on the
/// battlefield is face 0, and the ability is face 1's third sentence.
/// Drawing face 0 beside face 1's text would be one card illustrated
/// with another.
#[test]
fn an_abilitys_picture_is_taken_from_the_face_its_text_came_from() {
    let source = printed(1, 0, "Sheoldred", 33);
    let mut ability = token(2, 0, "Sheoldred", 0, 0);
    ability.card = None;
    ability.types = TypeSet::EMPTY;
    ability.power = None;
    ability.toughness = None;
    ability.stack_item = Some(StackItem::Ability {
        token: None,
        source: ObjectId::new(1, 0),
        ability: Some(AbilityRef::new(CardIndex::new(33), 2)),
        text: Some(StackText {
            face: 1,
            line: 2,
            of: 3,
        }),
        rules: Some(baylee_view::RulesFace {
            card: CardIndex::new(33),
            face: 1,
        }),
    });
    let view = ViewBuilder::new(2)
        .with_battlefield(0, [source])
        .with_stack(vec![ability])
        .build();

    let m = model(&view);
    assert_eq!(
        m.stack[0].art,
        Some(ImageKey::new(PrintRef::new(33), 1, ArtSize::Small)),
        "the source shows face 0 and the ability came off face 1"
    );
    assert!(
        m.required_images()
            .contains(&ImageKey::new(PrintRef::new(33), 1, ArtSize::Small)),
        "the face actually drawn is the face that has to be loaded"
    );
}

#[test]
fn an_ability_whose_source_is_gone_still_draws() {
    let mut ability = token(2, 0, "Cast Down", 0, 0);
    ability.card = None;
    ability.stack_item = Some(StackItem::Ability {
        token: None,
        source: ObjectId::new(99, 0),
        ability: Some(AbilityRef::new(CardIndex::new(1), 0)),
        text: None,
        rules: None,
    });
    let view = ViewBuilder::new(2).with_stack(vec![ability]).build();
    let m = model(&view);
    // CR 113.7a: the ability is independent of its source. No picture to
    // borrow is a missing picture, never a missing entry.
    assert_eq!(m.stack[0].art, None);
    assert_eq!(m.stack[0].name, "Cast Down");
}

#[test]
fn a_targeted_player_has_no_card_to_draw() {
    let mut bolt = printed(2, 0, "Lightning Bolt", 22);
    bolt.stack_item = Some(StackItem::Spell);
    bolt.targets = vec![TargetRef::Player(PlayerId::new(1))];
    let view = ViewBuilder::new(2).with_stack(vec![bolt]).build();
    let m = model(&view);
    let target = &m.stack[0].targets[0];
    assert_eq!(target.player(), Some(PlayerId::new(1)));
    assert_eq!(target.object(), None);
    // The seat's name lives in `GameStatic`, which this model has never
    // carried — so the renderer, not the model, spells a player out.
    assert_eq!(target.name, None);
    assert_eq!(target.art, None);
}

#[test]
fn historical_target_keeps_its_name_but_never_points_at_a_returned_object() {
    let mut view = bolt_at_bears();
    let old = view
        .target_objects
        .iter_mut()
        .find(|t| t.source.object == ObjectId::new(1, 0))
        .unwrap();
    old.is_current = false;
    old.name = "Original Bears".into();
    let returned = view
        .battlefield
        .iter_mut()
        .find(|o| o.id == ObjectId::new(1, 0))
        .unwrap();
    returned.name = "New concealed identity".into();
    let board = model(&view);
    let target = &board.stack[0].targets[0];
    assert_eq!(target.name.as_deref(), Some("Original Bears"));
    assert_eq!(target.object(), None);
    assert!(!target.is_current);
    assert!(!individual_objects(&view, &Lookup::new(&view)).contains_key(&ObjectId::new(1, 0)));
    view.target_objects.clear();
    let missing = model(&view);
    assert_eq!(missing.stack[0].targets[0].name, None);
    assert_eq!(missing.stack[0].targets[0].object(), None);
}

/// The modes the game log announced fill the stack's entries: a spell takes
/// those of its own line, an ability those of the line about an ability of
/// its source, and an entry nothing was announced for takes none (and loses
/// the modes of an earlier reading).
#[test]
fn a_stack_entry_takes_the_modes_the_log_announced() {
    use baylee_view::{LogEntry, LogEvent, LogObject, LogTail, RulesFace};

    let face = |card| RulesFace {
        card: CardIndex::new(card),
        face: 0,
    };
    let ability_of = |slot, source| {
        let mut ability = token(slot, 0, "Ability", 0, 0);
        ability.card = None;
        ability.stack_item = Some(StackItem::Ability {
            token: None,
            source: ObjectId::new(source, 0),
            ability: None,
            text: None,
            rules: None,
        });
        ability
    };
    let view = ViewBuilder::new(2)
        .with_battlefield(
            0,
            [printed(1, 0, "Source", 1), printed(2, 0, "Spellsource", 2)],
        )
        .with_stack(vec![
            printed(50, 0, "Charm", 50),
            printed(51, 0, "Plain Bolt", 51),
            ability_of(60, 1),
            ability_of(61, 2),
        ])
        .build();
    let named = |slot| LogObject::Known {
        id: ObjectId::new(slot, 0),
        card: None,
        token: None,
        name: format!("object {slot}"),
    };
    let aimed = |slot, ability, card, modes| LogEntry {
        at: 0,
        turn: 3,
        repeat: 1,
        event: LogEvent::Aimed {
            controller: PlayerId::new(0),
            source: named(slot),
            ability,
            rules: Some(face(card)),
            modes,
            targets: Vec::new(),
        },
        phase: None,
    };
    let mut log = crate::gamelog::LogBook::new();
    let entries = [
        aimed(50, false, 7, 0b10),
        // An aim of the spell with the ability's source's number is not
        // the ability's: only `ability: true` is read for an ability.
        aimed(2, false, 8, 0b1),
        aimed(1, true, 9, 0b100),
    ];
    log.append(
        &LogTail {
            from: 0,
            entries: entries.to_vec(),
        },
        &view,
    );

    let mut m = model(&view);
    assert!(m.stack.iter().all(|item| item.modes.is_none()), "unread");
    m.take_modes(&log);
    let modes: Vec<(ObjectId, Option<(RulesFace, u8)>)> =
        m.stack.iter().map(|item| (item.id, item.modes)).collect();
    // The model lists the entry that resolves next first.
    assert_eq!(
        modes,
        [
            (ObjectId::new(61, 0), None),
            (ObjectId::new(60, 0), Some((face(9), 0b100))),
            (ObjectId::new(51, 0), None),
            (ObjectId::new(50, 0), Some((face(7), 0b10))),
        ]
    );

    // Read again from a log that has said nothing, the modes are gone.
    m.take_modes(&crate::gamelog::LogBook::new());
    assert!(m.stack.iter().all(|item| item.modes.is_none()));
}
