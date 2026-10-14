//! The dev-control node dump, with explicit ancestry and pixel geometry.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct Node {
    pub d: usize,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    #[serde(default)]
    pub k: Option<String>,
    #[serde(default)]
    pub r: Option<String>,
    #[serde(default)]
    pub t: Option<String>,
    #[serde(default)]
    pub tw: Option<f64>,
    #[serde(default)]
    pub th: Option<f64>,
    #[serde(default)]
    pub ln: Option<usize>,
    #[serde(default)]
    pub hid: bool,
    #[serde(skip_deserializing)]
    pub parent: Option<usize>,
    #[serde(skip_deserializing)]
    pub index: usize,
    #[serde(skip)]
    pub role_present: bool,
}
impl Node {
    pub fn kind(&self) -> &str {
        self.k.as_deref().unwrap_or("")
    }
    pub fn text(&self) -> &str {
        self.t.as_deref().unwrap_or("")
    }
    pub fn right(&self) -> f64 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }
    pub fn rect(&self) -> String {
        format!("[{:.0},{:.0} {:.0}x{:.0}]", self.x, self.y, self.w, self.h)
    }
    pub fn inside(&self, parent: &Self, slack: f64) -> bool {
        self.x >= parent.x - slack
            && self.y >= parent.y - slack
            && self.right() <= parent.right() + slack
            && self.bottom() <= parent.bottom() + slack
    }
    pub fn overlap(&self, b: &Self) -> f64 {
        (self.right().min(b.right()) - self.x.max(b.x)).max(0.0)
            * (self.bottom().min(b.bottom()) - self.y.max(b.y)).max(0.0)
    }
}
#[derive(Clone, Debug)]
pub(super) struct Tree(pub Vec<Node>);
impl Tree {
    pub fn read(state: &Value, root: Option<&str>) -> Result<Self> {
        Self::from_nodes(serde_json::from_value(state["shell_nodes"].clone())?, root)
    }
    pub fn from_nodes(raw: Vec<Node>, root: Option<&str>) -> Result<Self> {
        let mut out: Vec<Node> = Vec::new();
        let mut stack: Vec<usize> = Vec::new();
        let mut keep = false;
        for mut n in raw {
            ensure!(
                [n.x, n.y, n.w, n.h].iter().all(|v| v.is_finite()),
                "nonfinite node geometry"
            );
            ensure!(
                n.tw.is_none_or(f64::is_finite)
                    && n.th.is_none_or(f64::is_finite)
                    && (n.tw.is_none() || n.th.is_some()),
                "incomplete or nonfinite glyph measurements"
            );
            if n.d == 0 {
                keep = root.is_none_or(|r| n.r.as_deref() == Some(r));
            }
            if !keep {
                continue;
            }
            while stack.last().is_some_and(|&i| out[i].d >= n.d) {
                stack.pop();
            }
            n.role_present |= n.k.is_some();
            n.parent = stack.last().copied();
            n.index = out.len();
            stack.push(n.index);
            out.push(n);
        }
        ensure!(!out.is_empty(), "no shell nodes for requested root");
        Ok(Self(out))
    }
    pub fn children(&self, i: usize) -> impl Iterator<Item = &Node> {
        self.0.iter().filter(move |n| n.parent == Some(i))
    }
    pub fn descendants(&self, i: usize) -> Vec<&Node> {
        let mut out = Vec::new();
        let mut frontier = vec![i];
        while let Some(i) = frontier.pop() {
            for n in self.children(i) {
                out.push(n);
                frontier.push(n.index);
            }
        }
        out
    }
    pub fn ancestors(&self, i: usize, stop: Option<usize>) -> Vec<&Node> {
        let mut out = Vec::new();
        let mut here = self.0[i].parent;
        while let Some(i) = here {
            if Some(i) == stop {
                break;
            }
            out.push(&self.0[i]);
            here = self.0[i].parent;
        }
        out
    }
    pub fn owner(&self, n: &Node, kinds: &[&str]) -> Option<&Node> {
        self.ancestors(n.index, None)
            .into_iter()
            .find(|a| kinds.contains(&a.kind()))
    }
    pub fn hidden(&self, n: &Node) -> bool {
        n.hid || n.w <= 0.0 || n.h <= 0.0 || self.ancestors(n.index, None).iter().any(|a| a.hid)
    }
    pub fn label(&self, i: usize) -> String {
        const NON_LABEL: &[&str] = &["keycap", "count"];
        self.descendants(i)
            .into_iter()
            .filter(|n| {
                !NON_LABEL.contains(&n.kind())
                    && !self
                        .ancestors(n.index, Some(i))
                        .iter()
                        .any(|a| NON_LABEL.contains(&a.kind()))
            })
            .map(Node::text)
            .filter(|t| !t.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }
    pub fn subtree(&self, i: usize) -> Result<Self> {
        // Source order (rather than DFS discovery order) preserves depth.
        let picked = self
            .0
            .iter()
            .filter(|n| n.index == i || self.ancestors(n.index, None).iter().any(|a| a.index == i));
        let depth = self.0[i].d;
        Self::from_nodes(
            picked
                .map(|n| {
                    let mut n = n.clone();
                    n.d -= depth;
                    n
                })
                .collect(),
            None,
        )
    }
}
