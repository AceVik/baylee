//! The deck builder, from the row in the deck list that opens it to the two presses it takes to leave one with unsaved work: which controls the screen offers, when the save button goes live, what typing and `Return` do to the name and the search, and reading a card without taking it. The scrolling lists belong here because all of them are the builder's — the arithmetic `hud::scrolled` does at both ends, the components an overflow needs before Bevy scrolls it rather than merely clipping, a swipe that must not also count as the tap on the card under the finger, and a list that keeps its place when adding a card rebuilds the whole tree. Which printing a row names, and what it then previews, is its own part.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn a_deck_can_be_opened_edited_and_thrown_away_from_the_list() {
    let mut app = headless();
    stocked(&mut app);
    app.update();
    to_decks(&mut app);
    let found = presses(&mut app);
    for wanted in [
        Press::Decks(DecksPress::NewDeck),
        Press::Decks(DecksPress::Edit(0)),
        Press::Shared(SharedPress::OpenMenu(ShellMenu::Deck(0))),
        Press::Decks(DecksPress::Tab(super::decks::DecksTab::House)),
    ] {
        assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
    }
    // Delete is in the tile's `⋯` (§6), last, after a rule.
    press(
        &mut app,
        Press::Shared(SharedPress::OpenMenu(ShellMenu::Deck(0))),
    );
    app.update();
    assert!(presses(&mut app).contains(&Press::Decks(DecksPress::Delete(0))));
}

/// #254: the deckbuilder says which build it is, as the lobby does, on every
/// frame: beside the brand where there is room (the shell design §2.1, the
/// short form), and in the gateway's popover — the whole build — everywhere,
/// a narrow header included.
#[test]
fn the_builder_names_its_build_on_every_frame() {
    for width in [1400.0, 390.0] {
        let mut app = headless();
        stocked(&mut app);
        sized(&mut app, width);
        app.world_mut()
            .resource_mut::<LobbyState>()
            .lobby
            .build_deck();
        app.update();
        if width > 1000.0 {
            let drawn = labels(&mut app);
            assert!(
                drawn.iter().any(|l| l == baylee_build::VERSION),
                "{width} px: {drawn:?}"
            );
        }
        press(&mut app, Press::Header(crate::lobby::HeaderPress::Gateway));
        let drawn = labels(&mut app);
        assert!(
            drawn.iter().any(|l| l.contains(baylee_build::short())),
            "{width} px: {drawn:?}"
        );
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
        Press::Build(BuildPress::CloseBuilder),
        Press::Build(BuildPress::FocusBuild(BuildField::Search)),
        Press::Build(BuildPress::Rename),
        Press::Build(BuildPress::SetTab(crate::buildui::DeckTab::Main)),
        Press::Build(BuildPress::SetTab(crate::buildui::DeckTab::Side)),
        Press::Build(BuildPress::SetTab(crate::buildui::DeckTab::Stats)),
        Press::Build(BuildPress::ToggleColor('G')),
        Press::Build(BuildPress::ToggleRail),
        Press::Build(BuildPress::ToggleSyntax),
        Press::Build(BuildPress::TogglePlayable),
        Press::Build(BuildPress::CycleSort),
        Press::Build(BuildPress::ToggleHeaderMenu),
        // Both pool rows are offered, so the search does not have to be
        // used to reach a two-card pool — one `+` each (§7).
        Press::Build(BuildPress::AddFromPool(0, false)),
        Press::Build(BuildPress::AddFromPool(1, false)),
        Press::Build(BuildPress::PoolMenu(0)),
        // Every row can be read as well as taken.
        Press::Build(BuildPress::Inspect(0)),
    ] {
        assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
    }
    // The rail's filters are behind Filters, not a second chip row (§7).
    assert!(!found.contains(&Press::Build(BuildPress::SetKind(Some("Creature")))));
    // Nothing is saveable yet: no name, no cards — Save stands, and is off.
    let save = press_target(&mut app, Press::Build(BuildPress::SaveDeck));
    assert!(
        app.world()
            .entity(save)
            .contains::<crate::shellkit::controls::Disabled>(),
        "a deck the gateway would refuse offers no live save"
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
    assert!(
        found.contains(&Press::Build(BuildPress::SaveDeck)),
        "{found:?}"
    );
    assert!(
        found.contains(&Press::Build(BuildPress::RemoveRow(0))),
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
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        // An empty deck shows its empty state, not a list.
        state.lobby.builder_mut().add(0, Zone::Main);
    }
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
        .find(|(_, press)| **press == Press::Build(BuildPress::AddFromPool(0, false)))
        .map(|(entity, _)| entity)
        .expect("a card row");
    let mut lists = app.world_mut().query::<(Entity, &Scrollable)>();
    let list = lists
        .iter(app.world())
        .find(|(_, list)| list.0 == List::Pool)
        .map(|(id, _)| id)
        .expect("the card pool's scrolling list");
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

/// Back with unsaved work asks "Discard changes?" (KEYBOARD §2.5): Keep
/// editing stays, Discard leaves; a saved deck leaves at once.
#[test]
fn leaving_a_deck_with_unsaved_work_asks_first() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        state.lobby.builder_mut().set_name("Half a deck");
    }
    app.update();
    press(&mut app, Press::Build(BuildPress::CloseBuilder));
    assert!(
        matches!(
            app.world().resource::<LobbyState>().lobby.screen(),
            Screen::Build
        ),
        "the first press asks rather than leaves"
    );
    assert!(
        labels(&mut app).iter().any(|l| l == "Discard changes?"),
        "and says so"
    );
    // Keep editing is focused: Enter keeps editing (KEYBOARD §2.5).
    app.update();
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(pressed(KeyCode::Enter, Key::Enter));
    app.update();
    assert!(!app.world().resource::<LobbyState>().confirm_leave);
    assert!(matches!(
        app.world().resource::<LobbyState>().lobby.screen(),
        Screen::Build
    ));

    press(&mut app, Press::Build(BuildPress::CloseBuilder));
    press(&mut app, Press::Build(BuildPress::DiscardAndLeave));
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
    let found = presses(&mut app);
    assert!(
        found.contains(&Press::Build(BuildPress::PickerClose)),
        "and closes"
    );
    // One window (windows-b6 §A.3): the row's door has the primary Add to
    // deck and the counts' steppers, and the printing's section.
    assert!(found.contains(&Press::Build(BuildPress::WindowAdd(Zone::Main))));
    assert!(found.contains(&Press::Build(BuildPress::WindowStep(Zone::Side, true))));
    assert!(
        shown.iter().any(|l| l == "PRINTING"),
        "the printing section: {shown:?}"
    );
}

