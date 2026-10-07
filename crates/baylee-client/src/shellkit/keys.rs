//! The shell's second keymap, wired (`KEYBOARD.md` §2, §3, §7.1).
//!
//! `baylee_client_core::shellkeys` decides: the keymap, the chord matching
//! by character or by code, and the pure resolver over the context stack.
//! This module feeds it: each key event becomes a [`KeyPress`], the stack
//! comes from [`ShellStack`] (which the screen on show writes, and the kit
//! narrows: a focused kit field, the `?` overlay), the keymap is the
//! account's (`Preferences.shell_keys`), and what resolves is written as a
//! [`ShellFired`] message for whoever owns the action's door.
//!
//! Never at the table: while a duel is up the resolver does not run at all
//! (`ShellLog::resolved` stays put, §9.8), so the table's keymap is alone.
//!
//! The kit answers the actions that are its own — the three text-size
//! chords and the overlay; the lobby answers the screen moves
//! (`lobby::shortcuts`).

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use baylee_client_core::shellkeys::{self, KeyPress, Learnt, ShellAction, Stack};

use super::TextSize;

/// What has the keys right now, as the screen on show says it.
///
/// The lobby writes it every frame in [`StackSystems`]; the kit then adds
/// what it knows better (a focused kit field, the overlay). `live` is false
/// where no shell screen is up (a duel, the arrival), and then nothing
/// resolves.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ShellStack {
    /// The resolver's stack.
    pub stack: Stack,
    /// A shell screen is on show.
    pub live: bool,
}

/// The systems that write [`ShellStack`]; the resolver runs after them.
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct StackSystems;

/// The resolver and the kit's own answers; whoever answers the rest of the
/// actions runs after it.
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct KeySystems;

/// A shell action a key asked for.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ShellFired(pub ShellAction);

/// What the resolver did, for `/state` and the tests: how many key events
/// it was asked about, and the latest actions it answered (at most 32).
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct ShellLog {
    /// Key events the resolver read (zero at the table, §9.8).
    pub resolved: u64,
    /// The latest actions, oldest first.
    pub fired: Vec<ShellAction>,
}

impl ShellLog {
    const KEPT: usize = 32;

    fn note(&mut self, action: ShellAction) {
        if self.fired.len() == Self::KEPT {
            self.fired.remove(0);
        }
        self.fired.push(action);
    }
}

/// What this session has seen keys produce (`KEYBOARD.md` §3.3): a code
/// chord's key cap shows the learnt character (`Cmd+Ü` for Back on a German
/// Mac once `BracketLeft` has typed `ü`).
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct LearntKeys(pub Learnt);

/// Whether chords take ⌘ (macOS) or Ctrl (everywhere else) as `command`.
#[must_use]
pub const fn mac() -> bool {
    cfg!(target_os = "macos")
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<ShellStack>()
        .init_resource::<ShellLog>()
        .init_resource::<LearntKeys>()
        .add_message::<ShellFired>()
        .configure_sets(Update, StackSystems.before(super::focus::FocusSystems))
        .add_systems(
            Update,
            (resolve_keys, size_the_text)
                .chain()
                .in_set(KeySystems)
                .after(super::focus::FocusSystems)
                .after(StackSystems),
        );
}

/// The physical key's name as the keymap spells it (`"Digit7"`).
#[must_use]
pub fn code_name(code: KeyCode) -> String {
    format!("{code:?}")
}

