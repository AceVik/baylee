//! The dev-control harness: drive and read this client without its window.
//!
//! A loopback HTTP server that presses keys, moves the pointer, dumps what
//! the client believes about the game, and saves a screenshot — so a duel can
//! be played, inspected and photographed while the window sits behind
//! everything else on the desktop. It exists because the alternative is
//! bringing a window to the front, pressing a key by hand and looking at it,
//! which is neither repeatable nor available to anything automated.
//!
//! Three decisions are load-bearing:
//!
//! **It is a compile-time feature (`dev-control`), not a runtime switch.** A
//! remote-control socket inside a game binary is a cheat vector, and the only
//! guarantee worth having is that the code is absent from the shipped build.
//! Binding to loopback is the second lock, never the first.
//!
//! **Keys are written into bevy's [`ButtonInput<KeyCode>`], not synthesised as
//! OS events.** That is both simpler and *more* faithful: `crate::keys` reads
//! exactly that resource, so an injected press goes through the account's
//! `Keymap` like any other — the keymap being the part most worth exercising.
//! It also means focus is irrelevant, which is the whole point.
//!
//! **The state dump is structured, not pixels.** [`crate::Duel`] already holds
//! the view, the board model, the interaction state and the last error, so
//! `/state` answers questions a screenshot cannot ("which targets is it
//! offering?") and can be asserted against. Screenshots answer the questions
//! it cannot: layout, colour, whether anything is drawn at all.
//!
//! # Protocol
//!
//! ```text
//! GET  /health                     → {"ok":true,"frame":1234,"width":…}
//! GET  /state                      → the dump below
//! POST /key      {"name":"Space","shift":false,"hold":false,"release":false}
//! POST /text     {"text":"dev@baylee.local"}
//! POST /pointer  {"x":100,"y":200,"button":"left","press":true}
//! POST /scroll   {"y":-3}   (wheel lines, over wherever the pointer is)
//! POST /screenshot {"path":"/tmp/table.png"}   (replies once written)
//! ```
//!
//! Run it with `BAYLEE_DEV_CONTROL=28770 cargo run -p baylee-client
//! --features dev-control`, then `curl -s localhost:28770/state`.

use crate::Duel;
use crate::settings::ClientSettings;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKeyCode};
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::window::{CursorMoved, PrimaryWindow, WindowEvent};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};

/// How long a request waits for the app to answer before giving up.
///
/// Generous on purpose: a frame is 16 ms, but a cold asset load or a blocked
/// main thread can stall one for a while, and a spurious timeout in a test
/// harness is worse than a slow one.
const REPLY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// One request from the socket thread, with the channel its answer goes back
/// on. Answering can outlive the frame that received it (a screenshot is not
/// ready until the render world has read the surface), which is why the
/// sender travels with the job instead of being a return value.
struct Job {
    path: String,
    body: String,
    reply: Sender<String>,
}

/// The plugin. Add it after [`crate::DuelPlugin`].
pub struct DevControlPlugin {
    /// TCP port on `127.0.0.1`.
    pub port: u16,
}

impl DevControlPlugin {
    /// The port from `BAYLEE_DEV_CONTROL`, or `None` when it is unset.
    ///
    /// Unset means "not today": the harness is compiled in but silent, so a
    /// dev build behaves exactly like a normal one until asked.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let raw = std::env::var("BAYLEE_DEV_CONTROL").ok()?;
        match raw.parse::<u16>() {
            Ok(port) if port > 0 => Some(Self { port }),
            _ => {
                eprintln!("BAYLEE_DEV_CONTROL={raw} is not a port; dev control is off");
                None
            }
        }
    }
}

impl Plugin for DevControlPlugin {
    fn build(&self, app: &mut App) {
        let Some(jobs) = serve(self.port) else {
            return;
        };
        app.insert_resource(DevControl {
            jobs: Mutex::new(jobs),
            held: Vec::new(),
            clicking: Vec::new(),
            frame: 0,
        })
        // After `InputSystem`: bevy has already cleared last frame's
        // `just_pressed` by then, so a key pressed here is `just_pressed`
        // for exactly the frame that follows, the way a real one is.
        .add_systems(PreUpdate, pump.after(bevy::input::InputSystems));
    }
}

