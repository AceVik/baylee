use super::*;

/// Text is asked for by card, so a copy names the card it is *and* the
/// card its abilities are printed on, a stack ability names the card it
/// was printed on, and an object showing no card — face down, here —
/// names none. The walk is `prints`' own, which the last line holds: one
/// printing shown, and nothing asked about the face-down one.
#[test]
fn a_copy_names_two_cards_and_a_face_down_permanent_none() {
    let rules = |card: u32| RulesFace {
        card: CardIndex::new(card),
        face: 0,
    };
    let mut copy = obj(1, 0);
    copy.card = Some(CardIdentity {
        index: CardIndex::new(10),
        print: PrintRef::new(0),
        face: 0,
    });
    copy.rules = Some(rules(20));
    let mut ability = obj(3, 1);
    ability.stack_item = Some(StackItem::Ability {
        token: None,
        source: ObjectId::new(1, 0),
        ability: None,
        text: None,
        rules: Some(rules(30)),
    });
    let mut v = view(2);
    v.battlefield = vec![copy, obj(2, 1)];
    v.stack = vec![ability];

    let cards: std::collections::BTreeSet<u32> = v.cards().map(CardIndex::get).collect();
    assert_eq!(cards, [10, 20, 30].into());
    assert_eq!(v.prints().count(), 1);
}

#[test]
fn status_bits_round_trip_through_the_wire_type() {
    let s = ObjectStatus::from_bits(ObjectStatus::TAPPED.bits() | ObjectStatus::PHASED_OUT.bits());
    assert!(s.is_tapped());
    assert!(s.is_phased_out());
    assert!(!s.is_face_down());
    assert_eq!(s.bits(), 5);
}

#[test]
fn remaining_toughness_accounts_for_marked_damage() {
    let mut o = obj(1, 0);
    o.toughness = Some(4);
    o.damage = 3;
    assert_eq!(o.remaining_toughness(), Some(1));
    assert!(!o.is_lethally_damaged());
    o.damage = 4;
    assert!(o.is_lethally_damaged());
}

#[test]
fn identical_tokens_share_a_summary_key_and_different_ones_do_not() {
    let a = obj(1, 0);
    let b = obj(2, 0);
    assert_eq!(a.summary_key(), b.summary_key());

    // A tapped token must not collapse into the untapped stack: whether a
    // blocker is available is exactly the kind of difference that decides
    // a turn.
    let mut tapped = obj(3, 0);
    tapped.status = ObjectStatus::TAPPED;
    assert_ne!(a.summary_key(), tapped.summary_key());

    // Nor may tokens of different controllers merge.
    let other_seat = obj(4, 1);
    assert_ne!(a.summary_key(), other_seat.summary_key());

    // Nor may a counter difference be hidden.
    let mut countered = obj(5, 0);
    countered.counters = vec![CounterEntry {
        kind: CounterKind::PLUS_ONE,
        count: 1,
    }];
    assert_ne!(a.summary_key(), countered.summary_key());

    // Nor may a commander merge into a stack of ordinary copies of
    // itself. A clone effect makes a token that matches its original in
    // every other field, and the commander is drawn with a marker — a
    // group wearing one of the two would be lying about the rest.
    let mut boss = obj(6, 0);
    boss.commander = true;
    assert_ne!(a.summary_key(), boss.summary_key());
}

#[test]
fn different_chosen_types_never_share_a_board_pile() {
    let mut a = obj(1, 0);
    let mut b = obj(2, 0);
    assert_eq!(a.summary_key(), b.summary_key());
    a.chosen_subtype = Some(baylee_core::generated::subtypes::creature::ALLY);
    assert_ne!(a.summary_key(), b.summary_key());
    b.chosen_subtype = a.chosen_subtype;
    assert_eq!(a.summary_key(), b.summary_key());
    b.chosen_subtype = Some(baylee_core::generated::subtypes::creature::ELF);
    assert_ne!(a.summary_key(), b.summary_key());
}

