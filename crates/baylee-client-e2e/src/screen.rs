//! What the client shows, read once: one `GET /state` beside one
//! `GET /health`, parsed into the interface tree and its controls.
//!
//! The tree is `/state.shell_nodes`: every node under the lobby's root (and
//! the gallery's and the report sheet's) in tree order, with its rectangle in
//! logical window pixels, its text, the size its glyphs measure, and whether
//! it can be pressed. It deliberately carries no entity index and no `Press`
//! spelling, so two dumps of one screen compare byte for byte. The controls
//! with their `Press` names come from `lobby_controls`, `desk_controls`,
//! `update_buttons` and the chat's `controls`. Everything else in `/state`
//! (the duel's `view`, `interaction`, `buttons`, …) is reachable through
//! [`Screen::at`] by JSON pointer.
use crate::viewport::Viewport;
use anyhow::{Result, bail};
use serde_json::Value;
use std::fmt;

/// A rectangle in logical window pixels, from its top-left corner.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

impl Rect {
    /// A rectangle by its corner and size.
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// A rectangle by its centre and size, as the control lists give it.
    pub fn around(cx: f32, cy: f32, w: f32, h: f32) -> Self {
        Self::new(cx - w / 2.0, cy - h / 2.0, w, h)
    }

    /// Where a pointer presses it.
    pub fn centre(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    /// Right edge.
    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    /// Bottom edge.
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    /// Whether a point lies on it.
    pub fn holds(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.right() && y >= self.y && y <= self.bottom()
    }

    /// Whether `other` lies inside this one, give or take `slack` pixels.
    pub fn contains(&self, other: &Self, slack: f32) -> bool {
        other.x >= self.x - slack
            && other.y >= self.y - slack
            && other.right() <= self.right() + slack
            && other.bottom() <= self.bottom() + slack
    }

    /// The area two rectangles share (zero when they only touch).
    #[must_use]
    pub fn overlap(&self, other: &Self) -> Self {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let w = (self.right().min(other.right()) - x).max(0.0);
        let h = (self.bottom().min(other.bottom()) - y).max(0.0);
        Self::new(x, y, w, h)
    }

    /// Whether it has no area to draw or press.
    pub fn is_empty(&self) -> bool {
        self.w <= 0.0 || self.h <= 0.0
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:.0}×{:.0} at ({:.0}, {:.0})",
            self.w, self.h, self.x, self.y
        )
    }
}

/// One node of the interface tree.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// Its depth under its root.
    pub depth: usize,
    /// Where it is drawn.
    pub rect: Rect,
    /// Its text, spans joined, if it is a text node.
    pub text: Option<String>,
    /// What its glyphs measure (width, height), beside the box they were given.
    pub text_size: Option<(f32, f32)>,
    /// How many lines its text broke into.
    pub lines: u32,
    /// Hidden by its own `Visibility` or an ancestor's.
    pub hidden: bool,
    /// It carries a `Press`: a pointer can press it.
    pub pressable: bool,
    /// Whether, and how, it clips its children across (`Overflow::x`).
    pub clip_x: Option<Clip>,
    /// Whether, and how, it clips its children down (`Overflow::y`).
    pub clip_y: Option<Clip>,
    /// Its accessibility role (`"Button"`, `"TextInput"`, …).
    pub role: Option<String>,
    /// Which tree it is in when it is not the lobby's (`"gallery"`, `"report"`).
    pub root: Option<String>,
    /// Its parent's index in [`Screen::nodes`].
    pub parent: Option<usize>,
}

/// How a node clips what it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clip {
    /// It scrolls: what lies outside can be brought into view.
    Scroll,
    /// It clips or hides: what lies outside is never drawn (the
    /// window-sized root is one).
    Cut,
}

/// Which harness list a control came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlList {
    /// The lobby and the front door (`lobby_controls`).
    Lobby,
    /// The report form (`desk_controls`).
    Desk,
    /// The updater's card (`update_buttons`).
    Update,
    /// The chat window (`chat.controls`).
    Chat,
    /// The table's buttons (`buttons`: the prompt's answers, the hand's
    /// sort, the menu), named by their `label` (`Keep`, `SortHand(Mana)`).
    Table,
}

