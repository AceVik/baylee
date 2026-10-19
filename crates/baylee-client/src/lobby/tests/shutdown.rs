//! Real plugin shutdown must not turn a disconnected seat into a departure.

#[allow(clippy::wildcard_imports)] // the lobby's own test vocabulary
use super::*;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// A private, bounded HTTP recorder. No GUI, external service or real session.
struct Gateway {
    base: String,
    stop: Arc<AtomicBool>,
    lines: Arc<Mutex<Vec<String>>>,
    worker: Option<JoinHandle<()>>,
}

impl Gateway {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let lines = Arc::new(Mutex::new(Vec::new()));
        let stopping = Arc::clone(&stop);
        let observed = Arc::clone(&lines);
        let worker = thread::spawn(move || {
            while !stopping.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => Self::answer(stream, &observed),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            base,
            stop,
            lines,
            worker: Some(worker),
        }
    }

    fn answer(mut stream: TcpStream, lines: &Mutex<Vec<String>>) {
        if stream
            .set_write_timeout(Some(Duration::from_millis(250)))
            .is_err()
        {
            return;
        }
        let until = Instant::now() + Duration::from_secs(1);
        let mut bytes = Vec::new();
        let mut buffer = [0u8; 1024];
        while bytes.len() < 16 * 1024 {
            let remaining = until.saturating_duration_since(Instant::now());
            if remaining.is_zero() || stream.set_read_timeout(Some(remaining)).is_err() {
                return;
            }
            let Ok(count) = stream.read(&mut buffer) else {
                return;
            };
            if count == 0 {
                return;
            }
            bytes.extend_from_slice(&buffer[..count]);
            if bytes.windows(4).any(|part| part == b"\r\n\r\n") {
                break;
            }
        }
        let first = String::from_utf8_lossy(&bytes)
            .lines()
            .next()
            .unwrap_or_default()
            .to_owned();
        lines.lock().unwrap().push(first);
        let _ = stream.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
        );
    }

    /// A round trip proves the recorder is alive before and after teardown.
    fn probe(&self, path: &str) {
        let response =
            crate::transport::fetch_blocking(&ehttp::Request::get(format!("{}{path}", self.base)))
                .unwrap();
        assert_eq!(response.status, 200);
        assert!(
            self.lines
                .lock()
                .unwrap()
                .iter()
                .any(|line| line.starts_with(&format!("GET {path} ")))
        );
    }

    fn assert_no_departure(&self) {
        self.probe("/after-shutdown");
        let lines = self.lines.lock().unwrap();
        assert!(
            !lines
                .iter()
                .any(|line| line.starts_with("POST /lobby/depart ")),
            "closing the app must preserve its hosted chair: {lines:?}"
        );
    }
}

impl Drop for Gateway {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let result = worker.join();
            if !thread::panicking() {
                result.expect("the owned HTTP recorder exited cleanly");
            }
        }
    }
}

fn seated_app(gateway: &Gateway) -> App {
    let mut app = headless();
    stocked(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.gateway.clone_from(&gateway.base);
        state.lobby.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "shutdown-game".to_owned(),
            seat: 0,
            seat_token: "synthetic-seat".to_owned(),
            local: false,
        }));
        // A real host would dial this handover. This test needs only the
        // signed-in lobby and its actual plugin lifecycle, not a game server.
        state.connected = true;
    }
    phase(&mut app, DuelPhase::Playing);
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.token(),
        Some("tok")
    );
    gateway.probe("/before-shutdown");
    app
}

#[test]
fn closing_the_native_window_does_not_depart_the_hosted_game() {
    let gateway = Gateway::new();
    let mut app = seated_app(&gateway);
    // WindowPlugin's real close/exit systems, without Winit or an OS window.
    app.add_plugins(bevy::window::WindowPlugin::default());
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .write_message(bevy::window::WindowCloseRequested { window });
    app.update();
    assert!(
        app.world()
            .entity(window)
            .contains::<bevy::window::ClosingWindow>()
    );
    // Bevy marks ClosingWindow first and despawns it on the following frame.
    app.update();
    assert_eq!(app.should_exit(), Some(AppExit::Success));
    drop(app);
    gateway.assert_no_departure();
}

#[test]
fn clearing_the_world_without_an_exit_message_preserves_the_hosted_game() {
    let gateway = Gateway::new();
    let mut app = seated_app(&gateway);
    assert!(app.world().resource::<Messages<AppExit>>().is_empty());
    // This is exactly Winit's exiting callback, including macOS Quit where
    // no AppExit message need reach a schedule before resources are dropped.
    app.world_mut().clear_all();
    assert!(!app.world().contains_resource::<LobbyState>());
    drop(app);
    gateway.assert_no_departure();
}
