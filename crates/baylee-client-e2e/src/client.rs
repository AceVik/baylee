//! A client under test: started isolated, driven and read through its
//! harness, and always closed.
//!
//! Each client gets directories of its own for configuration, cache and data,
//! a free loopback port, a passive and silent window at scale 1, no tours,
//! and dead loopback addresses for the gateway and the updater: without
//! `BAYLEE_GATEWAY` a client takes the production gateway, even one that is
//! only ever asked to play offline. Dropping a [`Client`] kills it and waits
//! for it; [`Client::close`] also proves its port closed. A test process that
//! aborts leaves the client to its own idle exit (`BAYLEE_DEV_IDLE_EXIT`).
use crate::{
    devctl::Devctl,
    monitor::Monitor,
    screen::{Rect, Screen, Target},
    viewport::{TextStep, Viewport},
};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Map, Value, json};
use std::{
    fs,
    io::ErrorKind,
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

/// How a client is started. Everything has a default; a test names only
/// what it is about.
#[derive(Clone, Debug)]
pub struct Launch {
    viewport: Viewport,
    text_step: TextStep,
    lang: String,
    tours: bool,
    settings: Map<String, Value>,
    env: Vec<(String, String)>,
    monitor: Option<Monitor>,
}

impl Default for Launch {
    fn default() -> Self {
        Self {
            viewport: Viewport::LAPTOP,
            text_step: TextStep::M,
            lang: "en".to_owned(),
            tours: false,
            settings: Map::new(),
            env: Vec::new(),
            monitor: None,
        }
    }
}

impl Launch {
    /// The defaults: 1440 × 900, English, text step M, no tours.
    pub fn new() -> Self {
        Self::default()
    }

    /// The window size to open at.
    #[must_use]
    pub fn viewport(mut self, viewport: Viewport) -> Self {
        self.viewport = viewport;
        self
    }

    /// The interface's text step (fixed for the client's life:
    /// `BAYLEE_TEXT_STEP`).
    #[must_use]
    pub fn text_step(mut self, step: TextStep) -> Self {
        self.text_step = step;
        self
    }

    /// The interface language (`"en"`, `"de"`).
    #[must_use]
    pub fn lang(mut self, lang: &str) -> Self {
        lang.clone_into(&mut self.lang);
        self
    }

    /// Lets the lobby and table tours run (off by default).
    #[must_use]
    pub fn tours(mut self, on: bool) -> Self {
        self.tours = on;
        self
    }

    /// One field of the client's `client-settings.json` (`"reduce_motion"`, …),
    /// written before it starts.
    #[must_use]
    pub fn setting(mut self, key: &str, value: Value) -> Self {
        self.settings.insert(key.to_owned(), value);
        self
    }

    /// An environment variable for the client.
    #[must_use]
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.to_owned(), value.to_owned()));
        self
    }

    /// The screen to open on, instead of `BAYLEE_E2E_MONITOR`'s.
    #[must_use]
    pub fn monitor(mut self, monitor: Monitor) -> Self {
        self.monitor = Some(monitor);
        self
    }

    /// The viewport it opens at.
    pub fn opens_at(&self) -> Viewport {
        self.viewport
    }
}

/// A key, with the modifiers held while it is pressed. Physical names
/// (`KeyK`, `Digit1`, `ArrowLeft`, `Space`, `Enter`); a single letter or
/// digit is accepted for its key.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Key {
    name: String,
    /// The modifiers held, by the harness's names for them.
    held: Vec<&'static str>,
    produces: Option<String>,
}

impl Key {
    /// A key by name.
    pub fn named(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            ..Self::default()
        }
    }

    fn holding(mut self, modifier: &'static str) -> Self {
        self.held.push(modifier);
        self
    }

    /// With Shift held.
    #[must_use]
    pub fn shift(self) -> Self {
        self.holding("shift")
    }

    /// With Control held.
    #[must_use]
    pub fn ctrl(self) -> Self {
        self.holding("ctrl")
    }

    /// With Alt/Option held.
    #[must_use]
    pub fn alt(self) -> Self {
        self.holding("alt")
    }

    /// With Command (macOS) or the Windows key held.
    #[must_use]
    pub fn command(self) -> Self {
        self.holding("super")
    }

    /// The character the key types on the layout under test (`Digit7` with
    /// Shift is `/` on a German keyboard).
    #[must_use]
    pub fn producing(mut self, character: &str) -> Self {
        self.produces = Some(character.to_owned());
        self
    }

    fn body(&self) -> Value {
        let mut body = json!({"name": self.name});
        for modifier in &self.held {
            body[*modifier] = json!(true);
        }
        if let Some(character) = &self.produces {
            body["char"] = json!(character);
        }
        body
    }
}

impl From<&str> for Key {
    fn from(name: &str) -> Self {
        Self::named(name)
    }
}