/// A card where the table draws it (`/state.cards`).
#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    /// Its printed name.
    pub name: String,
    /// `"hand"` or `"table"` (the battlefield and the command zone).
    pub zone: String,
    /// The engine's object number.
    pub object: u64,
    /// How many identical cards its pile stands for.
    pub count: u64,
    /// Where it is drawn.
    pub rect: Rect,
}

/// A control by its `Press`, the spelling a test names it by.
#[derive(Clone, Debug, PartialEq)]
pub struct Control {
    /// The `Press`'s Debug spelling: `Front(PlayOffline)`, `Header(Nav(3))`.
    pub press: String,
    /// Where it is drawn.
    pub rect: Rect,
    /// The list it was read from.
    pub list: ControlList,
}

/// What a test points at.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    /// The one control whose `Press` is spelled exactly so.
    Press(String),
    /// The one control whose `Press` starts so (`Front(SelectGateway(`).
    PressPrefix(String),
    /// The one pressable node whose label (its text and its descendants')
    /// reads exactly so, ignoring surrounding space.
    Label(String),
    /// The one visible text node that reads exactly so.
    Text(String),
    /// The one drawn card of this name, in `zone` when one is named.
    Card {
        /// Its printed name.
        name: String,
        /// `"hand"` or `"table"`, or anywhere.
        zone: Option<String>,
    },
    /// A point in logical window pixels.
    At(f32, f32),
}

/// A control by its `Press` spelling.
pub fn press(name: impl Into<String>) -> Target {
    Target::Press(name.into())
}

/// A control whose `Press` spelling starts so.
pub fn press_prefix(prefix: impl Into<String>) -> Target {
    Target::PressPrefix(prefix.into())
}

/// A pressable node by the words on it.
pub fn label(words: impl Into<String>) -> Target {
    Target::Label(words.into())
}

/// A visible piece of text.
pub fn text(words: impl Into<String>) -> Target {
    Target::Text(words.into())
}

/// A card by its name, wherever it is drawn.
pub fn card(name: impl Into<String>) -> Target {
    Target::Card {
        name: name.into(),
        zone: None,
    }
}

/// A card in the hand by its name.
pub fn in_hand(name: impl Into<String>) -> Target {
    Target::Card {
        name: name.into(),
        zone: Some("hand".to_owned()),
    }
}

/// A point in the window.
pub fn at(x: f32, y: f32) -> Target {
    Target::At(x, y)
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Press(name) => write!(f, "the control {name}"),
            Self::PressPrefix(prefix) => write!(f, "the control {prefix}…"),
            Self::Label(words) => write!(f, "the control labelled {words:?}"),
            Self::Text(words) => write!(f, "the text {words:?}"),
            Self::Card { name, zone: None } => write!(f, "the card {name}"),
            Self::Card {
                name,
                zone: Some(zone),
            } => write!(f, "the card {name} in the {zone}"),
            Self::At(x, y) => write!(f, "the point ({x:.0}, {y:.0})"),
        }
    }
}

/// One reading of the client.
#[derive(Clone, Debug)]
pub struct Screen {
    state: Value,
    window: (f32, f32),
    nodes: Vec<Node>,
    controls: Vec<Control>,
    cards: Vec<Card>,
}

impl Screen {
    /// Parses a `/state` reply beside the window's logical size.
    pub fn new(state: Value, window: (f32, f32)) -> Self {
        let nodes = nodes(&state);
        let controls = controls(&state);
        let cards = cards(&state);
        Self {
            state,
            window,
            nodes,
            controls,
            cards,
        }
    }

    /// Parses a recorded fixture: `{"window":[w,h],"state":{…}}`.
    pub fn from_recording(recorded: &str) -> Result<Self> {
        let value: Value = serde_json::from_str(recorded)?;
        let (Some(w), Some(h)) = (value["window"][0].as_f64(), value["window"][1].as_f64()) else {
            bail!("a recording without its window size");
        };
        Ok(Self::new(value["state"].clone(), (w as f32, h as f32)))
    }

    /// The recording [`Screen::from_recording`] reads back.
    pub fn recording(&self) -> Value {
        serde_json::json!({"window": [self.window.0, self.window.1], "state": self.state})
    }

    /// The window, in logical pixels.
    pub fn window(&self) -> Rect {
        Rect::new(0.0, 0.0, self.window.0, self.window.1)
    }

    /// Whether the window is the viewport's size, give or take the pixel a
    /// window manager rounds by.
    pub fn is_at(&self, viewport: Viewport) -> bool {
        viewport.is(self.window.0, self.window.1)
    }

