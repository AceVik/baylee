//! What a press reaches, one question per file.
//!
//! # Where a test sits
//!
//! A test goes with **the thing the key or the finger is aimed at**: the
//! card tray is `tray`, the ability menu is `menu`, a dialog's own keys are
//! `dialog_keys`, what the cursor is over is `hover`, what a tap does is
//! `taps`, arming and what it swallows is `arming`, picking a target is
//! `targets`, declaring an attack or a block is `combat`, and what reaches
//! the keymap at all is `keyboard`.
//!
//! # What the harnesses here do not supply
//!
//! The `Duel`s below are struct literals and set `view`, `interaction` and
//! the pointer state. They do **not** set `reachable` or `suspend_reach`, the
//! two sets `rebuild_board` fills, so the branches in `input.rs` that read
//! them — the offer to go and tap lands for a spell — are never taken from
//! here. That is deliberate rather than an oversight: `arming` is the file
//! that covers those, and it is the one harness in this tree that goes in
//! through [`crate::Duel::receive_view`] and `rebuild_board`. Anything added
//! here whose claim depends on a reach set belongs there instead, or it will
//! be asking a question of an empty set and getting a legal answer.
//!
//! # What stays here, and why it has to
//!
//! Every non-test item — the Bevy app builders, `finger_down`, `finger_up`,
//! `finger_click`, `pointer_at` — stays in this file. A child module reaches
//! its parent's private items through `use super::*`, and a **sibling**
//! reaches nothing at all, so a helper that moved into one part would be
//! invisible to the other seven.
//!
//! The parent's own items are *imported* rather than spelled `super::…` at
//! every call: once a test sits one level deeper, `super::` no longer means
//! what it meant, and a name the parent happens to define itself would
//! silently shadow the glob. An import says which module is meant, once.

mod arming;
mod combat;
mod dialog_keys;
mod hover;
mod keyboard;
mod menu;
mod taps;
mod targets;
mod tray;

use super::{
    activate_card, answer_the_question, armed_keys, arrange_keys, browser_answer_keys,
    browser_takes_the_keyboard, cursor_grid, keyboard, menu_click, move_cursor, pointer,
    pointer_hover, the_click,
};

use baylee_client_core::interaction::Interaction;

use baylee_core::ids::{ObjectId, PlayerId};

use baylee_engine::choice::{LegalActions, Pending, PlayerAction};

fn obj(slot: u32) -> ObjectId {
    ObjectId::new(slot, 0)
}

/// Presses a key at the real system and answers the outbox.
///
/// Shares [`the_primary_key_plays_the_land_under_the_cursor`]'s shape
/// rather than its body: the point of both is that nothing is hand-built
/// between a `KeyCode` and a `PlayerAction`.
fn pressing(pending: Pending, key: bevy::prelude::KeyCode) -> Vec<PlayerAction> {
    use bevy::input::ButtonInput;
    use bevy::input::keyboard::KeyboardInput;
    use bevy::prelude::*;

    let duel = crate::Duel {
        interaction: Some(baylee_client_core::interaction::Interaction::new(
            pending,
            PlayerId::new(0),
        )),
        ..Default::default()
    };

    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<KeyboardInput>()
        .insert_resource(duel)
        .add_systems(Update, keyboard);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
    app.world().resource::<crate::Duel>().outbox().to_vec()
}

