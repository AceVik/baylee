//! Front door faces and the always-present legal colophon.
use super::{
    check,
    common::{self, Report},
    devctl::{Devctl, read_stable, sleep},
    tree::Tree,
};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::path::Path;
const FACES: &[&str] = &["gateway", "signin", "create", "guest", "about"];
/// All geometry that determines visibility, including moving clip ancestors.
struct ScrollGeometry {
    control: [f64; 4],
    // The window first, then the actual scroll ancestors in tree order.
    clips: Vec<[f64; 4]>,
}

impl ScrollGeometry {
    fn agrees(&self, other: &Self) -> bool {
        self.clips.len() == other.clips.len()
            && self
                .control
                .iter()
                .chain(self.clips.iter().flatten())
                .zip(other.control.iter().chain(other.clips.iter().flatten()))
                .all(|(a, b)| (a - b).abs() <= 0.5)
    }
}

fn scroll_geometry(state: &Value, size: (f64, f64), name: &str) -> Result<ScrollGeometry> {
    let control = state["lobby_controls"]
        .as_array()
        .context("missing controls")?
        .iter()
        .rev()
        .find(|c| c["press"] == name)
        .with_context(|| format!("control not offered: {name}"))?;
    let number = |key| {
        control[key]
            .as_f64()
            .filter(|n| n.is_finite())
            .with_context(|| format!("missing control {key}: {name}"))
    };
    let (x, y, w, h) = (number("at_x")?, number("at_y")?, number("w")?, number("h")?);
    let tree = Tree::read(state, None)?;
    let hit = tree
        .0
        .iter()
        .rev()
        .find(|n| {
            !tree.hidden(n)
                && (n.x + n.w / 2.0 - x).abs() <= 1.0
                && (n.y + n.h / 2.0 - y).abs() <= 1.0
                && (n.w - w).abs() <= 1.0
                && (n.h - h).abs() <= 1.0
        })
        .with_context(|| format!("missing control node: {name}"))?;
    let mut clips = vec![[0.0, 0.0, size.0, size.1]];
    clips.extend(
        tree.ancestors(hit.index, None)
            .into_iter()
            .filter(|n| n.kind() == "scroll")
            .map(|n| [n.x, n.y, n.w, n.h]),
    );
    Ok(ScrollGeometry {
        control: [x, y, w, h],
        clips,
    })
}

/// A wheel aim only when the complete control lies outside the window or
/// one of its actual scroll ancestors. The fixed footer is outside that
/// ancestor, so a centre still inside /health.height is not enough.
fn scroll_target(state: &Value, size: (f64, f64), name: &str) -> Result<Option<(f64, f64, f64)>> {
    let geometry = scroll_geometry(state, size, name)?;
    let [x, y, w, h] = geometry.control;
    let (mut left, mut top, mut right, mut bottom) = (0.0_f64, 0.0_f64, size.0, size.1);
    for &[x, y, w, h] in &geometry.clips {
        left = left.max(x);
        top = top.max(y);
        right = right.min(x + w);
        bottom = bottom.min(y + h);
    }
    ensure!(
        w > 0.0 && h > 0.0 && h <= bottom - top + 1.0,
        "control does not fit its scroll viewport: {name}"
    );
    ensure!(
        x - w / 2.0 >= left - 1.0 && x + w / 2.0 <= right + 1.0,
        "control leaves its horizontal viewport: {name}"
    );
    if y - h / 2.0 >= top - 1.0 && y + h / 2.0 <= bottom + 1.0 {
        return Ok(None);
    }
    ensure!(
        geometry.clips.len() > 1,
        "offscreen control has no scroll ancestor: {name}"
    );
    let down = y + h / 2.0 > bottom;
    let inset = ((bottom - top) / 2.0).min(12.0);
    Ok(Some((
        x.clamp(left, right),
        if down { bottom - inset } else { top + inset },
        if down { -3.0 } else { 3.0 },
    )))
}