/// The receiving half of the socket thread, plus what has to be undone next
/// frame.
#[derive(Resource)]
struct DevControl {
    /// A `Receiver` is `Send` but not `Sync`, and a bevy resource has to be
    /// both. The lock is never contended (one system drains it), so it costs
    /// nothing beyond saying so to the type system.
    jobs: Mutex<Receiver<Job>>,
    /// Keys pressed last frame, released at the start of this one.
    held: Vec<KeyCode>,
    /// Clicks in flight, one stage per frame.
    clicking: Vec<Click>,
    frame: u64,
}

/// A click being played out over three frames.
///
/// It cannot be done in one. Bevy's picking backend does not read
/// [`ButtonInput`] at all — it reads [`WindowEvent`] messages, keeps the last
/// cursor location in a `Local`, and only turns a press into a `Pointer<Click>`
/// once a press and a release have landed on the same hovered entity. So the
/// move has to be seen, hovered against the UI tree, and only then pressed:
/// three frames, in that order, exactly as a real mouse produces them. Doing
/// it in one frame is what made an earlier version answer `{"ok":true}` while
/// nothing whatsoever was clicked.
struct Click {
    at: Vec2,
    button: MouseButton,
    stage: ClickStage,
    /// Held until the release is written, so a caller that got its answer
    /// knows the click is finished rather than merely begun.
    reply: Sender<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ClickStage {
    Press,
    Release,
}

/// Starts the listener thread, or returns `None` if the port is taken.
///
/// A bind failure is a warning rather than a panic: the client is still a
/// perfectly good client without a harness attached to it.
fn serve(port: u16) -> Option<Receiver<Job>> {
    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("dev control: cannot listen on 127.0.0.1:{port}: {err}");
            return None;
        }
    };
    let (tx, rx) = channel();
    std::thread::Builder::new()
        .name("baylee-dev-control".to_string())
        .spawn(move || {
            eprintln!("dev control: listening on http://127.0.0.1:{port}");
            for stream in listener.incoming().flatten() {
                if let Err(err) = handle(&stream, &tx) {
                    eprintln!("dev control: {err}");
                }
            }
        })
        .ok()?;
    Some(rx)
}

/// Reads one request, hands it to the app, writes the answer back.
fn handle(stream: &TcpStream, tx: &Sender<Job>) -> Result<(), String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .map_err(|e| e.to_string())?;
    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string();
    // Headers, only for the one field that changes how much is read.
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(value) = line
            .to_ascii_lowercase()
            .strip_prefix("content-length:")
            .map(str::trim)
            .and_then(|v| v.parse::<usize>().ok())
        {
            length = value;
        }
    }
    let mut body = vec![0u8; length];
    if length > 0 {
        reader.read_exact(&mut body).map_err(|e| e.to_string())?;
    }
    let (reply_tx, reply_rx) = channel();
    tx.send(Job {
        path: path.clone(),
        body: String::from_utf8_lossy(&body).into_owned(),
        reply: reply_tx,
    })
    .map_err(|_| "the app is gone".to_string())?;
    // A request that never comes back is worse than one that fails: the
    // caller would hang for as long as the client runs.
    let answer = reply_rx.recv_timeout(REPLY_TIMEOUT).unwrap_or_else(|_| {
        format!(
            "{{\"error\":\"no answer within {}s\"}}",
            REPLY_TIMEOUT.as_secs()
        )
    });
    respond(stream, &answer)
}

/// Writes a JSON body as a minimal HTTP/1.1 response.
fn respond(mut stream: &TcpStream, body: &str) -> Result<(), String> {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|e| e.to_string())
}

