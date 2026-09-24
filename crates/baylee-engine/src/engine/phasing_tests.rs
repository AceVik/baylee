//! Phased-out permanents are treated as though they do not exist, except by
//! rules and effects that mention them (CR 702.26b) — #209.
//!
//! `GameState::battlefield_seen` (and `battlefield_view`, which collects it)
//! is the battlefield as the rules see it. A raw
//! `zones.list(ZoneLocation::Battlefield)` walk sees phased-out permanents,
//! and `eval::matches` never reads `PHASED_OUT`, so every raw walk is either
//! a defect or a deliberate exception. The lint below makes each one say
//! which: a raw walk carries a `// phasing:` comment giving its reason, or it
//! is still in [`UNAUDITED`], a table that only shrinks as walks are
//! audited. The lint reads that one spelling. Walks over every object
//! (state hashing, the projection refresh over `arena`, cleanup's damage
//! wipe, which CR 514.2 extends to phased-out permanents) are not
//! battlefield queries and are not counted.

use super::testkit::*;
use super::*;
use crate::object::Status;
use crate::zone::ZoneLocation;

fn drowned_catacomb() -> baylee_core::ids::CardIndex {
    card_index("819fc966-434e-470f-91e9-a38df974ad17")
}

fn blackcleave_cliffs() -> baylee_core::ids::CardIndex {
    card_index("5ad94412-6f79-4c5d-bbd4-4ef5779a7b6d")
}

fn swamp() -> baylee_core::ids::CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

/// Plays `land` from hand on turn one over `board`, with the first
/// `phased` permanents of `board` phased out, and says whether it arrived
/// tapped.
fn arrives_tapped(
    board: &[baylee_core::ids::CardIndex],
    phased: usize,
    land: baylee_core::ids::CardIndex,
) -> bool {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(209, basic_forest())
        .battlefield(0, board)
        .hand(0, &[land])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let seated: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Battlefield).clone();
    let state = engine.dev_state_mut(p0).expect("the harness trusts itself");
    for id in seated.iter().take(phased) {
        state
            .object_mut(*id)
            .expect("seated")
            .status
            .insert(Status::PHASED_OUT);
    }
    engine.refresh_offer();
    let card = in_hand(&engine, p0, land).expect("in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card })
        .expect("a land drop on turn one");
    engine
        .state()
        .object(card)
        .expect("played")
        .status
        .contains(Status::TAPPED)
}

/// "This land enters tapped unless you control an Island or a Swamp." A
/// phased-out Swamp is not controlled by anybody the rules can see.
#[test]
fn a_phased_out_swamp_does_not_untap_a_checkland() {
    assert!(
        !arrives_tapped(&[swamp()], 0, drowned_catacomb()),
        "the control: a Swamp that is there lets the Catacomb enter untapped"
    );
    assert!(
        arrives_tapped(&[swamp()], 1, drowned_catacomb()),
        "a phased-out Swamp still satisfied the check"
    );
}

/// "This land enters tapped unless you control two or fewer other lands."
/// Three lands with one phased out is two.
#[test]
fn a_phased_out_land_does_not_count_against_a_fastland() {
    let three = [swamp(), swamp(), swamp()];
    assert!(
        arrives_tapped(&three, 0, blackcleave_cliffs()),
        "the control: three other lands make the Cliffs enter tapped"
    );
    assert!(
        !arrives_tapped(&three, 1, blackcleave_cliffs()),
        "a phased-out land was counted as one of the three"
    );
}

/// Raw battlefield walks with no `// phasing:` reason yet, per file, relative
/// to `crates/baylee-engine/src`. Measured 2026-09-24. Equality, not a
/// ceiling: auditing a walk (switching it to `battlefield_seen`, or giving
/// it a reason) makes this test fail until the row is lowered, so the table
/// cannot go stale in the direction that hides work.
const UNAUDITED: &[(&str, usize)] = &[
    ("combat.rs", 2),
    ("engine/abilities.rs", 1),
    ("engine/progress.rs", 11),
    ("eval.rs", 6),
    ("layers.rs", 1),
    ("resolve/chosen.rs", 1),
    ("resolve/control.rs", 1),
    ("resolve/counters.rs", 2),
    ("resolve/mana.rs", 1),
    ("resolve/mod.rs", 4),
    ("resolve/tokens.rs", 2),
    ("resolve/zones.rs", 2),
    ("sba.rs", 3),
    ("state.rs", 2),
    ("trigger.rs", 1),
];

