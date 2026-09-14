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
//! The one thing it also has to answer is **where** — `cards` gives every
//! drawn card's box in the logical pixels `/pointer` takes, because the
//! alternative is finding a card by eye on a downscaled screenshot and doing
//! it again every time a lane repacks. See [`cards_json`].
//!
//! # Protocol
//!
//! ```text
//! GET  /health                     → {"ok":true,"frame":1234,"width":…}
//! GET  /state                      → the dump below
//! POST /key      {"name":"Space","shift":false,"hold":false,"release":false}
//! POST /text     {"text":"dev@baylee.local"}
//! POST /pointer  {"x":100,"y":200,"button":"left","press":true,"hold":false,"release":false}
//! POST /scroll   {"y":-3}   (wheel lines, over wherever the pointer is)
//! POST /screenshot {"path":"/tmp/table.png"}   (replies once written)
//! POST /timescale {"speed":0.1}   (the whole picture, a tenth as fast)
//! POST /pause    {"paused":false}   (absent or true stops the clock)
//! POST /step     {"frames":6}   (replies once they have run)
//! ```
//!
//! The last three are one tool. Almost everything worth photographing here is
//! over before a screenshot can be asked for — a card's exit lives 0.55 s —
//! so pause the clock, step it a handful of frames at a time and photograph
//! each one, or slow the whole picture to a tenth and watch it at leisure.
//! Both work on `Time<Virtual>`, which is what `table::glide`, `table::retire`,
//! the sheen clock and every shader's `globals.time` read through, so the
//! parts of one movement stay together. `pump` counts frames rather than
//! seconds, which is what keeps the harness answering while the clock is
//! stopped.
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
            stepping: None,
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
    /// A `/step` running: the clock has been let go until it counts out.
    stepping: Option<Stepping>,
    frame: u64,
}

/// A run of frames the clock was let go for.
///
/// Counted in frames rather than in seconds, because the request after a step
/// is always a screenshot and a screenshot is a frame. Asking for a tenth of
/// a second would leave the caller to work out how many frames that was, and
/// get a different answer on a different machine.
struct Stepping {
    /// What was asked for, kept so the answer can say it back.
    frames: u32,
    /// How many are left.
    left: u32,
    /// Held until the last of them has run, so a caller that got its answer
    /// knows the frames have happened rather than merely been scheduled.
    reply: Sender<String>,
}

