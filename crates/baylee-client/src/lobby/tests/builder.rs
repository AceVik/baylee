//! The deck builder, from the row in the deck list that opens it to the two presses it takes to leave one with unsaved work: which controls the screen offers, when the save button goes live, what typing and `Return` do to the name and the search, and reading a card without taking it. The scrolling lists belong here because all of them are the builder's — the arithmetic `hud::scrolled` does at both ends, the components an overflow needs before Bevy scrolls it rather than merely clipping, a swipe that must not also count as the tap on the card under the finger, and a list that keeps its place when adding a card rebuilds the whole tree. Which printing a row names, and what it then previews, is its own part.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn a_deck_can_be_opened_edited_and_thrown_away_from_the_list() {
    let mut app = headless();
    stocked(&mut app);
    app.update();
    let found = presses(&mut app);
    for wanted in [
        Press::NewDeck,
        Press::EditDeck(0),
        Press::DeleteDeck(0),
        Press::StarterDeck,
    ] {
        assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
    }
}

#[test]
fn the_builder_screen_builds_with_its_controls() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .build_deck();
    app.update();
    let found = presses(&mut app);
    for wanted in [
        Press::CloseBuilder,
        Press::FocusBuild(BuildField::Search),
        Press::FocusBuild(BuildField::Name),
        Press::SetZone(Zone::Main),
        Press::SetZone(Zone::Side),
        Press::ToggleColor('G'),
        Press::SetKind(Some("Creature")),
        Press::SetCmc(0),
        Press::TogglePlayable,
        Press::CycleSort,
        // Both pool rows are offered, so the search does not have to be
        // used to reach a two-card pool.
        Press::AddCard(0),
        Press::AddCard(1),
        // Every row can be read as well as taken.
        Press::Inspect(0),
    ] {
        assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
    }
    // Nothing is saveable yet: no name, no cards.
    assert!(
        !found.contains(&Press::SaveDeck),
        "a deck the gateway would refuse offers no save"
    );
}

#[test]
fn a_deck_worth_saving_offers_the_save() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        let builder = state.lobby.builder_mut();
        builder.set_name("Elves");
        assert!(builder.add(0, Zone::Main), "the pool has that card");
    }
    app.update();
    let found = presses(&mut app);
    assert!(found.contains(&Press::SaveDeck), "{found:?}");
    assert!(
        found.contains(&Press::RemoveRow(0)),
        "a card in the deck can come back out: {found:?}"
    );
}

#[test]
fn typing_reaches_the_builder_and_return_adds_the_first_hit() {
    let mut app = headless();
    stocked(&mut app);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .build_deck();
    // A new deck starts on its name, which is what stops it being saved.
    for ch in ['E', 'l', 'f'] {
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(typed(ch));
    }
    app.update();
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.builder().name(),
        "Elf"
    );

    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .builder_mut()
        .focus_on(BuildField::Search);
    for ch in ['E', 'l', 'v'] {
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(typed(ch));
    }
    app.update();
    {
        let state = app.world().resource::<LobbyState>();
        assert_eq!(state.lobby.builder().text(), "Elv");
        assert_eq!(state.lobby.builder().results().len(), 1, "one match");
    }
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(KeyboardInput {
            key_code: KeyCode::Enter,
            logical_key: Key::Enter,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    let state = app.world().resource::<LobbyState>();
    assert_eq!(
        state.lobby.builder().count_of(0, Zone::Main),
        1,
        "return took the one card the search left"
    );
}

#[test]
#[allow(clippy::float_cmp)] // every value here is exact by construction
fn a_long_list_can_be_scrolled_and_stops_at_both_ends() {
    // Three hundred pixels of window over nine hundred of cards.
    assert_eq!(crate::hud::scrolled(0.0, 120.0, 300.0, 900.0, 1.0), 120.0);
    assert_eq!(
        crate::hud::scrolled(500.0, 400.0, 300.0, 900.0, 1.0),
        600.0,
        "the bottom of the list is the end of it"
    );
    assert_eq!(
        crate::hud::scrolled(40.0, -400.0, 300.0, 900.0, 1.0),
        0.0,
        "and so is the top"
    );
    assert_eq!(
        crate::hud::scrolled(0.0, 50.0, 300.0, 300.0, 1.0),
        0.0,
        "a list that fits does not move at all"
    );
    // Physical sizes, logical offset: a 2× screen has half the room.
    assert_eq!(crate::hud::scrolled(0.0, 999.0, 300.0, 900.0, 0.5), 300.0);
}

#[test]
fn every_scrolling_list_carries_what_bevy_needs_to_scroll_it() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .build_deck();
    app.update();
    let mut query = app
        .world_mut()
        .query_filtered::<(&Node, Option<&ScrollPosition>), With<Scrollable>>();
    let lists: Vec<_> = query.iter(app.world()).collect();
    assert_eq!(lists.len(), 2, "the pool and the deck each scroll");
    for (node, position) in lists {
        assert_eq!(node.overflow.y, OverflowAxis::Scroll);
        assert!(
            position.is_some(),
            "an overflow with no ScrollPosition only clips"
        );
    }
}