/// The finger, the hand row and the real `pointer`, with a land to play.
///
/// Everything a tap travels through except the tree that gets rebuilt
/// underneath it — which is the point: the rebuild is done by hand in the
/// tests below, because that is what the client does to itself on every
/// hover change and every arriving view.
fn hand_app() -> bevy::app::App {
    use bevy::picking::events::{Click, Pointer, Press, Release};
    use bevy::prelude::*;

    // Spelled out, because `bevy::prelude` brings a `bevy_ui::Interaction`
    // of its own and shadows the one this client means.
    let duel = crate::Duel {
        interaction: Some(baylee_client_core::interaction::Interaction::new(
            Pending::Priority {
                player: PlayerId::new(0),
                legal: Box::new(LegalActions {
                    can_pass: true,
                    lands: vec![obj(3)],
                    castable: vec![],
                    mana_abilities: vec![],
                    abilities: vec![],
                    suspendable: vec![],
                }),
            },
            PlayerId::new(0),
        )),
        ..Default::default()
    };

    let mut app = App::new();
    app.init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::touch::Touched>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<Pointer<Press>>()
        .add_message::<Pointer<Release>>()
        .add_message::<Pointer<Click>>()
        .insert_resource(duel)
        // `browser_click` reaches `TrayWidgets`, which holds the maximise
        // button's flight as a `ResMut`. A resource that is not there is a
        // panic on the first click at anything, so it is the harness's
        // business and not one test's.
        .init_resource::<crate::input::TrayGlide>()
        .add_systems(Update, (crate::touch::watch_the_finger, pointer).chain());
    let mut window = Window::default();
    window.resolution.set(1728.0, 1052.0);
    app.world_mut().spawn((window, bevy::window::PrimaryWindow));
    app
}

/// One node in the hand row, as `spawn_hand_zone` builds one.
///
/// Both components, because the pair is what the two systems ask for:
/// the wider one is what a press and a click resolve through, the marker
/// is what says this node is the row's and not the stack panel's.
fn row_card(app: &mut bevy::app::App, object: ObjectId) -> bevy::prelude::Entity {
    app.world_mut()
        .spawn((
            crate::hud::HandCardVisual { object },
            crate::hud::HandRowCard,
        ))
        .id()
}

/// Where the pointer is, as the picking backend would report it.
fn pointer_at(app: &mut bevy::app::App) -> bevy::picking::pointer::Location {
    use bevy::camera::NormalizedRenderTarget;
    use bevy::prelude::*;
    use bevy::window::{PrimaryWindow, WindowRef};

    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("the harness made a window");
    let target = WindowRef::Entity(window)
        .normalize(Some(window))
        .expect("a window is a render target");
    bevy::picking::pointer::Location {
        target: NormalizedRenderTarget::Window(target),
        position: Vec2::ZERO,
    }
}

fn finger_down(app: &mut bevy::app::App, entity: bevy::prelude::Entity) {
    use bevy::picking::events::{Pointer, Press};
    use bevy::picking::pointer::PointerId;

    let location = pointer_at(app);
    let event = Press {
        button: bevy::picking::pointer::PointerButton::Primary,
        hit: bevy::picking::backend::HitData::new(entity, 0.0, None, None),
        count: 1,
    };
    app.world_mut()
        .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
}

fn finger_up(app: &mut bevy::app::App, entity: bevy::prelude::Entity) {
    use bevy::picking::events::{Pointer, Release};
    use bevy::picking::pointer::PointerId;

    let location = pointer_at(app);
    let event = Release {
        button: bevy::picking::pointer::PointerButton::Primary,
        hit: bevy::picking::backend::HitData::new(entity, 0.0, None, None),
    };
    app.world_mut()
        .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
}

/// The click bevy raises when the press and the release agree on an
/// entity — the half that goes missing when the tree is rebuilt.
///
/// Named for the finger rather than for the event, so that it does not
/// shadow the `the_click` system it is here to exercise.
fn finger_click(app: &mut bevy::app::App, entity: bevy::prelude::Entity) {
    use bevy::picking::events::{Click, Pointer};
    use bevy::picking::pointer::PointerId;

    let location = pointer_at(app);
    let event = Click {
        button: bevy::picking::pointer::PointerButton::Primary,
        hit: bevy::picking::backend::HitData::new(entity, 0.0, None, None),
        duration: std::time::Duration::from_millis(10),
        count: 1,
    };
    app.world_mut()
        .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
}