/// IN THIS DECK keeps each label with its stepper: the row wraps between
/// the pairs, never between "Sideboard" and its − n + (4K pass,
/// 09.10.2026, where the sideboard's stepper wrapped under its label).
#[test]
fn the_card_window_keeps_each_count_label_with_its_stepper() {
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
    for (zone, said) in [(Zone::Main, "Main"), (Zone::Side, "Sideboard")] {
        let minus = {
            let mut presses = app.world_mut().query::<(Entity, &Press)>();
            presses
                .iter(app.world())
                .find(|(_, p)| **p == Press::Build(BuildPress::WindowStep(zone, false)))
                .map(|(e, _)| e)
                .expect("the stepper's minus")
        };
        // The nearest ancestor of the stepper that holds the label: it must
        // not wrap.
        let world = app.world();
        let mut at = minus;
        let holder = loop {
            let parent = world.get::<ChildOf>(at).map_or_else(
                || panic!("no ancestor of {said}'s stepper holds its label"),
                ChildOf::parent,
            );
            // The label is a clip around its words (`buildui::cell`).
            let says = |e: Entity| world.get::<Text>(e).is_some_and(|t| t.0 == said);
            let holds = world.get::<Children>(parent).is_some_and(|children| {
                children.iter().any(|c| {
                    says(c)
                        || world
                            .get::<Children>(c)
                            .is_some_and(|inner| inner.iter().any(says))
                })
            });
            if holds {
                break parent;
            }
            at = parent;
        };
        let node = world.get::<Node>(holder).expect("a node");
        assert_eq!(
            node.flex_wrap,
            FlexWrap::NoWrap,
            "{said} and its stepper may wrap apart"
        );
    }
}