/// A minimal `"key": value` reader over the request body.
///
/// A whole serde derive per endpoint would be four structs to keep in step
/// with four one-line request shapes; these bodies are written by hand at a
/// terminal, so the parser only has to be honest about what it did not find.
fn field<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    let at = body.find(&format!("\"{key}\""))? + key.len() + 2;
    let rest = body[at..].trim_start().strip_prefix(':')?.trim_start();
    if let Some(quoted) = rest.strip_prefix('"') {
        Some(&quoted[..quoted.find('"')?])
    } else {
        let end = rest.find([',', '}']).unwrap_or(rest.len());
        Some(rest[..end].trim())
    }
}

/// Whether a boolean field is present and true.
fn flag(body: &str, key: &str) -> bool {
    field(body, key) == Some("true")
}

/// Drains the request queue once per frame and answers it.
#[allow(clippy::too_many_arguments)]
fn pump(
    mut commands: Commands,
    mut control: ResMut<DevControl>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut buttons: ResMut<ButtonInput<MouseButton>>,
    mut clicks: MessageWriter<MouseButtonInput>,
    mut window_events: MessageWriter<WindowEvent>,
    mut moves: MessageWriter<CursorMoved>,
    mut typing: MessageWriter<KeyboardInput>,
    mut wheels: MessageWriter<bevy::input::mouse::MouseWheel>,
    mut windows: Query<(Entity, &mut Window), With<PrimaryWindow>>,
    duel: Option<Res<Duel>>,
    settings: Option<Res<ClientSettings>>,
) {
    control.frame += 1;
    // Undo last frame's injection first: a key held forever would look like a
    // stuck keyboard, and every consumer reads `just_pressed`.
    for key in control.held.drain(..) {
        keys.release(key);
    }
    let window = windows.single_mut().ok().map(|(entity, _)| entity);
    if let Some(entity) = window {
        advance_clicks(
            &mut control,
            entity,
            &mut buttons,
            &mut clicks,
            &mut window_events,
        );
    }

    let jobs: Vec<Job> = control
        .jobs
        .get_mut()
        .map(|rx| rx.try_iter().collect())
        .unwrap_or_default();
    for job in jobs {
        let answer = match job.path.as_str() {
            // The window's size comes with it because `/pointer` speaks
            // logical pixels while a screenshot is physical: without the
            // scale factor a caller has to guess the ratio between the two,
            // and on a Retina display the guess is wrong by a factor of two.
            "/health" => {
                let (w, h, scale) = windows.single().map_or((0.0, 0.0, 0.0), |(_, window)| {
                    (
                        window.width(),
                        window.height(),
                        window.resolution.scale_factor(),
                    )
                });
                format!(
                    "{{\"ok\":true,\"frame\":{},\"width\":{w},\"height\":{h},\"scale\":{scale}}}",
                    control.frame
                )
            }
            "/state" => state_dump(duel.as_deref(), settings.as_deref()),
            "/key" => {
                let pressed = press_chord(&job.body, &mut keys, &mut typing, window);
                match pressed {
                    Ok(down) => {
                        control.held.extend_from_slice(&down);
                        format!("{{\"ok\":true,\"pressed\":{}}}", down.len())
                    }
                    Err(err) => format!("{{\"error\":\"{err}\"}}"),
                }
            }
            "/text" => match window {
                Some(entity) => {
                    let typed = type_text(&job.body, entity, &mut typing);
                    format!("{{\"ok\":true,\"typed\":{typed}}}")
                }
                None => "{\"error\":\"no primary window\"}".to_string(),
            },
            "/pointer" => {
                match move_pointer(&job.body, &mut windows, &mut moves, &mut window_events) {
                    Err(err) => format!("{{\"error\":\"{err}\"}}"),
                    Ok((_, None)) => "{\"ok\":true,\"clicked\":false}".to_string(),
                    Ok((at, Some(button))) => {
                        // The answer is owed after the release, not now.
                        control.clicking.push(Click {
                            at,
                            button,
                            stage: ClickStage::Press,
                            reply: job.reply,
                        });
                        continue;
                    }
                }
            }
            // Anything below the fold is otherwise unreachable: a control the
            // harness cannot scroll to is a control it cannot press. The
            // wheel lands wherever the pointer was last put, which is how a
            // real one picks the list it scrolls.
            "/scroll" => match window {
                Some(entity) => {
                    let lines: f32 = field(&job.body, "y")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(-3.0);
                    let wheel = bevy::input::mouse::MouseWheel {
                        unit: bevy::input::mouse::MouseScrollUnit::Line,
                        x: 0.0,
                        y: lines,
                        window: entity,
                        // What a mouse always sends; a finger is the other
                        // gesture entirely and the lobby reads it as a drag.
                        phase: bevy::input::touch::TouchPhase::Moved,
                    };
                    // Both, and for the same reason a click writes both: the
                    // picking backend reads `WindowEvent`, and it is picking
                    // that turns a wheel into the `Pointer<Scroll>` a list
                    // listens for. The plain message is what everything else
                    // reads.
                    wheels.write(wheel);
                    window_events.write(WindowEvent::MouseWheel(wheel));
                    format!("{{\"ok\":true,\"lines\":{lines}}}")
                }
                None => "{\"error\":\"no primary window\"}".to_string(),
            },
            "/screenshot" => {
                let Some(path) = field(&job.body, "path").map(str::to_string) else {
                    let _ = job.reply.send("{\"error\":\"no path\"}".to_string());
                    continue;
                };
                // The answer waits for the render world, so the sender goes
                // with the observer rather than being used here: replying now
                // would tell the caller a file exists that does not yet.
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(write_screenshot(path, job.reply));
                continue;
            }
            other => format!("{{\"error\":\"no such endpoint: {other}\"}}"),
        };
        let _ = job.reply.send(answer);
    }
}