/// A pointer deed being played out one stage per frame.
///
/// It cannot be done in one. Bevy's picking backend does not read
/// [`ButtonInput`] at all — it reads [`WindowEvent`] messages, keeps the last
/// cursor location in a `Local`, and only turns a press into a `Pointer<Click>`
/// once a press and a release have landed on the same hovered entity. So the
/// move has to be seen, hovered against the UI tree, and only then pressed,
/// in that order, exactly as a real mouse produces them. Doing it in one
/// frame is what made an earlier version answer `{"ok":true}` while nothing
/// whatsoever was clicked.
///
/// Two stages were added to the three once the remaining trouble was taken
/// apart with a click helper, and each answers one half of it.
///
/// [`ClickStage::Aim`] writes the cursor move a **second** time, a whole
/// frame after the request wrote it. A move is sometimes simply lost: the
/// pointer was put on a card and `hovered` read fifteen times running as
/// `None`, and sending the *same* move again named the object on the first
/// read. Repeated reading never repaired it and repeated sending always did,
/// which is the shape of an event that did not arrive rather than a state
/// that had not settled — so the harness sends it twice and stops guessing.
///
/// [`ClickStage::Settle`] writes nothing at all and exists to hold the
/// answer back one more frame. The reply used to go out on the frame the
/// release was *written*, which is the frame before anything reads it: a
/// caller that clicked and then asked for `/state` saw the board from before
/// its own click (`armed: None`, and armed a moment later without a second
/// click). A harness that answers before the deed has been read is a harness
/// whose every measurement is off by one.
struct Click {
    at: Vec2,
    /// `None` for a bare move, which now rides the same machine so that it
    /// is sent twice and answered late like everything else here.
    button: Option<MouseButton>,
    stage: ClickStage,
    /// The stage [`ClickStage::Aim`] hands over to — a press for a click or
    /// a hold, the release for a call that only lets go.
    then: ClickStage,
    /// Whether the button stays down once it has been pressed.
    ///
    /// A press and its release are one call by default, which is right for a
    /// tap and no use at all for the things that exist only *while* the
    /// button is down: a drag, and a card giving way under the finger. Both
    /// are drawn by code of the kind that has shipped unwired here before,
    /// and neither could be photographed. Named after [`press_key`]'s pair,
    /// because it is the same pair.
    hold: bool,
    /// What goes back to the caller once the last stage has been read.
    answer: String,
    /// Held until then, so a caller that got its answer knows the deed is
    /// finished rather than merely begun.
    reply: Sender<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ClickStage {
    Aim,
    Press,
    Release,
    Settle,
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

/// Everything the socket thread has queued since the last frame.
///
/// Collected rather than iterated in place: the loop that answers these needs
/// the resource back for `/step`, and a poisoned lock is an empty frame
/// rather than a panic — a harness that took the client down when its own
/// mutex went wrong would be the worst possible failure mode for a debugging
/// tool.
fn waiting(control: &mut DevControl) -> Vec<Job> {
    control
        .jobs
        .get_mut()
        .map(|rx| rx.try_iter().collect())
        .unwrap_or_default()
}

/// What `/health` says, given the frame count, the window's `(width, height,
/// scale)` and the clock.
///
/// The window's size comes with it because `/pointer` speaks logical pixels
/// while a screenshot is physical: without the scale factor a caller has to
/// guess the ratio between the two, and on a Retina display the guess is
/// wrong by a factor of two. The clock is there for the same kind of reason —
/// a harness that had slowed or stopped the picture and then reconnected
/// would otherwise have no way to ask what it had left running.
fn health(frame: u64, size: (f32, f32, f32), clock: &Time<Virtual>) -> String {
    let (w, h, scale) = size;
    format!(
        "{{\"ok\":true,\"frame\":{frame},\"width\":{w},\"height\":{h},\"scale\":{scale},\
         \"speed\":{},\"paused\":{}}}",
        clock.relative_speed(),
        clock.is_paused(),
    )
}

/// Lets the clock go for `frames` frames, and takes the caller's reply
/// channel with it.
///
/// It answers rather than returning an answer, for the reason `/screenshot`
/// does the same: the reply belongs after the frames have run, and a
/// function that returned a string here would have said they had.
fn start_step(
    body: &str,
    control: &mut DevControl,
    clock: &mut Time<Virtual>,
    reply: Sender<String>,
) {
    let frames = field(body, "frames")
        .and_then(|f| f.parse::<u32>().ok())
        .unwrap_or(1);
    if frames == 0 {
        let _ = reply.send("{\"error\":\"a step of no frames is not a step\"}".to_string());
        return;
    }
    if control.stepping.is_some() {
        let _ = reply.send("{\"error\":\"a step is already running\"}".to_string());
        return;
    }
    clock.unpause();
    control.stepping = Some(Stepping {
        frames,
        left: frames,
        reply,
    });
}

/// Counts a running `/step` down by a frame and stops the clock at the end
/// of it.
///
/// The countdown runs at the top of `PreUpdate`, before the jobs are drained,
/// so the frame that started the step is the first of the frames it asked
/// for and `frames: 1` advances the picture exactly once.
fn catch_the_clock(control: &mut DevControl, clock: &mut Time<Virtual>) {
    let counted_out = control.stepping.as_mut().is_some_and(|step| {
        step.left = step.left.saturating_sub(1);
        step.left == 0
    });
    if !counted_out {
        return;
    }
    clock.pause();
    if let Some(step) = control.stepping.take() {
        let _ = step
            .reply
            .send(format!("{{\"ok\":true,\"frames\":{}}}", step.frames));
    }
}

/// The two clock routes that answer at once, and the reason all three exist:
/// almost everything this client does that is worth photographing is over
/// before a screenshot can be asked for. A card's exit lives 0.55 s, a sheen
/// sweep less, and a `/screenshot` round trip is a frame plus a file write —
/// so the harness could prove an animation had *finished* and never that it
/// had happened.
///
/// `Time<Virtual>` is the right lever because everything the table draws with
/// reads through it: `table::glide`, `table::retire`, the sheen clock and
/// every shader's `globals.time` all take `Res<Time>`, which bevy sets from
/// the virtual clock each frame. A tenth speed therefore slows the whole
/// picture together and keeps it consistent, where a per-system knob would
/// have pulled the parts of one movement apart.
fn set_clock(path: &str, body: &str, clock: &mut Time<Virtual>) -> String {
    if path == "/pause" {
        if field(body, "paused").is_some_and(|v| v == "false" || v == "0") {
            clock.unpause();
        } else {
            clock.pause();
        }
        return clock_answer(clock);
    }
    match field(body, "speed").and_then(|s| s.parse::<f32>().ok()) {
        Some(speed) if speed > 0.0 && speed.is_finite() => {
            clock.set_relative_speed(speed);
            clock_answer(clock)
        }
        // Zero is refused rather than taken as a pause: they are different
        // states, and a caller who could stop the clock two ways would have
        // to remember which one to undo.
        _ => "{\"error\":\"speed must be a finite number above zero\"}".to_string(),
    }
}

/// The clock as the harness reports it, which is two numbers because either
/// one alone is a lie: a speed of 0.1 on a paused clock is still stopped, and
/// a running clock says nothing about how fast.
fn clock_answer(clock: &Time<Virtual>) -> String {
    format!(
        "{{\"ok\":true,\"speed\":{},\"paused\":{}}}",
        clock.relative_speed(),
        clock.is_paused()
    )
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
    believed: Believed,
    mut clock: ResMut<Time<Virtual>>,
) {
    control.frame += 1;
    catch_the_clock(&mut control, &mut clock);
    // Undo last frame's injection first: a key held forever would look like a
    // stuck keyboard, and every consumer reads `just_pressed`.
    for key in control.held.drain(..) {
        keys.release(key);
    }
    let window = windows.single_mut().ok().map(|(entity, _)| entity);
    if let Ok((entity, mut win)) = windows.single_mut() {
        advance_clicks(
            &mut control,
            entity,
            &mut win,
            &mut buttons,
            &mut clicks,
            &mut moves,
            &mut window_events,
        );
    }

    for job in waiting(&mut control) {
        let answer = match job.path.as_str() {
            "/health" => {
                let size = windows.single().map_or((0.0, 0.0, 0.0), |(_, window)| {
                    (
                        window.width(),
                        window.height(),
                        window.resolution.scale_factor(),
                    )
                });
                health(control.frame, size, &clock)
            }
            "/state" => {
                let size = windows
                    .single()
                    .map_or(Vec2::ZERO, |(_, w)| Vec2::new(w.width(), w.height()));
                state_dump(&believed, size)
            }
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
                    // Both shapes go on the queue, and a bare move with them:
                    // it is the move that goes missing, so the aim-again
                    // frame is worth exactly as much to a call that only
                    // aims. The answer is owed after the last stage in
                    // either case, not now.
                    Ok((at, deed)) => {
                        let answer = match &deed {
                            Some(deed) => format!(
                                "{{\"ok\":true,\"{}\":true,\"x\":{},\"y\":{}}}",
                                deed.word, at.x, at.y
                            ),
                            None => "{\"ok\":true,\"clicked\":false}".to_string(),
                        };
                        control.clicking.push(Click {
                            at,
                            button: deed.as_ref().map(|d| d.button),
                            stage: ClickStage::Aim,
                            then: deed.as_ref().map_or(ClickStage::Settle, |d| d.stage),
                            hold: deed.as_ref().is_some_and(|d| d.hold),
                            answer,
                            reply: job.reply,
                        });
                        continue;
                    }
                }
            }
            "/scroll" => match window {
                Some(entity) => turn_the_wheel(&job.body, entity, &mut wheels, &mut window_events),
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
            "/timescale" | "/pause" => set_clock(&job.path, &job.body, &mut clock),
            "/step" => {
                start_step(&job.body, &mut control, &mut clock, job.reply);
                continue;
            }
            other => format!("{{\"error\":\"no such endpoint: {other}\"}}"),
        };
        let _ = job.reply.send(answer);
    }
}

/// Spins the wheel where the pointer is.
///
/// Anything below the fold is otherwise unreachable: a control the harness
/// cannot scroll to is a control it cannot press. The wheel lands wherever the
/// pointer was last put, which is how a real one picks the list it scrolls.
///
/// It is written twice for the same reason a click is: the picking backend
/// reads [`WindowEvent`], and it is picking that turns a wheel into the
/// `Pointer<Scroll>` a list listens for, while the plain message is what
/// everything else reads.
fn turn_the_wheel(
    body: &str,
    window: Entity,
    wheels: &mut MessageWriter<bevy::input::mouse::MouseWheel>,
    window_events: &mut MessageWriter<WindowEvent>,
) -> String {
    let lines: f32 = field(body, "y")
        .and_then(|v| v.parse().ok())
        .unwrap_or(-3.0);
    let wheel = bevy::input::mouse::MouseWheel {
        unit: bevy::input::mouse::MouseScrollUnit::Line,
        x: 0.0,
        y: lines,
        window,
        // What a mouse always sends; a finger is the other gesture entirely
        // and the lobby reads it as a drag.
        phase: bevy::input::touch::TouchPhase::Moved,
    };
    wheels.write(wheel);
    window_events.write(WindowEvent::MouseWheel(wheel));
    format!("{{\"ok\":true,\"lines\":{lines}}}")
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

/// `Y` for `KeyY`, `1` for `Digit1`.
///
/// The keymap's own table spells a letter `KeyY`, because that is the name a
/// stored keymap has to survive being read by; a caller writing a script by
/// hand reaches for the letter. A refused key answers `200` with an error in
/// it, so `{"name":"Y"}` looked exactly like a key that reached the client and
/// did nothing — which is how `docs/observed-faults.md` came to carry an entry
/// claiming a yes/no question has no keyboard answer at all, since withdrawn.
/// The alias fixes the trap rather than that one morning: a harness that
/// refuses the obvious spelling of a key will be handed it again.
fn harness_alias(name: &str) -> Option<KeyCode> {
    let mut chars = name.chars();
    let only = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    let spelled = if only.is_ascii_alphabetic() {
        format!("Key{}", only.to_ascii_uppercase())
    } else if only.is_ascii_digit() {
        format!("Digit{only}")
    } else {
        return None;
    };
    crate::keys::key_code(&spelled)
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
        // The canonical spellings, which `harness_alias` lets a caller write
        // either way round. `Digit2` pressed the physical key and reported no
        // logical one at all, so the ability sheet — which reads its digits
        // as characters, the way the subtype filter does — saw nothing;
        // `2` worked and `Digit2` did not, which is the harness disagreeing
        // with itself about one key.
        _ if name.len() == 6 && name.starts_with("Digit") => Key::Character(name[5..].into()),
        _ if name.len() == 4 && name.starts_with("Key") => {
            Key::Character(name[3..].to_lowercase().into())
        }
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
        .or_else(|| harness_alias(name))
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
) -> Result<(Vec2, Option<ButtonDeed>), String> {
    let (entity, mut window) = windows.single_mut().map_err(|_| "no primary window")?;
    let at = match (field(body, "x"), field(body, "y")) {
        (Some(x), Some(y)) => {
            let x: f32 = x.parse().map_err(|_| "x is not a number")?;
            let y: f32 = y.parse().map_err(|_| "y is not a number")?;
            let at = Vec2::new(x, y);
            aim_cursor(at, entity, &mut window, moves, window_events);
            at
        }
        _ => window.cursor_position().ok_or("no x/y and no cursor")?,
    };
    Ok((at, button_deed(body)?))
}

/// Puts the cursor at `at` and tells everything that watches it.
///
/// Its own function because [`ClickStage::Aim`] writes the very same move a
/// frame later, and a second spelling of it is a second thing to keep in
/// step: the whole point of the repeat is that the two are identical.
///
/// The delta is computed against where the cursor was, so a repeat writes a
/// zero delta — which is what a real mouse held still reports, and what the
/// picking backend expects of one.
fn aim_cursor(
    at: Vec2,
    entity: Entity,
    window: &mut Window,
    moves: &mut MessageWriter<CursorMoved>,
    window_events: &mut MessageWriter<WindowEvent>,
) {
    let previous = window.cursor_position();
    window.set_cursor_position(Some(at));
    let moved = CursorMoved {
        window: entity,
        position: at,
        delta: previous.map(|from| at - from),
    };
    moves.write(moved.clone());
    window_events.write(WindowEvent::CursorMoved(moved));
}

/// What a `/pointer` body asks the button to do, or `None` for a bare move.
///
/// Its own function because it is the half a test can reach: `move_pointer`
/// needs a window and two message writers, so the decision inside it had no
/// way of being asserted except by driving a whole app.
///
/// `hold` and `release` each imply the press they are a stage of, and that is
/// the fix for a trap rather than a convenience. `{"hold":true}` alone used
/// to fall through to the bare-move return and answer `{"ok":true,
/// "clicked":false}` — a `200` that reads like a press that happened, on a
/// route whose whole purpose is photographing what lives between a press and
/// a release. It cost a live proof of `docs/observed-faults.md` 51 one whole
/// gesture, which is the second time this harness has answered a caller's
/// obvious spelling with something that looks like the client doing nothing;
/// `harness_alias` is the first.
fn button_deed(body: &str) -> Result<Option<ButtonDeed>, String> {
    let hold = flag(body, "hold");
    let let_go = flag(body, "release");
    if !flag(body, "press") && !let_go && !hold {
        return Ok(None);
    }
    let button = match field(body, "button").unwrap_or("left") {
        "left" => MouseButton::Left,
        "right" => MouseButton::Right,
        "middle" => MouseButton::Middle,
        other => return Err(format!("unknown button: {other}")),
    };
    // `release` wins over `press`, so a caller that sends both gets the
    // half it can only have meant: you cannot let go of a button on the
    // same call that presses it and still have held it.
    let (stage, hold, word) = if let_go {
        (ClickStage::Release, false, "released")
    } else if hold {
        (ClickStage::Press, true, "held")
    } else {
        (ClickStage::Press, false, "clicked")
    };
    Ok(Some(ButtonDeed {
        button,
        stage,
        hold,
        word,
    }))
}

/// What a `/pointer` call does with the button once the cursor has moved.
struct ButtonDeed {
    button: MouseButton,
    /// The stage to start at: a press for a click or a hold, the release for
    /// a call that only lets go.
    stage: ClickStage,
    /// Whether the press is the end of it.
    hold: bool,
    /// What the answer calls it.
    word: &'static str,
}

/// Plays the next stage of every pointer deed in flight, one stage per frame.
///
/// The press carries no position of its own — bevy's backend pairs it with the
/// last cursor location it saw, which is the move written the frame before,
/// and written again by [`ClickStage::Aim`] the frame before that.
fn advance_clicks(
    control: &mut DevControl,
    window: Entity,
    win: &mut Window,
    buttons: &mut ButtonInput<MouseButton>,
    clicks: &mut MessageWriter<MouseButtonInput>,
    moves: &mut MessageWriter<CursorMoved>,
    window_events: &mut MessageWriter<WindowEvent>,
) {
    for click in std::mem::take(&mut control.clicking) {
        let next = match click.stage {
            // The move, a second time and a whole frame later. A bare move
            // has nothing to press afterwards and goes straight to settling.
            ClickStage::Aim => {
                aim_cursor(click.at, window, win, moves, window_events);
                Some(if click.button.is_some() {
                    click.then
                } else {
                    ClickStage::Settle
                })
            }
            // A held press has no release of its own: the button stays down
            // until a `{"release":true}` comes to lift it.
            ClickStage::Press | ClickStage::Release => {
                let state = if click.stage == ClickStage::Press {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                };
                if let Some(button) = click.button {
                    match state {
                        ButtonState::Pressed => buttons.press(button),
                        ButtonState::Released => buttons.release(button),
                    }
                    let input = MouseButtonInput {
                        button,
                        state,
                        window,
                    };
                    clicks.write(input);
                    window_events.write(WindowEvent::MouseButtonInput(input));
                }
                Some(if click.stage == ClickStage::Press && !click.hold {
                    ClickStage::Release
                } else {
                    ClickStage::Settle
                })
            }
            // A frame in which the harness writes nothing, so that the
            // answer leaves after the systems that read the last stage have
            // run rather than before them.
            ClickStage::Settle => None,
        };
        match next {
            Some(stage) => control.clicking.push(Click { stage, ..click }),
            None => {
                let _ = click.reply.send(click.answer);
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

/// Everything `/state` reads, in one parameter.
///
/// A bundle rather than six more arguments on [`pump`], which is already at
/// bevy's sixteen-parameter ceiling — and the grouping is honest: these are
/// the things the client *believes*, as against the input queues and the
/// clock around them.
#[derive(bevy::ecs::system::SystemParam)]
struct Believed<'w, 's> {
    duel: Option<Res<'w, Duel>>,
    settings: Option<Res<'w, ClientSettings>>,
    /// Cards playing their way off the table; only the count is reported.
    leaving: Query<'w, 's, &'static crate::table::Departing>,
    shelves: Option<Res<'w, crate::hud::Shelves>>,
    /// Where the camera stood at the end of the last frame, which is the
    /// camera the last rendered frame was drawn with — so a rect measured
    /// here answers for the picture a `/screenshot` would return.
    rig: Option<Res<'w, crate::table::ShownRig>>,
    /// Every card drawn on the table, with the transform `glide` has it at
    /// right now rather than the one it is heading for.
    cards: Query<'w, 's, (&'static crate::table::CardVisual, &'static Transform)>,
    /// Every card standing in the player's own hand row.
    ///
    /// A different kind of thing entirely — the hand is `bevy_ui` and the
    /// table is a 3D scene — but the same question is being asked of it, and
    /// a caller that has to find a hand card by eye is no better off for the
    /// table's cards being free. `HandRowCard` and not the wider
    /// `HandCardVisual`, which the stack panel also puts on its slots.
    hand: Query<
        'w,
        's,
        (
            &'static crate::hud::HandCardVisual,
            &'static bevy::ui::ComputedNode,
            &'static bevy::ui::UiGlobalTransform,
        ),
        With<crate::hud::HandRowCard>,
    >,
    /// Every row of the stack panel.
    ///
    /// The third zone, and it was missing for the same reason the hand once
    /// was: a driver could read `interaction.pending` and see a
    /// `ChooseTargets` naming object 200, and then had no way on earth to
    /// find object 200 on the screen — the stack is neither a card on the
    /// felt nor a card in the hand, and it is where every "target spell"
    /// lives. `StackRowCard` and not the wider `HandCardVisual` for the
    /// reason the hand gives above: the panel puts that one on target chips
    /// too, and a chip is a picture *of* an object elsewhere.
    stack: Query<
        'w,
        's,
        (
            &'static crate::hud::HandCardVisual,
            &'static bevy::ui::ComputedNode,
            &'static bevy::ui::UiGlobalTransform,
        ),
        With<crate::hud::StackRowCard>,
    >,
    /// The prompt bar's answers, and the two choosers under it.
    ///
    /// Added for the same reason and by the same road as the cards: a
    /// shockland asked `PayLifeOrEnterTapped` and there was no way to answer
    /// it. `PromptAction::Yes` and `No` are reachable *only* through a
    /// pointer click — no key binding fires either — so a harness that cannot
    /// find the button cannot get past the question at all.
    prompts: Query<
        'w,
        's,
        (
            &'static crate::hud::PromptButton,
            &'static bevy::ui::ComputedNode,
            &'static bevy::ui::UiGlobalTransform,
        ),
    >,
    abilities: Query<
        'w,
        's,
        (
            &'static crate::hud::AbilityButton,
            &'static bevy::ui::ComputedNode,
            &'static bevy::ui::UiGlobalTransform,
        ),
    >,
    choices: Query<
        'w,
        's,
        (
            &'static crate::hud::ChoiceButton,
            &'static bevy::ui::ComputedNode,
            &'static bevy::ui::UiGlobalTransform,
        ),
    >,
    /// Everything a player can press that is not an answer: the concession,
    /// the draw offer, the armed card's two halves and "resolve the stack".
    ///
    /// Left out until the shelf put one of them in the row of answers, where
    /// a harness reading only `prompt` rows sees a gap between two buttons and
    /// nothing in it. They were always worth having — a concession has no key
    /// at all, and the way to end a driven game was to close the window.
    menus: Query<
        'w,
        's,
        (
            &'static crate::hud::MenuButton,
            &'static bevy::ui::ComputedNode,
            &'static bevy::ui::UiGlobalTransform,
        ),
    >,
}

/// What the client believes, as JSON.
///
/// Deliberately the *client's* answer and not the engine's: this is the thing
/// under test. `view` is what the host last sent, `interaction` is what the
/// client made of it, and a disagreement between them is exactly the class of
/// bug this endpoint exists to show.
fn state_dump(believed: &Believed, window: Vec2) -> String {
    let Some(duel) = believed.duel.as_deref() else {
        return "{\"duel\":null}".to_string();
    };
    let settings = believed.settings.as_deref();
    let departing = believed.leaving.iter().count();
    let shelves = believed.shelves.as_deref();
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
            // `selected` is empty in both combat modes — an attack and a
            // block are *pairs*, and they live in `assignments` instead. A
            // caller reading only the count therefore watches a declaration
            // being built and sees nothing happen, which is a morning this
            // harness has already cost once.
            let pairs: Vec<String> = i
                .assignments()
                .into_iter()
                .map(|(creature, at)| {
                    format!(
                        "{{\"creature\":{},\"at\":{}}}",
                        creature.slot(),
                        quoted(&format!("{at:?}"))
                    )
                })
                .collect();
            // `focus` is combat's alone, and `aim` is the same question asked
            // of every mode that has an answer to it — the row a dialog's
            // keyboard is standing on included. Both, rather than the second
            // in place of the first: `CombatFocus` says whether the thing
            // aimed at is a defender or an attacker, which `Pick` drops.
            //
            // Without `aim`, a `Mode::Objects` focus is invisible here, and
            // proving that a key moved it takes a photograph and a pixel
            // diff — which is what it took once.
            format!(
                "{{\"pending\":{pending},\"selected\":{selected},\"selected_players\":{seats},\
                 \"assignments\":[{pairs}],\"focus\":{focus},\"aim\":{aim}}}",
                selected = i.selected().count(),
                seats = i.selected_players().count(),
                pairs = pairs.join(","),
                focus = quoted(&format!("{:?}", i.combat_focus())),
                aim = quoted(&format!("{:?}", i.aim())),
            )
        },
    );
    // The two-stage arm: the first tap on anything irreversible only arms it,
    // and a caller that does not know a tap armed rather than fired reads the
    // second tap as the one that did nothing.
    let armed = duel.armed.as_ref().map_or_else(
        || "null".to_string(),
        |armed| {
            format!(
                "{{\"object\":{},\"deed\":{}}}",
                armed.object.slot(),
                quoted(&format!("{:?}", armed.deed))
            )
        },
    );
    let error = duel
        .last_error
        .as_deref()
        .map_or_else(|| "null".to_string(), quoted);
    let lang = settings.map_or_else(|| "null".to_string(), |s| quoted(&s.lang));
    format!(
        "{{\"view\":{view},\"interaction\":{interaction},\"hovered\":{hovered},\
         \"autopilot\":{autopilot},\"last_error\":{error},\"lang\":{lang},\
         \"reachable\":{reachable},\"activatable\":{activatable},\"armed\":{armed},\
         \"outbox\":{outbox},\"mana_run\":{mana_run},\"ability_menu\":{menu},\
         \"ability_tap\":{tap},\"cast_menu\":{cast_menu},\"cast_answer\":{cast_answer},\
         \"last_cue\":{last_cue},\"last_count\":{last_count},\
         \"departing\":{departing},\"cards\":{cards},\"buttons\":{buttons},\"shelves\":{shelves}}}",
        cards = cards_json(believed, duel, window),
        buttons = buttons_json(believed),
        shelves = shelves_json(
            shelves,
            duel.view.as_ref().is_some_and(|v| v.day_night.is_some())
        ),
        hovered = duel
            .hovered
            .map_or_else(|| "null".to_string(), |h| quoted(&format!("{h:?}"))),
        autopilot = duel
            .autopilot
            .map_or_else(|| "null".to_string(), |a| quoted(&format!("{a:?}"))),
        reachable = duel.reachable.len(),
        activatable = duel.activatable.len(),
        // Four states that answer silently and are all but invisible in a
        // screenshot: an action queued but never sent, a mana run that owns
        // the next few keys, an ability menu that swallows the keyboard
        // whole, and a card still playing its way off the table. The first
        // three look exactly like "the key did nothing"; the fourth is the
        // opposite problem — it is over in half a second, so a caller that
        // wants to photograph it has to be told when to look.
        outbox = duel.outbox().len(),
        mana_run = duel.mana_run.is_some(),
        menu = duel
            .ability_menu
            .map_or_else(|| "null".to_string(), |m| quoted(&format!("{m:?}"))),
        // And which tap of it the sheet has stepped into, which is the sixth
        // silent state: the sheet is a bubble of one ability's colours and
        // the screenshot of that is a row of five discs — the same picture a
        // permanent whose own pips those are would draw.
        tap = duel
            .asking_tap()
            .map_or_else(|| "null".to_string(), |t| t.to_string()),
        // The seventh and eighth, and they are one state read at its two
        // ends. The cast chooser swallows the keyboard exactly as the ability
        // sheet does, and the way it was answered with then travels silently
        // through a whole mana run to meet the engine's own question several
        // round trips later — so a caller that could see neither could not
        // tell "the chooser is standing" from "the click did nothing", nor
        // "the evoke was chosen" from "the engine picked for us again". See
        // [`crate::CastMenu`].
        cast_menu = cast_menu_json(duel),
        cast_answer = cast_answer_json(duel),
        // The fifth thing that happens without leaving a mark on the screen,
        // and the only one that is meant to leave none: the client decides
        // what is worth hearing (`baylee_client_core::cue`) before anything
        // can play it, so the last cue is how that decision is *proved* —
        // by a read, rather than by somebody listening at the right moment.
        last_cue = duel
            .cues
            .last()
            .map_or_else(|| "null".to_string(), |beat| quoted(beat.cue.name())),
        // And how many of it, which is the half a name cannot carry: three
        // cards drawn and one drawn are the same cue and two different
        // sounds, so a harness that could read only the name could not tell a
        // burst from a tap. `0` when nothing has been heard yet, and `1` for
        // every cue that has no amount in it.
        last_count = duel.cues.last().map_or(0, |beat| beat.count),
    )
}

/// The cast chooser, while it stands: which card, how many ways, which row.
///
/// The near end of a state that is invisible in a screenshot in the way the
/// ability sheet's is — it swallows the keyboard, and a caller that could not
/// see it could not tell it from a click that did nothing.
fn cast_menu_json(duel: &Duel) -> String {
    duel.cast_menu.as_ref().map_or_else(
        || "null".to_string(),
        |menu| {
            format!(
                "{{\"card\":{},\"ways\":{},\"pick\":{}}}",
                menu.card.slot(),
                menu.modes.len(),
                menu.pick
            )
        },
    )
}

/// The far end of the same state: the way the player picked, still owed to a
/// question the engine has not asked yet.
///
/// It travels through a whole mana run — several round trips — before it is
/// spent, and nothing on the screen says so. Without it a caller cannot tell
/// "the evoke was chosen" from "the engine picked for us again", which is the
/// distinction the whole of [`crate::CastMenu`] exists to make.
fn cast_answer_json(duel: &Duel) -> String {
    duel.cast_answer.as_ref().map_or_else(
        || "null".to_string(),
        |(card, kind)| {
            format!(
                "{{\"card\":{},\"kind\":{}}}",
                card.slot(),
                quoted(&format!("{kind:?}"))
            )
        },
    )
}

/// Where every drawn card is on screen, and which card it is.
///
/// This is the endpoint's answer to the thing that has cost this harness the
/// most time by a distance: **finding a card to click**. The advice was to
/// read the object out of the view, the lane out of the board and the pixels
/// out of a screenshot — three lookups, the last of them by eye on a
/// downscaled image, and every one of them repeated after the lane repacked.
/// `at_x`/`at_y` are logical pixels and go straight into `/pointer`.
///
/// **Every part of the question**, because a caller that can find a permanent
/// but not a card in hand still cannot play a game, and one that can find
/// both but not a spell on the stack cannot answer a counterspell: `zone` is
/// `table` for the 3D scene, `hand` for the row and `stack` for the panel,
/// and all three answer in the same logical pixels, so a caller need not know
/// which kind of thing it is clicking.
///
/// Two things it is careful about. A table card's rect is measured from the
/// **live** `Transform`, so a card mid-glide reports where it is rather than
/// where it is going. And the box is the card's own four corners put through
/// that transform, so a tapped card reports the wider, shorter box it
/// actually covers rather than an upright one around its middle. The height a
/// card is drawn at is *not* one of the careful parts: `CARD_LIFT` moves a
/// card 0.14 px at a duel, which is why aiming at the felt under one has
/// worked all along.
fn cards_json(believed: &Believed, duel: &Duel, window: Vec2) -> String {
    let Some(rig) = believed.rig.as_deref().and_then(|shown| shown.rig()) else {
        return "null".to_string();
    };
    if window.x <= 0.0 || window.y <= 0.0 {
        return "null".to_string();
    }
    let lens = crate::table::Lens::new(rig, window);
    let on_the_table = believed.cards.iter().filter_map(|(visual, at)| {
        let (mid, size) = crate::table::card_box(&lens, at)?;
        Some(card_row(
            duel,
            "table",
            visual.object,
            visual.count,
            mid,
            size,
        ))
    });
    // The hand is `bevy_ui` and needs no projection at all: the layout has
    // already put the node somewhere, in *physical* pixels, and
    // `inverse_scale_factor` is the way back to the logical ones `/pointer`
    // speaks. Reading the computed node rather than recomputing the row's
    // arithmetic is also what carries the scroll offset and whatever `touch`
    // has the card doing under the finger.
    let in_the_hand = believed.hand.iter().map(|(visual, computed, place)| {
        let scale = computed.inverse_scale_factor;
        card_row(
            duel,
            "hand",
            visual.object,
            1,
            place.translation * scale,
            computed.size() * scale,
        )
    });
    // The stack reads exactly like the hand — a `bevy_ui` node whose computed
    // box is already in physical pixels — and reports the *row*, not the
    // picture on it, because the row is what answers a click now.
    let on_the_stack = believed.stack.iter().map(|(visual, computed, place)| {
        let scale = computed.inverse_scale_factor;
        card_row(
            duel,
            "stack",
            visual.object,
            1,
            place.translation * scale,
            computed.size() * scale,
        )
    });
    // By zone and then by object, so two runs of the same board answer in the
    // same order and a diff between them is about the table rather than about
    // the ECS.
    let mut rows: Vec<(&str, u32, String)> = on_the_table
        .chain(in_the_hand)
        .chain(on_the_stack)
        .collect();
    rows.sort_unstable_by_key(|(zone, object, _)| (*zone, *object));
    let rows: Vec<String> = rows.into_iter().map(|(_, _, row)| row).collect();
    format!("[{}]", rows.join(","))
}

/// Where the answers are: the shelf's buttons, the two choosers, and
/// everything that acts on the game without answering it.
///
/// `kind` says which list a button came from and `label` which one it is —
/// the prompt or menu action by name (`Yes`, `No`, `Confirm`,
/// `DeclareNothing`, `Step(1)`, `Concede`, `HoldForStack`), and a position for
/// the ability and choice rows, which is what those carry themselves: both are
/// rebuilt from the current `LegalActions` when pressed, so an index is the
/// only stable handle there is.
///
/// This exists because the keyboard does not reach all of it. `Yes` and `No`
/// have no binding at all — see `docs/observed-faults.md` — so without these
/// coordinates a driven client stops dead at the first shockland.
fn buttons_json(believed: &Believed) -> String {
    let mut rows: Vec<String> = Vec::new();
    let mut push = |kind: &str, label: String, node: &bevy::ui::ComputedNode, at: Vec2| {
        let scale = node.inverse_scale_factor;
        let size = node.size() * scale;
        let mid = at * scale;
        rows.push(format!(
            "{{\"kind\":\"{kind}\",\"label\":{label},\"at_x\":{x:.1},\"at_y\":{y:.1},\
             \"w\":{w:.1},\"h\":{h:.1}}}",
            label = quoted(&label),
            x = mid.x,
            y = mid.y,
            w = size.x,
            h = size.y,
        ));
    };
    for (button, node, place) in &believed.prompts {
        push(
            "prompt",
            format!("{:?}", button.action),
            node,
            place.translation,
        );
    }
    for (button, node, place) in &believed.abilities {
        push("ability", button.index.to_string(), node, place.translation);
    }
    for (button, node, place) in &believed.choices {
        push("choice", button.index.to_string(), node, place.translation);
    }
    for (button, node, place) in &believed.menus {
        push(
            "menu",
            format!("{:?}", button.action),
            node,
            place.translation,
        );
    }
    rows.sort_unstable();
    format!("[{}]", rows.join(","))
}

/// One card of the answer, wherever it is drawn.
fn card_row(
    duel: &Duel,
    zone: &'static str,
    object: baylee_core::ids::ObjectId,
    count: usize,
    mid: Vec2,
    size: Vec2,
) -> (&'static str, u32, String) {
    (
        zone,
        object.slot(),
        format!(
            "{{\"object\":{object},\"zone\":\"{zone}\",\"count\":{count},\"name\":{name},\
             \"at_x\":{x:.1},\"at_y\":{y:.1},\"w\":{w:.1},\"h\":{h:.1}}}",
            object = object.slot(),
            name = name_of(duel, object),
            x = mid.x,
            y = mid.y,
            w = size.x,
            h = size.y,
        ),
    )
}

/// One string, as JSON.
///
/// `str::escape_default` is the obvious thing and is **not** JSON: it writes
/// an apostrophe as `\'` and anything outside ASCII as `\u{2014}`, neither of
/// which a JSON parser accepts. `Earth King's Lieutenant` is in the dev
/// board, so the first card name carrying an apostrophe made the whole dump
/// unreadable — every field in it, not just the name. Only the quote, the
/// backslash and the control characters need escaping; UTF-8 is already JSON.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// What the board model calls this object, if it is drawing it.
///
/// The board's name and not the view's: it is the name on the card the player
/// is looking at, and for a group of identical permanents it is the one name
/// that stands for all of them. The lanes, the piles and the hand are all
/// searched, because every one of them draws a card a caller may want to
/// click and the handle has to be the same in each.
///
/// `null` is a real answer rather than a failure: a library is face down to
/// everybody, its owner included (CR 401.2), so it has no name to give. The
/// object is still there and still clickable.
fn name_of(duel: &Duel, object: baylee_core::ids::ObjectId) -> String {
    let Some(board) = duel.board.as_ref() else {
        return "null".to_string();
    };
    board
        .pods
        .iter()
        .flat_map(|pod| pod.lanes.iter())
        .flat_map(|lane| lane.groups.iter())
        .find(|group| group.representative == object)
        .map(|group| group.name.clone())
        .or_else(|| {
            board
                .pods
                .iter()
                .flat_map(|pod| pod.piles.iter())
                .find(|pile| pile.top == Some(object))
                .and_then(|pile| pile.name.clone())
        })
        .or_else(|| {
            board
                .hand
                .iter()
                .find(|held| held.id == object)
                .map(|held| held.name.clone())
        })
        .or_else(|| {
            board
                .stack
                .iter()
                .find(|item| item.id == object)
                .map(|item| item.name.clone())
        })
        .map_or_else(|| "null".to_string(), |name| quoted(&name))
}

/// Where each seat's bar is drawn, and what it was allowed to be.
///
/// A bar is placed from a projection, not from a layout pass, so "the bar is
/// in the wrong place" is a claim about arithmetic that a screenshot can only
/// ever suggest. These are the numbers the placement was made from, in the
/// same logical pixels `/pointer` takes: `mid` is the centre the box is hung
/// on, `along` and `depth` are the projected ledge, and `ink` is what the
/// depth has to be able to hold. A shelf whose `ink` is close to its `depth`
/// is a bar about to stand on the creature lane behind it.
fn shelves_json(shelves: Option<&crate::hud::Shelves>, designated: bool) -> String {
    let Some(shelves) = shelves else {
        return "null".to_string();
    };
    let rows: Vec<String> = shelves
        .0
        .iter()
        .map(|(player, shelf)| {
            let box_size = shelf.box_size(designated);
            format!(
                "{{\"player\":{player},\"mid_x\":{mx:.1},\"mid_y\":{my:.1},\
                 \"along\":{along:.1},\"depth\":{depth:.1},\"tilt\":{tilt:.3},\
                 \"density\":\"{density:?}\",\"box_w\":{bw:.1},\"box_h\":{bh:.1},\
                 \"ink\":{ink:.1}}}",
                player = player.get(),
                mx = shelf.middle.x,
                my = shelf.middle.y,
                along = shelf.along,
                depth = shelf.depth,
                tilt = shelf.tilt,
                density = shelf.density,
                bw = box_size.x,
                bh = box_size.y,
                ink = shelf.density.ink_height(),
            )
        })
        .collect();
    format!("[{}]", rows.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::message::Messages;

    /// A key named either way round says the same thing.
    ///
    /// `harness_alias` already let a caller write `2` for `Digit2`, but only
    /// the *physical* code went both ways: `Digit2` reported no logical key
    /// at all, so a reader that takes its digits as characters — the ability
    /// sheet, the subtype filter, a number entry — saw the short spelling and
    /// not the canonical one. The harness must not disagree with itself
    /// about one key.
    #[test]
    fn a_key_named_either_way_round_produces_the_same_character() {
        assert_eq!(logical_key("Digit2"), logical_key("2"));
        assert_eq!(logical_key("KeyG"), logical_key("g"));
        assert_eq!(logical_key("Digit0"), Key::Character("0".into()));
        assert_eq!(logical_key("KeyA"), Key::Character("a".into()));
        // Still no logical key where there is none to report.
        assert!(matches!(logical_key("F5"), Key::Unidentified(_)));
        assert!(matches!(logical_key("ShiftLeft"), Key::Unidentified(_)));
    }

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

    /// A lens onto a duel at a plausible window, and the local seat it looks
    /// at — everything the rect tests need and nothing else.
    fn a_table() -> (crate::table::Lens, baylee_client_core::layout::SeatSlot) {
        use crate::table::Canvas;
        use baylee_client_core::layout::TableLayout;
        let canvas = Canvas::hud(Vec2::new(1728.0, 1052.0));
        let seats: Vec<_> = (0..2).map(baylee_core::ids::PlayerId::new).collect();
        let table = TableLayout::new(&seats, canvas.aspect(), None);
        let lens =
            crate::table::Lens::new(crate::table::CameraRig::home(&table, canvas), canvas.window);
        let slot = *table.local().expect("a local seat");
        (lens, slot)
    }

    /// A key spelled the way a caller spells it.
    ///
    /// The counter-half is what makes this worth a test: an alias that
    /// accepted anything would turn a typo into a key press somewhere else on
    /// the board, and the whole reason this exists is that a *refused* key
    /// and a key that did nothing are indistinguishable from outside.
    #[test]
    fn a_bare_letter_is_the_key_it_obviously_means() {
        use bevy::prelude::KeyCode;
        for (name, want) in [
            ("Y", KeyCode::KeyY),
            ("y", KeyCode::KeyY),
            ("N", KeyCode::KeyN),
            ("1", KeyCode::Digit1),
        ] {
            assert_eq!(super::harness_alias(name), Some(want), "{name}");
        }
        for name in ["", "KeyY", "Yes", "-", "Space"] {
            assert_eq!(super::harness_alias(name), None, "{name}");
        }
    }

    /// A press that is held does not have to say `press` as well.
    ///
    /// The counter-halves are the point again: a bare move must stay a bare
    /// move, or every `/pointer` call that only aims the cursor would press
    /// the button under it — and `release` must keep winning over `press`,
    /// because a call that sends both cannot mean "press and then hold".
    #[test]
    fn a_held_press_does_not_have_to_say_press_as_well() {
        let word = |body: &str| {
            super::button_deed(body)
                .expect("a well-formed body")
                .map(|deed| deed.word)
        };
        assert_eq!(word(r#"{"x":1,"y":2}"#), None, "a move is only a move");
        assert_eq!(word(r#"{"x":1,"y":2,"press":true}"#), Some("clicked"));
        assert_eq!(word(r#"{"hold":true}"#), Some("held"), "hold implies press");
        assert_eq!(word(r#"{"press":true,"hold":true}"#), Some("held"));
        assert_eq!(word(r#"{"release":true}"#), Some("released"));
        assert_eq!(
            word(r#"{"press":true,"release":true}"#),
            Some("released"),
            "release wins over press"
        );
        assert!(
            super::button_deed(r#"{"press":true,"button":"thumb"}"#).is_err(),
            "an unknown button is still refused"
        );
    }

    /// Every string in the dump is JSON, apostrophes and em dashes included.
    ///
    /// Against `serde_json` rather than against a written-out expectation,
    /// because the claim is that a parser accepts it and not that it looks a
    /// particular way. `str::escape_default` passes neither test: it writes
    /// `\'` and `\u{2014}`, and the dev board's `Earth King's Lieutenant` was
    /// enough to make the whole `/state` answer unreadable — every field in
    /// it, not only the name.
    #[test]
    fn a_name_with_an_apostrophe_in_it_is_still_json() {
        for text in [
            "Earth King's Lieutenant",
            "a \"quoted\" name",
            "a back\\slash",
            "an em — dash",
            "a\nnewline",
        ] {
            let json = format!("{{\"name\":{}}}", quoted(text));
            let back: serde_json::Value =
                serde_json::from_str(&json).unwrap_or_else(|e| panic!("{json} is not JSON: {e}"));
            assert_eq!(back["name"], text, "it came back changed: {json}");
        }
    }

    /// A card's box is as big as a card is drawn.
    ///
    /// Against a scale measured from the felt itself — two points a table
    /// unit apart, projected — rather than against a number written down
    /// here, because the camera's distance is computed from the window and
    /// any constant would be a copy of it. A rect built from a unit quad, or
    /// from extents instead of half-extents, misses by a factor and fails.
    #[test]
    fn a_cards_box_is_the_size_a_card_is_drawn() {
        use baylee_client_core::layout::{CARD_HEIGHT, CARD_WIDTH};
        let (lens, slot) = a_table();
        let at = slot.lane_center(baylee_client_core::layout::LaneKind::Creatures);
        let middle = lens.project(at).expect("the lane is in front of the eye");
        let across = (lens.project(at + Vec2::X).expect("and so is a unit east") - middle).length();
        let along = (lens.project(at + Vec2::Y).expect("and a unit north") - middle).length();

        let card = Transform::from_translation(crate::table::to_world(at, crate::table::CARD_LIFT))
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2));
        let (_, size) = crate::table::card_box(&lens, &card).expect("a card on it");

        let want = Vec2::new(CARD_WIDTH * across, CARD_HEIGHT * along);
        assert!(
            (size.x - want.x).abs() < want.x * 0.05 && (size.y - want.y).abs() < want.y * 0.05,
            "a card covers {size:?}, and a card's worth of felt covers {want:?}"
        );
    }

    /// A tapped card covers a wider, shorter box, and the rect says so.
    ///
    /// The claim is that the corners are turned by the card's own transform
    /// rather than assumed to be axis-aligned around it: a tapped permanent
    /// is the commonest thing on a board and it is a quarter turn over.
    #[test]
    fn a_tapped_card_reports_the_box_it_actually_covers() {
        let (lens, slot) = a_table();
        let at = slot.lane_center(baylee_client_core::layout::LaneKind::Creatures);
        let world = crate::table::to_world(at, crate::table::CARD_LIFT);
        let flat = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        let upright = Transform::from_translation(world).with_rotation(flat);
        let tapped = Transform::from_translation(world)
            .with_rotation(flat * Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2));

        let (_, standing) =
            crate::table::card_box(&lens, &upright).expect("a card in front of the eye");
        let (_, turned) = crate::table::card_box(&lens, &tapped).expect("the same card, tapped");
        assert!(
            standing.y > standing.x,
            "an untapped card is taller than it is wide: {standing:?}"
        );
        assert!(
            turned.x > turned.y,
            "a tapped one is wider than it is tall: {turned:?}"
        );
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
            .init_resource::<Time<Virtual>>()
            .insert_resource(DevControl {
                jobs: Mutex::new(rx),
                held: Vec::new(),
                clicking: Vec::new(),
                stepping: None,
                frame: 0,
            })
            .add_systems(Update, pump);
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        (app, tx)
    }

    /// Queues one request and hands back the channel its answer will arrive
    /// on — which is not always the same frame.
    fn ask(tx: &Sender<Job>, path: &str, body: &str) -> Receiver<String> {
        let (reply, answers) = channel();
        tx.send(Job {
            path: path.to_string(),
            body: body.to_string(),
            reply,
        })
        .unwrap();
        answers
    }

    /// A hundred milliseconds, which is what one frame of the clock harness
    /// is worth in raw time.
    const RAW: std::time::Duration = std::time::Duration::from_millis(100);

    /// The harness with a real clock in it, wound by hand.
    ///
    /// `TimeUpdateStrategy` is bevy's own seam for this, and it is what lets
    /// the three clock routes be tested on what the picture does rather than
    /// on a flag. It is a *second* harness rather than the first one grown,
    /// because `TimePlugin` also takes over when message buffers are swapped
    /// — it holds them an extra frame so a fixed-update schedule cannot miss
    /// one — and the click and wheel tests read exactly that buffer.
    fn clock_harness() -> (App, Sender<Job>) {
        let (mut app, tx) = harness();
        app.add_plugins(bevy::time::TimePlugin)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(RAW));
        (app, tx)
    }

    /// How far the virtual clock moved on the frame just run.
    ///
    /// The assertion that matters is on the *delta*, never on the flag: a
    /// route that set `paused` and left the clock running would pass every
    /// test written against `is_paused`, and the whole point of these three
    /// endpoints is what the picture does.
    ///
    /// It lags a request by one frame, and honestly so. Bevy sets the clock
    /// in `First` and `pump` runs in `PreUpdate`, so the frame a route is
    /// answered on already has its delta.
    fn advanced(app: &App) -> std::time::Duration {
        app.world().resource::<Time<Virtual>>().delta()
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
    ///
    /// Five frames, and the two that are not the press and the release are
    /// the repairs `AL6a` asked for. The move is written **twice**, a whole
    /// frame apart, because a single one is sometimes simply lost — measured
    /// live as a pointer put on a card and `hovered` read fifteen times as
    /// `None`, where sending the same move again named the object at once.
    /// And the last frame writes nothing: the answer leaves after the frame
    /// in which the release was *read*, not the one in which it was written,
    /// or a caller that clicks and then asks `/state` is shown the board from
    /// before its own click.
    #[test]
    fn a_click_is_the_move_twice_then_a_press_a_release_and_a_frame_to_read_it() {
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
        assert_eq!(
            window_events(&app),
            ["move 40 60"],
            "the same move again, a frame later: repeated sending is what repairs a lost one"
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
        assert!(
            answers.try_recv().is_err(),
            "the release has been written and not yet read by anything"
        );

        app.update();
        assert!(
            window_events(&app).is_empty(),
            "the settling frame writes nothing; it exists to be read in"
        );
        assert!(answers.try_recv().unwrap().contains("\"clicked\":true"));
    }

    /// A held press stays down, and the release that lifts it is a call of
    /// its own.
    ///
    /// Without this the harness could not photograph anything that exists
    /// only while a button is down — a drag, or a card giving way under the
    /// finger — because press and release were one call and a screenshot
    /// cannot be asked for in between. The answer word says which of the
    /// three a caller got, so a script cannot mistake a hold for a click.
    #[test]
    fn a_press_can_be_held_and_let_go_of_separately() {
        let (mut app, tx) = harness();
        let (reply, answers) = channel();
        tx.send(Job {
            path: "/pointer".to_string(),
            body: r#"{"x":40,"y":60,"press":true,"hold":true}"#.to_string(),
            reply,
        })
        .unwrap();

        app.update();
        assert_eq!(window_events(&app), ["move 40 60"]);
        app.update();
        assert_eq!(window_events(&app), ["move 40 60"], "aimed twice");
        app.update();
        assert_eq!(window_events(&app), ["press"]);
        app.update();
        assert!(answers.try_recv().unwrap().contains("\"held\":true"));

        // The button is still down, which is the whole point: the ordinary
        // click would have let go by now.
        assert_eq!(window_events(&app), [] as [String; 0]);
        assert!(
            app.world()
                .resource::<ButtonInput<MouseButton>>()
                .pressed(MouseButton::Left)
        );

        let (reply, answers) = channel();
        tx.send(Job {
            path: "/pointer".to_string(),
            body: r#"{"release":true}"#.to_string(),
            reply,
        })
        .unwrap();
        // The job is drained after the stages have been played, so its own
        // first stage is the next frame's — and the aim comes before the
        // release for a call that carries no coordinates too, because the
        // cursor is put back where it already was and the picking backend is
        // reminded of it.
        app.update();
        app.update();
        app.update();
        assert_eq!(window_events(&app), ["release"]);
        assert!(
            !app.world()
                .resource::<ButtonInput<MouseButton>>()
                .pressed(MouseButton::Left)
        );
        app.update();
        assert!(answers.try_recv().unwrap().contains("\"released\":true"));
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

    /// A move without `press` presses nothing — the hover path, which is how
    /// a card preview is opened — and it is sent twice and answered late all
    /// the same.
    ///
    /// It used to be answered on the frame it was written, which made it the
    /// one call the caller could not trust: a hover read straight after was
    /// read a frame before anything had looked at the move, and the lost
    /// move `AL6a` measured was a bare one. The repeat costs two frames and
    /// buys a `/pointer` whose answer means the pointer is there.
    #[test]
    fn a_move_without_a_press_is_only_a_move_and_is_still_sent_twice() {
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
        assert!(
            answers.try_recv().is_err(),
            "answered once the move has been read, not once it has been written"
        );
        app.update();
        assert_eq!(window_events(&app), ["move 10 20"]);
        app.update();
        assert!(window_events(&app).is_empty(), "and nothing was pressed");
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

    /// A tenth speed is a tenth of the picture, not a flag saying so.
    #[test]
    fn a_slowed_clock_moves_a_tenth_as_far() {
        let (mut app, tx) = clock_harness();
        let answers = ask(&tx, "/timescale", r#"{"speed":0.1}"#);
        app.update();
        assert!(answers.try_recv().unwrap().contains("\"speed\":0.1"));
        app.update();
        assert_eq!(advanced(&app), RAW / 10);

        // Zero is not a pause, and a refused request must leave the clock
        // where it was rather than half-applying itself.
        let refused = ask(&tx, "/timescale", r#"{"speed":0}"#);
        app.update();
        assert!(refused.try_recv().unwrap().contains("\"error\""));
        app.update();
        assert_eq!(advanced(&app), RAW / 10);
    }

    /// The harness has to keep answering while the picture is stopped, which
    /// is why `pump` counts frames and not seconds. A paused clock that took
    /// the harness with it would be a screenshot nobody could ever ask for.
    #[test]
    fn a_pause_stops_the_picture_and_not_the_harness() {
        let (mut app, tx) = clock_harness();
        let paused = ask(&tx, "/pause", "{}");
        app.update();
        assert!(paused.try_recv().unwrap().contains("\"paused\":true"));
        app.update();
        assert_eq!(advanced(&app), std::time::Duration::ZERO);

        let health = ask(&tx, "/health", "{}");
        app.update();
        let answer = health.try_recv().expect("a stopped clock still answers");
        assert!(answer.contains("\"paused\":true"), "got {answer}");
        assert!(answer.contains("\"frame\":3"), "got {answer}");

        let running = ask(&tx, "/pause", r#"{"paused":false}"#);
        app.update();
        assert!(running.try_recv().unwrap().contains("\"paused\":false"));
        app.update();
        assert_eq!(advanced(&app), RAW);
    }

    /// A step is counted in frames and answered at the end of them, for the
    /// reason a click is: a caller that was told "ok" up front would take its
    /// screenshot of the frame it started from.
    #[test]
    fn a_step_runs_the_frames_it_asked_for_and_then_stops_again() {
        let (mut app, tx) = clock_harness();
        app.world_mut().resource_mut::<Time<Virtual>>().pause();

        let stepped = ask(&tx, "/step", r#"{"frames":3}"#);
        app.update();
        for frame in 1..=3 {
            assert!(
                stepped.try_recv().is_err(),
                "answered before frame {frame} of 3"
            );
            app.update();
            assert_eq!(
                advanced(&app),
                RAW,
                "the clock was still stopped on frame {frame} of 3"
            );
        }

        assert!(stepped.try_recv().unwrap().contains("\"frames\":3"));
        app.update();
        assert_eq!(
            advanced(&app),
            std::time::Duration::ZERO,
            "the clock was left running after the step counted out"
        );
    }

    /// Two steps at once would share one countdown and one reply channel, so
    /// the second is refused rather than quietly stealing the first.
    #[test]
    fn a_second_step_is_refused_while_the_first_is_running() {
        let (mut app, tx) = clock_harness();
        app.world_mut().resource_mut::<Time<Virtual>>().pause();
        let first = ask(&tx, "/step", r#"{"frames":4}"#);
        app.update();

        let second = ask(&tx, "/step", r#"{"frames":1}"#);
        app.update();
        assert!(second.try_recv().unwrap().contains("\"error\""));
        assert!(first.try_recv().is_err(), "the first step lost its answer");
    }
}