#[test]
fn a_swipe_scrolls_the_list_rather_than_adding_the_card_under_it() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .build_deck();
    app.update();

    // The row a finger would land on, and the list it sits in. Layout
    // never runs here, so the list is told how big it is.
    let mut rows = app.world_mut().query::<(Entity, &Press)>();
    let card = rows
        .iter(app.world())
        .find(|(_, press)| **press == Press::AddCard(0))
        .map(|(entity, _)| entity)
        .expect("a card row");
    let mut lists = app.world_mut().query_filtered::<Entity, With<Scrollable>>();
    let list = lists.iter(app.world()).next().expect("a scrolling list");
    app.world_mut().entity_mut(list).insert(ComputedNode {
        size: Vec2::new(300.0, 300.0),
        content_size: Vec2::new(300.0, 900.0),
        ..default()
    });

    app.world_mut()
        .resource_mut::<Messages<Pointer<Drag>>>()
        .write(aimed(
            card,
            Drag {
                button: PointerButton::Primary,
                distance: Vec2::new(0.0, -40.0),
                delta: Vec2::new(0.0, -40.0),
            },
        ));
    app.world_mut()
        .resource_mut::<Messages<Pointer<DragEnd>>>()
        .write(aimed(
            card,
            DragEnd {
                button: PointerButton::Primary,
                distance: Vec2::new(0.0, -40.0),
            },
        ));
    app.world_mut()
        .resource_mut::<Messages<Pointer<Click>>>()
        .write(aimed(
            card,
            Click {
                button: PointerButton::Primary,
                hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                duration: std::time::Duration::ZERO,
                count: 1,
            },
        ));
    app.update();

    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .builder()
            .count_of(0, Zone::Main),
        0,
        "a swipe is not a tap"
    );
    assert_eq!(
        app.world()
            .entity(list)
            .get::<ScrollPosition>()
            .map(|p| p.y),
        Some(40.0),
        "and it moved the list under the finger"
    );
}

#[test]
fn leaving_a_deck_with_unsaved_work_takes_two_presses() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        state.lobby.builder_mut().set_name("Half a deck");
    }
    app.update();
    let back = press_target(&mut app, Press::CloseBuilder);

    tap(&mut app, back);
    app.update();
    assert!(
        matches!(
            app.world().resource::<LobbyState>().lobby.screen(),
            Screen::Build
        ),
        "the first press asks rather than leaves"
    );
    assert!(
        labels(&mut app).iter().any(|l| l == "Leave without saving"),
        "and says so"
    );

    let back = press_target(&mut app, Press::CloseBuilder);
    tap(&mut app, back);
    app.update();
    assert!(matches!(
        app.world().resource::<LobbyState>().lobby.screen(),
        Screen::Table
    ));
}

#[test]
fn a_card_can_be_read_in_the_builder() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .build_deck();
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .builder_mut()
        .inspect(0);
    app.update();
    let shown = labels(&mut app);
    assert!(
        shown.iter().any(|l| l == "{T}: Add {G}."),
        "the rules text is on screen: {shown:?}"
    );
    assert!(presses(&mut app).contains(&Press::CloseCard), "and closes");
}

#[test]
fn a_list_keeps_its_place_when_adding_a_card_rebuilds_it() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .build_deck();
    app.update();
    app.world_mut()
        .resource_mut::<Scrolled>()
        .set(List::Pool, 90.0);

    // Adding a card changes the lobby, which rebuilds the whole tree.
    let card = press_target(&mut app, Press::AddCard(0));
    tap(&mut app, card);
    app.update();

    let mut lists = app.world_mut().query::<(&ScrollPosition, &Scrollable)>();
    let pool = lists
        .iter(app.world())
        .find(|(_, which)| which.0 == List::Pool)
        .map(|(position, _)| position.y)
        .expect("the pool list");
    assert!(
        (pool - 90.0).abs() < f32::EPSILON,
        "the new list opens where the old one was, not at the top: {pool}"
    );

    // A different search *is* a different list, and starts at the top.
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .builder_mut()
        .focus_on(BuildField::Search);
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(typed('F'));
    app.update();
    assert!(app.world().resource::<Scrolled>().get(List::Pool).abs() < f32::EPSILON);
}