    /// The whole `/state` reply.
    pub fn state(&self) -> &Value {
        &self.state
    }

    /// Any field of `/state` by JSON pointer (`"/account/offline"`); `null`
    /// where there is none.
    pub fn at(&self, pointer: &str) -> &Value {
        self.state.pointer(pointer).unwrap_or(&Value::Null)
    }

    /// A boolean field by JSON pointer; absent or not a boolean reads false.
    pub fn flag(&self, pointer: &str) -> bool {
        self.at(pointer).as_bool().unwrap_or(false)
    }

    /// The interface tree in tree order.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Every control with a `Press`.
    pub fn controls(&self) -> &[Control] {
        &self.controls
    }

    /// Every card the table draws, hand first.
    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    /// The cards drawn in one zone (`"hand"`, `"table"`).
    pub fn cards_in<'a>(&'a self, zone: &'a str) -> impl Iterator<Item = &'a Card> {
        self.cards.iter().filter(move |c| c.zone == zone)
    }

    /// The object the keyboard cursor or the pointer is on (`ObjectId(52#0)`
    /// is read as 52).
    pub fn hovered(&self) -> Option<u64> {
        let spelled = self.at("/hovered").as_str()?;
        let inside = spelled.strip_prefix("ObjectId(")?;
        inside.split(['#', ')']).next()?.parse().ok()
    }

    /// Whether a duel is open: the host has sent a view.
    pub fn in_duel(&self) -> bool {
        !self.at("/view").is_null()
    }

    /// The startup veil has lifted and the front door is not moving.
    pub fn at_rest(&self) -> bool {
        let loading = self.at("/loading/destination");
        (loading.is_null() || loading == "hidden") && !self.flag("/account/front_moving")
    }

    /// The node's own text and every descendant's, in tree order, joined by
    /// a space: what a person reads on it.
    pub fn label_of(&self, index: usize) -> String {
        let mut words: Vec<&str> = Vec::new();
        for (at, node) in self.subtree(index) {
            if !self.nodes[at].hidden
                && let Some(text) = node.text.as_deref()
            {
                let text = text.trim();
                if !text.is_empty() {
                    words.push(text);
                }
            }
        }
        words.join(" ")
    }

    /// The node and its descendants, with their indices.
    pub fn subtree(&self, index: usize) -> impl Iterator<Item = (usize, &Node)> {
        let depth = self.nodes[index].depth;
        let root = self.nodes[index].root.clone();
        std::iter::once((index, &self.nodes[index])).chain(
            self.nodes
                .iter()
                .enumerate()
                .skip(index + 1)
                .take_while(move |(_, n)| n.depth > depth && n.root == root),
        )
    }

    /// Whether `ancestor` is `node` or one of its ancestors.
    pub fn is_within(&self, node: usize, ancestor: usize) -> bool {
        let mut at = Some(node);
        while let Some(index) = at {
            if index == ancestor {
                return true;
            }
            at = self.nodes[index].parent;
        }
        false
    }

    /// What clips part or all of the node away, and how, decided per axis:
    /// on each axis the nearest ancestor that clips that axis and leaves the
    /// node outside it. A scroll there can bring the node into view (the
    /// ancestors above judge the viewport, not the node); a cut never can.
    /// [`Clip::Cut`] when either axis is cut, else [`Clip::Scroll`] when
    /// either scrolls, else `None`.
    pub fn clipped_by(&self, index: usize) -> Option<(usize, Clip)> {
        let rect = self.nodes[index].rect;
        let along = |outer: &Node, across: bool| {
            if across {
                (
                    outer.clip_x,
                    rect.x < outer.rect.x - 1.0 || rect.right() > outer.rect.right() + 1.0,
                )
            } else {
                (
                    outer.clip_y,
                    rect.y < outer.rect.y - 1.0 || rect.bottom() > outer.rect.bottom() + 1.0,
                )
            }
        };
        let nearest = |across: bool| {
            let mut at = self.nodes[index].parent;
            while let Some(parent) = at {
                if let (Some(clip), true) = along(&self.nodes[parent], across) {
                    return Some((parent, clip));
                }
                at = self.nodes[parent].parent;
            }
            None
        };
        let (x, y) = (nearest(true), nearest(false));
        [x, y]
            .into_iter()
            .flatten()
            .find(|(_, clip)| *clip == Clip::Cut)
            .or(x)
            .or(y)
    }