/// Types a line of text as keyboard events.
///
/// Separate from `/key` because the two are read in different places, and
/// only one of them can type. `/key` writes [`ButtonInput`], which is what
/// the *duel's* shortcuts read through the account's keymap; every text field
/// in the client reads [`KeyboardInput`] messages instead, because a
/// character is a logical key and a keymap has nothing to say about it. A
/// harness that could only press keys could sign nobody in.
fn type_text(body: &str, window: Entity, keys: &mut MessageWriter<KeyboardInput>) -> usize {
    let Some(text) = field(body, "text") else {
        return 0;
    };
    let mut typed = 0;
    for character in text.chars() {
        // The physical key is a best guess and mostly unread: a text field
        // takes the logical key. Where nothing sensible maps, the character
        // still arrives.
        let key_code = crate::keys::key_code(&character.to_uppercase().to_string())
            .unwrap_or(KeyCode::Unidentified(NativeKeyCode::Unidentified));
        let logical_key = match character {
            ' ' => Key::Space,
            '\n' => Key::Enter,
            other => Key::Character(other.to_string().into()),
        };
        for state in [ButtonState::Pressed, ButtonState::Released] {
            keys.write(KeyboardInput {
                key_code,
                logical_key: logical_key.clone(),
                state,
                text: Some(character.to_string().into()),
                repeat: false,
                window,
            });
        }
        typed += 1;
    }
    typed
}

/// A modifier by name.
///
/// `crate::keys` deliberately does not list these: in a keymap a modifier is a
/// *flag* on a chord, never a binding of its own, so its table has no entry
/// for one. The harness still has to be able to hold shift, because part of
/// the client is about shift being down — a double-faced card turns over for
/// as long as it is.
fn modifier_code(name: &str) -> Option<KeyCode> {
    Some(match name {
        "ShiftLeft" | "Shift" => KeyCode::ShiftLeft,
        "ShiftRight" => KeyCode::ShiftRight,
        "ControlLeft" | "Control" | "Ctrl" => KeyCode::ControlLeft,
        "ControlRight" => KeyCode::ControlRight,
        "AltLeft" | "Alt" => KeyCode::AltLeft,
        "AltRight" => KeyCode::AltRight,
        "SuperLeft" | "Super" | "Meta" => KeyCode::SuperLeft,
        "SuperRight" => KeyCode::SuperRight,
        _ => return None,
    })
}

