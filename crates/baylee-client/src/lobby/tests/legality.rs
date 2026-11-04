//! The deck check through the controls a player has: the line under the list,
//! the sheet, a list being edited, and a refusal. What the check decides is
//! client-core's `legality_tests`; this is that the screen says it and that a
//! press reaches it, drawn headless (no window is looked at).

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;
use baylee_cardtext::{DeckRefusal, ListBody, ListRow, Problem};
use baylee_client_core::lobby::ListFor;

/// The builder open on a new deck holding a card, at a desktop width.
fn builder() -> App {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.build = crate::buildui::BuildUi::opened();
        state.lobby.build_deck();
        state.lobby.builder_mut().add(0, Zone::Main);
    }
    for _ in 0..3 {
        app.update();
    }
    app
}

fn list(id: &str, name: &str, builtin: bool) -> ListBody {
    ListBody {
        row: ListRow {
            id: id.to_string(),
            name: name.to_string(),
            format: "modern".to_string(),
            builtin,
            mine: !builtin,
            ..ListRow::default()
        },
        entries: Vec::new(),
    }
}

fn apply(app: &mut App, event: LobbyEvent) {
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .apply(event);
    app.update();
    app.update();
}

#[test]
fn the_foot_says_how_the_deck_stands_and_the_sheet_chooses_the_format() {
    let mut app = builder();
    let drawn = labels(&mut app);
    assert!(
        drawn.iter().any(|l| l.contains("No format chosen")),
        "a new deck keeps to nothing: {drawn:?}"
    );

    press(&mut app, Press::Build(BuildPress::OpenCheck));
    let found = presses(&mut app);
    assert!(found.contains(&Press::Build(BuildPress::SetFormat(1))));
    assert!(found.contains(&Press::Build(BuildPress::CloseCheck)));

    let choices = app
        .world()
        .resource::<LobbyState>()
        .lobby
        .builder()
        .format_choices();
    press(&mut app, Press::Build(BuildPress::SetFormat(1)));
    app.update();
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.lobby.builder().format(), choices[1]);
    assert!(state.lobby.builder().dirty(), "saved with the deck");

    let drawn = labels(&mut app);
    assert!(
        drawn.iter().any(|l| l.contains("Not legal for")),
        "one card is no deck of any format: {drawn:?}"
    );
    assert!(
        !drawn.iter().any(|l| l.starts_with("Legal for")),
        "{drawn:?}"
    );
    press(&mut app, Press::Build(BuildPress::CloseCheck));
    assert!(!app.world().resource::<LobbyState>().build.check_sheet);
}

/// Offline, or from a gateway with no catalog: said, and never legal.
#[test]
fn without_the_table_the_foot_says_so() {
    let mut app = builder();
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .builder_mut()
        .set_format("vintage");
    app.update();
    app.update();
    press(&mut app, Press::Build(BuildPress::OpenCheck));
    let drawn = labels(&mut app);
    assert!(drawn.iter().any(|l| l.contains("card table")), "{drawn:?}");
}

#[test]
fn a_list_is_picked_read_and_edited_over_the_builder() {
    let mut app = builder();
    apply(
        &mut app,
        LobbyEvent::Banlists(vec![
            list("o1", "modern", true).row,
            list("m1", "Kitchen table", false).row,
        ]),
    );
    press(&mut app, Press::Build(BuildPress::OpenCheck));
    let found = presses(&mut app);
    assert!(found.contains(&Press::Build(BuildPress::ListChoose(None))));
    assert!(found.contains(&Press::Build(BuildPress::ListChoose(Some(1)))));
    assert!(found.contains(&Press::Build(BuildPress::ListNew)));

    // An own list held: Edit and Copy stand beside it.
    apply(
        &mut app,
        LobbyEvent::Banlist(list("m1", "Kitchen table", false), ListFor::Deck),
    );
    let found = presses(&mut app);
    assert!(found.contains(&Press::Build(BuildPress::ListOpen(1))));
    assert!(found.contains(&Press::Build(BuildPress::ListCopyOf(1))));

    // Opened, it can be written.
    apply(
        &mut app,
        LobbyEvent::Banlist(list("m1", "Kitchen table", false), ListFor::Editor),
    );
    let found = presses(&mut app);
    assert!(found.contains(&Press::Build(BuildPress::ListDelete)));
    assert!(found.contains(&Press::Build(BuildPress::ListClose)));

    // An official one is read, and nothing there can write it.
    apply(
        &mut app,
        LobbyEvent::Banlist(list("o1", "modern", true), ListFor::Editor),
    );
    let found = presses(&mut app);
    assert!(!found.contains(&Press::Build(BuildPress::ListDelete)));
    assert!(!found.contains(&Press::Build(BuildPress::ListSave)));
    press(&mut app, Press::Build(BuildPress::ListClose));
    assert!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .builder()
            .list_editor()
            .is_none()
    );
}

/// A card typed into the search goes on the list with a press, and the list
/// reads the query language the builder reads.
#[test]
fn a_card_goes_on_a_list_through_the_builders_search() {
    let mut app = builder();
    press(&mut app, Press::Build(BuildPress::OpenCheck));
    press(&mut app, Press::Build(BuildPress::ListNew));
    let slot = {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let deck = state.lobby.builder_mut();
        // A list keys on the oracle id, which a real pool row carries.
        let mut cards = pool_cards();
        cards[0].oracle_id = "22e3cf1d-3559-4ce1-954c-8dc815342979".to_string();
        deck.set_pool(cards, true);
        let card = deck.card(0).expect("a pool card").english_name.clone();
        deck.focus_on(BuildField::ListSearch);
        deck.edit_buffer(BuildField::ListSearch, |buffer| buffer.insert(&card));
        deck.list_editor().expect("open").results()[0]
    };
    app.update();
    app.update();
    press(
        &mut app,
        Press::Build(BuildPress::ListPut(
            slot,
            baylee_cardtext::ListStatus::Banned,
        )),
    );
    let state = app.world().resource::<LobbyState>();
    let open = state.lobby.builder().list_editor().expect("open");
    assert_eq!(open.entries().len(), 1);
    assert!(open.saveable());
}

/// A refusal stands over whatever screen the deck came from, a line a reason.
#[test]
fn a_refused_deck_reads_why() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.update();
    apply(
        &mut app,
        LobbyEvent::DeckRefused(DeckRefusal {
            list: "No rings".to_string(),
            format: "commander".to_string(),
            problems: vec![Problem::WrongSize {
                have: 60,
                need: 100,
            }],
        }),
    );
    let drawn = labels(&mut app);
    assert!(drawn.iter().any(|l| l.contains("No rings")), "{drawn:?}");
    assert!(drawn.iter().any(|l| l.contains("100")), "{drawn:?}");
    press(&mut app, Press::Shared(SharedPress::CloseRefusal));
    assert!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .refusal()
            .is_none()
    );
}