/// Offline, or with no catalog, the window says why there is one printing
/// in a sentence, and its text is never blank: the compiled English Oracle
/// stands in, tagged for a German reader (windows-b6 §A.3).
#[test]
fn the_card_window_offline_says_why_and_is_never_blank() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        let _ = state.lobby.builder_mut().inspect(0);
        let index = state
            .lobby
            .builder()
            .card(0)
            .map(|c| c.index)
            .unwrap_or_default();
        state
            .lobby
            .builder_mut()
            .set_printings(index, Vec::new(), false);
    }
    app.update();
    let shown = labels(&mut app);
    assert!(
        shown.iter().any(|l| l == "More printings need the gateway"),
        "the one-printing sentence: {shown:?}"
    );
}

#[test]
fn issue_188_adding_a_card_keeps_the_pool_and_its_scroll_position() {
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

    let mut lists = app.world_mut().query::<(Entity, &Scrollable)>();
    let pool_entity = lists
        .iter(app.world())
        .find(|(_, list)| list.0 == List::Pool)
        .unwrap()
        .0;
    app.world_mut()
        .entity_mut(pool_entity)
        .insert(ScrollPosition(Vec2::new(0.0, 90.0)));

    let card = press_target(&mut app, Press::Build(BuildPress::AddFromPool(0, false)));
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

    assert!(
        app.world().get_entity(pool_entity).is_ok(),
        "adding must retain the pool tree"
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
    press(&mut app, Press::Build(BuildPress::ToggleFilterPanel));
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
    press(&mut app, Press::Build(BuildPress::ToggleFilterPanel));

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

    // A colour and a type join it, each in its own words (the type from
    // the Filters rail).
    press(&mut app, Press::Build(BuildPress::ToggleColor('G')));
    press(&mut app, Press::Build(BuildPress::ToggleRail));
    press(
        &mut app,
        Press::Build(BuildPress::SetKind(Some("Creature"))),
    );
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
    press(&mut app, Press::Build(BuildPress::ToggleColor('G')));
    // A second tap on the open chip is how a type is cleared; `SetKind(None)`
    // is on no button, so pressing it would be answering a question the
    // screen never asks.
    press(
        &mut app,
        Press::Build(BuildPress::SetKind(Some("Creature"))),
    );
    press(&mut app, Press::Build(BuildPress::TogglePlayable));
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

#[test]
fn issue_191_clear_requires_confirmation_and_cancel_preserves_both_zones() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        state.lobby.builder_mut().add(0, Zone::Main);
        state.lobby.builder_mut().add(1, Zone::Side);
    }
    app.update();
    if !presses(&mut app).contains(&Press::Build(BuildPress::ClearDeck)) {
        press(&mut app, Press::Build(BuildPress::ToggleHeaderMenu));
    }
    press(&mut app, Press::Build(BuildPress::ClearDeck));
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .builder()
            .entries(Zone::Main)
            .len(),
        1
    );
    press(&mut app, Press::Shared(SharedPress::CancelDestructive));
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .builder()
            .entries(Zone::Side)
            .len(),
        1
    );
    if !presses(&mut app).contains(&Press::Build(BuildPress::ClearDeck)) {
        press(&mut app, Press::Build(BuildPress::ToggleHeaderMenu));
    }
    press(&mut app, Press::Build(BuildPress::ClearDeck));
    press(&mut app, Press::Shared(SharedPress::ConfirmDestructive));
    let builder = app.world().resource::<LobbyState>().lobby.builder();
    assert!(builder.entries(Zone::Main).is_empty());
    assert!(builder.entries(Zone::Side).is_empty());
}