/// The logical key a named physical key produces.
///
/// A real keyboard reports both, and the client reads both: shortcuts go
/// through [`ButtonInput`] and the account's keymap, while text fields read
/// the logical key out of a [`KeyboardInput`] message. `Tab` was the case
/// that proved it — pressed through the resource alone it moved no focus at
/// all, because the form's tab handling is on the message.
fn logical_key(name: &str) -> Key {
    match name {
        "Tab" => Key::Tab,
        "Enter" | "Return" => Key::Enter,
        "Escape" => Key::Escape,
        "Backspace" => Key::Backspace,
        "Delete" => Key::Delete,
        "Space" => Key::Space,
        "ArrowUp" => Key::ArrowUp,
        "ArrowDown" => Key::ArrowDown,
        "ArrowLeft" => Key::ArrowLeft,
        "ArrowRight" => Key::ArrowRight,
        other => match other.chars().next() {
            Some(first) if other.chars().count() == 1 => {
                Key::Character(first.to_lowercase().to_string().into())
            }
            // A key with no logical meaning of its own (`F5`, a modifier).
            // The physical code carries it; consumers that read text ignore
            // this one, which is exactly right.
            _ => Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
        },
    }
}

/// Presses the keys of one chord, returning what was pressed so it can be
/// released next frame.
fn press_chord(
    body: &str,
    keys: &mut ButtonInput<KeyCode>,
    typing: &mut MessageWriter<KeyboardInput>,
    window: Option<Entity>,
) -> Result<Vec<KeyCode>, String> {
    let name = field(body, "name").ok_or("no key name")?;
    let key = crate::keys::key_code(name)
        .or_else(|| modifier_code(name))
        .ok_or_else(|| format!("unknown key: {name}"))?;
    // `hold` keeps the key down until a matching `release`, because some of
    // the client is about a key *being* held rather than pressed: shift turns
    // a double-faced card over for as long as it is down. A harness that
    // could only tap could not reach that at all.
    let hold = flag(body, "hold");
    let release = flag(body, "release");
    if let Some(window) = window {
        // Both channels, because a real key reaches both.
        let states: &[ButtonState] = if hold {
            &[ButtonState::Pressed]
        } else if release {
            &[ButtonState::Released]
        } else {
            &[ButtonState::Pressed, ButtonState::Released]
        };
        for state in states {
            typing.write(KeyboardInput {
                key_code: key,
                logical_key: logical_key(name),
                state: *state,
                text: None,
                repeat: false,
                window,
            });
        }
    }
    if release {
        keys.release(key);
        return Ok(Vec::new());
    }
    let mut down = Vec::new();
    for (present, modifier) in [
        (flag(body, "shift"), KeyCode::ShiftLeft),
        (flag(body, "ctrl"), KeyCode::ControlLeft),
        (flag(body, "alt"), KeyCode::AltLeft),
        (flag(body, "super"), KeyCode::SuperLeft),
    ] {
        if present {
            keys.press(modifier);
            down.push(modifier);
        }
    }
    keys.press(key);
    down.push(key);
    // A held key is not returned: what is returned is released next frame,
    // and this one stays down until it is asked for by name.
    Ok(if hold { Vec::new() } else { down })
}