pub(super) fn into_view(d: &Devctl, name: &str) -> Result<()> {
    let mut before = None;
    for attempt in 0..=12 {
        let (state, size, _) = read_stable(
            8.0,
            || {
                let size = d.size()?;
                let state = d.state()?;
                let screen = json!([
                    state["shell"]["screen"],
                    state["shell"]["table"],
                    state["shell"]["modal_by"],
                    state["shell"]["overlay"]
                ]);
                if let Some(before) = &before {
                    ensure!(
                        *before == screen,
                        "screen changed while scrolling to {name}"
                    );
                } else {
                    before = Some(screen);
                }
                let geometry = scroll_geometry(&state, size, name)?;
                Ok((state, size, geometry))
            },
            |a, b| a.2.agrees(&b.2),
        )
        .with_context(|| format!("waiting for scroll geometry: {name}"))?;
        let Some((x, y, by)) = scroll_target(&state, size, name)? else {
            return Ok(());
        };
        ensure!(attempt < 12, "could not scroll control into view: {name}");
        d.pointer(x, y, false)?;
        d.post("/scroll", &json!({"y":by}))?;
        sleep(0.15);
    }
    unreachable!("the bounded scroll loop returns or refuses")
}

fn press_visible(
    d: &Devctl,
    name: &str,
    done: impl FnMut() -> Result<bool>,
    tries: usize,
) -> Result<()> {
    into_view(d, name)?;
    // press_until rereads stable geometry, prehovers, and rereads once more
    // before the click. Never reuse the point from the scroll decision.
    d.press_until(name, done, tries)
}

