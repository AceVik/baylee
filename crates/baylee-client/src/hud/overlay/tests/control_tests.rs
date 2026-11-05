//! A turn this seat controls (CR 723.5) as the overlay draws it: the hand
//! of the chair shown, its frame and words, and the badges over the chips.

#[allow(clippy::wildcard_imports)] // the tests' shared fixtures
use super::*;
use baylee_engine::choice::Pending;

/// This seat controls seat 1 and is asked seat 1's priority.
fn controlling_duel() -> Duel {
    let view = baylee_client_core::test_support::ViewBuilder::new(2)
        .with_hand(vec![("Own card", 1, 10)])
        .controlling(1, vec![("Their card", 1, 20)])
        .build();
    // The hand zone is drawn only for a table whose roster has arrived.
    let mut duel = Duel {
        statics: Some(baylee_client_core::test_support::statics(0)),
        ..Duel::default()
    };
    duel.receive_view(view);
    duel.receive_choice(Pending::Priority {
        player: PlayerId::new(1),
        legal: Box::default(),
    });
    duel
}

/// The bar, with the chips' control badges written as the app writes them.
fn with_badges(duel: Duel) -> App {
    let mut app = bar_of(duel);
    app.add_systems(Update, ledge::players::show_the_control_badges);
    app.update();
    app
}

/// What the badge over `player`'s chip says, while it shows.
fn badge(app: &mut App, player: PlayerId) -> Option<String> {
    let mut badges = app
        .world_mut()
        .query::<(&crate::hud::ControlBadge, &Visibility, &Children)>();
    let (_, seen, children) = badges
        .iter(app.world())
        .find(|(badge, ..)| badge.player == player)?;
    if *seen == Visibility::Hidden {
        return None;
    }
    let children: Vec<Entity> = children.iter().collect();
    let mut texts = app.world_mut().query::<&Text>();
    children
        .iter()
        .filter_map(|child| texts.get(app.world(), *child).ok())
        .map(|text| text.0.clone())
        .find(|words| !words.is_empty() && words.chars().count() > 1)
}

fn frames(app: &mut App) -> usize {
    app.world_mut()
        .query::<&crate::hud::ControlHandFrame>()
        .iter(app.world())
        .count()
}

/// Their chair is shown as their decision arrives: their hand in its rose
/// frame with the words saying whose it is, and the badge over their chip
/// saying it is the chair shown. Back on this seat's own chair the frame
/// and its words go, and the badge says only whose control they are under.
#[test]
fn their_chair_is_framed_named_and_badged() {
    let theirs = baylee_client_core::i18n::seat_name(Lang::En, None, PlayerId::new(1));
    let hand_words = Phrase::ControlHandOf.fill(Lang::En, &[&theirs]);
    let mut app = with_badges(controlling_duel());
    assert!(said(&mut app).contains(&hand_words), "{:?}", said(&mut app));
    assert_eq!(frames(&mut app), 1, "one frame round their hand");
    assert_eq!(
        badge(&mut app, PlayerId::new(1)).as_deref(),
        Some(Phrase::ControlUnderYoursShown.text(Lang::En))
    );
    assert_eq!(
        badge(&mut app, PlayerId::new(0)),
        None,
        "my own chip is mine"
    );

    app.world_mut()
        .resource_mut::<Duel>()
        .show_from(PlayerId::new(0));
    app.update();
    assert!(
        !said(&mut app).contains(&hand_words),
        "my own hand is unframed"
    );
    assert_eq!(frames(&mut app), 0);
    assert_eq!(
        badge(&mut app, PlayerId::new(1)).as_deref(),
        Some(Phrase::ControlUnderYours.text(Lang::En))
    );
}