/// Moves the cursor, and says which button (if any) is to be clicked there.
///
/// Coordinates are *logical* pixels — what a bevy UI node's `Node` measures
/// in — so a Retina display needs no doubling on the caller's side.
///
/// Two things are updated, not one, because two different consumers read two
/// different places: [`Window::cursor_position`] is what this client's own hit
/// tests and `set_cursor_position` round-trips read, and a [`CursorMoved`]
/// message (mirrored into [`WindowEvent`], exactly as `bevy_winit` does) is
/// what the picking backend reads. Writing only the first is what left an
/// earlier version clicking at whatever position the pointer had never left.
fn move_pointer(
    body: &str,
    windows: &mut Query<(Entity, &mut Window), With<PrimaryWindow>>,
    moves: &mut MessageWriter<CursorMoved>,
    window_events: &mut MessageWriter<WindowEvent>,
) -> Result<(Vec2, Option<MouseButton>), String> {
    let (entity, mut window) = windows.single_mut().map_err(|_| "no primary window")?;
    let at = match (field(body, "x"), field(body, "y")) {
        (Some(x), Some(y)) => {
            let x: f32 = x.parse().map_err(|_| "x is not a number")?;
            let y: f32 = y.parse().map_err(|_| "y is not a number")?;
            let at = Vec2::new(x, y);
            let previous = window.cursor_position();
            window.set_cursor_position(Some(at));
            let moved = CursorMoved {
                window: entity,
                position: at,
                delta: previous.map(|from| at - from),
            };
            moves.write(moved.clone());
            window_events.write(WindowEvent::CursorMoved(moved));
            at
        }
        _ => window.cursor_position().ok_or("no x/y and no cursor")?,
    };
    if !flag(body, "press") {
        return Ok((at, None));
    }
    let button = match field(body, "button").unwrap_or("left") {
        "left" => MouseButton::Left,
        "right" => MouseButton::Right,
        "middle" => MouseButton::Middle,
        other => return Err(format!("unknown button: {other}")),
    };
    Ok((at, Some(button)))
}

/// Plays the next stage of every click in flight, one stage per frame.
///
/// The press carries no position of its own — bevy's backend pairs it with the
/// last cursor location it saw, which is the move written the frame before.
fn advance_clicks(
    control: &mut DevControl,
    window: Entity,
    buttons: &mut ButtonInput<MouseButton>,
    clicks: &mut MessageWriter<MouseButtonInput>,
    window_events: &mut MessageWriter<WindowEvent>,
) {
    for click in std::mem::take(&mut control.clicking) {
        let (state, next) = match click.stage {
            ClickStage::Press => (ButtonState::Pressed, Some(ClickStage::Release)),
            ClickStage::Release => (ButtonState::Released, None),
        };
        match state {
            ButtonState::Pressed => buttons.press(click.button),
            ButtonState::Released => buttons.release(click.button),
        }
        let input = MouseButtonInput {
            button: click.button,
            state,
            window,
        };
        clicks.write(input);
        window_events.write(WindowEvent::MouseButtonInput(input));
        match next {
            Some(stage) => control.clicking.push(Click { stage, ..click }),
            None => {
                let _ = click.reply.send(format!(
                    "{{\"ok\":true,\"clicked\":true,\"x\":{},\"y\":{}}}",
                    click.at.x, click.at.y
                ));
            }
        }
    }
}

/// Saves the captured frame and only then answers the waiting request.
///
/// One observer does both so the ordering is not a guess: two observers on
/// the same entity have no defined order between them, and a reply that
/// arrives before the file does is a race the caller cannot see.
fn write_screenshot(
    path: String,
    reply: Sender<String>,
) -> impl FnMut(bevy::ecs::observer::On<ScreenshotCaptured>) {
    move |captured| {
        let answer = match captured.image.clone().try_into_dynamic() {
            Ok(image) => match image.to_rgb8().save(&path) {
                Ok(()) => format!("{{\"ok\":true,\"path\":\"{path}\"}}"),
                Err(err) => format!("{{\"error\":\"cannot write {path}: {err}\"}}"),
            },
            Err(err) => format!("{{\"error\":\"unreadable frame: {err}\"}}"),
        };
        let _ = reply.send(answer);
    }
}

