//! The sweeps' verbs over the shared harness driver. No request or response
//! body reaches a diagnostic.
use super::tree::Tree;
use anyhow::{Context, Result, bail, ensure};
use baylee_client_e2e::{Devctl as Harness, Refused};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

pub(super) fn sleep(seconds: f64) {
    thread::sleep(Duration::from_secs_f64(seconds));
}
#[derive(Clone, Debug, Deserialize)]
pub(super) struct Control {
    pub press: String,
    pub at_x: f64,
    pub at_y: f64,
}
#[derive(Clone)]
pub(super) struct Devctl {
    harness: Harness,
}
impl Devctl {
    pub fn new(port: u16) -> Result<Self> {
        ensure!(port > 0, "port must be nonzero");
        Ok(Self {
            harness: Harness::new(port),
        })
    }
    pub fn call(&self, path: &str, body: Option<&Value>) -> Result<Value> {
        self.call_within(path, body, Duration::from_secs(60))
    }
    /// A refusal can repeat a field of the request, and `shell raw` sends
    /// whatever it was given, so the harness's reason is withheld here.
    pub fn call_within(
        &self,
        path: &str,
        body: Option<&Value>,
        timeout: Duration,
    ) -> Result<Value> {
        ensure!(path.starts_with('/'), "invalid local route");
        let reply = match body {
            Some(body) => self.harness.post_within(path, body, timeout),
            None => self.harness.get(path, timeout),
        };
        reply.map_err(|error| {
            if error.is::<Refused>() {
                anyhow::anyhow!("local request refused ({path}); response body withheld")
            } else {
                error
            }
        })
    }
    pub fn post(&self, path: &str, body: &Value) -> Result<()> {
        self.call(path, Some(body))?;
        Ok(())
    }
    pub fn state(&self) -> Result<Value> {
        self.call("/state", None)
    }
    pub fn health(&self) -> Result<Value> {
        self.call("/health", None)
    }
    pub fn nodes(&self, root: Option<&str>) -> Result<Tree> {
        Tree::read(&self.state()?, root)
    }
    pub fn shell_state(&self) -> Result<Value> {
        let s = self.state()?;
        ensure!(s["shell"].is_object(), "state has no shell");
        Ok(s["shell"].clone())
    }
    pub fn controls(&self) -> Result<Vec<Control>> {
        Ok(serde_json::from_value(
            self.state()?["lobby_controls"].clone(),
        )?)
    }
    pub fn names(&self) -> Result<Vec<String>> {
        Ok(self.controls()?.into_iter().map(|c| c.press).collect())
    }
    pub fn has(&self, name: &str) -> Result<bool> {
        Ok(self.names()?.iter().any(|p| p == name))
    }
    pub fn size(&self) -> Result<(f64, f64)> {
        let h = self.health()?;
        Ok((number(&h, "width")?, number(&h, "height")?))
    }
    pub fn unit_scale(&self) -> Result<()> {
        let h = self.health()?;
        ensure!(
            (number(&h, "scale")? - 1.0).abs() < 0.001,
            "pixel-ring/credit checks require BAYLEE_DEV_SCALE=1"
        );
        Ok(())
    }
    pub fn until(seconds: f64, mut predicate: impl FnMut() -> Result<bool>) -> Result<()> {
        let end = Instant::now() + Duration::from_secs_f64(seconds);
        loop {
            if predicate()? {
                return Ok(());
            }
            if Instant::now() >= end {
                bail!("condition not met within {seconds}s");
            }
            sleep(0.2);
        }
    }
    pub fn resize(&self, w: u32, h: u32) -> Result<()> {
        self.post("/window", &json!({"width":w,"height":h}))?;
        Self::until(10.0, || {
            let (x, y) = self.size()?;
            Ok((x - f64::from(w)).abs() <= 1.0 && (y - f64::from(h)).abs() <= 1.0)
        })
    }
    pub fn shell(&self, fields: &Value) -> Result<()> {
        let fields = fields
            .as_object()
            .context("shell knobs must be an object")?;
        let body: serde_json::Map<String, Value> = fields
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    Value::String(v.as_str().map_or_else(|| v.to_string(), str::to_owned)),
                )
            })
            .collect();
        self.post("/shell", &Value::Object(body))
    }
    pub fn knobs(&self, step: &str, lang: &str, touch: bool) -> Result<()> {
        self.shell(&json!({"text_size":step,"lang":lang,"input":if touch{"touch"}else{"pointer"}}))
    }
    pub fn screenshot(&self, path: &Path) -> Result<()> {
        let path = absolute(path)?;
        self.call_within(
            "/screenshot",
            Some(&json!({"path":path})),
            Duration::from_secs(120),
        )?;
        ensure!(path.is_file(), "screenshot response without a local file");
        Ok(())
    }
    pub fn key(&self, name: &str, character: Option<&str>, wait: f64) -> Result<()> {
        let mut body = json!({"name":name});
        if let Some(c) = character {
            body["char"] = json!(c);
        }
        self.post("/key", &body)?;
        sleep(wait);
        Ok(())
    }
    pub fn chord(&self, modifier: &str, name: &str, character: Option<&str>) -> Result<()> {
        self.post("/key", &json!({"name":modifier,"hold":true}))?;
        sleep(0.05);
        let result = self.key(name, character, 0.15);
        let release = self.post("/key", &json!({"name":modifier,"release":true}));
        sleep(0.4);
        result?;
        release
    }
    pub fn typed(&self, text: &str, wait: f64) -> Result<()> {
        self.post("/text", &json!({"text":text}))?;
        sleep(wait);
        Ok(())
    }
    pub fn pointer(&self, x: f64, y: f64, click: bool) -> Result<()> {
        ensure!(x.is_finite() && y.is_finite(), "invalid pointer position");
        let mut body = json!({"x":x,"y":y});
        if click {
            body["button"] = json!("left");
            body["press"] = json!(true);
        }
        self.post("/pointer", &body)
    }
    fn control(&self, name: &str) -> Result<Control> {
        let (w, h) = self.size()?;
        let controls = self.controls()?;
        controls
            .iter()
            .rev()
            .find(|c| {
                c.press == name
                    && c.at_x > 4.0
                    && c.at_x < w - 4.0
                    && c.at_y > 4.0
                    && c.at_y < h - 4.0
            })
            .cloned()
            .with_context(|| format!("no visible control {name}"))
    }
    pub fn press(&self, name: &str) -> Result<()> {
        let end = Instant::now() + Duration::from_secs(8);
        let mut last = self.control(name)?;
        loop {
            sleep(0.2);
            let now = self.control(name)?;
            if stable(&last, &now) {
                self.pointer(now.at_x, now.at_y, false)?;
                let fresh = self.control(name)?;
                if stable(&now, &fresh) {
                    return self.pointer(fresh.at_x, fresh.at_y, true);
                }
                last = fresh;
            } else {
                last = now;
            }
            ensure!(
                Instant::now() < end,
                "control geometry did not settle: {name}"
            );
        }
    }
    pub fn tap(&self, name: &str, wait: f64) -> Result<()> {
        self.press(name)?;
        sleep(wait);
        Ok(())
    }
    pub fn press_until(
        &self,
        name: &str,
        mut done: impl FnMut() -> Result<bool>,
        tries: usize,
    ) -> Result<()> {
        for _ in 0..tries {
            let before = self.shell_state()?["screen"].clone();
            if self.has(name)? {
                self.press(name)?;
            }
            let end = Instant::now() + Duration::from_secs(3);
            loop {
                if done()? {
                    return Ok(());
                }
                if Instant::now() >= end {
                    break;
                }
                sleep(0.2);
            }
            ensure!(
                self.shell_state()?["screen"] == before,
                "{name} changed screen but missed its postcondition; refusing another click"
            );
        }
        bail!("{name} did not reach its postcondition")
    }
    pub fn settle(&self, seconds: f64) -> Result<()> {
        let end = Instant::now() + Duration::from_secs_f64(seconds);
        let mut last = self.state()?["ui_rebuilds"].clone();
        let mut quiet = 0;
        ensure!(!last.is_null(), "missing ui_rebuilds");
        while Instant::now() < end {
            sleep(0.25);
            let now = self.state()?["ui_rebuilds"].clone();
            quiet = if now == last { quiet + 1 } else { 0 };
            last = now;
            if quiet >= 4 {
                return Ok(());
            }
        }
        bail!("lobby tree did not settle within {seconds}s")
    }
}
/// Read-only geometry gate: three samples spanning 400 ms must agree with
/// the same anchor. Comparing only adjacent frames would accept slow drift.
/// The caller decides which geometry matters; malformed reads still fail.
pub(super) fn read_stable<T>(
    seconds: f64,
    mut read: impl FnMut() -> Result<T>,
    agrees: impl Fn(&T, &T) -> bool,
) -> Result<T> {
    let end = Instant::now() + Duration::from_secs_f64(seconds);
    let mut anchor = read()?;
    let mut quiet = 0;
    loop {
        ensure!(
            Instant::now() < end,
            "geometry did not settle within {seconds}s"
        );
        sleep(0.2);
        let now = read()?;
        ensure!(
            Instant::now() < end,
            "geometry did not settle within {seconds}s"
        );
        if agrees(&anchor, &now) {
            quiet += 1;
            if quiet >= 2 {
                return Ok(now);
            }
        } else {
            anchor = now;
            quiet = 0;
        }
    }
}

pub(super) fn stable(a: &Control, b: &Control) -> bool {
    a.press == b.press && (a.at_x - b.at_x).abs() <= 0.5 && (a.at_y - b.at_y).abs() <= 0.5
}
pub(super) fn number(v: &Value, key: &str) -> Result<f64> {
    v[key]
        .as_f64()
        .filter(|v| v.is_finite())
        .with_context(|| format!("missing numeric field {key}"))
}
pub(super) fn absolute(path: &Path) -> Result<PathBuf> {
    Ok(if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    })
}
pub(super) const COMMAND_KEY: &str = if cfg!(target_os = "macos") {
    "SuperLeft"
} else {
    "ControlLeft"
};
