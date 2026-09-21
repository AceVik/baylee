//! The lobby's tests: route mapping without a gateway, and node
//! decisions without a window.
//!
//! A test goes with **the screen it is about** — signing in, the table
//! list, the deck builder, printings, settings, the end screen — plus
//! `gateway` for what the client asks of the server and `frame` for the
//! chrome every screen sits in.
//!
//! Every non-test item stays here, because a child module reaches its
//! parent's private items through `use super::*` and a **sibling** reaches
//! nothing at all.

mod builder;
mod end_screen;
mod frame;
mod gateway;
mod printings;
mod settings;
mod sign_in;
mod tables;

#[allow(clippy::wildcard_imports)]
use super::*;

#[allow(clippy::wildcard_imports)]
use super::{http::*, preview::*, systems::*, ui::*};

use baylee_client_core::lobby::{
    DeckSummary, GameListing, GameQuery, GameSeat, GameSummary, SeatHandover,
};

fn body(request: &ehttp::Request) -> serde_json::Value {
    serde_json::from_slice(&request.body).expect("a JSON body")
}

fn answer(status: u16, body: &str) -> ehttp::Response {
    ehttp::Response {
        url: "http://gw/".to_string(),
        ok: (200..300).contains(&status),
        status,
        status_text: String::new(),
        headers: ehttp::Headers::new(&[]),
        bytes: body.as_bytes().to_vec(),
    }
}

/// A headless app wired exactly as the plugin wires a real one. No
/// renderer, so this exercises the systems and the node tree, not pixels.
fn headless() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .init_state::<DuelPhase>()
        .add_message::<DuelCommand>()
        .add_message::<KeyboardInput>()
        .add_message::<Pointer<Click>>()
        .add_message::<Pointer<Scroll>>()
        .add_message::<Pointer<Drag>>()
        .add_message::<Pointer<DragEnd>>()
        // The duel plugin's startup system would load these; a test has
        // no asset server and does not need one to build a tree.
        .insert_resource(UiFonts {
            text: Handle::default(),
            medium: Handle::default(),
            bold: Handle::default(),
            italic: Handle::default(),
            medium_italic: Handle::default(),
            serif: Handle::default(),
            serif_italic: Handle::default(),
            icons: Handle::default(),
            mana: Handle::default(),
        })
        // `MinimalPlugins` brings no `InputPlugin`, and `leave_keys` reads the
        // key state the way every other handler does. The resource alone and
        // not the plugin: the plugin clears `just_pressed` in `PreUpdate`, so
        // a key pressed by a test would be gone before `Update` ran.
        .init_resource::<ButtonInput<KeyCode>>()
        .add_plugins(LobbyPlugin);
    // The startup probe asks a gateway whether sign-ups are open. Left
    // pointing at the default address it reaches a gateway that happens to
    // be running on this machine, and that answer lands a frame or two
    // later — inside whatever the test is measuring. An address no request
    // can be built from keeps a headless test off the network entirely.
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.gateway.clear();
        state.gateway_selected = true;
        state.lobby.set_gateway_ready(true);
        state.lobby.set_registration_enabled(true);
    }
    app.update();
    app
}

fn presses(app: &mut App) -> Vec<Press> {
    let mut query = app.world_mut().query::<&Press>();
    let mut found: Vec<Press> = query.iter(app.world()).copied().collect();
    found.sort_by_key(|p| format!("{p:?}"));
    found
}

fn roots(app: &mut App) -> Vec<Entity> {
    let mut query = app.world_mut().query_filtered::<Entity, With<LobbyRoot>>();
    query.iter(app.world()).collect()
}

fn typed(ch: char) -> KeyboardInput {
    KeyboardInput {
        key_code: KeyCode::KeyA,
        logical_key: Key::Character(ch.to_string().into()),
        state: bevy::input::ButtonState::Pressed,
        text: Some(ch.to_string().into()),
        repeat: false,
        window: Entity::PLACEHOLDER,
    }
}

/// A named key with no text of its own, for the ones a text field answers.
fn pressed(key_code: KeyCode, logical_key: Key) -> KeyboardInput {
    KeyboardInput {
        key_code,
        logical_key,
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    }
}

/// Finds the control carrying a press this predicate accepts, and taps it.
///
/// By what the press *is*, not by which entity holds it: a room's buttons
/// carry the row they belong to and the chair they point at, and a test that
/// went looking for an entity would be asserting about the tree rather than
/// about the control.
fn tap_control(app: &mut App, what: &str, pick: impl Fn(&Press) -> bool) {
    let entity = {
        let mut query = app.world_mut().query::<(Entity, &Press)>();
        let found: Vec<(Entity, Press)> = query
            .iter(app.world())
            .map(|(e, p)| (e, *p))
            .filter(|(_, p)| pick(p))
            .collect();
        assert_eq!(found.len(), 1, "controls that are {what}: {found:?}");
        found[0].0
    };
    tap(app, entity);
    // The mailbox is drained one frame at a time and an answer chains into
    // the next request, so one press settles over several frames.
    for _ in 0..6 {
        app.update();
    }
}

