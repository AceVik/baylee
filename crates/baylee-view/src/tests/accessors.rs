use super::*;

#[test]
fn a_pool_counts_restricted_mana_in_its_total_and_empty_means_nothing_floats() {
    let mut pool = ManaPoolView::default();
    assert!(pool.is_empty());
    assert_eq!(pool.total(), 0);
    pool.green = 2;
    pool.colorless = 1;
    pool.restricted[0] = 1;
    pool.restricted[3] = 2;
    assert_eq!(pool.restricted_total(), 3);
    assert_eq!(pool.total(), 6);
    assert!(!pool.is_empty());
    // Restricted mana alone still counts as floating.
    let only = ManaPoolView {
        restricted: [0, 0, 0, 0, 0, 1],
        ..ManaPoolView::default()
    };
    assert!(!only.is_empty());
}

#[test]
fn an_older_mana_pool_defaults_to_exact_spending_and_current_permissions_round_trip() {
    use baylee_core::mana::{ManaColor, ManaSpending};
    let old_pool = serde_json::json!({
        "white": 2, "blue": 0, "black": 0, "red": 0,
        "green": 0, "colorless": 1, "restricted": [1, 0, 0, 0, 0, 0]
    });
    let mut pool: ManaPoolView = serde_json::from_value(old_pool.clone()).unwrap();
    assert_eq!(pool.spending, ManaSpending::EXACT);
    assert_eq!(
        (pool.white, pool.red, pool.colorless, pool.restricted[0]),
        (2, 0, 1, 1)
    );
    assert!(!pool.spending.permits(ManaColor::White, ManaColor::Red));
    pool.spending.allow(ManaColor::White, ManaColor::Red);
    let mut current = serde_json::to_value(pool).unwrap();
    assert_eq!(
        serde_json::from_value::<ManaPoolView>(current.clone()).unwrap(),
        pool
    );
    current.as_object_mut().unwrap().remove("spending");
    assert_eq!(
        current, old_pool,
        "the existing wire fields remain unchanged"
    );
}

/// CR 704.5g: lethal damage is toughness minus marked damage at or below 0.
#[test]
fn damage_equal_to_toughness_is_lethal_and_a_non_creature_never_is() {
    let mut o = obj(1, 0);
    o.toughness = Some(3);
    o.damage = 2;
    assert!(!o.is_lethally_damaged());
    o.damage = 3;
    assert!(o.is_lethally_damaged());
    o.damage = 5;
    assert_eq!(o.remaining_toughness(), Some(-2));
    assert!(o.is_lethally_damaged());
    o.toughness = None;
    assert_eq!(o.remaining_toughness(), None);
    assert!(!o.is_lethally_damaged());
}

#[test]
fn a_seat_has_lost_when_it_names_a_cause() {
    let mut v = view(2);
    assert!(!v.seats[0].has_lost());
    v.seats[0].loss = Some(LossCause::Poison);
    assert!(v.seats[0].has_lost());
    assert!(!v.seats[1].has_lost());
}

#[test]
fn a_library_of_three_or_fewer_warns_of_decking() {
    let mut v = view(1);
    v.seats[0].library_count = 4;
    assert!(!v.seats[0].is_decking_out());
    v.seats[0].library_count = 3;
    assert!(v.seats[0].is_decking_out());
    v.seats[0].library_count = 0;
    assert!(v.seats[0].is_decking_out());
}

#[test]
fn combat_with_no_attackers_is_not_active_and_every_unnamed_creature_is_unblocked() {
    let c = CombatView::default();
    assert!(!c.is_active());
    assert!(c.is_unblocked(ObjectId::new(5, 0)));
    assert_eq!(c.blockers_of(ObjectId::new(5, 0)).count(), 0);
}

#[test]
fn a_print_is_found_by_its_index_and_a_hidden_or_missing_one_is_none() {
    let game = GameStatic {
        view_version: VIEW_VERSION,
        game_id: "g".to_string(),
        your_seat: PlayerId::new(0),
        seats: vec![],
        prints: vec![
            Some(PrintEntry {
                scryfall_id: "abc".to_string(),
                lang: "EN".to_string(),
                finish: Finish::Foil,
            }),
            None,
        ],
        decision_secs: None,
        reconnect_secs: None,
    };
    assert_eq!(game.print(PrintRef::new(0)).unwrap().scryfall_id, "abc");
    assert!(
        game.print(PrintRef::new(1)).is_none(),
        "hidden: None, not shortened"
    );
    assert!(game.print(PrintRef::new(2)).is_none(), "past the table");
}

#[test]
fn a_seat_with_no_identity_is_named_by_a_fallback_never_by_another_seat() {
    let game = GameStatic {
        view_version: VIEW_VERSION,
        game_id: "g".to_string(),
        your_seat: PlayerId::new(0),
        seats: vec![SeatIdentity {
            player: PlayerId::new(1),
            display_name: "Alice".to_string(),
            is_ai: false,
            away: false,
            team: None,
        }],
        prints: vec![],
        decision_secs: None,
        reconnect_secs: None,
    };
    assert_eq!(game.seat_name(PlayerId::new(1)), "Alice");
    assert_eq!(game.seat_name(PlayerId::new(0)), "Unknown seat");
}