    /// Whether a scroll viewport hides part of the node: scrolling can bring
    /// it into view, so it is not misplaced.
    pub fn scrolled_away(&self, index: usize) -> bool {
        matches!(self.clipped_by(index), Some((_, Clip::Scroll)))
    }

    /// Every visible text, in tree order.
    pub fn texts(&self) -> impl Iterator<Item = &str> {
        self.nodes
            .iter()
            .filter(|n| !n.hidden)
            .filter_map(|n| n.text.as_deref())
            .filter(|t| !t.trim().is_empty())
    }

    /// Whether any visible text contains `words`.
    pub fn shows(&self, words: &str) -> bool {
        self.texts().any(|t| t.contains(words))
    }

    /// Fails unless some visible text contains `words`, listing what is shown.
    pub fn expect_shows(&self, words: &str) -> Result<()> {
        if self.shows(words) {
            return Ok(());
        }
        bail!(
            "{words:?} is not on screen; it shows: {}",
            self.texts().take(40).collect::<Vec<_>>().join(" | ")
        )
    }

    /// Fails if any visible text contains `words`.
    pub fn expect_not_shown(&self, words: &str) -> Result<()> {
        if let Some(found) = self.texts().find(|t| t.contains(words)) {
            bail!("{words:?} is on screen: {found:?}");
        }
        Ok(())
    }

    /// The one control with this `Press` spelling, if it is drawn.
    pub fn control(&self, name: &str) -> Option<&Control> {
        self.controls.iter().find(|c| c.press == name)
    }

