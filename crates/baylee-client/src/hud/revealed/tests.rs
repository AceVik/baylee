//! The sheet that holds up another seat's reveal: it stands for the reveal
//! and with every card in it, says whose it is, stands under a dialog that
//! answers a question, stands where the decision sheet does, and goes only
//! when the player puts it away: never by itself.

use super::*;
use baylee_client_core::test_support::{hear_live, revealed_line, statics};
use std::time::Duration;

fn fonts() -> UiFonts {
    UiFonts {
        text: default(),
        medium: default(),
        bold: default(),
        italic: default(),
        medium_italic: default(),
        serif: default(),
        serif_italic: default(),
        icons: default(),
        mana: default(),
    }
}

/// The overlay's root, the clock and the sheet's two systems, over a duel
/// with a print table.
fn table() -> App {
    bevy::tasks::IoTaskPool::get_or_init(Default::default);
    let mut app = App::new();
    app.add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Image>();
    let textures = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        CardTextures::new(&mut images, 1 << 20)
    };
    let duel = Duel {
        statics: Some(statics(16)),
        ..Default::default()
    };
    app.insert_resource(textures)
        .insert_resource(duel)
        .insert_resource(fonts())
        .init_resource::<crate::settings::ClientSettings>()
        .init_resource::<RevealRevision>()
        .init_resource::<Time>()
        .add_systems(Update, sync);
    app.world_mut().spawn((HudRoot, Node::default()));
    app
}

fn reveal(app: &mut App, lines: Vec<baylee_view::LogEntry>) {
    let mut duel = app.world_mut().resource_mut::<Duel>();
    let duel = &mut *duel;
    hear_live(&mut duel.log, &mut duel.reveals, lines);
}

fn sheets(app: &mut App) -> Vec<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<RevealSheet>>();
    q.iter(app.world()).collect()
}

fn cards(app: &mut App) -> usize {
    let mut q = app.world_mut().query_filtered::<(), With<RevealCard>>();
    q.iter(app.world()).count()
}

fn words(app: &mut App) -> Vec<String> {
    let mut q = app.world_mut().query::<&Text>();
    q.iter(app.world()).map(|t| t.0.clone()).collect()
}

#[test]
fn the_sheet_stands_for_another_seat_s_reveal_with_every_card_and_its_name() {
    let mut app = table();
    app.update();
    assert!(sheets(&mut app).is_empty(), "no reveal, no sheet");

    let mut line = revealed_line(1, 40, 7);
    if let baylee_view::LogEvent::Revealed { cards, .. } = &mut line.event {
        cards.extend(revealed_line(1, 41, 8).event.objects().cloned());
    }
    reveal(&mut app, vec![line]);
    app.update();
    let standing = sheets(&mut app);
    assert_eq!(standing.len(), 1, "the reveal stood a sheet up");
    assert_eq!(cards(&mut app), 2, "every card the line named is on it");
    assert!(
        words(&mut app).iter().any(|w| w.ends_with("reveals")),
        "the head says whose reveal it is: {:?}",
        words(&mut app)
    );
    let root = app
        .world_mut()
        .query_filtered::<Entity, With<HudRoot>>()
        .single(app.world())
        .unwrap();
    assert_eq!(
        app.world().get::<ChildOf>(standing[0]).map(ChildOf::parent),
        Some(root),
        "it hangs off the overlay's root, so it leaves with the table"
    );

    // Frames that change nothing leave the sheet that is standing alone.
    app.update();
    assert_eq!(
        sheets(&mut app),
        standing,
        "a quiet frame rebuilt the sheet"
    );
}

#[test]
fn the_sheet_stands_under_a_dialog_answering_a_question_and_under_the_preview() {
    let mut app = table();
    reveal(&mut app, vec![revealed_line(1, 40, 7)]);
    app.update();
    let sheet = sheets(&mut app)[0];
    let rung = app.world().get::<ZIndex>(sheet).map(|z| z.0);
    assert_eq!(rung, Some(Z_LOG), "a sheet only showing the game");
    assert!(
        app.world().get::<Pickable>(sheet) == Some(&Pickable::IGNORE),
        "the band across the window lets the pointer through to the table"
    );
}

#[test]
fn the_sheet_goes_when_the_reveal_is_put_away_and_the_next_one_takes_its_place() {
    let mut app = table();
    reveal(
        &mut app,
        vec![revealed_line(1, 40, 7), revealed_line(1, 41, 8)],
    );
    app.update();
    let first = sheets(&mut app);
    assert_eq!(first.len(), 1);
    assert!(
        words(&mut app).iter().any(|w| w.contains("1 more")),
        "the head says one more waits: {:?}",
        words(&mut app)
    );
    app.world_mut().resource_mut::<Duel>().reveals.dismiss();
    app.update();
    let second = sheets(&mut app);
    assert_eq!(second.len(), 1, "the waiting reveal stood up");
    assert_ne!(second, first, "and it is a sheet of its own");
    app.world_mut().resource_mut::<Duel>().reveals.dismiss();
    app.update();
    assert!(sheets(&mut app).is_empty(), "the sheet outlived its reveal");
}

#[test]
fn the_sheet_never_goes_by_itself_however_long_it_stands() {
    let mut app = table();
    reveal(&mut app, vec![revealed_line(1, 40, 7)]);
    app.update();
    let standing = sheets(&mut app);
    assert_eq!(standing.len(), 1);
    // A minute of game time and many frames: the old clock let it go at 7 s.
    for _ in 0..60 {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs(1));
        app.update();
    }
    assert_eq!(
        sheets(&mut app),
        standing,
        "the reveal closed or was rebuilt without the player's answer"
    );
    // The player's answer is what takes it away.
    app.world_mut().resource_mut::<Duel>().reveals.dismiss();
    app.update();
    assert!(sheets(&mut app).is_empty(), "the answer did not close it");
}

