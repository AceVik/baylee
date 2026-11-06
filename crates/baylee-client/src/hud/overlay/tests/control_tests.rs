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

/// The pointer on a badge opens the card that gave the control, in the
/// printing its object wears, and says who holds the control and by what.
/// Mindslaver is sacrificed for it, so that object lies in a graveyard.
#[test]
fn a_badge_opens_the_source_in_its_print_and_names_it() {
    use baylee_core::ids::{CardIndex, PrintRef};
    let source = CardIndex::new(6506);
    let mut duel = controlling_duel();
    let card = baylee_view::CardIdentity {
        index: source,
        print: PrintRef::new(195),
        face: 0,
    };
    {
        let view = duel.view.as_mut().expect("a view");
        view.controls = vec![baylee_view::PlayerControl {
            player: PlayerId::new(1),
            controller: PlayerId::new(0),
            source: Some(source),
        }];
        let mut object = baylee_client_core::test_support::printed(90, 0, "Mindslaver", 195);
        object.card = Some(card);
        view.graveyards[0].push(object);
    }
    let mut app = with_badges(duel);
    app.update();
    let mut links = app
        .world_mut()
        .query_filtered::<(&crate::hud::LogLink, &crate::hud::Hint), With<crate::hud::ControlBadge>>();
    let (link, hint) = links.iter(app.world()).next().expect("a linked badge");
    assert_eq!(link.card, Some(card));
    let name = baylee_cards::by_index(source)
        .and_then(|card| card.faces.first())
        .expect("a registered card")
        .name;
    assert!(hint.0.contains(name), "{}", hint.0);
}

/// Whether the guest mark over `player`'s chip shows.
fn guest_mark(app: &mut App, player: PlayerId) -> bool {
    let mut marks = app
        .world_mut()
        .query::<(&crate::hud::GuestBadge, &Visibility)>();
    marks
        .iter(app.world())
        .find(|(mark, _)| mark.player == player)
        .is_some_and(|(_, seen)| *seen != Visibility::Hidden)
}

/// The roster says who is a guest, and the mark stands over another player's
/// chip: a name alone (`Bob#1a2b`) does not say so. Never over this seat's own.
#[test]
fn a_guest_wears_the_mark_on_the_others_chips_only() {
    let mut duel = controlling_duel();
    let statics = duel.statics.as_mut().expect("a roster");
    statics.seats = (0..2)
        .map(|seat| baylee_view::SeatIdentity {
            player: PlayerId::new(seat),
            display_name: format!("Seat {seat}"),
            is_ai: false,
            away: false,
            team: None,
            guest: true,
        })
        .collect();
    let mut app = with_badges(duel);
    app.update();
    assert!(guest_mark(&mut app, PlayerId::new(1)), "another's chip");
    assert!(!guest_mark(&mut app, PlayerId::new(0)), "my own chip");
}

/// An account wears no mark.
#[test]
fn an_account_wears_no_guest_mark() {
    let mut app = with_badges(controlling_duel());
    app.update();
    assert!(!guest_mark(&mut app, PlayerId::new(1)));
}

/// With no object to show, the badge opens nothing: no printing is guessed.
#[test]
fn a_badge_with_no_object_to_show_opens_nothing() {
    let mut app = with_badges(controlling_duel());
    app.update();
    let mut links = app
        .world_mut()
        .query_filtered::<&crate::hud::LogLink, With<crate::hud::ControlBadge>>();
    assert_eq!(links.iter(app.world()).count(), 0);
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