#[test]
fn issue_188_search_keeps_deck_rows_and_repeated_listings_keep_the_root() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        state.lobby.builder_mut().add(0, Zone::Main);
        state.lobby.builder_mut().focus_on(BuildField::Search);
    }
    app.update();
    let remove = press_target(&mut app, Press::Build(BuildPress::RemoveRow(0)));
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(typed('F'));
    app.update();
    assert_eq!(
        press_target(&mut app, Press::Build(BuildPress::RemoveRow(0))),
        remove
    );
    let search = press_target(
        &mut app,
        Press::Build(BuildPress::FocusBuild(BuildField::Search)),
    );
    for _ in 0..5 {
        app.world()
            .resource::<Mailbox>()
            .0
            .lock()
            .unwrap()
            .push(Reply::Event(LobbyEvent::Games(GameListing::default())));
        app.update();
        assert_eq!(
            press_target(
                &mut app,
                Press::Build(BuildPress::FocusBuild(BuildField::Search))
            ),
            search
        );
    }
}

#[test]
fn issue_191_a_deleted_deck_leaves_the_shelf_and_undo_never_dispatches_a_delete() {
    let mut app = headless();
    stocked(&mut app);
    app.update();
    to_decks(&mut app);
    press(
        &mut app,
        Press::Shared(SharedPress::OpenMenu(ShellMenu::Deck(0))),
    );
    app.update();
    press(&mut app, Press::Decks(DecksPress::Delete(0)));
    // No confirm sheet (S-10): the deck leaves the shelf at once, an Undo
    // toast stands, and nothing has gone to the gateway.
    {
        let state = app.world().resource::<LobbyState>();
        assert!(state.confirmation.is_none());
        assert_eq!(state.lobby.staged_delete(), Some("d1"));
        assert!(state.undo.is_some());
        assert!(!state.lobby.busy(), "nothing was sent");
        assert_eq!(state.lobby.decks().len(), 1);
    }
    assert!(!presses(&mut app).contains(&Press::Decks(DecksPress::Edit(0))));
    // The toast lane draws its Undo a frame after the press.
    app.update();
    press(&mut app, Press::Decks(DecksPress::Undo));
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.lobby.staged_delete(), None);
    assert!(state.undo.is_none());
    assert!(!state.lobby.busy(), "Undo sends nothing either");
    assert!(presses(&mut app).contains(&Press::Decks(DecksPress::Edit(0))));
}

/// The Undo runs out on its own and only then is the deletion sent; leaving
/// the Decks screen sends it at once (S-10).
#[test]
fn a_waiting_deletion_is_sent_when_its_undo_runs_out_or_the_screen_changes() {
    for leave in [false, true] {
        let mut app = headless();
        stocked(&mut app);
        app.update();
        to_decks(&mut app);
        press(
            &mut app,
            Press::Shared(SharedPress::OpenMenu(ShellMenu::Deck(0))),
        );
        app.update();
        press(&mut app, Press::Decks(DecksPress::Delete(0)));
        assert!(!app.world().resource::<LobbyState>().lobby.busy());
        if leave {
            app.world_mut().resource_mut::<LobbyState>().hub = Hub::Play;
            app.update();
        } else {
            app.world_mut()
                .resource_mut::<LobbyState>()
                .undo
                .as_mut()
                .expect("an undo")
                .left = 0.0;
            app.update();
        }
        let state = app.world().resource::<LobbyState>();
        assert_eq!(state.lobby.staged_delete(), None, "leave: {leave}");
        // Flushed: what it sends is `Lobby::flush_delete`'s (client-core's
        // tests); the answer of a gateway this app does not have is not.
        assert!(state.undo.is_none());
    }
}

#[test]
fn issue_192_search_supports_select_all_replacement_and_middle_insertion() {
    let mut app = headless();
    stocked(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        state.lobby.builder_mut().focus_on(BuildField::Search);
        state.lobby.builder_mut().set_text("Old query");
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::SuperLeft);
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(typed('a'));
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::SuperLeft);
    {
        let mut keys = app.world_mut().resource_mut::<Messages<KeyboardInput>>();
        keys.write(typed('F'));
        keys.write(typed('r'));
        keys.write(pressed(KeyCode::ArrowLeft, Key::ArrowLeft));
        keys.write(typed('o'));
    }
    app.update();
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.builder().text(),
        "For"
    );
}