/// Builds the app the two menu-button tests share: the real `pointer`
/// system, and a click helper that goes through the real message.
fn menu_app(duel: crate::Duel) -> (bevy::app::App, bevy::prelude::Entity, bevy::prelude::Entity) {
    use crate::hud::MenuAction;
    use bevy::prelude::*;

    let mut app = App::new();
    app.init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::touch::Touched>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Click>>()
        .insert_resource(duel)
        .init_resource::<crate::input::TrayGlide>()
        .add_systems(Update, pointer);
    let draw = app
        .world_mut()
        .spawn(crate::hud::MenuButton {
            action: MenuAction::OfferDraw,
        })
        .id();
    let concede = app
        .world_mut()
        .spawn(crate::hud::MenuButton {
            action: MenuAction::Concede,
        })
        .id();
    (app, draw, concede)
}

/// One click on one entity, as the picking backend would report it.
fn click(app: &mut bevy::app::App, entity: bevy::prelude::Entity) {
    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::events::{Click, Pointer};
    use bevy::picking::pointer::{Location, PointerId};
    use bevy::prelude::*;
    use bevy::window::{PrimaryWindow, WindowRef};

    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .unwrap_or_else(|_| {
            app.world_mut()
                .spawn((Window::default(), PrimaryWindow))
                .id()
        });
    let camera = app.world_mut().spawn_empty().id();
    let target = WindowRef::Entity(window)
        .normalize(Some(window))
        .expect("a window is a render target");
    let location = Location {
        target: NormalizedRenderTarget::Window(target),
        position: Vec2::ZERO,
    };
    let event = Click {
        button: bevy::picking::pointer::PointerButton::Primary,
        hit: bevy::picking::backend::HitData::new(camera, 0.0, None, None),
        duration: std::time::Duration::from_millis(10),
        count: 1,
    };
    app.world_mut()
        .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
    app.update();
}

/// An app with `pointer_hover` and one permanent on the table, seen from
/// a seat that also has a graveyard to lose it into.
fn hover_app(view: baylee_view::PlayerView) -> (bevy::app::App, bevy::prelude::Entity) {
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_message::<bevy::picking::events::Pointer<bevy::picking::events::Over>>()
        .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Out>>()
        .add_message::<bevy::window::CursorMoved>()
        .insert_resource(crate::Duel {
            view: Some(view),
            ..crate::Duel::default()
        })
        .add_systems(Update, pointer_hover);
    let card = app
        .world_mut()
        .spawn(crate::table::CardVisual {
            object: obj(9),
            count: 1,
        })
        .id();
    (app, card)
}

/// One pointer entering one entity, as the picking backend would report
/// it. The cursor move is what opens the system's grace window: a still
/// pointer is deliberately silent.
fn hover(app: &mut bevy::app::App, entity: bevy::prelude::Entity) {
    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::events::{Over, Pointer};
    use bevy::picking::pointer::{Location, PointerId};
    use bevy::prelude::*;
    use bevy::window::{PrimaryWindow, WindowRef};

    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .unwrap_or_else(|_| {
            app.world_mut()
                .spawn((Window::default(), PrimaryWindow))
                .id()
        });
    let camera = app.world_mut().spawn_empty().id();
    let target = WindowRef::Entity(window)
        .normalize(Some(window))
        .expect("a window is a render target");
    let location = Location {
        target: NormalizedRenderTarget::Window(target),
        position: Vec2::ZERO,
    };
    app.world_mut().write_message(bevy::window::CursorMoved {
        window,
        position: Vec2::ZERO,
        delta: None,
    });
    app.world_mut().write_message(Pointer::new(
        PointerId::Mouse,
        location,
        Over {
            hit: bevy::picking::backend::HitData::new(camera, 0.0, None, None),
        },
        entity,
    ));
    app.update();
}

/// A number choice with a range, ready to be typed at.
fn number_duel(max: u32) -> crate::Duel {
    crate::Duel {
        interaction: Some(baylee_client_core::interaction::Interaction::new(
            Pending::ChooseNumber {
                player: PlayerId::new(0),
                min: 0,
                max,
            },
            PlayerId::new(0),
        )),
        ..Default::default()
    }
}