#[test]
fn the_sheet_stands_in_the_decision_area_over_the_shelf_open_at_its_foot() {
    let mut app = table();
    reveal(&mut app, vec![revealed_line(1, 40, 7)]);
    app.update();
    let band = sheets(&mut app)[0];
    let node = app.world().get::<Node>(band).expect("a node").clone();
    let drawer = crate::hud::ledge::drawer::root_node();
    assert_eq!(node.position_type, PositionType::Absolute);
    assert_eq!(
        node.bottom, drawer.bottom,
        "where the decision sheet stands"
    );
    assert_eq!(node.top, Val::Auto, "no longer hung from the top");
    let page = app.world().get::<Children>(band).expect("its paper")[0];
    let paper = app.world().get::<Node>(page).expect("a node");
    assert_eq!(
        paper.border.bottom,
        px(0),
        "open at the foot, as the sheet is"
    );
}

#[test]
fn nothing_is_held_up_over_the_end_screen() {
    let mut app = table();
    reveal(&mut app, vec![revealed_line(1, 40, 7)]);
    app.update();
    assert_eq!(sheets(&mut app).len(), 1);
    app.world_mut().resource_mut::<Duel>().interaction =
        Some(baylee_client_core::Interaction::new(
            baylee_engine::choice::Pending::GameOver(baylee_engine::win::GameResult {
                winner: None,
                reason: baylee_engine::win::EndReason::LastPlayerStanding,
            }),
            baylee_core::ids::PlayerId::new(0),
        ));
    app.update();
    assert!(
        sheets(&mut app).is_empty(),
        "the end screen shows the whole log, this reveal's line too"
    );
}

#[test]
fn the_cards_fit_any_window_the_client_supports() {
    for window in [
        Vec2::new(1920.0, 1080.0),
        Vec2::new(1280.0, 720.0),
        Vec2::new(844.0, 390.0),
    ] {
        let (across, down) = room(window);
        assert!(across > 0.0 && down > 0.0, "{window}: no room at all");
        let (width, _) = fit(3, (across, down), GAP, WIDEST);
        assert!(width > 0.0 && width <= WIDEST, "{window}: {width}");
    }
}

fn count<C: Component>(app: &mut App) -> usize {
    let mut q = app.world_mut().query_filtered::<(), With<C>>();
    q.iter(app.world()).count()
}

fn actions(app: &mut App) -> Vec<MenuAction> {
    let mut q = app.world_mut().query::<&super::super::MenuButton>();
    q.iter(app.world()).map(|b| b.action).collect()
}

/// The decision sheet's parts (the owner, 08.10.2026): a head with the fold
/// and the close cross, a foot with the close answer, and on every card
/// the log's own link, which is what opens the table's preview of it.
#[test]
fn a_reveal_is_the_sheet_with_its_fold_its_close_and_a_preview_on_every_card() {
    let mut app = table();
    let mut line = revealed_line(1, 40, 7);
    if let baylee_view::LogEvent::Revealed { cards, .. } = &mut line.event {
        cards.extend(revealed_line(1, 41, 8).event.objects().cloned());
    }
    reveal(&mut app, vec![line]);
    app.update();
    let said = actions(&mut app);
    assert_eq!(
        said.iter()
            .filter(|a| **a == MenuAction::FoldReveal)
            .count(),
        1,
        "one fold: {said:?}"
    );
    assert_eq!(
        said.iter()
            .filter(|a| **a == MenuAction::DismissReveal)
            .count(),
        2,
        "the head's cross and the foot's answer: {said:?}"
    );
    let links: Vec<Entity> = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, (With<LogLink>, With<Node>)>();
        q.iter(app.world()).collect()
    };
    assert_eq!(links.len(), 2, "a link on each card");
    for link in links {
        let parent = app.world().get::<ChildOf>(link).map(ChildOf::parent);
        assert!(
            parent.is_some_and(|p| app.world().get::<RevealCard>(p).is_some()),
            "the link lies on its card"
        );
        assert_ne!(
            app.world().get::<Pickable>(link),
            Some(&Pickable::IGNORE),
            "and takes the pointer, so the preview opens"
        );
    }
}

/// Folded, the reveal is a pill and nothing else of it stands; the pill
/// opens it again; the next reveal stands up open. Its time runs either way.
#[test]
fn a_folded_reveal_is_a_pill_and_the_next_one_opens() {
    let mut app = table();
    reveal(
        &mut app,
        vec![revealed_line(1, 40, 7), revealed_line(1, 41, 8)],
    );
    app.update();
    assert_eq!(count::<RevealCard>(&mut app), 1);
    let fold = |app: &mut App| {
        let mut duel = app.world_mut().resource_mut::<Duel>();
        let number = duel.reveals.current().map(|r| r.number);
        duel.reveal_fold.toggle(number);
    };
    fold(&mut app);
    app.update();
    assert_eq!(count::<RevealPill>(&mut app), 1, "folded to its pill");
    assert_eq!(count::<RevealCard>(&mut app), 0, "and no card stands");
    fold(&mut app);
    app.update();
    assert_eq!(count::<RevealPill>(&mut app), 0, "opened again");
    assert_eq!(count::<RevealCard>(&mut app), 1);

    fold(&mut app);
    app.update();
    app.world_mut().resource_mut::<Duel>().reveals.dismiss();
    app.update();
    assert_eq!(count::<RevealPill>(&mut app), 0, "the next reveal is open");
    assert_eq!(count::<RevealCard>(&mut app), 1);

    fold(&mut app);
    app.update();
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs(60));
    app.update();
    assert_eq!(
        count::<RevealPill>(&mut app),
        1,
        "a folded reveal never times out"
    );
}