#[test]
fn issue_194_commander_management_is_visible_without_opening_card_details() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        let mut cards = pool_cards();
        cards[0].commander = true;
        cards[0].partners = vec![cards[1].index];
        cards[1].commander = true;
        state.lobby.builder_mut().set_pool(cards, true);
    }
    app.update();
    press(&mut app, Press::Build(BuildPress::ChooseCommander(false)));
    press(&mut app, Press::Build(BuildPress::SetCommander(0)));
    assert!(presses(&mut app).contains(&Press::Build(BuildPress::RemoveCommander(0))));
    press(&mut app, Press::Build(BuildPress::ChooseCommander(true)));
    assert!(presses(&mut app).contains(&Press::Build(BuildPress::AddPartner(1))));
    assert!(!presses(&mut app).contains(&Press::Build(BuildPress::AddPartner(0))));
    press(&mut app, Press::Build(BuildPress::AddPartner(1)));
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .builder()
            .commanders(),
        &[0, 1]
    );
    press(&mut app, Press::Build(BuildPress::RemoveCommander(0)));
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .builder()
            .commanders(),
        &[1]
    );
}

/// #255: the commander's picture is a deck row's picture. It is drawn at the
/// row's full height, shows the printing the deck holds rather than the
/// registry's, and opens the picker on the commander's own row, even while
/// the sideboard is the list on screen. A commander moved out of the deck
/// keeps its line but offers no picker, since no row is left to restyle.
#[test]
fn a_commanders_picture_is_a_deck_rows_picture() {
    let chosen = "11111111-2222-3333-4444-555555555555";
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        let mut cards = pool_cards();
        cards[0].commander = true;
        cards[0].scryfall_id = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string();
        let deck = state.lobby.builder_mut();
        deck.set_pool(cards, true);
        assert!(deck.add(1, Zone::Main));
        let foil = baylee_core::deckrow::PrintChoice {
            scryfall_id: Some(chosen.to_string()),
            finish: Some(Finish::Foil),
            ..baylee_core::deckrow::PrintChoice::default()
        };
        assert!(deck.add_print(0, Zone::Main, foil));
        assert!(deck.set_commander(0));
        deck.set_zone(Zone::Side);
    }
    app.update();

    let (picture, node) = app
        .world_mut()
        .query::<(Entity, &Press, &Node)>()
        .iter(app.world())
        .find(|(_, p, _)| **p == Press::Build(BuildPress::PickCommanderPrint(0)))
        .map(|(entity, _, node)| (entity, node.clone()))
        .expect("the commander's picture opens the picker");
    let (Val::Px(w), Val::Px(h)) = (node.width, node.height) else {
        panic!("a print at a fixed size: {node:?}");
    };
    assert!(
        (w / h - 5.0 / 7.0).abs() < 0.05,
        "a card's own aspect: {w} × {h}"
    );
    let hover = app
        .world()
        .entity(picture)
        .get::<HoverCard>()
        .expect("the picture previews its card");
    let url = hover.url.clone().expect("a real id has art");
    assert!(
        url.contains(chosen),
        "the deck's printing, not the pool's: {url}"
    );
    assert_eq!(hover.finish, FinishTreatment::Foil);

    press(&mut app, Press::Build(BuildPress::PickCommanderPrint(0)));
    let state = app.world().resource::<LobbyState>();
    let picker = state.lobby.builder().picker().expect("the picker is open");
    assert!(picker.replacing(), "it restyles the commander's row");
    assert_eq!(picker.slot(), 0);
    assert_eq!(picker.zone(), Zone::Main, "with the sideboard on screen");

    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let deck = state.lobby.builder_mut();
        deck.close_picker();
        let at = deck.commander_row(0).unwrap();
        assert!(deck.move_entry(at, Zone::Main, Zone::Side));
    }
    app.update();
    let found = presses(&mut app);
    assert!(
        found.contains(&Press::Build(BuildPress::RemoveCommander(0))),
        "{found:?}"
    );
    assert!(
        !found.contains(&Press::Build(BuildPress::PickCommanderPrint(0))),
        "{found:?}"
    );
}

