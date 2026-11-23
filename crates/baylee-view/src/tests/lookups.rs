//! What a reader finds by a handle: the object itself, never a neighbour.

use super::*;
use baylee_core::ids::DamageSourceRef;
use baylee_core::mana::ManaColor;
use std::hash::{DefaultHasher, Hash, Hasher};

const fn mana(slot: u32, amount: u8) -> GrantedMana {
    GrantedMana {
        slot,
        colors: Vec::new(),
        amount,
    }
}

const fn grant(slot: u32, mana: Option<GrantedMana>) -> GrantSource {
    GrantSource {
        slot,
        ability: None,
        mana_ability: mana.is_some(),
        mana,
        source: None,
        rules: None,
        text: None,
        taps: false,
    }
}

#[test]
fn a_grant_is_found_by_its_slot_and_a_missing_slot_finds_none() {
    let mut object = obj(1, 0);
    object.grants = vec![grant(4, None), grant(9, Some(mana(9, 2)))];
    assert_eq!(object.grant(4).map(|g| g.mana_ability), Some(false));
    assert_eq!(object.grant(9).map(|g| g.mana_ability), Some(true));
    assert_eq!(object.grant(0), None);
    assert_eq!(object.grant(5), None);
}

#[test]
fn granted_mana_lists_each_slot_once_and_prefers_the_grant_that_names_it() {
    let mut object = obj(1, 0);
    object.granted_mana = Some(mana(9, 1));
    assert_eq!(object.granted_mana_for(9).map(|m| m.amount), Some(1));
    assert_eq!(object.granted_mana_options().count(), 1);

    // A host that says the same slot both ways means one ability.
    object.grants = vec![grant(4, None), grant(9, Some(mana(9, 2)))];
    let amounts: Vec<_> = object.granted_mana_options().map(|m| m.amount).collect();
    assert_eq!(amounts, [2]);
    assert_eq!(object.granted_mana_for(9).map(|m| m.amount), Some(2));
    assert_eq!(object.granted_mana_for(4), None);

    // Another slot is another ability, and both are offered.
    object.grants = vec![grant(5, Some(mana(5, 3)))];
    let slots: Vec<_> = object.granted_mana_options().map(|m| m.slot).collect();
    assert_eq!(slots, [5, 9]);
    assert_eq!(object.granted_mana_for(5).map(|m| m.amount), Some(3));
    assert_eq!(object.granted_mana_for(9).map(|m| m.amount), Some(1));
}

#[test]
fn objects_that_pile_together_hash_alike_and_objects_that_do_not_hash_apart() {
    let hash = |object: &PublicObject| {
        let mut hasher = DefaultHasher::new();
        object.summary_key().hash(&mut hasher);
        hasher.finish()
    };
    let one = obj(1, 0);
    let twin = obj(2, 0);
    let mut tapped = obj(3, 0);
    tapped.status = ObjectStatus::TAPPED;
    assert_eq!(one.summary_key(), twin.summary_key());
    assert_eq!(hash(&one), hash(&twin));
    assert_ne!(hash(&one), hash(&tapped));
}

#[test]
fn a_division_is_read_off_the_object_that_divides_and_off_no_other() {
    let mut v = view(2);
    let share = |seat, amount| (TargetRef::Player(PlayerId::new(seat)), amount);
    v.divided = vec![
        Division {
            object: ObjectId::new(1, 0),
            shares: vec![share(0, 3), share(1, 1)],
        },
        Division {
            object: ObjectId::new(2, 0),
            shares: vec![share(1, 4)],
        },
    ];
    assert_eq!(
        v.division(ObjectId::new(1, 0)),
        [share(0, 3), share(1, 1)].as_slice()
    );
    assert_eq!(v.division(ObjectId::new(2, 0)), [share(1, 4)].as_slice());
    assert!(v.division(ObjectId::new(3, 0)).is_empty());
    // The slot is the same, the object is not.
    assert!(v.division(ObjectId::new(1, 1)).is_empty());
}