#[test]
fn different_chosen_names_never_share_a_board_pile() {
    let named = |card: u32, face: u8| {
        Some(NamedFace {
            card: CardIndex::new(card),
            face,
        })
    };
    let mut a = obj(1, 0);
    let mut b = obj(2, 0);
    a.chosen_name = named(7, 0);
    assert_ne!(a.summary_key(), b.summary_key());
    b.chosen_name = a.chosen_name;
    assert_eq!(a.summary_key(), b.summary_key());
    b.chosen_name = named(7, 1);
    assert_ne!(
        a.summary_key(),
        b.summary_key(),
        "the back face's name is another name"
    );
}

#[test]
fn chosen_opponents_are_public_and_keep_different_vises_apart() {
    let mut a = obj(1, 0);
    let mut b = obj(2, 0);
    a.chosen_opponent = Some(PlayerId::new(1));
    assert_ne!(a.summary_key(), b.summary_key());
    b.chosen_opponent = a.chosen_opponent;
    assert_eq!(a.summary_key(), b.summary_key());
    b.chosen_opponent = Some(PlayerId::new(2));
    assert_ne!(a.summary_key(), b.summary_key());
    let mut json = serde_json::to_value(obj(1, 0)).unwrap();
    json.as_object_mut().unwrap().remove("chosen_opponent");
    let decoded: PublicObject = serde_json::from_value(json).unwrap();
    assert_eq!(decoded.chosen_opponent, None);
}

#[test]
fn rooms_with_different_doors_open_never_share_a_board_pile() {
    let mut a = obj(1, 0);
    let mut b = obj(2, 0);
    a.unlocked_doors = Some([true, false]);
    assert_ne!(a.summary_key(), b.summary_key());
    b.unlocked_doors = Some([true, false]);
    assert_eq!(a.summary_key(), b.summary_key());
    b.unlocked_doors = Some([true, true]);
    assert_ne!(a.summary_key(), b.summary_key(), "one door left to open");
}

#[test]
fn an_older_public_object_without_doors_still_decodes() {
    let object = obj(1, 0);
    let mut json = serde_json::to_value(&object).unwrap();
    json.as_object_mut().unwrap().remove("unlocked_doors");
    let decoded: PublicObject = serde_json::from_value(json).unwrap();
    assert_eq!(decoded, object);
}

#[test]
fn an_older_public_object_without_a_chosen_name_still_decodes() {
    let object = obj(1, 0);
    let mut json = serde_json::to_value(&object).unwrap();
    json.as_object_mut().unwrap().remove("chosen_name");
    let decoded: PublicObject = serde_json::from_value(json).unwrap();
    assert_eq!(decoded, object);
}

#[test]
fn an_older_public_object_without_a_chosen_type_still_decodes() {
    let object = obj(1, 0);
    let mut json = serde_json::to_value(&object).unwrap();
    json.as_object_mut().unwrap().remove("chosen_subtype");
    let decoded: PublicObject = serde_json::from_value(json).unwrap();
    assert_eq!(decoded, object);
}

#[test]
fn summary_key_ignores_counter_ordering() {
    let mut a = obj(1, 0);
    let mut b = obj(2, 0);
    a.counters = vec![
        CounterEntry {
            kind: CounterKind::PLUS_ONE,
            count: 2,
        },
        CounterEntry {
            kind: CounterKind::Charge,
            count: 1,
        },
    ];
    b.counters = vec![
        CounterEntry {
            kind: CounterKind::Charge,
            count: 1,
        },
        CounterEntry {
            kind: CounterKind::PLUS_ONE,
            count: 2,
        },
    ];
    assert_eq!(a.summary_key(), b.summary_key());
}