/// An app holding the key path, with the zone dialog one frame past
/// opening.
///
/// That frame matters and is why it is spent here: in the client it is
/// the frame `G` was pressed on, so it is also the frame whose keystroke
/// the box must not take. Letters typed after it are the player's *next*
/// ones, which is what a real keyboard produces.
fn a_zone_dialog_that_has_just_opened() -> (bevy::prelude::App, bevy::prelude::Entity) {
    use bevy::input::ButtonInput;
    use bevy::input::keyboard::KeyboardInput;
    use bevy::prelude::*;

    let mut app = App::new();
    let mut duel = crate::Duel::default();
    duel.browser.open();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<KeyboardInput>()
        .init_resource::<Keystrokes>()
        .insert_resource(duel)
        .add_systems(PreUpdate, deliver_keystrokes)
        .add_systems(
            Update,
            (browser_takes_the_keyboard.before(keyboard), keyboard),
        );
    let window = app.world_mut().spawn_empty().id();
    app.update();
    (app, window)
}

/// Keystrokes waiting to be written *inside* a frame.
///
/// A message written before `App::update` is not the same message a
/// keyboard writes, and the difference is exactly the one under test.
/// `Messages::update` runs in `First`, so a write from outside the
/// schedule is already one swap old when `Update` sees it and is dropped
/// at the start of the next frame — it lives one frame, not two. A real
/// keystroke is written by `bevy_input` in `PreUpdate`, *after* that swap,
/// so it is still readable on the following frame. That following frame is
/// the frame the filter box takes the keyboard on, which is the whole
/// reason `keyboard` clears its reader there.
///
/// Writing from outside hid that: with the guard commented out the test
/// still passed, because the character the box would have eaten had
/// already expired.
#[derive(bevy::prelude::Resource, Default)]
struct Keystrokes(Vec<bevy::input::keyboard::KeyboardInput>);

/// `bevy_input`'s half, in the one place it matters.
fn deliver_keystrokes(
    mut queued: bevy::prelude::ResMut<Keystrokes>,
    mut out: bevy::prelude::MessageWriter<bevy::input::keyboard::KeyboardInput>,
) {
    for key in queued.0.drain(..) {
        out.write(key);
    }
}

/// One letter, pressed the way a real keyboard presses it.
///
/// A letter is two things at once — a character for a text box and a
/// bound action for the game — and a real press sends both, so this does
/// too. Nothing in these apps clears `ButtonInput` between frames (that
/// is `bevy_input`'s own system, and they have none), so a press has to
/// be released by hand or every later frame sees it held.
fn type_letter(
    app: &mut bevy::prelude::App,
    window: bevy::prelude::Entity,
    code: bevy::prelude::KeyCode,
    c: char,
) {
    use bevy::input::ButtonInput;
    use bevy::input::keyboard::{Key, KeyboardInput};
    use bevy::prelude::*;

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(code);
    app.world_mut()
        .resource_mut::<Keystrokes>()
        .0
        .push(KeyboardInput {
            key_code: code,
            logical_key: Key::Character(c.to_string().into()),
            state: bevy::input::ButtonState::Pressed,
            text: Some(c.to_string().into()),
            repeat: false,
            window,
        });
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
}

fn filter_reads(app: &bevy::prelude::App) -> String {
    app.world()
        .resource::<crate::Duel>()
        .browser
        .filter()
        .to_string()
}

fn panel_stands(app: &bevy::prelude::App) -> bool {
    app.world().resource::<crate::Duel>().browser.is_open()
}

/// A duel driven to seat 0's first main phase, out of a real `LocalHost`.
///
/// Every one of the arming tests needs a `LegalActions` that actually
/// offers something, and a hand-built one offers whatever the test wanted
/// it to. This plays the opening the way the client does — keep the hand,
/// pass until the engine offers a land — and stops at the window where a
/// player has a decision to make.
fn duel_in_main_phase() -> (crate::Duel, crate::host::LocalHost) {
    use crate::host::{DuelHost, HostMessage, LocalHost};
    use baylee_engine::choice::Pending;

    let seat = PlayerId::new(0);
    let mut host = LocalHost::new(&crate::host::tests::duel_preset(), seat, &["You", "AI"])
        .expect("the preset makes a game");
    let mut duel = crate::Duel::default();
    for _ in 0..64 {
        for message in host.poll() {
            match message {
                HostMessage::Static(s) => duel.statics = Some(*s),
                HostMessage::View(v) => duel.view = Some(*v),
                HostMessage::Choice(p) => {
                    duel.interaction = Some(Interaction::new(*p, seat));
                }
                HostMessage::Failed(why) => panic!("the host refused: {why}"),
                HostMessage::Curtain => duel.curtain_up = true,
            }
        }
        match duel.interaction.as_ref().map(Interaction::pending) {
            Some(Pending::Mulligan { .. }) => host.submit(PlayerAction::MulliganKeep),
            Some(Pending::Priority { legal, .. }) if !legal.lands.is_empty() => {
                return (duel, host);
            }
            Some(Pending::Priority { .. }) => host.submit(PlayerAction::PassPriority),
            _ => break,
        }
    }
    panic!("the game never offered seat 0 a land to play");
}