/// What the client believes, as JSON.
///
/// Deliberately the *client's* answer and not the engine's: this is the thing
/// under test. `view` is what the host last sent, `interaction` is what the
/// client made of it, and a disagreement between them is exactly the class of
/// bug this endpoint exists to show.
fn state_dump(duel: Option<&Duel>, settings: Option<&ClientSettings>) -> String {
    let Some(duel) = duel else {
        return "{\"duel\":null}".to_string();
    };
    let view = duel
        .view
        .as_ref()
        .and_then(|v| serde_json::to_string(v).ok())
        .unwrap_or_else(|| "null".to_string());
    // `Interaction` itself is not serialisable, and giving it derives to
    // suit this endpoint would be a real API change to client-core for a
    // debugging convenience. Its substance is public anyway: the choice the
    // engine posed, and what has been picked towards answering it.
    let interaction = duel.interaction.as_ref().map_or_else(
        || "null".to_string(),
        |i| {
            let pending = serde_json::to_string(i.pending()).unwrap_or_else(|_| "null".to_string());
            format!(
                "{{\"pending\":{pending},\"selected\":{selected},\"selected_players\":{seats}}}",
                selected = i.selected().len(),
                seats = i.selected_players().len(),
            )
        },
    );
    let error = duel
        .last_error
        .as_deref()
        .map_or_else(|| "null".to_string(), |e| format!("\"{e}\""));
    let lang = settings.map_or_else(
        || "null".to_string(),
        |s| format!("\"{}\"", s.lang.escape_default()),
    );
    format!(
        "{{\"view\":{view},\"interaction\":{interaction},\"hovered\":{hovered},\
         \"overlay_open\":{overlay},\"last_error\":{error},\"lang\":{lang},\
         \"reachable\":{reachable},\"activatable\":{activatable},\
         \"outbox\":{outbox},\"mana_run\":{mana_run},\"ability_menu\":{menu}}}",
        hovered = duel
            .hovered
            .map_or_else(|| "null".to_string(), |h| format!("\"{h:?}\"")),
        overlay = duel.overlay_open,
        reachable = duel.reachable.len(),
        activatable = duel.activatable.len(),
        // Three states that answer silently and are invisible in a
        // screenshot: an action queued but never sent, a mana run that owns
        // the next few keys, and an ability menu that swallows the keyboard
        // whole. Every one of them looks exactly like "the key did nothing".
        outbox = duel.outbox().len(),
        mana_run = duel.mana_run.is_some(),
        menu = duel
            .ability_menu
            .map_or_else(|| "null".to_string(), |m| format!("\"{m:?}\"")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::message::Messages;

    #[test]
    fn a_request_body_yields_its_fields() {
        let body = r#"{"x":100.5,"y":-2,"button":"left","press":true,"shift":false}"#;
        assert_eq!(field(body, "x"), Some("100.5"));
        assert_eq!(field(body, "y"), Some("-2"));
        assert_eq!(field(body, "button"), Some("left"));
        assert!(flag(body, "press"));
        assert!(!flag(body, "shift"));
        assert!(!flag(body, "ctrl"), "a missing flag is not a set one");
        assert_eq!(field(body, "path"), None);
    }

    /// Builds an app carrying only what `pump` reads, plus the job channel.
    fn harness() -> (App, Sender<Job>) {
        let (tx, rx) = channel();
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_message::<MouseButtonInput>()
            .add_message::<KeyboardInput>()
            .add_message::<WindowEvent>()
            .add_message::<CursorMoved>()
            .add_message::<bevy::input::mouse::MouseWheel>()
            .insert_resource(DevControl {
                jobs: Mutex::new(rx),
                held: Vec::new(),
                clicking: Vec::new(),
                frame: 0,
            })
            .add_systems(Update, pump);
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        (app, tx)
    }

    /// Everything the window events of one frame said, as short tags.
    fn window_events(app: &App) -> Vec<String> {
        app.world()
            .resource::<Messages<WindowEvent>>()
            .iter_current_update_messages()
            .map(|event| match event {
                WindowEvent::CursorMoved(moved) => {
                    format!("move {} {}", moved.position.x, moved.position.y)
                }
                WindowEvent::MouseButtonInput(input) => match input.state {
                    ButtonState::Pressed => "press".to_string(),
                    ButtonState::Released => "release".to_string(),
                },
                other => format!("{other:?}"),
            })
            .collect()
    }

    /// The regression this endpoint was rebuilt for: a click that reported
    /// success without ever reaching bevy's picking backend. Picking reads
    /// `WindowEvent`, and it pairs a press with the *last cursor location it
    /// saw*, so the move must be a message of its own and must come first.
    #[test]
    fn a_click_is_a_move_then_a_press_then_a_release() {
        let (mut app, tx) = harness();
        let (reply, answers) = channel();
        tx.send(Job {
            path: "/pointer".to_string(),
            body: r#"{"x":40,"y":60,"press":true}"#.to_string(),
            reply,
        })
        .unwrap();

        app.update();
        assert_eq!(window_events(&app), ["move 40 60"]);
        assert!(
            answers.try_recv().is_err(),
            "the caller is answered when the click is finished, not when it starts"
        );

        app.update();
        assert_eq!(window_events(&app), ["press"]);
        assert!(
            app.world()
                .resource::<ButtonInput<MouseButton>>()
                .pressed(MouseButton::Left)
        );

        app.update();
        assert_eq!(window_events(&app), ["release"]);
        assert!(
            !app.world()
                .resource::<ButtonInput<MouseButton>>()
                .pressed(MouseButton::Left)
        );
        assert!(answers.try_recv().unwrap().contains("\"clicked\":true"));
    }

    /// The same regression one gesture along: a wheel written only as a
    /// `MouseWheel` message scrolled nothing at all, because it is picking
    /// that turns a wheel into the `Pointer<Scroll>` a list listens for, and
    /// picking reads `WindowEvent`. Both, or neither is any use.
    #[test]
    fn a_wheel_is_written_where_picking_reads_it() {
        let (mut app, tx) = harness();
        let (reply, answers) = channel();
        tx.send(Job {
            path: "/scroll".to_string(),
            body: r#"{"y":-6}"#.to_string(),
            reply,
        })
        .unwrap();
        app.update();
        assert!(answers.try_recv().unwrap().contains("\"lines\":-6"));

        let plain: Vec<f32> = app
            .world()
            .resource::<Messages<bevy::input::mouse::MouseWheel>>()
            .iter_current_update_messages()
            .map(|wheel| wheel.y)
            .collect();
        assert_eq!(plain, [-6.0], "nothing else reads the window event");
        let mirrored = window_events(&app);
        assert_eq!(mirrored.len(), 1, "{mirrored:?}");
        assert!(mirrored[0].contains("MouseWheel"), "{mirrored:?}");
    }

    /// A move without `press` is answered at once and presses nothing — the
    /// hover path, which is how a card preview is opened.
    #[test]
    fn a_move_without_a_press_is_only_a_move() {
        let (mut app, tx) = harness();
        let (reply, answers) = channel();
        tx.send(Job {
            path: "/pointer".to_string(),
            body: r#"{"x":10,"y":20}"#.to_string(),
            reply,
        })
        .unwrap();
        app.update();
        assert_eq!(window_events(&app), ["move 10 20"]);
        assert!(answers.try_recv().unwrap().contains("\"clicked\":false"));
        let mut windows = app
            .world_mut()
            .query_filtered::<&Window, With<PrimaryWindow>>();
        let cursor = windows.single(app.world()).unwrap().cursor_position();
        assert_eq!(cursor, Some(Vec2::new(10.0, 20.0)));
    }

    /// A key goes in as `just_pressed` for one frame and is released on the
    /// next, the way a real key is — a stuck modifier would change what every
    /// later chord means.
    #[test]
    fn a_key_is_held_for_exactly_one_frame() {
        let (mut app, tx) = harness();
        let (reply, _answers) = channel();
        tx.send(Job {
            path: "/key".to_string(),
            body: r#"{"name":"Space","shift":true}"#.to_string(),
            reply,
        })
        .unwrap();
        app.update();
        let keys = app.world().resource::<ButtonInput<KeyCode>>();
        assert!(keys.pressed(KeyCode::Space));
        assert!(keys.pressed(KeyCode::ShiftLeft));
        app.update();
        let keys = app.world().resource::<ButtonInput<KeyCode>>();
        assert!(!keys.pressed(KeyCode::Space));
        assert!(!keys.pressed(KeyCode::ShiftLeft));
    }
}