/// Where the lint looks: the engine's own source, tests excluded.
fn is_test_source(rel: &str) -> bool {
    rel.ends_with("_tests.rs")
        || rel.starts_with("engine/card_tests/")
        || rel.starts_with("engine/combo_tests/")
        || matches!(
            rel,
            "engine/testkit.rs" | "engine/synthetic.rs" | "engine/tests.rs"
        )
}

fn sources(dir: &std::path::Path, root: &std::path::Path, out: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).expect("engine source directory") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            sources(&path, root, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let rel = path
                .strip_prefix(root)
                .expect("under the root")
                .to_string_lossy()
                .replace('\\', "/");
            if !is_test_source(&rel) {
                let text = std::fs::read_to_string(&path).expect("readable source");
                out.push((rel, text));
            }
        }
    }
}

/// Per file: raw walks, and how many of them give no reason.
fn walks(text: &str) -> (usize, usize) {
    // An inline test module is test code too, and closes its file by
    // convention; a `#[cfg(test)] mod x;` declaration is not one.
    let all: Vec<&str> = text.lines().collect();
    let cut = all
        .windows(2)
        .position(|w| {
            w[0].trim() == "#[cfg(test)]"
                && w[1].starts_with("mod ")
                && w[1].trim_end().ends_with('{')
        })
        .unwrap_or(all.len());
    let lines = &all[..cut];
    let mut raw = 0;
    let mut unexplained = 0;
    for (i, line) in lines.iter().enumerate() {
        let comment = line.trim_start().starts_with("//");
        if !comment && line.contains(".list(") && line.contains("ZoneLocation::Battlefield)") {
            raw += 1;
            let reasoned = lines[i.saturating_sub(3)..=i]
                .iter()
                .any(|l| l.contains("// phasing:"));
            if !reasoned {
                unexplained += 1;
            }
        }
    }
    (raw, unexplained)
}

/// Every raw battlefield walk in the engine says why it may see phased-out
/// permanents, or is still on the list of walks nobody has audited.
#[test]
fn every_raw_battlefield_walk_gives_its_phasing_reason() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&root, &root, &mut files);
    let mut total = 0;
    let mut found: Vec<(String, usize)> = Vec::new();
    for (rel, text) in &files {
        let (raw, unexplained) = walks(text);
        total += raw;
        if unexplained > 0 {
            found.push((rel.clone(), unexplained));
        }
    }
    found.sort();
    // A floor and a ceiling on what was read: a walk that found no files,
    // or matched something it should not, would otherwise pass as clean.
    assert!(
        (30..=120).contains(&files.len()),
        "read {} engine source files, measured 36 on 2026-09-24",
        files.len()
    );
    assert!(
        (30..=200).contains(&total),
        "found {total} raw battlefield walks, measured 43 on 2026-09-24"
    );
    let want: Vec<(String, usize)> = UNAUDITED
        .iter()
        .map(|(file, n)| ((*file).to_string(), *n))
        .collect();
    assert_eq!(
        found,
        want,
        "raw battlefield walks without a `// phasing:` reason changed. A new \
         walk wants `GameState::battlefield_seen` or a reason; an audited one \
         wants its row lowered. Today's table:\n{}",
        found
            .iter()
            .map(|(file, n)| format!("    (\"{file}\", {n}),"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The lint's counter-test: it reads both halves of a walk.
#[test]
fn the_lint_tells_a_reasoned_walk_from_a_bare_one() {
    let bare = "fn f() {\n    s.zones\n        .list(ZoneLocation::Battlefield)\n}\n";
    let reasoned = "fn f() {\n    // phasing: the untap step phases them in.\n    s.zones\n        .list(ZoneLocation::Battlefield)\n}\n";
    let tested =
        "fn f() {}\n#[cfg(test)]\nmod tests {\n    s.zones.list(ZoneLocation::Battlefield);\n}\n";
    let declared =
        "#[cfg(test)]\nmod x_tests;\nfn f() {\n    s.zones.list(ZoneLocation::Battlefield)\n}\n";
    let quoted =
        "/// Unlike `zones.list(ZoneLocation::Battlefield)`, this skips them.\nfn f() {}\n";
    assert_eq!(walks(bare), (1, 1));
    assert_eq!(walks(reasoned), (1, 0));
    assert_eq!(walks(tested), (0, 0));
    assert_eq!(walks(declared), (1, 1), "a module declaration is not a cut");
    assert_eq!(walks(quoted), (0, 0), "a comment quoting a walk is not one");
}