fn labels(app: &mut App) -> Vec<String> {
    let mut query = app.world_mut().query::<&Text>();
    query.iter(app.world()).map(|t| t.0.clone()).collect()
}

/// Moves the app into a phase and lets that transition's systems run.
fn phase(app: &mut App, next: DuelPhase) {
    app.world_mut()
        .resource_mut::<NextState<DuelPhase>>()
        .set(next);
    app.update();
}

/// A window of a given width, so the breakpoints can be exercised without
/// a windowing system.
fn sized(app: &mut App, width: f32) {
    let mut existing = app.world_mut().query::<&mut Window>();
    if let Some(mut window) = existing.iter_mut(app.world_mut()).next() {
        window.resolution.set(width, 900.0);
        return;
    }
    let mut window = Window::default();
    window.resolution.set(width, 900.0);
    app.world_mut().spawn(window);
}

/// Two cards, in the shape `GET /pool` sends them.
fn pool_cards() -> Vec<baylee_client_core::PoolCard> {
    serde_json::from_value(serde_json::json!([
        {
            "index": 1,
            "name": "Llanowar Elves",
            "english_name": "Llanowar Elves",
            "mana_cost": "{G}",
            "cmc": 1,
            "colors": "G",
            "identity": "G",
            "type_line": "Creature — Elf Druid",
            "kinds": ["Creature"],
            "stats": "1/1",
            "oracle_text": "{T}: Add {G}.",
            "coverage": "implemented",
            "note": null,
            "commander": false,
            "basic_land": false
        },
        {
            "index": 2,
            "name": "Forest",
            "english_name": "Forest",
            "mana_cost": "",
            "cmc": 0,
            "colors": "",
            "identity": "G",
            "type_line": "Basic Land — Forest",
            "kinds": ["Land"],
            "stats": null,
            "oracle_text": "",
            "coverage": "implemented",
            "note": null,
            "commander": false,
            "basic_land": true
        }
    ]))
    .expect("the pool shape")
}

/// A lobby signed in, with a deck listed and the pool loaded.
fn stocked(app: &mut App) {
    let mut state = app.world_mut().resource_mut::<LobbyState>();
    state.lobby.apply(LobbyEvent::LoggedIn {
        token: "tok".to_string(),
    });
    state.lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
        id: "d1".to_string(),
        name: "Allytifact".to_string(),
        cards: 96,
        sideboard: 0,
        commanders: Vec::new(),
    }]));
    state.lobby.apply(LobbyEvent::Pool {
        cards: pool_cards(),
        has_text: true,
    });
}

/// Presses one control by name, in one line.
fn press(app: &mut App, wanted: Press) {
    let target = press_target(app, wanted);
    tap(app, target);
    app.update();
}

/// The entity carrying a control, so a test can press it.
fn press_target(app: &mut App, wanted: Press) -> Entity {
    let mut query = app.world_mut().query::<(Entity, &Press)>();
    let found = query.iter(app.world()).find(|(_, press)| **press == wanted);
    match found {
        Some((entity, _)) => entity,
        None => panic!("{wanted:?} is on screen"),
    }
}

/// A plain press on one control.
fn tap(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<Messages<Pointer<Click>>>()
        .write(aimed(
            entity,
            Click {
                button: PointerButton::Primary,
                hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                duration: std::time::Duration::ZERO,
                count: 1,
            },
        ));
}

/// A pointer event aimed at one entity. The location is required and
/// never read by anything the lobby runs.
fn aimed<E: std::fmt::Debug + Clone + Reflect>(entity: Entity, event: E) -> Pointer<E> {
    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::pointer::{Location, PointerId};
    use bevy::window::WindowRef;
    Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::Window(
                WindowRef::Primary
                    .normalize(Some(Entity::PLACEHOLDER))
                    .expect("a window reference"),
            ),
            position: Vec2::ZERO,
        },
        event,
        entity,
    )
}

/// What a field's box holds, in order: each run of text as itself, and the
/// caret as a bar.
///
/// Reading the *children* rather than a string is the whole point — the caret
/// is a node between two runs now, and where it sits in that row is where it
/// is drawn on screen.
fn drawn_field(app: &mut App, field: Field) -> Vec<String> {
    let mut boxes = app.world_mut().query::<(&Press, &Children)>();
    let kids: Vec<Entity> = boxes
        .iter(app.world())
        .find(|(press, _)| **press == Press::Focus(field))
        .map(|(_, children)| children.iter().collect())
        .unwrap_or_default();
    kids.into_iter()
        .filter_map(|kid| {
            if app.world().get::<Caret>(kid).is_some() {
                Some("|".to_string())
            } else {
                // The spacer and the eye at the end of a password box carry
                // neither a `Text` nor a `Caret`, and are not what this reads.
                app.world().get::<Text>(kid).map(|text| text.0.clone())
            }
        })
        .collect()
}

/// How many runs on screen carry the selection fill.
fn fills(app: &mut App) -> usize {
    let mut query = app.world_mut().query::<&BackgroundColor>();
    query
        .iter(app.world())
        .filter(|fill| fill.0 == palette::SELECTION)
        .count()
}