    /// Where a target is, refusing a target that matches nothing or more
    /// than one thing: a click must never be a guess.
    pub fn locate(&self, target: &Target) -> Result<Rect> {
        let found: Vec<Rect> = match target {
            Target::At(x, y) => return Ok(Rect::new(*x, *y, 0.0, 0.0)),
            Target::Press(name) => self
                .controls
                .iter()
                .filter(|c| &c.press == name)
                .map(|c| c.rect)
                .collect(),
            Target::PressPrefix(prefix) => self
                .controls
                .iter()
                .filter(|c| c.press.starts_with(prefix.as_str()))
                .map(|c| c.rect)
                .collect(),
            Target::Label(words) => (0..self.nodes.len())
                .filter(|&i| self.nodes[i].pressable && !self.nodes[i].hidden)
                .filter(|&i| self.label_of(i) == words.trim())
                .map(|i| self.nodes[i].rect)
                .collect(),
            Target::Card { name, zone } => self
                .cards
                .iter()
                .filter(|c| &c.name == name && zone.as_ref().is_none_or(|z| &c.zone == z))
                .map(|c| c.rect)
                .collect(),
            Target::Text(words) => self
                .nodes
                .iter()
                .filter(|n| !n.hidden && n.text.as_deref().map(str::trim) == Some(words.trim()))
                .map(|n| n.rect)
                .collect(),
        };
        match found.as_slice() {
            [one] => Ok(*one),
            [] => bail!("{target} is not on screen; {}", self.offer(target)),
            many => bail!(
                "{target} is ambiguous: {} matches ({})",
                many.len(),
                many.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
        }
    }

    /// What a test could have meant instead, for a target that matched nothing.
    fn offer(&self, target: &Target) -> String {
        match target {
            Target::Card { .. } => format!(
                "the cards drawn are: {}",
                self.cards
                    .iter()
                    .map(|c| format!("{} ({})", c.name, c.zone))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Target::Press(_) | Target::PressPrefix(_) => format!(
                "the controls drawn are: {}",
                self.controls
                    .iter()
                    .map(|c| c.press.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            _ => format!(
                "the pressable labels are: {}",
                (0..self.nodes.len())
                    .filter(|&i| self.nodes[i].pressable && !self.nodes[i].hidden)
                    .map(|i| format!("{:?}", self.label_of(i)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    /// Refuses a press on a gateway row the front door lists by itself.
    ///
    /// The front door always lists the built-in production gateway beside
    /// the saved ones, in an order that can change between runs; only a
    /// saved row has a bin (`Front(ForgetGateway(k))`). A test that pressed
    /// the built-in row would sign in to production (29.10.2026: a run picked
    /// it by index), so every click is checked here, whatever named it.
    pub fn refuse_a_real_gateway(&self, x: f32, y: f32) -> Result<()> {
        for control in &self.controls {
            let Some(row) = control
                .press
                .strip_prefix("Front(SelectGateway(")
                .and_then(|rest| rest.strip_suffix("))"))
            else {
                continue;
            };
            let bin = format!("Front(ForgetGateway({row}))");
            if control.rect.holds(x, y) && self.control(&bin).is_none() {
                bail!(
                    "{} is a gateway row the client lists by itself (production); \
                     an end-to-end test never signs in there",
                    control.press
                );
            }
        }
        Ok(())
    }

    /// What a screen is made of, for telling when it has stopped moving:
    /// the tree and the controls exactly as drawn.
    pub(crate) fn fingerprint(&self) -> (Value, Vec<Control>) {
        (self.at("/shell_nodes").clone(), self.controls.clone())
    }
}

fn number(value: &Value, key: &str) -> f32 {
    value[key].as_f64().unwrap_or(0.0) as f32
}

fn clip(value: &Value) -> Option<Clip> {
    match value.as_str()? {
        "scroll" => Some(Clip::Scroll),
        _ => Some(Clip::Cut),
    }
}

/// The tree, with parents found from the depths and visibility inherited.
fn nodes(state: &Value) -> Vec<Node> {
    let mut out: Vec<Node> = Vec::new();
    // The open ancestors of the next node, by depth.
    let mut path: Vec<usize> = Vec::new();
    let rows = [&state["shell_nodes"], &state["ui_nodes"]];
    for row in rows.into_iter().filter_map(Value::as_array).flatten() {
        let depth = row["d"].as_u64().unwrap_or(0) as usize;
        path.truncate(depth);
        let parent = if depth == 0 {
            None
        } else {
            path.last().copied()
        };
        let own_hidden = row["hid"].as_bool().unwrap_or(false);
        let hidden = own_hidden || parent.is_some_and(|p| out[p].hidden);
        let root = row["r"]
            .as_str()
            .map(str::to_owned)
            .or_else(|| parent.and_then(|p| out[p].root.clone()));
        let text_size = row["tw"].as_f64().map(|w| (w as f32, number(row, "th")));
        out.push(Node {
            depth,
            rect: Rect::new(
                number(row, "x"),
                number(row, "y"),
                number(row, "w"),
                number(row, "h"),
            ),
            text: row["t"].as_str().map(str::to_owned),
            text_size,
            lines: row["ln"].as_u64().unwrap_or(0) as u32,
            hidden,
            pressable: row["i"].as_bool().unwrap_or(false),
            clip_x: clip(&row["cx"]),
            clip_y: clip(&row["cy"]),
            role: row["k"].as_str().map(str::to_owned),
            root,
            parent,
        });
        path.push(out.len() - 1);
    }
    out
}

fn controls(state: &Value) -> Vec<Control> {
    let mut out = Vec::new();
    for (pointer, key, list) in [
        ("/lobby_controls", "press", ControlList::Lobby),
        ("/desk_controls", "press", ControlList::Desk),
        ("/update_buttons", "button", ControlList::Update),
        ("/buttons", "label", ControlList::Table),
    ] {
        for row in state
            .pointer(pointer)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            out.push(Control {
                press: row[key].as_str().unwrap_or_default().to_owned(),
                rect: Rect::around(
                    number(row, "at_x"),
                    number(row, "at_y"),
                    number(row, "w"),
                    number(row, "h"),
                ),
                list,
            });
        }
    }
    for row in state
        .pointer("/chat/controls")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        out.push(Control {
            press: row["press"].as_str().unwrap_or_default().to_owned(),
            rect: Rect::around(
                number(row, "x"),
                number(row, "y"),
                number(row, "w"),
                number(row, "h"),
            ),
            list: ControlList::Chat,
        });
    }
    out
}

fn cards(state: &Value) -> Vec<Card> {
    state["cards"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| Card {
            name: row["name"].as_str().unwrap_or_default().to_owned(),
            zone: row["zone"].as_str().unwrap_or_default().to_owned(),
            object: row["object"].as_u64().unwrap_or_default(),
            count: row["count"].as_u64().unwrap_or(1),
            rect: Rect::around(
                number(row, "at_x"),
                number(row, "at_y"),
                number(row, "w"),
                number(row, "h"),
            ),
        })
        .collect()
}
