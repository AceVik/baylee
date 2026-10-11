use super::*;
#[test]
fn shown_hands_are_browsable_but_do_not_grant_control_and_disappear_when_hidden() {
    let other = PlayerId::new(1);
    let mut view = ViewBuilder::new(2).build();
    view.shared_hands.push(baylee_view::SharedHand {
        player: other,
        public: true,
        cards: ViewBuilder::new(2)
            .with_hand(vec![("Shown Forest", 1, 20), ("Shown Bear", 2, 21)])
            .build()
            .hand,
    });
    let mut browser = Browser::new();
    let zone = BrowseZone::ShownHand(other);
    browser.open_at(zone);
    assert_eq!(zone.count_in(&view), 2);
    assert!(browser.zones(&view).contains(&zone));
    let rows = browser.rows(&view, None, Names::projected());
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .all(|row| row.zone == zone && !row.standing.selectable)
    );
    assert_eq!(crate::decision::known_cards(&view).count(), 2);
    assert!(
        crate::decision::hand(&view).is_empty(),
        "visibility grants no control"
    );
    browser.set_filter("Bear");
    assert_eq!(
        browser.rows(&view, None, Names::projected())[0].name,
        "Shown Bear"
    );
    view.shared_hands.clear();
    assert!(!browser.zones(&view).contains(&zone));
    assert!(browser.rows(&view, None, Names::projected()).is_empty());
    assert_eq!(crate::decision::known_cards(&view).count(), 0);
}

#[test]
fn shown_hand_search_matches_the_displayed_language_and_the_original_name() {
    let mut view = ViewBuilder::new(2).build();
    view.shared_hands.push(baylee_view::SharedHand {
        player: PlayerId::new(1),
        public: true,
        cards: ViewBuilder::new(2)
            .with_hand(vec![("Forest", 1, 20)])
            .build()
            .hand,
    });
    let translated = |_: &baylee_view::HandObject| Some("Wald".to_string());
    let names = Names {
        hand: &translated,
        ..Names::projected()
    };
    let mut browser = Browser::new();
    for needle in ["Wald", "Forest"] {
        browser.set_filter(needle);
        let rows = browser.rows(&view, None, names);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Wald");
    }
}