/// How long the waits wait.
const STARTUP: Duration = Duration::from_secs(90);
const WAIT: Duration = Duration::from_secs(20);
const POLL: Duration = Duration::from_millis(100);
const READ: Duration = Duration::from_secs(15);
const SHOT: Duration = Duration::from_secs(120);

/// A running client, owned by one test.
pub struct Client {
    devctl: Devctl,
    child: Option<Child>,
    dir: PathBuf,
    viewport: Viewport,
    shots: u32,
}

impl Client {
    /// Starts the client at `binary` with its files under `dir`, and returns
    /// once the startup veil has lifted and the first screen is still.
    pub fn start(binary: &Path, launch: &Launch, dir: &Path) -> Result<Self> {
        ensure!(
            binary.is_file(),
            "no client at {}: build it with `cargo build -p baylee-client --features dev-control` \
             or run the suite through `cargo dev e2e`",
            binary.display()
        );
        let config = dir.join("config");
        fs::create_dir_all(config.join("baylee"))?;
        let mut settings = json!({
            "lang": launch.lang,
            "feedback_url": "",
            "report_url": "",
            "usage_count": false,
            "gateways": [],
        });
        for (key, value) in &launch.settings {
            settings[key] = value.clone();
        }
        fs::write(
            config.join("baylee/client-settings.json"),
            serde_json::to_vec_pretty(&settings)?,
        )?;
        let (monitor, built_in) = launch
            .monitor
            .map_or_else(Monitor::from_env, Ok)?
            .resolve()?;
        let port = free_port()?;
        let mut command = Command::new(binary);
        command
            .env("XDG_CONFIG_HOME", &config)
            .env("XDG_CACHE_HOME", dir.join("cache"))
            .env("XDG_DATA_HOME", dir.join("data"))
            .env("BAYLEE_DEV_CONTROL", port.to_string())
            .env("BAYLEE_DEV_PASSIVE", "1")
            .env("BAYLEE_DEV_SOUND", "")
            .env("BAYLEE_DEV_SCALE", "1")
            .env("BAYLEE_DEV_WINDOW", launch.viewport.spelled())
            .env("BAYLEE_DEV_IDLE_EXIT", "120")
            .env("BAYLEE_TEXT_STEP", launch.text_step.stored().to_string())
            // A dead loopback port: nothing a test does reaches a real gateway.
            .env("BAYLEE_GATEWAY", "http://127.0.0.1:9")
            .env("BAYLEE_UPDATE_API", "http://127.0.0.1:9/updates-disabled")
            .env_remove("BAYLEE_DEV_MONITOR")
            .stdin(Stdio::null())
            .stdout(fs::File::create(dir.join("client.stdout.log"))?)
            .stderr(fs::File::create(dir.join("client.stderr.log"))?);
        if !launch.tours {
            command.env("BAYLEE_NO_TOURS", "1");
        }
        if let Some(monitor) = &monitor {
            command.env("BAYLEE_DEV_MONITOR", monitor);
        }
        for (key, value) in &launch.env {
            command.env(key, value);
        }
        let child = command
            .spawn()
            .with_context(|| format!("starting {}", binary.display()))?;
        let mut client = Self {
            devctl: Devctl::new(port),
            child: Some(child),
            dir: dir.to_owned(),
            viewport: launch.viewport,
            shots: 0,
        };
        client.await_harness()?;
        if let Some(display) = built_in {
            client.confirm_monitor(display)?;
        }
        client.wait_for("the startup veil to lift", WAIT * 3, Screen::at_rest)?;
        client.confirm_size(launch.viewport)?;
        client.settled()?;
        Ok(client)
    }

