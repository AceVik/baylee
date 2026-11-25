use super::*;

fn refused(change: impl FnOnce(&mut Package)) -> bool {
    let mut package = package();
    change(&mut package);
    package.validate(executor()).is_err()
}

#[test]
fn a_complete_package_survives_the_wire_unchanged() {
    let package = package();
    let bytes = package.encode().unwrap();
    assert!(Package::decode(&bytes, executor()).is_ok_and(|back| back == package));
}

#[test]
fn a_package_of_another_build_or_schema_is_refused() {
    assert!(!refused(|_| ()));
    assert!(package().validate("another build").is_err());
    assert!(refused(|p| p.schema = 0));
    assert!(refused(|p| p.schema = 2));
    let mut nameless = package();
    nameless.executor = String::new();
    assert!(nameless.validate("").is_err(), "matching empty names");
}

#[test]
fn an_executor_name_may_be_exactly_160_bytes_long() {
    let mut package = package();
    package.executor = "x".repeat(160);
    assert!(package.validate(&"x".repeat(160)).is_ok());
    package.executor = "x".repeat(161);
    assert!(package.validate(&"x".repeat(161)).is_err());
}

#[test]
fn the_target_turn_lies_between_one_and_five_hundred() {
    for (turn, bad) in [(0, true), (1, false), (500, false), (501, true)] {
        assert_eq!(refused(|p| p.target_turn = turn), bad, "{turn}");
    }
}

#[test]
fn only_one_open_seat_against_one_house_ai_without_teams_is_a_challenge() {
    let ai = || seat(SeatController::Ai(AIProfile::STEADY));
    assert!(refused(|p| {
        p.preset.seats.truncate(1);
    }));
    assert!(refused(|p| p.preset.seats.push(ai())));
    assert!(refused(|p| p.preset.seats.reverse()));
    assert!(refused(|p| {
        p.preset.seats[0].controller = SeatController::Human { user_id: 7 };
    }));
    assert!(refused(
        |p| p.preset.seats[1].controller = SeatController::Open
    ));
    assert!(refused(|p| p.preset.seats[0] = ai()));
    // Different teams are a legal table, and still not a challenge.
    assert!(refused(|p| {
        p.preset.seats[0].team = Some(0);
        p.preset.seats[1].team = Some(1);
    }));
    assert!(refused(|p| p.preset.seats[1].team = Some(1)));
}

#[test]
fn an_opponent_that_searches_ahead_is_refused() {
    let searching = AIProfile {
        foresight: 1,
        generation: AIProfile::FORESIGHT,
        ..AIProfile::STEADY
    };
    assert!(searching.foresight().is_some());
    assert!(refused(|p| {
        p.preset.seats[1].controller = SeatController::Ai(searching);
    }));
}

#[test]
fn every_rule_that_would_make_a_run_unrepeatable_is_refused() {
    assert!(refused(
        |p| p.preset.house_rules.challenge_policy_seed = None
    ));
    assert!(refused(|p| p.preset.house_rules.decision_timeout_secs = 1));
    assert!(refused(|p| p.preset.house_rules.reconnect_window_secs = 1));
    assert!(refused(|p| p.preset.house_rules.takebacks = true));
    assert!(refused(|p| p.preset.house_rules.time_extension_votes = true));
    assert!(refused(|p| p.preset.house_rules.starting_player = Some(2)));
}

#[test]
fn an_invalid_package_is_never_encoded() {
    let mut package = package();
    package.target_turn = 0;
    assert!(package.encode().is_err());
}

/// A package padded by one print's language to exactly `bytes` encoded bytes.
fn padded(bytes: usize) -> Package {
    let mut package = package();
    let bare = serde_json::to_vec(&package).unwrap().len();
    package.preset.prints[0].lang = "x".repeat(bytes + 2 - bare);
    assert_eq!(serde_json::to_vec(&package).unwrap().len(), bytes);
    package
}

#[test]
fn the_size_limit_holds_to_the_byte_in_both_directions() {
    let largest = padded(MAX_PACKAGE_BYTES);
    let bytes = largest.encode().unwrap();
    assert!(Package::decode(&bytes, executor()).is_ok());
    let oversized = padded(MAX_PACKAGE_BYTES + 1);
    assert!(oversized.encode().is_err());
    let bytes = serde_json::to_vec(&oversized).unwrap();
    assert!(Package::decode(&bytes, executor()).is_err());
}

#[test]
fn malformed_or_extended_material_is_refused() {
    assert!(Package::decode(b"", executor()).is_err());
    assert!(Package::decode(b"{}", executor()).is_err());
    let bytes = package().encode().unwrap();
    assert!(Package::decode(&bytes[..bytes.len() - 1], executor()).is_err());
    let mut trailing = bytes.clone();
    trailing.extend_from_slice(b"{}");
    assert!(Package::decode(&trailing, executor()).is_err());
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["hint"] = "look at the opening hand".into();
    let extended = serde_json::to_vec(&value).unwrap();
    assert!(Package::decode(&extended, executor()).is_err());
}

#[test]
fn a_publication_must_describe_the_package_it_carries() {
    let period = slot(Kind::Daily, MONDAY);
    let good = publish(&package(), period, "Daily");
    assert!(Package::from_publication(&good).is_ok_and(|read| read == package()));
    assert!(Package::from_publication_for(&good, "another build").is_err());

    let mut lying = good.clone();
    lying.description.target_turn += 1;
    sign(&mut lying);
    assert!(lying.validate().is_ok(), "only the package disagrees");
    assert!(Package::from_publication(&lying).is_err());

    let mut lying = good.clone();
    lying.description.executor = "another build".into();
    sign(&mut lying);
    assert!(lying.validate().is_ok(), "only the package disagrees");
    assert!(Package::from_publication(&lying).is_err());
}

#[test]
fn a_publication_changed_after_it_was_signed_is_refused() {
    let mut publication = publish(&package(), slot(Kind::Daily, MONDAY), "Daily");
    publication.hints[0] = words("Concede.");
    assert!(Package::from_publication(&publication).is_err());
}

#[test]
fn a_retained_executor_reads_its_own_publications_only() {
    let mut old = package();
    old.executor = "retained build".into();
    let publication = publish(&old, slot(Kind::Daily, MONDAY), "Daily");
    assert!(Package::from_publication_for(&publication, "retained build").is_ok());
    assert!(Package::from_publication(&publication).is_err());
}

#[test]
fn a_seed_depends_on_its_domain_and_its_value_alone() {
    assert_eq!(seed(b"deck", 7), seed(b"deck", 7));
    assert_ne!(seed(b"deck", 7), seed(b"deck", 8));
    assert_ne!(seed(b"deck", 7), seed(b"policy", 7));
    // Moving a byte from the seed into the domain must not keep the stream.
    assert_ne!(seed(b"", 0x61), seed(b"a", 0));
    // Frozen: a calendar already published must keep its decks.
    assert_eq!(seed(b"player deck", 1), 3_706_238_869_626_075_439);
}