#[test]
fn a_target_is_described_by_its_own_incarnation_and_never_by_a_later_one() {
    // CR 400.7: an object that changes zones is a new object.
    let at = |version| DamageSourceRef {
        object: ObjectId::new(7, 0),
        version,
    };
    let described = |version, name: &str| DamageSourceView {
        source: at(version),
        name: name.to_owned(),
        card: None,
        rules: None,
        token: None,
        controller: PlayerId::new(0),
        zone: LogZone::Battlefield,
        is_current: false,
        referenced_by: Vec::new(),
        colors: ColorSet::default(),
        types: TypeSet::CREATURE,
        power: None,
        toughness: None,
        keywords: 0,
    };
    let mut v = view(2);
    v.target_objects = vec![described(1, "before"), described(3, "after")];
    let name = |version| v.target_object(at(version)).map(|t| t.name.as_str());
    assert_eq!(name(1), Some("before"));
    assert_eq!(name(3), Some("after"));
    assert_eq!(name(2), None);
}

#[test]
fn a_seats_permanents_are_the_ones_it_controls_whoever_owns_them() {
    let mut v = view(2);
    let mut stolen = obj(2, 0);
    stolen.owner = PlayerId::new(1);
    v.battlefield = vec![obj(1, 0), obj(3, 1), stolen];
    let of = |seat| {
        v.battlefield_of(PlayerId::new(seat))
            .map(|o| o.id.slot())
            .collect::<Vec<_>>()
    };
    assert_eq!(of(0), [1, 2]);
    assert_eq!(of(1), [3]);
    assert!(of(2).is_empty());
}

#[test]
fn a_seat_line_is_found_by_player_even_when_the_list_is_in_another_order() {
    let mut v = view(3);
    v.seats[0].life = 10;
    v.seats[2].life = 30;
    v.seats.reverse();
    assert_eq!(v.seat(PlayerId::new(0)).map(|s| s.life), Some(10));
    assert_eq!(v.seat(PlayerId::new(2)).map(|s| s.life), Some(30));
    assert!(v.seat(PlayerId::new(3)).is_none());
}

#[test]
fn a_card_on_the_table_answers_before_the_same_card_being_shown() {
    let mut v = view(2);
    let mut shown = obj(5, 0);
    shown.name = "as printed".into();
    let mut table = obj(5, 0);
    table.name = "as projected".into();
    v.looking_at = vec![shown];
    assert_eq!(
        v.object(ObjectId::new(5, 0)).map(|o| o.name.as_str()),
        Some("as printed")
    );
    v.battlefield = vec![table];
    assert_eq!(
        v.object(ObjectId::new(5, 0)).map(|o| o.name.as_str()),
        Some("as projected")
    );
}

#[test]
fn mana_is_named_by_the_slot_it_belongs_to_and_not_by_its_colours() {
    let mut object = obj(1, 0);
    let mut white = mana(1, 1);
    white.colors = vec![ManaColor::White];
    let mut blue = mana(2, 1);
    blue.colors = vec![ManaColor::Blue];
    object.grants = vec![grant(1, Some(white)), grant(2, Some(blue))];
    assert_eq!(
        object.granted_mana_for(2).map(|m| m.colors.as_slice()),
        Some([ManaColor::Blue].as_slice())
    );
}

#[test]
fn a_table_is_drawn_from_the_seed_the_host_sent_or_else_from_the_games_name() {
    let game = |game_id: &str, table_seed| GameStatic {
        view_version: VIEW_VERSION,
        game_id: game_id.to_owned(),
        your_seat: PlayerId::new(0),
        seats: vec![],
        prints: vec![],
        decision_secs: None,
        reconnect_secs: None,
        table_seed,
    };
    assert_eq!(game("g", 41).surface_seed(), 41);
    assert_eq!(game("", 41).surface_seed(), 41);
    // FNV-1a's published test vectors: replays recorded before hosts sent a
    // seed must keep drawing the table they were played on.
    assert_eq!(game("a", 0).surface_seed(), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(game("foobar", 0).surface_seed(), 0x8594_4171_f739_67e8);
    assert_eq!(game("", 0).surface_seed(), 0);
}