    /// The harness answers, or the client died trying.
    fn await_harness(&mut self) -> Result<()> {
        let deadline = Instant::now() + STARTUP;
        loop {
            if self.devctl.get("health", Duration::from_secs(2)).is_ok() {
                return Ok(());
            }
            if let Some(status) = self
                .child
                .as_mut()
                .and_then(|c| c.try_wait().ok().flatten())
            {
                bail!(
                    "the client exited during startup ({status}); was it built with \
                     `--features dev-control`? Its log: {}",
                    self.dir.join("client.stderr.log").display()
                );
            }
            ensure!(
                Instant::now() < deadline,
                "the client's harness did not answer within {STARTUP:?}"
            );
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    /// The built-in display is where the index said it would be.
    fn confirm_monitor(&self, display: crate::monitor::BuiltIn) -> Result<()> {
        let monitors = self.devctl.get("monitors", READ)?;
        let listed = monitors["monitors"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|m| m["index"] == display.index);
        let size = listed.map(|m| (m["width"].as_u64(), m["height"].as_u64()));
        ensure!(
            size == Some((Some(display.width.into()), Some(display.height.into()))),
            "monitor {} is not the built-in display ({}×{}): {monitors}; \
             set BAYLEE_E2E_MONITOR",
            display.index,
            display.width,
            display.height
        );
        Ok(())
    }

    /// The window is the size asked for, or the screen cannot hold it.
    fn confirm_size(&mut self, viewport: Viewport) -> Result<()> {
        let deadline = Instant::now() + WAIT;
        loop {
            let health = self.devctl.get("health", READ)?;
            let size = (number(&health, "width"), number(&health, "height"));
            if viewport.is(size.0, size.1) {
                self.viewport = viewport;
                return Ok(());
            }
            ensure!(
                Instant::now() < deadline,
                "the window stays {}×{} where {viewport} was asked for: \
                 the screen cannot hold it at scale 1",
                size.0,
                size.1
            );
            std::thread::sleep(POLL);
        }
    }

    /// The harness itself, for a route this type has no word for.
    pub fn devctl(&self) -> &Devctl {
        &self.devctl
    }

    /// Where this test's files are: logs, screenshots, failure dumps.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The viewport the window is at.
    pub fn viewport(&self) -> Viewport {
        self.viewport
    }

    /// What the client shows now.
    pub fn screen(&self) -> Result<Screen> {
        let health = self.devctl.get("health", READ)?;
        let state = self.devctl.get("state", READ)?;
        Ok(Screen::new(
            state,
            (number(&health, "width"), number(&health, "height")),
        ))
    }

    /// The screen once nothing on it moves: the startup veil is gone, the
    /// front door is still, and two readings a beat apart draw the same lobby
    /// tree and the same controls. The HUD's nodes (`ui_nodes`) are not
    /// compared: the seat chips and clocks breathe, and a table would never
    /// settle. At the table this waits for the buttons to hold still.
    pub fn settled(&self) -> Result<Screen> {
        let deadline = Instant::now() + WAIT;
        let mut last = self.screen()?;
        loop {
            std::thread::sleep(POLL * 2);
            let now = self.screen()?;
            if now.at_rest() && now.fingerprint() == last.fingerprint() {
                return Ok(now);
            }
            if Instant::now() >= deadline {
                self.keep("unsettled", &now);
                bail!("the screen did not come to rest within {WAIT:?}");
            }
            last = now;
        }
    }

    /// The first screen on which `holds` is true, waiting up to 20 s.
    pub fn wait_until(&self, what: &str, holds: impl Fn(&Screen) -> bool) -> Result<Screen> {
        self.wait_for(what, WAIT, holds)
    }

    /// [`Client::wait_until`] with a deadline of its own.
    pub fn wait_for(
        &self,
        what: &str,
        limit: Duration,
        holds: impl Fn(&Screen) -> bool,
    ) -> Result<Screen> {
        let deadline = Instant::now() + limit;
        loop {
            let screen = self.screen()?;
            if holds(&screen) {
                return Ok(screen);
            }
            if Instant::now() >= deadline {
                self.keep("timeout", &screen);
                bail!("waited {limit:?} for {what}");
            }
            std::thread::sleep(POLL);
        }
    }

    /// Clicks a target where it is drawn now.
    pub fn click(&self, target: impl Into<Target>) -> Result<()> {
        let screen = self.screen()?;
        let rect = screen
            .locate(&target.into())
            .inspect_err(|_| self.keep("miss", &screen))?;
        let (x, y) = rect.centre();
        screen.refuse_a_real_gateway(x, y)?;
        self.devctl.post(
            "pointer",
            &json!({"x": x, "y": y, "button": "left", "press": true}),
        )?;
        Ok(())
    }

    /// Clicks with the right button.
    pub fn right_click(&self, target: impl Into<Target>) -> Result<()> {
        let rect = self.locate(&target.into())?;
        let (x, y) = rect.centre();
        self.screen()?.refuse_a_real_gateway(x, y)?;
        self.devctl.post(
            "pointer",
            &json!({"x": x, "y": y, "button": "right", "press": true}),
        )?;
        Ok(())
    }

    /// Moves the pointer over a target and leaves it there.
    pub fn hover(&self, target: impl Into<Target>) -> Result<()> {
        let rect = self.locate(&target.into())?;
        let (x, y) = rect.centre();
        self.devctl.post("pointer", &json!({"x": x, "y": y}))?;
        Ok(())
    }

    /// Presses the left button over `from`, moves to `to` and lets go there.
    pub fn drag(&self, from: impl Into<Target>, to: impl Into<Target>) -> Result<()> {
        let (fx, fy) = self.locate(&from.into())?.centre();
        let (tx, ty) = self.locate(&to.into())?.centre();
        self.devctl.post(
            "pointer",
            &json!({"x": fx, "y": fy, "button": "left", "hold": true}),
        )?;
        self.devctl.post("pointer", &json!({"x": tx, "y": ty}))?;
        self.devctl.post("pointer", &json!({"release": true}))?;
        Ok(())
    }

    /// Spins the wheel over a target, `lines` down (negative: up).
    pub fn scroll(&self, over: impl Into<Target>, lines: f32) -> Result<()> {
        self.hover(over)?;
        self.devctl.post("scroll", &json!({"y": -lines}))?;
        Ok(())
    }

    /// Presses and releases a key, with its modifiers.
    pub fn press(&self, key: impl Into<Key>) -> Result<()> {
        self.devctl.post("key", &key.into().body())?;
        Ok(())
    }

    /// Types text into whatever has the keyboard.
    pub fn type_text(&self, words: &str) -> Result<()> {
        self.devctl.post("text", &json!({"text": words}))?;
        Ok(())
    }

    /// Resizes the window and waits for the new layout to come to rest.
    pub fn resize(&mut self, viewport: Viewport) -> Result<Screen> {
        self.devctl.post(
            "window",
            &json!({"width": viewport.width, "height": viewport.height}),
        )?;
        self.confirm_size(viewport)?;
        self.settled()
    }

    /// Runs `check` at every viewport in turn and fails naming each one it
    /// failed at, with a screenshot and the screen's reading for each;
    /// the window is put back to the size it had.
    pub fn across(
        &mut self,
        viewports: &[Viewport],
        mut check: impl FnMut(&mut Self, Viewport) -> Result<()>,
    ) -> Result<()> {
        let home = self.viewport;
        let mut failed = Vec::new();
        for &viewport in viewports {
            let outcome = self.resize(viewport).and_then(|_| check(self, viewport));
            if let Err(error) = outcome {
                if let Ok(screen) = self.screen() {
                    self.keep(&format!("failure-{}", viewport.name), &screen);
                }
                let _ = self.screenshot(&format!("failure-{}", viewport.name));
                failed.push(format!("at {viewport}: {error:#}"));
            }
        }
        self.resize(home)?;
        if failed.is_empty() {
            return Ok(());
        }
        bail!(
            "failed at {} of {} viewports:\n{}",
            failed.len(),
            viewports.len(),
            failed.join("\n")
        )
    }

    /// Saves a screenshot as `<n>-<name>.png` in the test's directory.
    pub fn screenshot(&mut self, name: &str) -> Result<PathBuf> {
        self.shots += 1;
        let path = self.dir.join(format!("{:02}-{name}.png", self.shots));
        self.devctl
            .post_within("screenshot", &json!({"path": path}), SHOT)?;
        Ok(path)
    }

    /// Stops or restarts the game clock (`Time<Virtual>`).
    pub fn pause(&self, paused: bool) -> Result<()> {
        self.devctl.post("pause", &json!({"paused": paused}))?;
        Ok(())
    }

    /// Slows the game clock (`0 < speed`; 1 is real time).
    pub fn timescale(&self, speed: f32) -> Result<()> {
        self.devctl.post("timescale", &json!({"speed": speed}))?;
        Ok(())
    }

    /// Lets a paused clock run for `frames` rendered frames.
    pub fn step(&self, frames: u32) -> Result<()> {
        self.devctl
            .post_within("step", &json!({"frames": frames}), SHOT)?;
        Ok(())
    }

    /// Where a target is drawn now, refusing a miss or an ambiguity.
    pub fn locate(&self, target: &Target) -> Result<Rect> {
        let screen = self.screen()?;
        screen
            .locate(target)
            .inspect_err(|_| self.keep("miss", &screen))
    }

    /// Writes a screen's reading next to the test's logs, for the reader of
    /// a failure. Best effort: the failure being reported matters more.
    pub fn keep(&self, name: &str, screen: &Screen) {
        let _ = fs::write(
            self.dir.join(format!("{name}.state.json")),
            serde_json::to_vec_pretty(&screen.recording()).unwrap_or_default(),
        );
    }

    /// Kills the client, waits for it and proves its port closed.
    pub fn close(mut self) -> Result<()> {
        self.stop()?;
        refused(self.devctl.port())
    }

    fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            child
                .wait()
                .context("the client's exit could not be confirmed")?;
        }
        Ok(())
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn number(value: &Value, key: &str) -> f32 {
    value[key].as_f64().unwrap_or(0.0) as f32
}

fn free_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("no free loopback port")?;
    Ok(listener.local_addr()?.port())
}

/// Proves a loopback port refuses connections; a timeout proves nothing.
fn refused(port: u16) -> Result<()> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    match TcpStream::connect_timeout(&address, Duration::from_secs(5)) {
        Ok(_) => bail!("the client's harness still listens on {port}"),
        Err(e) if e.kind() == ErrorKind::ConnectionRefused => Ok(()),
        Err(e) => bail!("probing the client's port failed: {e}"),
    }
}
