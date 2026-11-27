use super::*;

fn badge(kind: AiKind, label: &str) -> AiBadge {
    AiBadge {
        kind,
        label: label.into(),
    }
}

#[test]
fn only_an_explicit_choice_lets_an_ai_into_a_search() {
    assert_eq!(Opponents::People.wire(), AiOpponents::None);
    assert_eq!(Opponents::Local.wire(), AiOpponents::Local);
    assert_eq!(Opponents::Any.wire(), AiOpponents::Any);
    assert!(!Opponents::People.accepts_ai());
    assert!(Opponents::Local.accepts_ai() && Opponents::Any.accepts_ai());
    // A person who never chose has nothing remembered, and is asked.
    let remembered = Remembered::default();
    assert_eq!(remembered.of("a"), None);
    for (i, choice) in Opponents::ALL.into_iter().enumerate() {
        assert_eq!(choice.index(), i);
    }
}

#[test]
fn a_remembered_choice_belongs_to_the_account_that_made_it() {
    let mut remembered = Remembered::default();
    assert!(remembered.keep("ann", Opponents::Local));
    assert!(!remembered.keep("ann", Opponents::Local), "nothing changed");
    assert_eq!(remembered.of("ann"), Some(Opponents::Local));
    assert_eq!(remembered.of("ben"), None, "another person has not chosen");
    assert!(remembered.keep("ann", Opponents::People));
    assert_eq!(remembered.of("ann"), Some(Opponents::People));
    let text = serde_json::to_string(&remembered).unwrap();
    assert_eq!(text, r#"{"ann":"people"}"#);
    let back: Remembered = serde_json::from_str(&text).unwrap();
    assert_eq!(back, remembered);
}

#[test]
fn a_choice_this_build_does_not_know_reads_as_never_asked() {
    let read: Remembered = serde_json::from_str(r#"{"ann":"robots","ben":"any"}"#).unwrap();
    assert_eq!(read.of("ann"), None);
    assert_eq!(read.of("ben"), Some(Opponents::Any));
}

#[test]
fn the_device_remembers_a_bounded_number_of_accounts() {
    let mut remembered = Remembered::default();
    for n in 0..Remembered::MAX + 5 {
        remembered.keep(&format!("account-{n:03}"), Opponents::Local);
    }
    assert_eq!(remembered.0.len(), Remembered::MAX);
    let last = format!("account-{:03}", Remembered::MAX + 4);
    assert_eq!(
        remembered.of(&last),
        Some(Opponents::Local),
        "the newest stays"
    );
}

#[test]
fn the_countdown_shows_only_its_last_fifteen_seconds_and_never_zero() {
    let at = 1_000;
    assert_eq!(FillIn::at(900, None), FillIn::Off);
    assert_eq!(FillIn::at(at - 16, Some(at)), FillIn::Later);
    assert_eq!(FillIn::at(at - 15, Some(at)), FillIn::Counting(15));
    assert_eq!(FillIn::at(at - 1, Some(at)), FillIn::Counting(1));
    assert_eq!(FillIn::at(at, Some(at)), FillIn::Due);
    assert_eq!(FillIn::at(at + 40, Some(at)), FillIn::Due);
    // Central's fill-in time is a setting (30 to 600 s): nothing here
    // assumes when the search began.
    assert_eq!(FillIn::at(0, Some(600)), FillIn::Later);
    assert_eq!(
        FillIn::Counting(12).words(Lang::De).as_deref(),
        Some("KI-Gegner in 0:12")
    );
    assert_eq!(
        FillIn::Counting(9).words(Lang::En).as_deref(),
        Some("AI opponent in 0:09")
    );
    for quiet in [FillIn::Off, FillIn::Later, FillIn::Due] {
        assert_eq!(quiet.words(Lang::En), None);
    }
}

#[test]
fn the_countdown_fades_in_and_is_gone_before_it_would_say_zero() {
    let at = 1_000;
    let ms = |s: i64| s * 1000;
    assert!(FillIn::opacity(ms(at - 30), Some(at)).abs() < f32::EPSILON);
    assert!(FillIn::opacity(ms(at - 15), Some(at)).abs() < f32::EPSILON);
    let half = FillIn::opacity(ms(at - 15) + FADE_MS / 2, Some(at));
    assert!((half - 0.5).abs() < 0.01, "{half}");
    assert!((FillIn::opacity(ms(at - 8), Some(at)) - 1.0).abs() < f32::EPSILON);
    let going = FillIn::opacity(ms(at) - FADE_MS / 2, Some(at));
    assert!((going - 0.5).abs() < 0.01, "{going}");
    assert!(FillIn::opacity(ms(at), Some(at)).abs() < f32::EPSILON);
    assert!(FillIn::opacity(ms(at + 5), Some(at)).abs() < f32::EPSILON);
    assert!(FillIn::opacity(ms(at), None).abs() < f32::EPSILON);
}

#[test]
fn every_kind_wears_its_own_mark_and_a_later_kind_wears_the_chip() {
    let marks: std::collections::HashSet<Mark> = [AiKind::House, AiKind::Model, AiKind::Net]
        .into_iter()
        .map(Mark::of)
        .collect();
    assert_eq!(marks.len(), 3);
    assert_eq!(Mark::of(AiKind::Other), Mark::Other);
    let later: AiKind = serde_json::from_str("\"oracle\"").unwrap();
    assert_eq!(Mark::of(later), Mark::Other);
}

#[test]
fn a_badge_is_cleaned_and_never_drawn_nameless() {
    let lang = Lang::De;
    assert_eq!(
        label(&badge(AiKind::House, "Haus-KI · Expert"), lang),
        "Haus-KI · Expert"
    );
    let sneaky = badge(AiKind::Model, "\u{202E}Claude\u{200B} Haiku\n · Hoch ");
    assert_eq!(label(&sneaky, lang), "Claude Haiku · Hoch");
    let long = badge(AiKind::Model, &"x".repeat(200));
    assert_eq!(label(&long, lang).chars().count(), LABEL_CHARS);
    assert_eq!(label(&badge(AiKind::House, " \u{200B} "), lang), "Haus-KI");
    assert_eq!(label(&badge(AiKind::Other, ""), Lang::En), "AI");
}

#[test]
fn a_badge_is_said_with_its_kind_and_a_model_names_where_moves_go() {
    let house = badge(AiKind::House, "Haus-KI · Expert");
    assert_eq!(
        spoken(&house, Lang::De),
        "KI-Spieler: Haus-KI · Expert (Haus-KI)"
    );
    let model = badge(AiKind::Model, "Claude Haiku · Hoch");
    assert_eq!(
        spoken(&model, Lang::En),
        "AI player: Claude Haiku · Hoch (language model)"
    );
    assert!(!hint(&house, Lang::En).contains(Phrase::AiModelProvider.text(Lang::En)));
    assert!(hint(&model, Lang::En).contains(Phrase::AiModelProvider.text(Lang::En)));
}

#[test]
fn the_ladder_groups_keep_their_order() {
    for (i, players) in BOARD_PLAYERS.into_iter().enumerate() {
        assert_eq!(board_players_index(players), i);
    }
    assert_eq!(BOARD_PLAYERS[0], BoardPlayers::default());
}