/// The deck list is virtual too: rows far from the window are unmounted and
/// come back when the window returns (§10 #6).
#[test]
fn virtual_rows_unmount_offscreen_controls_and_restore_them_on_return() {
    use crate::buildui::virtual_rows::VirtualList;
    use bevy::ui::CalculatedClip;
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
        .add(0, Zone::Main);
    app.update();
    let lists: Vec<_> = app
        .world_mut()
        .query_filtered::<Entity, With<VirtualList>>()
        .iter(app.world())
        .collect();
    assert_eq!(lists.len(), 2, "the pool and the deck");
    assert!(presses(&mut app).contains(&Press::Build(BuildPress::RemoveRow(0))));
    let place = |app: &mut App, clip: Rect| {
        for &list in &lists {
            let mut entity = app.world_mut().entity_mut(list);
            entity.get_mut::<ComputedNode>().unwrap().size = Vec2::new(400.0, 300.0);
            entity.insert(UiGlobalTransform::from(
                bevy::math::Affine2::from_translation(Vec2::new(200.0, 150.0)),
            ));
            entity.insert(CalculatedClip { clip });
        }
        app.update();
    };
    // The window far below the list: everything is unmounted.
    place(&mut app, Rect::new(0.0, 5000.0, 400.0, 5300.0));
    assert!(!presses(&mut app).contains(&Press::Build(BuildPress::RemoveRow(0))));
    // And back.
    place(&mut app, Rect::new(0.0, 0.0, 400.0, 300.0));
    assert!(presses(&mut app).contains(&Press::Build(BuildPress::RemoveRow(0))));
}

#[test]
fn thumbnails_open_printing_and_empty_deck_is_inside_the_menu() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        state.lobby.builder_mut().add(0, Zone::Main);
    }
    app.update();
    assert!(!presses(&mut app).contains(&Press::Build(BuildPress::ClearDeck)));
    assert!(presses(&mut app).contains(&Press::Build(BuildPress::ToggleHeaderMenu)));
    let target = app
        .world_mut()
        .query::<(&Press, &Node)>()
        .iter(app.world())
        .find(|(p, _)| **p == Press::Build(BuildPress::PickRowPrint(0)))
        .unwrap()
        .1;
    // The print stands at a card's own aspect (§7: 48 × 67 in a pool row).
    let (Val::Px(w), Val::Px(h)) = (target.width, target.height) else {
        panic!("a print at a fixed size: {target:?}");
    };
    assert!((w / h - 5.0 / 7.0).abs() < 0.05, "{w} × {h}");
    press(&mut app, Press::Build(BuildPress::ToggleHeaderMenu));
    assert!(presses(&mut app).contains(&Press::Build(BuildPress::ClearDeck)));
}

#[test]
fn virtual_catalog_reaches_past_sixty_results_without_mounting_every_row() {
    use crate::buildui::virtual_rows::VirtualList;
    use bevy::ui::CalculatedClip;
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        let template = state.lobby.builder().card(0).unwrap().clone();
        let cards = (0..120_u32)
            .map(|i| {
                let mut c = template.clone();
                c.index = i;
                c.name = format!("Card {i:03}");
                c.english_name.clone_from(&c.name);
                c
            })
            .collect();
        state.lobby.builder_mut().set_pool(cards, true);
    }
    app.update();
    assert!(
        presses(&mut app)
            .iter()
            .filter(|p| matches!(p, Press::Build(BuildPress::AddFromPool(_, false))))
            .count()
            <= 24,
        "the first draw mounts two screens' worth, not the catalog"
    );
    let list = app
        .world_mut()
        .query_filtered::<(Entity, &VirtualList), ()>()
        .iter(app.world())
        .find(|(_, list)| crate::buildui::virtual_rows::is_pool(list))
        .map(|(e, _)| e)
        .unwrap();
    {
        let mut entity = app.world_mut().entity_mut(list);
        // 120 rows at the kit's 72-px pitch (step 4).
        entity.get_mut::<ComputedNode>().unwrap().size = Vec2::new(600.0, 8640.0);
        entity.insert(UiGlobalTransform::from(
            bevy::math::Affine2::from_translation(Vec2::new(0.0, 4320.0)),
        ));
        entity.insert(CalculatedClip {
            clip: Rect::new(0.0, 0.0, 600.0, 380.0),
        });
    }
    // The new wheel position must mount the destination before layout has
    // updated the old transforms; otherwise a fast scroll flashes blank.
    let scroller = app.world().entity(list).get::<ChildOf>().unwrap().parent();
    app.world_mut()
        .entity_mut(scroller)
        .get_mut::<ScrollPosition>()
        .unwrap()
        .y = 8260.0;
    app.update();
    let visible = presses(&mut app);
    assert!(visible.contains(&Press::Build(BuildPress::AddFromPool(119, false))));
    assert!(!visible.contains(&Press::Build(BuildPress::AddFromPool(0, false))));
    assert!(
        visible
            .iter()
            .filter(|p| matches!(p, Press::Build(BuildPress::AddFromPool(_, false))))
            .count()
            < 24,
        "a window and a window's worth on either side"
    );
}