/// A key event and the modifiers held, as the resolver reads it.
#[must_use]
pub fn press_of(key: &KeyboardInput, codes: Option<&ButtonInput<KeyCode>>) -> KeyPress {
    let held = |pair: [KeyCode; 2]| codes.is_some_and(|c| c.any_pressed(pair));
    let ch = match &key.logical_key {
        Key::Character(text) => Some(text.to_string()),
        Key::Space => Some(" ".to_string()),
        _ => None,
    };
    KeyPress {
        code: code_name(key.key_code),
        ch,
        shift: held([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
        ctrl: held([KeyCode::ControlLeft, KeyCode::ControlRight]),
        alt: held([KeyCode::AltLeft, KeyCode::AltRight]),
        meta: held([KeyCode::SuperLeft, KeyCode::SuperRight]),
        repeat: key.repeat,
    }
}

/// Every key event against the shell's keymap, innermost layer first.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
fn resolve_keys(
    mut keys: MessageReader<KeyboardInput>,
    codes: Option<Res<ButtonInput<KeyCode>>>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    shell: Res<ShellStack>,
    focus: Res<bevy::input_focus::InputFocus>,
    fields: Query<(), With<super::focus::ShellField>>,
    overlay: Res<super::overlay::Overlay>,
    mut learnt: ResMut<LearntKeys>,
    mut log: ResMut<ShellLog>,
    mut fired: MessageWriter<ShellFired>,
) {
    if phase.is_some_and(|p| *p.get() != crate::DuelPhase::Closed) || !shell.live {
        keys.clear();
        return;
    }
    let mut stack = shell.stack;
    stack.field |= focus.get().is_some_and(|f| fields.contains(f));
    stack.modal |= overlay.open;
    let standard;
    let keymap = if let Some(prefs) = prefs.as_deref() {
        &prefs.all().shell_keys
    } else {
        standard = shellkeys::ShellKeymap::standard();
        &standard
    };
    for key in keys.read() {
        if key.state != ButtonState::Pressed {
            continue;
        }
        let press = press_of(key, codes.as_deref());
        // Written only when it learns something: the key caps redraw on it.
        let mut knows = learnt.0.clone();
        knows.learn(&press);
        if knows != learnt.0 {
            learnt.0 = knows;
        }
        log.bypass_change_detection().resolved += 1;
        if let Some(action) = shellkeys::resolve(keymap, &press, stack, mac()) {
            log.note(action);
            fired.write(ShellFired(action));
        }
    }
}

/// `Ctrl/Cmd + = − 0` step the shell's text size up, down and back (§8),
/// stored per device.
fn size_the_text(
    mut fired: MessageReader<ShellFired>,
    settings: Option<ResMut<crate::settings::ClientSettings>>,
) {
    let Some(mut settings) = settings else {
        fired.clear();
        return;
    };
    for ShellFired(action) in fired.read() {
        let next = match action {
            ShellAction::TextLarger => settings.text_size.larger(),
            ShellAction::TextSmaller => settings.text_size.smaller(),
            ShellAction::TextReset => TextSize::default(),
            _ => continue,
        };
        if next != settings.text_size {
            settings.text_size = next;
            settings.save();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::shellkeys::Context;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<crate::DuelPhase>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_message::<KeyboardInput>()
            .init_resource::<super::super::InputClass>()
            .insert_resource(crate::settings::ClientSettings::default());
        super::super::focus::install(&mut app);
        super::super::overlay::install(&mut app);
        install(&mut app);
        app.insert_resource(ShellStack {
            stack: Stack {
                screen: Some(Context::Play),
                ..Stack::default()
            },
            live: true,
        });
        app
    }

    fn send(app: &mut App, code: KeyCode, logical: Key, held: &[KeyCode]) {
        {
            let mut codes = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            codes.reset_all();
            for k in held {
                codes.press(*k);
            }
            codes.press(code);
        }
        let text = match &logical {
            Key::Character(t) => Some(t.clone()),
            _ => None,
        };
        app.world_mut().write_message(KeyboardInput {
            key_code: code,
            logical_key: logical,
            state: ButtonState::Pressed,
            text,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        app.update();
    }

    fn command() -> KeyCode {
        if mac() {
            KeyCode::SuperLeft
        } else {
            KeyCode::ControlLeft
        }
    }

    /// `Ctrl/Cmd + =` steps the text up through the resolver; the German
    /// layout's `=` (Shift+0) does too and does not reset; `Ctrl/Cmd + 0`
    /// resets; `−` steps down (§8, S4-8).
    #[test]
    fn the_command_chords_step_the_text_size() {
        let mut app = app();
        let size = |app: &App| {
            app.world()
                .resource::<crate::settings::ClientSettings>()
                .text_size
        };
        send(
            &mut app,
            KeyCode::Equal,
            Key::Character("=".into()),
            &[command()],
        );
        assert_eq!(size(&app), TextSize::Xl);
        send(
            &mut app,
            KeyCode::Minus,
            Key::Character("-".into()),
            &[command()],
        );
        send(
            &mut app,
            KeyCode::Minus,
            Key::Character("-".into()),
            &[command()],
        );
        assert_eq!(size(&app), TextSize::M);
        send(
            &mut app,
            KeyCode::Digit0,
            Key::Character("=".into()),
            &[command(), KeyCode::ShiftLeft],
        );
        assert_eq!(size(&app), TextSize::L, "German Shift+0 is = and steps up");
        send(
            &mut app,
            KeyCode::Minus,
            Key::Character("-".into()),
            &[command()],
        );
        send(
            &mut app,
            KeyCode::Digit0,
            Key::Character("0".into()),
            &[command()],
        );
        assert_eq!(size(&app), TextSize::L);
        // A bare `=` is nobody's.
        send(&mut app, KeyCode::Equal, Key::Character("=".into()), &[]);
        assert_eq!(size(&app), TextSize::L);
    }

    /// At the table the resolver does not run at all: the table's keymap is
    /// alone (`KEYBOARD.md` §9.8).
    #[test]
    fn at_the_table_the_resolver_reads_nothing() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<NextState<crate::DuelPhase>>()
            .set(crate::DuelPhase::Playing);
        app.update();
        send(
            &mut app,
            KeyCode::Equal,
            Key::Character("=".into()),
            &[command()],
        );
        send(&mut app, KeyCode::Digit1, Key::Character("1".into()), &[]);
        let log = app.world().resource::<ShellLog>();
        assert_eq!(log.resolved, 0);
        assert!(log.fired.is_empty());
        assert_eq!(
            app.world()
                .resource::<crate::settings::ClientSettings>()
                .text_size,
            TextSize::L
        );
    }

    /// The session learns what an unshifted key typed, for the hints.
    #[test]
    fn a_key_teaches_its_character() {
        let mut app = app();
        send(
            &mut app,
            KeyCode::BracketLeft,
            Key::Character("ü".into()),
            &[],
        );
        assert_eq!(
            app.world()
                .resource::<LearntKeys>()
                .0
                .produced("BracketLeft"),
            Some("ü")
        );
    }
}