/// The whole of arm-then-act: the first tap says nothing, the second
/// sends, and cancel leaves the wire empty.
///
/// A window offering one land, one spell, and one card that is both.
///
/// Synthetic rather than dealt, because what is under test is the click
/// path and a real deck cannot be relied on to hold a castable spell and a
/// modal double-faced land in the same opening hand.
fn window_with(lands: Vec<ObjectId>, castable: Vec<ObjectId>) -> crate::Duel {
    crate::Duel {
        interaction: Some(Interaction::new(
            Pending::Priority {
                player: PlayerId::new(0),
                legal: Box::new(LegalActions {
                    can_pass: true,
                    lands,
                    castable,
                    mana_abilities: vec![],
                    abilities: vec![],
                    suspendable: vec![],
                }),
            },
            PlayerId::new(0),
        )),
        ..Default::default()
    }
}

/// A duel sitting on a `ChooseCards { min: 0 }` with the dialog open on
/// it — a Solemn Simulacrum's search for a basic land, which is the
/// shape AE6 was seen in twice.
fn duel_searching(min: u8) -> crate::Duel {
    use baylee_engine::choice::ChoicePrompt;
    let mut duel = crate::Duel {
        interaction: Some(Interaction::new(
            Pending::ChooseCards {
                player: PlayerId::new(0),
                options: vec![obj(1), obj(2)],
                min,
                max: 1,
                prompt: ChoicePrompt::Generic,
            },
            PlayerId::new(0),
        )),
        ..Default::default()
    };
    // Through the real door: `follow` is what opens the sheet for a
    // question, and `answers_here` is false for a sheet opened any other
    // way — so poking the state would test a configuration the client
    // cannot reach.
    let view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    // Destructured so the two fields are borrowed apart: `Interaction`
    // is not `Clone`, and it should not become one for a test.
    let crate::Duel {
        browser,
        interaction,
        ..
    } = &mut duel;
    browser.follow(&view, interaction.as_ref());
    assert!(
        duel.browser.answers_here(duel.interaction.as_ref()),
        "the dialog is the surface holding the question"
    );
    duel
}

/// A duel whose standing question is "put these three back in any order",
/// with the sheet opened for it through the real door.
fn duel_arranging() -> crate::Duel {
    use baylee_engine::choice::{ArrangePile, ArrangePlace, ArrangePrompt};
    let mut duel = crate::Duel {
        interaction: Some(Interaction::new(
            Pending::Arrange {
                player: PlayerId::new(0),
                cards: vec![obj(1), obj(2), obj(3)],
                piles: vec![ArrangePile::all_of(ArrangePlace::LibraryTop, 3)],
                prompt: ArrangePrompt::Order,
            },
            PlayerId::new(0),
        )),
        ..Default::default()
    };
    let view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    let crate::Duel {
        browser,
        interaction,
        ..
    } = &mut duel;
    browser.follow(&view, interaction.as_ref());
    assert!(
        duel.browser.answers_here(duel.interaction.as_ref()),
        "the dialog is the surface holding the question"
    );
    duel
}

fn press(name: bevy::prelude::KeyCode) -> bevy::input::ButtonInput<bevy::prelude::KeyCode> {
    let mut keys = bevy::input::ButtonInput::default();
    keys.press(name);
    keys
}