/// Every field the key is made of, one at a time.
///
/// The promise `summary_key` makes is that "two objects group only when
/// every visible property matches, so collapsing can never hide a
/// difference that matters to a decision". A field left out of the key is
/// exactly that hidden difference — a summoning-sick creature collapsed
/// into a stack with one that can attack is a lie about what a player may
/// do this turn.
///
/// The count at the end is what keeps this list honest: the key's own
/// `Debug` names its fields, so a field added to it without a mutation
/// here fails rather than passing quietly.
#[test]
#[allow(clippy::too_many_lines)] // one row per field of the key, and the key grows
fn every_field_the_key_is_made_of_keeps_two_objects_apart() {
    type Change = (&'static str, fn(&mut PublicObject));
    const CHANGES: &[Change] = &[
        ("card", |o| {
            o.card = Some(CardIdentity {
                index: CardIndex::new(7),
                print: PrintRef::new(0),
                face: 0,
            });
        }),
        ("face", |o| {
            o.card = Some(CardIdentity {
                index: CardIndex::new(7),
                print: PrintRef::new(0),
                face: 1,
            });
        }),
        ("name", |o| o.name = "Zombie".to_string()),
        ("controller", |o| o.controller = PlayerId::new(1)),
        ("commander", |o| o.commander = true),
        ("status", |o| o.status = ObjectStatus::TAPPED),
        ("types", |o| o.types = TypeSet::ARTIFACT),
        ("power", |o| o.power = Some(2)),
        ("toughness", |o| o.toughness = Some(2)),
        ("base_power", |o| o.base_power = Some(1)),
        ("base_toughness", |o| o.base_toughness = Some(1)),
        ("damage", |o| o.damage = 1),
        ("loyalty", |o| o.loyalty = Some(3)),
        ("counters", |o| {
            o.counters = vec![CounterEntry {
                kind: CounterKind::PLUS_ONE,
                count: 1,
            }];
        }),
        ("attached", |o| o.attached_to = Some(ObjectId::new(99, 0))),
        ("summoning_sick", |o| o.summoning_sick = true),
        ("token", |o| o.token = Some(3)),
        ("owner", |o| o.owner = PlayerId::new(1)),
        ("supertypes", |o| o.supertypes = SupertypeSet::LEGENDARY),
        ("subtypes", |o| {
            o.subtypes.insert(baylee_core::ids::SubtypeId::new(1));
        }),
        ("suspended", |o| o.suspended = true),
        ("chosen_subtype", |o| {
            o.chosen_subtype = Some(baylee_core::generated::subtypes::creature::ALLY);
        }),
        ("chosen_opponent", |o| {
            o.chosen_opponent = Some(PlayerId::new(1));
        }),
        ("chosen_name", |o| {
            o.chosen_name = Some(NamedFace {
                card: CardIndex::new(7),
                face: 0,
            });
        }),
        ("unlocked_doors", |o| o.unlocked_doors = Some([false, true])),
        ("colors", |o| o.colors = ColorSet::ALL),
        ("keywords", |o| o.keywords = 1),
        ("granted_mana", |o| {
            o.granted_mana = Some(GrantedMana {
                slot: 0,
                colors: vec![baylee_core::mana::ManaColor::Blue],
                amount: 1,
            });
        }),
        ("board_mana", |o| {
            o.board_mana = Some(BoardMana {
                index: 0,
                colors: vec![baylee_core::mana::ManaColor::Red],
            });
        }),
    ];

    let base = obj(1, 0);
    // The same object twice, so the comparison below is about the change
    // and not about the id — which is deliberately not in the key.
    assert_eq!(base.summary_key(), obj(2, 0).summary_key());

    for (what, change) in CHANGES {
        let mut other = obj(3, 0);
        change(&mut other);
        assert_ne!(
            base.summary_key(),
            other.summary_key(),
            "two objects differing in {what} collapse into one stack"
        );
    }

    // The printing is deliberately *not* in it. Two Forests with
    // different art are still two Forests, and a fourteenth is what
    // turns them into one card saying fourteen — the board collapses on
    // what a player would conclude, and the art is not part of that.
    let printing = |print: u16| {
        let mut o = obj(7, 0);
        o.card = Some(CardIdentity {
            index: CardIndex::new(7),
            print: PrintRef::new(print),
            face: 0,
        });
        o.summary_key()
    };
    assert_eq!(
        printing(0),
        printing(1),
        "a second printing of one card splits a stack that should collapse"
    );

    // `card` and `face` are one field of the key, so the list is one
    // longer than the key is wide.
    let printed = format!("{:?}", base.summary_key());
    let named = printed.matches(": ").count();
    assert_eq!(
        named,
        CHANGES.len() - 1,
        "the key prints {named} fields and this test changes \
         {} of them: {printed}",
        CHANGES.len() - 1
    );
}