fn choose(d: &Devctl, gateway: &str) -> Result<()> {
    if d.has("Front(LeaveGateway)")? {
        press_visible(d, "Front(LeaveGateway)", || d.has("Front(AddGateway)"), 3)?;
        sleep(1.0);
    }
    let state = d.state()?;
    let tree = Tree::read(&state, None)?;
    let controls = d.controls()?;
    for n in tree.0.iter().filter(|n| n.text() == gateway) {
        if let Some(row) = controls.iter().find(|c| {
            c.press.starts_with("Front(SelectGateway") && (c.at_y - (n.y + n.h / 2.0)).abs() < 30.0
        }) {
            press_visible(d, &row.press, || d.has("Front(LeaveGateway)"), 3)?;
            sleep(1.2);
            return Ok(());
        }
    }
    bail!("named gateway is not present")
}
fn face(d: &Devctl, face: &str, gateway: &str) -> Result<()> {
    for name in ["Settings(CloseSettings)", "Front(About(false))"] {
        if d.has(name)? {
            press_visible(d, name, || Ok(!d.has(name)?), 3)?;
        }
    }
    if face == "gateway" {
        if d.has("Front(BackToSignIn)")? {
            press_visible(d, "Front(BackToSignIn)", || d.has("Front(LeaveGateway)"), 3)?;
        }
        if d.has("Front(LeaveGateway)")? {
            press_visible(d, "Front(LeaveGateway)", || d.has("Front(AddGateway)"), 3)?;
            sleep(1.0);
        }
        return Ok(());
    }
    if face == "about" {
        return press_visible(d, "Front(About(true))", || d.has("Front(About(false))"), 3);
    }
    if d.has("Front(AddGateway)")? {
        choose(d, gateway)?;
    }
    if d.has("Front(BackToSignIn)")? {
        press_visible(d, "Front(BackToSignIn)", || d.has("Front(GuestFace)"), 3)?;
        sleep(0.6);
    }
    match face {
        "create" => press_visible(
            d,
            "Front(ToggleRegistering)",
            || d.has("Shared(Focus(DisplayName))"),
            3,
        )?,
        "guest" => press_visible(
            d,
            "Front(GuestFace)",
            || d.has("Shared(Focus(GuestName))"),
            3,
        )?,
        _ => {}
    }
    sleep(0.8);
    Ok(())
}
pub(super) fn run(d: &Devctl, out: &Path, gateway: &str) -> Result<()> {
    common::prepare(out)?;
    let (sizes, steps) = common::matrix()?;
    let result = (|| {
        let mut report = Report::default();
        for (w, h) in sizes {
            d.resize(w, h)?;
            sleep(1.0);
            let touch = check::TOUCH.contains(&(w, h));
            for step in &steps {
                for lang in ["en", "de"] {
                    d.knobs(step, lang, touch)?;
                    sleep(0.6);
                    for &which in FACES {
                        face(d, which, gateway)?;
                        d.settle(4.0)?;
                        let mut tree = d.nodes(None)?;
                        let mut faults =
                            check::layout(&tree, f64::from(w), lang == "de", touch, true);
                        if which != "about" {
                            faults.extend(check::colophon(&common::words(&tree)));
                        }
                        let tag = format!("{which}-{w}x{h}-{step}-{lang}");
                        let mut extra = String::new();
                        if common::medium_shot(step, lang, (w, h)) {
                            let png = out.join(format!("front-{tag}.png"));
                            if lang == "en" {
                                if which == "about" {
                                    for n in &mut tree.0 {
                                        if matches!(n.kind(), "panel" | "mist") {
                                            n.k = None;
                                        }
                                    }
                                }
                                let (more, contrast) = common::photograph(d, &tree, &png, None)?;
                                faults.extend(more);
                                extra = contrast;
                            } else {
                                d.screenshot(&png)?;
                            }
                        }
                        report.case(&tag, tree.0.len(), &faults, &extra);
                    }
                }
            }
        }
        face(d, "signin", gateway)?;
        report.finish(Some(&out.join("front-check.log")), false)
    })();
    common::cleanup(result, common::reset(d))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::TcpListener,
        thread,
        time::Duration,
    };

    fn state(name: &str, y: f64) -> Value {
        json!({
            "shell":{"screen":"Front"},
            "lobby_controls":[{"press":name,"at_x":480,"at_y":y,"w":200,"h":56}],
            "shell_nodes":[
                {"d":0,"x":0,"y":0,"w":960,"h":700},
                {"d":1,"k":"scroll","x":0,"y":0,"w":960,"h":644},
                {"d":2,"k":"button","x":380,"y":y-28.0,"w":200,"h":56}
            ]
        })
    }

    #[test]
    fn a_button_centre_inside_the_window_can_still_need_root_scrolling() {
        // The live GuestFace geometry: the footer begins at 644, centre 648.
        let clipped = state("Front(GuestFace)", 648.0);
        assert_eq!(
            scroll_target(&clipped, (960.0, 700.0), "Front(GuestFace)").unwrap(),
            Some((480.0, 632.0, -3.0))
        );
        assert_eq!(
            scroll_target(
                &state("Front(GuestFace)", 450.0),
                (960.0, 700.0),
                "Front(GuestFace)"
            )
            .unwrap(),
            None
        );
        let mut broken = clipped;
        broken["lobby_controls"][0]["w"] = json!(1000);
        broken["shell_nodes"][2]["x"] = json!(-20);
        broken["shell_nodes"][2]["w"] = json!(1000);
        assert!(
            scroll_target(&broken, (960.0, 700.0), "Front(GuestFace)").is_err(),
            "scrolling must not excuse horizontal layout overflow"
        );
    }

    /// A scripted loopback fixture; unexpected writes or stale reads fail.
    fn serve(replies: Vec<(&'static str, Value)>) -> (Devctl, thread::JoinHandle<Vec<Value>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let worker = thread::spawn(move || {
            let mut bodies = Vec::new();
            for (route, reply) in replies {
                let until = std::time::Instant::now() + Duration::from_secs(5);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(std::time::Instant::now() < until, "missing {route}");
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(e) => panic!("{e}"),
                    }
                };
                // Accepted sockets can inherit the listener's nonblocking mode.
                // Poll accept above, but read each request with the timeout below.
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut reader = BufReader::new(&mut stream);
                let mut line = String::new();
                assert!(
                    reader.read_line(&mut line).unwrap() > 0,
                    "unexpected request EOF"
                );
                assert!(line.starts_with(route), "unexpected request: {line}");
                let mut length = 0;
                loop {
                    line.clear();
                    assert!(
                        reader.read_line(&mut line).unwrap() > 0,
                        "unexpected request EOF"
                    );
                    if line == "\r\n" {
                        break;
                    }
                    if let Some((key, value)) = line.split_once(':')
                        && key.eq_ignore_ascii_case("content-length")
                    {
                        length = value.trim().parse::<usize>().unwrap();
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                if !body.is_empty() {
                    bodies.push(serde_json::from_slice::<Value>(&body).unwrap());
                }
                drop(reader);
                let body = reply.to_string();
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
            bodies
        });
        (Devctl::new(port).unwrap(), worker)
    }

    fn samples(states: impl IntoIterator<Item = Value>) -> Vec<(&'static str, Value)> {
        states
            .into_iter()
            .flat_map(|state| {
                [
                    ("GET /health", json!({"width":960,"height":700})),
                    ("GET /state", state),
                ]
            })
            .collect()
    }

    #[test]
    fn visibility_uses_a_fresh_state_after_wheeling_for_front_and_settings_cleanup() {
        for name in ["Front(GuestFace)", "Settings(Seat(Revert))"] {
            let mut replies = samples(vec![state(name, 648.0); 3]);
            replies.extend([
                ("POST /pointer", json!({"ok":true})),
                ("POST /scroll", json!({"ok":true})),
            ]);
            replies.extend(samples(vec![state(name, 450.0); 3]));
            let (d, worker) = serve(replies);
            into_view(&d, name).unwrap();
            let bodies = worker.join().unwrap();
            assert_eq!(
                bodies.len(),
                2,
                "only prehover and one bounded wheel; no click"
            );
            assert_eq!(bodies[1], json!({"y":-3.0}));
            assert_eq!(bodies[0]["x"], 480.0);
            assert_eq!(bodies[0]["y"], 632.0);
        }
    }

    fn at_x(name: &str, x: f64) -> Value {
        let mut s = state(name, 450.0);
        s["lobby_controls"][0]["at_x"] = json!(x);
        s["shell_nodes"][2]["x"] = json!(x - 100.0);
        s
    }

    #[test]
    fn visibility_waits_for_animation_but_still_rejects_settled_overflow() {
        let name = "Front(LeaveGateway)";
        for (positions, expected_error) in [
            (vec![-100.0, 300.0, 480.0, 480.0, 480.0], false),
            (vec![-100.0; 3], true),
        ] {
            let (d, worker) = serve(samples(positions.into_iter().map(|x| at_x(name, x))));
            let result = into_view(&d, name);
            let bodies = worker.join().unwrap();
            assert!(bodies.is_empty(), "settling must neither scroll nor click");
            if expected_error {
                assert_eq!(
                    result.unwrap_err().to_string(),
                    "control leaves its horizontal viewport: Front(LeaveGateway)"
                );
            } else {
                result.unwrap();
            }
        }
    }

    #[test]
    fn visibility_waits_for_scroll_ancestors_even_when_the_control_stands_still() {
        let name = "Front(LeaveGateway)";
        let states = [600.0, 300.0, 0.0, 0.0, 0.0].map(|x| {
            let mut s = state(name, 450.0);
            s["shell_nodes"][1]["x"] = json!(x);
            s
        });
        let (d, worker) = serve(samples(states));
        into_view(&d, name).unwrap();
        assert!(worker.join().unwrap().is_empty());
    }

    #[test]
    fn visibility_refuses_screen_changes_during_stabilization() {
        let name = "Front(LeaveGateway)";
        let first = state(name, 450.0);
        let mut changed = first.clone();
        changed["shell"]["table"] = json!("another-face");
        let (d, worker) = serve(samples([first, changed]));
        let error = into_view(&d, name).unwrap_err();
        assert!(worker.join().unwrap().is_empty());
        assert!(format!("{error:#}").contains("screen changed while scrolling"));
    }

    #[test]
    fn stabilization_is_bounded_and_does_not_accept_slow_drift() {
        let mut position = 0.0_f64;
        let result = read_stable(
            0.5,
            || {
                position += 0.4;
                Ok(position)
            },
            |a, b| (a - b).abs() <= 0.5,
        );
        assert!(result.unwrap_err().to_string().contains("did not settle"));
    }
}