#[test]
fn a_pool_reply_from_the_previous_language_cannot_replace_the_current_one() {
    let mut app = headless();
    stocked(&mut app);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .set_lang(Lang::De);
    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .unwrap()
        .push(Reply::PoolLanguage(
            Lang::En,
            LobbyEvent::Pool {
                cards: Vec::new(),
                has_text: false,
            },
        ));
    app.update();
    assert!(
        !app.world()
            .resource::<LobbyState>()
            .lobby
            .builder()
            .pool()
            .is_empty()
    );
}

/// #270: a sign-out forgets the pool, because `/pool` answers a session.
/// Offline play has its own and no session, and after a sign-out its
/// builder still opens with every card: asked again, of this process.
#[test]
fn offline_play_builds_from_its_own_pool_after_a_sign_out() {
    let mut app = headless();
    for round in ["the first visit", "after a sign-out"] {
        app.world_mut().resource_mut::<LobbyState>().offline =
            Some(super::offline::Offline::without_a_file());
        to_gateway_face(&mut app);
        tap_control(&mut app, "play offline", |p| {
            *p == Press::Front(FrontPress::PlayOffline)
        });
        to_decks(&mut app);
        press(&mut app, Press::Decks(DecksPress::NewDeck));
        settle(&mut app);
        {
            let state = app.world().resource::<LobbyState>();
            assert_eq!(*state.lobby.screen(), Screen::Build, "{round}");
            assert!(
                !state.lobby.builder().pool().is_empty(),
                "{round}: the pool is here"
            );
        }
        tap_control(&mut app, "the way out", |p| {
            *p == Press::Build(BuildPress::CloseBuilder)
        });
        sign_out_from_the_header(&mut app);
        assert!(
            !app.world()
                .resource::<LobbyState>()
                .lobby
                .builder()
                .loaded(),
            "{round}: signed out, no pool is held"
        );
    }
}

/// The builder runs the text face's scrollbar (#259). The duel's copy of the
/// system runs only while a duel is open, and a preview whose rules text
/// still runs over at the floor would otherwise show no bar at all.
#[test]
fn a_text_face_in_the_builder_shows_its_scrollbar() {
    let mut app = headless();
    let text_box = app
        .world_mut()
        .spawn((
            crate::face::FaceTextBox,
            ScrollPosition(Vec2::ZERO),
            ComputedNode {
                size: Vec2::new(200.0, 100.0),
                content_size: Vec2::new(200.0, 200.0),
                ..default()
            },
        ))
        .id();
    let thumb = app
        .world_mut()
        .spawn((crate::face::FaceScrollThumb, Node::default()))
        .id();
    let track = app
        .world_mut()
        .spawn((
            crate::face::FaceScrollbar { text_box },
            Node {
                display: Display::None,
                ..default()
            },
        ))
        .add_child(thumb)
        .id();
    app.update();
    assert_eq!(
        app.world().get::<Node>(track).unwrap().display,
        Display::Flex
    );
}