/// Every button of the filter panel is drawn, and pressing one reaches the
/// model.
///
/// Pressed rather than called. `FilterPanel` has a method for each of these
/// and calling them would prove what `filterdialog`'s own tests already
/// prove; what this asks is whether the **button** exists and is wired — the
/// question a test that calls the method cannot ask, and the one this client
/// has answered wrongly before.
#[test]
fn the_filter_panel_is_drawn_and_its_buttons_reach_the_builder() {
    use baylee_client_core::cardquery::Key;
    use baylee_client_core::filterdialog::{Act, Adding, Control, OFFERED};

    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .build_deck();
    app.update();

    // The cogwheel inside the search box.
    press(&mut app, Press::ToggleFilterPanel);
    assert!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .builder()
            .panel()
            .is_some(),
        "the gear did not open the panel"
    );

    // Two steps to a condition: what kind, then which key of that kind.
    act(&mut app, Act::AddStep(Adding::Kinds));
    act(&mut app, Act::AddStep(Adding::Keys(Control::Text)));
    let at = OFFERED
        .iter()
        .position(|key| *key == Key::Type)
        .expect("the menu offers a type");
    act(&mut app, Act::Add(at));
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.builder().text(),
        "t:\"\"",
        "the condition did not reach the box"
    );

    // The row's own controls are on screen: the minus and the ✕.
    act(&mut app, Act::Negate(0, true));
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.builder().text(),
        "-t:\"\""
    );
    act(&mut app, Act::Remove(0));
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.builder().text(),
        ""
    );
}

/// Presses one button of the filter panel by what it means, and panics if
/// nothing on screen means that.
///
/// The panic is the point: the model can do all of these whether or not a
/// button reaches them, so a test that could not fail on a missing button
/// would be measuring the model twice.
fn act(app: &mut App, wanted: baylee_client_core::filterdialog::Act) {
    let mut query = app
        .world_mut()
        .query::<(Entity, &crate::filterui::FilterAct)>();
    let found = query
        .iter(app.world())
        .find(|(_, act)| act.0 == wanted)
        .map(|(entity, _)| entity);
    let Some(target) = found else {
        panic!("{wanted:?} is on screen");
    };
    tap(app, target);
    app.update();
}

/// The panel says out loud that a chip is filtering too, and says nothing
/// when none is.
///
/// The deck builder filters twice and the panel edits one of the two, so the
/// line is the only thing that keeps a player who has typed a query from
/// reading half the truth. Both directions, because "the line is always
/// there" would pass the first half on its own and would teach the player to
/// stop reading it.
///
/// It is checked on the **drawn** text and not on `chips_in_force`, which the
/// model's own test already pins: what is worth proving here is that a panel
/// which knows is a panel that says.
#[test]
fn the_filter_panel_says_when_a_chip_is_filtering_as_well() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .build_deck();
    app.update();
    press(&mut app, Press::ToggleFilterPanel);

    // The switch is on by default, so the line is up before anything is
    // picked — which is the case it was written for.
    let said = labels(&mut app).join(" | ");
    let playable = Phrase::FilterChipPlayable.text(Lang::En);
    assert!(
        said.contains("Chips are narrowing this list as well"),
        "the panel said nothing about the chip that was already filtering: \
         {said}"
    );
    assert!(said.contains(playable), "and did not name it: {said}");

    // A colour and a type join it, each in its own words.
    press(&mut app, Press::ToggleColor('G'));
    press(&mut app, Press::SetKind(Some("Creature")));
    let said = labels(&mut app).join(" | ");
    for wanted in [
        Phrase::ColorGreen.text(Lang::En),
        Phrase::KindCreature.text(Lang::En),
        playable,
    ] {
        assert!(said.contains(wanted), "{wanted:?} was not named: {said}");
    }

    // And with every chip off, the line goes: a notice that is always there
    // is not a notice.
    press(&mut app, Press::ToggleColor('G'));
    // A second tap on the open chip is how a type is cleared; `SetKind(None)`
    // is on no button, so pressing it would be answering a question the
    // screen never asks.
    press(&mut app, Press::SetKind(Some("Creature")));
    press(&mut app, Press::TogglePlayable);
    let said = labels(&mut app).join(" | ");
    assert!(
        !said.contains("Chips are narrowing this list as well"),
        "nothing is filtering and the panel still says something is: {said}"
    );
    assert!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .builder()
            .panel()
            .is_some(),
        "this test's premise: the panel is still open, so the line's absence \
         is the line's and not the panel's"
    );
}
