//! Layout contracts, ported without changing the original tolerances.
use super::{
    role_contract::ROLE_BUDGETS,
    tree::{Node, Tree},
};
pub(super) const SIZES: &[(u32, u32)] = &[
    (1920, 1080),
    (960, 700),
    (1180, 820),
    (844, 390),
    (920, 443),
    (640, 360),
    (2560, 1440),
];
pub(super) const TOUCH: &[(u32, u32)] = &[(1180, 820), (844, 390), (920, 443), (640, 360)];
const CONTROLS: &[&str] = &[
    "button", "segment", "chip", "tab", "nav", "pill", "keycap", "stepper", "toggle",
];

pub(super) fn overflow(tree: &Tree) -> Vec<String> {
    tree.0
        .iter()
        .filter_map(|n| {
            let p = &tree.0[n.parent?];
            if n.text().trim().is_empty()
                || n.hid
                || p.kind() == "scroll"
                || (n.w == 0.0 && n.h == 0.0)
                || n.inside(p, 1.0)
            {
                return None;
            }
            Some(format!(
                "overflow: {:?} {} leaves {}",
                short(n.text(), 40),
                n.rect(),
                p.rect()
            ))
        })
        .collect()
}
fn short(s: &str, len: usize) -> String {
    s.chars().take(len).collect()
}
pub(super) fn window(tree: &Tree, width: f64) -> Vec<String> {
    tree.0
        .iter()
        .filter(|n| n.kind() == "hit" && (n.x < -1.0 || n.right() > width + 1.0))
        .map(|n| format!("window: {} leaves width {width:.0}", n.rect()))
        .collect()
}
pub(super) fn siblings(tree: &Tree) -> Vec<String> {
    let mut faults = Vec::new();
    for n in &tree.0 {
        let rows: Vec<_> = tree
            .children(n.index)
            .filter(|n| matches!(n.kind(), "row" | "tile"))
            .collect();
        for (i, a) in rows.iter().enumerate() {
            for b in &rows[i + 1..] {
                if a.overlap(b) > 1.0 {
                    faults.push(format!("siblings: {} and {} intersect", a.rect(), b.rect()));
                }
            }
        }
    }
    faults
}
pub(super) fn budget(tree: &Tree, german: bool) -> Vec<String> {
    tree.0
        .iter()
        .filter_map(|n| {
            let &(_, en, de) = ROLE_BUDGETS.iter().find(|&&(k, _, _)| k == n.kind())?;
            let text = tree.label(n.index);
            let len = text.chars().count();
            let limit = if german { de } else { en };
            (len > limit).then(|| format!("budget: {} {text:?} is {len} > {limit}", n.kind()))
        })
        .collect()
}
pub(super) fn hit(tree: &Tree, touch: bool) -> Vec<String> {
    tree.0
        .iter()
        .filter(|n| touch && n.kind() == "hit" && (n.w < 43.5 || n.h < 43.5))
        .map(|n| format!("hit: {} under 44 x 44", n.rect()))
        .collect()
}
pub(super) fn fit(tree: &Tree) -> Vec<String> {
    tree.0
        .iter()
        .filter_map(|n| {
            let tw = n.tw?;
            let th = n.th?;
            if n.text().trim().is_empty() || tree.hidden(n) || tree.owner(n, &["field"]).is_some() {
                return None;
            }
            let slack = if n.ln.unwrap_or(1) <= 1 { 4.0 } else { 10.0 };
            (tw > n.w + slack || th > n.h + 1.5).then(|| {
                format!(
                    "fit: {:?} needs {tw:.0}x{th:.0} in {}",
                    short(n.text(), 40),
                    n.rect()
                )
            })
        })
        .collect()
}
pub(super) fn lines(tree: &Tree) -> Vec<String> {
    tree.0
        .iter()
        .filter_map(|n| {
            if n.ln.unwrap_or(1) <= 1 || tree.hidden(n) {
                return None;
            }
            let owner = tree.owner(n, CONTROLS)?;
            Some(format!(
                "lines: {:?} breaks into {} lines in a {}",
                short(n.text(), 40),
                n.ln.unwrap_or(1),
                owner.kind()
            ))
        })
        .collect()
}
fn clipped(tree: &Tree, n: &Node) -> Node {
    let mut seen = n.clone();
    for up in tree
        .ancestors(n.index, None)
        .into_iter()
        .filter(|n| n.kind() == "scroll")
    {
        let x = seen.x.max(up.x);
        let y = seen.y.max(up.y);
        seen.w = (seen.right().min(up.right()) - x).max(0.0);
        seen.h = (seen.bottom().min(up.bottom()) - y).max(0.0);
        seen.x = x;
        seen.y = y;
    }
    seen
}
pub(super) fn text_overlap(tree: &Tree) -> Vec<String> {
    let texts: Vec<_> = tree
        .0
        .iter()
        .filter(|n| !n.text().trim().is_empty() && !tree.hidden(n))
        .map(|n| clipped(tree, n))
        .filter(|n| n.w > 0.0 && n.h > 0.0)
        .collect();
    let mut faults = Vec::new();
    for (i, a) in texts.iter().enumerate() {
        let layer = tree.owner(a, &["opaque"]).map(|n| n.index);
        for b in &texts[i + 1..] {
            if tree.owner(b, &["opaque"]).map(|n| n.index) != layer
                || a.overlap(b) <= 2.0
                || tree
                    .ancestors(b.index, None)
                    .iter()
                    .any(|n| n.index == a.index)
            {
                continue;
            }
            faults.push(format!(
                "overlap: {:?} {} and {:?} {}",
                short(a.text(), 30),
                a.rect(),
                short(b.text(), 30),
                b.rect()
            ));
        }
    }
    faults
}
pub(super) fn text(tree: &Tree) -> Vec<String> {
    let mut f = fit(tree);
    f.extend(lines(tree));
    f.extend(text_overlap(tree));
    f
}
pub(super) fn layout(
    tree: &Tree,
    width: f64,
    german: bool,
    touch: bool,
    with_text: bool,
) -> Vec<String> {
    let mut f = overflow(tree);
    f.extend(window(tree, width));
    f.extend(siblings(tree));
    f.extend(budget(tree, german));
    f.extend(hit(tree, touch));
    if with_text {
        f.extend(text(tree));
    }
    f
}
pub(super) fn centred(tree: &Tree, width: f64) -> Vec<String> {
    let top = tree
        .0
        .iter()
        .filter(|n| n.kind() == "header")
        .map(Node::bottom)
        .reduce(f64::max)
        .unwrap_or(0.0);
    let panels: Vec<_> = tree
        .0
        .iter()
        .filter(|n| {
            n.kind() == "panel"
                && n.y >= top - 1.0
                && !tree.hidden(n)
                && !(n.x <= 1.0 && n.w >= width - 2.0)
        })
        .collect();
    if panels.is_empty() {
        return vec!["centred: no panel under the header".into()];
    }
    let left = panels.iter().map(|n| n.x).fold(f64::INFINITY, f64::min);
    let right = width
        - panels
            .iter()
            .map(|n| n.right())
            .fold(f64::NEG_INFINITY, f64::max);
    let mut f = Vec::new();
    if (left - right).abs() > 0.03 * width {
        f.push(format!(
            "centred: margins {left:.0} left, {right:.0} right of {width:.0}"
        ));
    }
    let used = (width - left - right) / width;
    if width >= 2560.0 && used < 0.45 {
        f.push(format!(
            "centred: content fills {:.0}% of {width:.0}",
            used * 100.0
        ));
    }
    let tiles: Vec<_> = tree
        .0
        .iter()
        .filter(|n| n.kind() == "tile" && !tree.hidden(n))
        .collect();
    if width >= 2560.0 && tiles.is_empty() && used > 0.9 {
        f.push(format!(
            "centred: form fills {:.0}% of {width:.0}",
            used * 100.0
        ));
    }
    if let Some(holder) = panels.iter().max_by(|a, b| a.w.total_cmp(&b.w))
        && !tiles.is_empty()
    {
        let l = tiles.iter().map(|n| n.x).fold(f64::INFINITY, f64::min) - holder.x;
        let r = holder.right()
            - tiles
                .iter()
                .map(|n| n.right())
                .fold(f64::NEG_INFINITY, f64::max);
        if (l - r).abs() > 0.03 * width {
            f.push(format!(
                "centred: tiles {l:.0} from left, {r:.0} from right"
            ));
        }
    }
    f
}
pub(super) fn colophon(words: &str) -> Vec<String> {
    [
        (
            "the Fan Content notice",
            words.contains("Fan Content") || words.contains("Fan-Content"),
        ),
        ("Scryfall's credit", words.contains("Scryfall")),
        ("the AGPL source", words.contains("AGPL-3.0")),
    ]
    .into_iter()
    .filter(|&(_, ok)| !ok)
    .map(|(s, _)| format!("colophon: {s} missing"))
    .collect()
}