/// The owner's beta.6 review: "deck list → edit deck shows an old page".
/// The house list a player looked at on the Decks screen stayed open
/// behind its tab, and the builder's own full-screen library page stood
/// where the deck should have opened ("Hausdecks", with Back). The way from
/// the list into a deck passes through no library page on any frame.
#[test]
fn the_way_from_the_deck_list_into_a_deck_passes_through_no_library_page() {
    use baylee_client_core::lobby::library::{HouseDeck, Reply};
    let mut app = headless();
    stocked(&mut app);
    app.update();
    to_decks(&mut app);
    press(
        &mut app,
        Press::Decks(DecksPress::Tab(super::decks::DecksTab::House)),
    );
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .apply(LobbyEvent::Library(Reply::House(vec![HouseDeck {
            id: "shared".into(),
            name: "House".into(),
            version: 1,
            ..Default::default()
        }])));
    app.update();
    press(
        &mut app,
        Press::Decks(DecksPress::Tab(super::decks::DecksTab::Mine)),
    );
    press(&mut app, Press::Decks(DecksPress::Edit(0)));
    // The old page's title, as it read before it went.
    let house = "House decks".to_string();
    for frame in 0..4 {
        let library: Vec<Press> = presses(&mut app)
            .into_iter()
            .filter(|p| matches!(p, Press::Library(l) if *l != super::LibraryPress::BrowseHistory))
            .collect();
        assert!(library.is_empty(), "frame {frame}: {library:?}");
        assert!(
            !labels(&mut app).contains(&house),
            "frame {frame}: the house page"
        );
        app.update();
    }
    assert!(
        presses(&mut app).contains(&Press::Build(BuildPress::CloseBuilder)),
        "the builder is what opened"
    );
}

/// One history sheet from both doors (`DESIGN` §C.3): over the builder it
/// stands on top of the builder (no page of its own any more), shows the
/// classified changes — a foil change as one `finish` row — and Esc leaves
/// the player in the builder.
#[test]
fn the_builders_history_is_the_one_sheet_over_the_builder_and_esc_stays_there() {
    use baylee_client_core::lobby::library::{History, Reply, Revision, Snapshot};
    let mut app = headless();
    stocked(&mut app);
    app.update();
    to_decks(&mut app);
    press(&mut app, Press::Decks(DecksPress::Edit(0)));
    let revision = |version| Revision {
        version,
        summary: None,
        superseded_at: 100,
        cards: 1,
        sideboard: 0,
        delta: None,
        card_count: None,
    };
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        assert_eq!(state.lobby.screen(), &Screen::Build);
        state.lobby.apply(LobbyEvent::DeckLoaded {
            id: "d1".to_string(),
            name: "Allytifact".to_string(),
            cards: vec!["4 Lightning Bolt (M11) 149 *F*".to_string()],
            sideboard: Vec::new(),
            commanders: Vec::new(),
        });
        assert!(state.lobby.browse_history().is_some());
        state.lobby.apply(LobbyEvent::Library(Reply::History(
            "d1".into(),
            History {
                version: 2,
                updated_at: 200,
                past: vec![revision(1)],
                delta: None,
                card_count: None,
            },
        )));
        for (version, row) in [
            (2, "4 Lightning Bolt (M11) 149 *F*"),
            (1, "4 Lightning Bolt (M11) 149"),
        ] {
            state.lobby.apply(LobbyEvent::Library(Reply::Version(
                "d1".into(),
                Snapshot {
                    version,
                    cards: vec![row.into()],
                    sideboard: vec![],
                    commanders: vec![],
                },
            )));
        }
    }
    app.update();
    let sheets = app
        .world_mut()
        .query::<&super::history::HistorySheet>()
        .iter(app.world())
        .count();
    assert_eq!(sheets, 1, "one history sheet");
    let rows = app
        .world_mut()
        .query::<&super::history::HistoryChange>()
        .iter(app.world())
        .count();
    assert_eq!(rows, 1, "a foil change is one row, never −4 and +4");
    let said = labels(&mut app);
    assert!(said.contains(&"v1 \u{2192} v2".to_string()), "{said:?}");
    assert!(said.contains(&"finish".to_string()), "{said:?}");
    assert!(
        presses(&mut app).contains(&Press::Build(BuildPress::CloseBuilder)),
        "the builder stands under the sheet"
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::Escape);
        keys.clear();
    }
    app.update();
    let state = app.world().resource::<LobbyState>();
    assert_eq!(
        state.lobby.screen(),
        &Screen::Build,
        "Esc stays in the builder"
    );
    assert!(state.lobby.library().page.is_none());
}
