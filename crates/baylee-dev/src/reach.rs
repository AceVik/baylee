//! What a diff reaches: the crates whose tests can tell the change from no
//! change. `cargo dev gate` tests those and leaves the rest to `cargo dev
//! land`, which tests every crate (owner, 13.11.2026).
//!
//! Three things carry a change from a file to a crate:
//!
//! - **ownership**: the crate whose directory the file lies in;
//! - **a path in a string**: a crate that names the file, or a directory above
//!   it, in its source. `include_str!("../../../docs/card-dsl.md")`,
//!   `#[path = "../../src/filters.rs"]`, `root.join("docs/releasing.md")` and
//!   a walk from `root.join("crates")` are all of this kind, and Cargo knows
//!   none of them;
//! - **a dependency** from there on, of any kind (normal, dev, build), and
//!   the binaries a test starts by their path ([`STARTED_BY_PATH`]). A crate
//!   that only reads a file while its tests run is tested itself, and the
//!   crates above it are not: their build has not changed.
//!
//! Where the question cannot be answered the answer is everything: a file
//! that changes the build for every crate, a file outside every crate that
//! nothing names and CI would compile or package, and a diff that reaches no
//! crate at all.
//!
//! What this cannot see is a path put together from pieces without a slash
//! (`dir.join(a).join(b)`), a reader that names a file by its tail alone, and
//! a test that is right only while another crate's untouched file stays as
//! it is. `cargo dev land` runs every test for that reason, and clippy still
//! compiles every crate here, so a change that breaks a dependent's build is
//! caught either way.
use crate::changes::{self, Class};
use crate::run::{cargo_with, command};
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// A binary a test starts by its path in the target directory, and the crate
/// whose tests do. `e2e_processes` looks for the agent beside its own
/// binary; the gateway does not depend on it.
const STARTED_BY_PATH: &[(&str, &str)] = &[("baylee-agent", "baylee-gateway")];

/// The source lines worth reading for a path: a string with a slash in it,
/// or a `join` of one name (`root.join("crates")`).
const PATHS: &str = r#""[^" ]*/[^" ]*"|join\("[^"/ ]+"\)"#;

/// One crate of the workspace.
#[derive(Debug)]
struct Member {
    name: String,
    /// Its directory from the root, with the slash that ends it.
    dir: String,
    /// The workspace crates it depends on, of every kind.
    deps: Vec<String>,
}

/// A path a crate's source names outside its own directory.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Named {
    /// From the root, without a trailing slash.
    target: String,
    /// Everything below `target` counts, not `target` alone.
    below: bool,
    /// Compiled into the crate (`include_str!`, `#[path]`, a build script)
    /// and not only read by its tests: the crates that depend on it are
    /// reached as well.
    taken: bool,
    by: String,
}

/// The paths of the checkout: what a string has to resolve to before it
/// counts as a path.
#[derive(Default)]
struct Tree {
    files: BTreeSet<String>,
    dirs: BTreeSet<String>,
}

impl Tree {
    fn new<'a>(files: impl IntoIterator<Item = &'a str>) -> Self {
        let mut tree = Self::default();
        for file in files {
            let mut dir = file;
            while let Some((parent, _)) = dir.rsplit_once('/') {
                if !tree.dirs.insert(parent.to_owned()) {
                    break;
                }
                dir = parent;
            }
            tree.files.insert(file.to_owned());
        }
        tree
    }
}

/// The crates and what their sources name.
pub struct Workspace {
    members: Vec<Member>,
    named: Vec<Named>,
}

/// What a diff reaches.
#[derive(Debug, PartialEq, Eq)]
pub enum Reach {
    /// Every crate, and why.
    Everything(String),
    /// These crates, each with the first thing that carried the change to
    /// it, out of this many.
    Crates(BTreeMap<String, String>, usize),
}

impl Reach {
    /// The nextest filterset that selects the reached crates' tests; `None`
    /// where that is every test.
    pub fn filterset(&self) -> Option<String> {
        match self {
            Self::Crates(crates, of) if crates.len() < *of => Some(
                crates
                    .keys()
                    .map(|name| format!("package(={name})"))
                    .collect::<Vec<_>>()
                    .join(" | "),
            ),
            _ => None,
        }
    }
}

impl Workspace {
    /// Reads the workspace: its crates from `cargo metadata`, its paths and
    /// the source lines that may name one from git, untracked files included.
    pub fn load(root: &Path) -> Result<Self> {
        let out = cargo_with(["metadata", "--no-deps", "--format-version", "1"])
            .current_dir(root)
            .output()
            .context("running cargo metadata")?;
        ensure!(out.status.success(), "cargo metadata failed");
        let members = members(&serde_json::from_slice(&out.stdout)?)?;
        let files = git(
            root,
            &["ls-files", "--cached", "--others", "--exclude-standard"],
        )?;
        let lines = git(
            root,
            &["grep", "--untracked", "-I", "-n", "-E", PATHS, "--", "*.rs"],
        )?;
        let named = named(&lines, &members, &Tree::new(files.lines()));
        Ok(Self { members, named })
    }

    /// What `changed` reaches; see the module's note.
    pub fn reach(&self, changed: &[String]) -> Reach {
        // `reached`: the crates to test. `queue`: those among them whose own
        // build changes, which is what a dependent is reached through.
        let mut reached = BTreeMap::new();
        let mut queue = Vec::new();
        for path in changed {
            if builds_every_crate(path) {
                return Reach::Everything(format!("{path} changes the build of every crate"));
            }
            let own = owner(&self.members, path);
            if let Some(own) = own {
                reached
                    .entry(own.name.clone())
                    .or_insert_with(|| format!("owns {path}"));
                queue.push(own.name.clone());
            }
            let mut named = false;
            for name in self.named.iter().filter(|name| name.holds(path)) {
                named = true;
                reached
                    .entry(name.by.clone())
                    .or_insert_with(|| format!("names {}", name.target));
                if name.taken {
                    queue.push(name.by.clone());
                }
            }
            if own.is_none() && !named && changes::class(path) == Class::Code {
                return Reach::Everything(format!(
                    "{path} lies in no crate and no source names it"
                ));
            }
        }
        let mut built = BTreeSet::new();
        while let Some(name) = queue.pop() {
            if !built.insert(name.clone()) {
                continue;
            }
            for member in &self.members {
                if member.deps.contains(&name) {
                    reached
                        .entry(member.name.clone())
                        .or_insert_with(|| format!("depends on {name}"));
                    queue.push(member.name.clone());
                }
            }
            for (binary, by) in STARTED_BY_PATH {
                if *binary == name {
                    reached
                        .entry((*by).to_owned())
                        .or_insert_with(|| format!("starts {name} by its path"));
                }
            }
        }
        if reached.is_empty() {
            return Reach::Everything("the diff reaches no crate".into());
        }
        Reach::Crates(reached, self.members.len())
    }
}

impl Named {
    fn holds(&self, path: &str) -> bool {
        path == self.target
            || (self.below
                && path
                    .strip_prefix(&self.target)
                    .is_some_and(|rest| rest.starts_with('/')))
    }
}

/// Files that change what every crate is built from or how its tests run:
/// any manifest (features unify across the workspace), the lock file, the
/// toolchain, cargo's own configuration and nextest's.
fn builds_every_crate(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    matches!(name, "Cargo.toml" | "Cargo.lock" | "rust-toolchain.toml")
        || path.starts_with(".cargo/")
        || path.starts_with(".config/")
}

/// The crate a path lies in: the one with the longest directory, since
/// crates nest (`crates/cards/` holds `crates/cards/base/`).
fn owner<'a>(members: &'a [Member], path: &str) -> Option<&'a Member> {
    members
        .iter()
        .filter(|member| path.starts_with(&member.dir))
        .max_by_key(|member| member.dir.len())
}

/// The workspace's crates out of `cargo metadata --no-deps`.
fn members(meta: &serde_json::Value) -> Result<Vec<Member>> {
    let root = meta["workspace_root"]
        .as_str()
        .context("cargo metadata names no workspace root")?;
    let packages = meta["packages"]
        .as_array()
        .context("cargo metadata lists no packages")?;
    let names: BTreeSet<&str> = packages.iter().filter_map(|p| p["name"].as_str()).collect();
    let mut members = Vec::new();
    for package in packages {
        let name = package["name"].as_str().context("a package has no name")?;
        let dir = package["manifest_path"]
            .as_str()
            .and_then(|manifest| manifest.strip_prefix(root))
            .and_then(|manifest| manifest.strip_suffix("Cargo.toml"))
            .with_context(|| format!("{name} has no manifest below the workspace root"))?;
        let deps = package["dependencies"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|dep| dep["name"].as_str())
            .filter(|dep| names.contains(dep))
            .map(str::to_owned)
            .collect();
        members.push(Member {
            name: name.to_owned(),
            dir: dir.trim_start_matches(['/', '\\']).replace('\\', "/"),
            deps,
        });
    }
    Ok(members)
}

/// The paths the sources name outside their own crate, out of `git grep -n`
/// lines (`file:line:text`).
///
/// A string counts when it resolves to a path of the checkout: from the
/// file's directory (`include_str!`, `#[path]`), from its crate's directory
/// (a join onto `CARGO_MANIFEST_DIR`) or from the root. A directory, and a
/// Rust file's directory (a module brings its children), count with
/// everything below them. A directory the crate itself lies in counts only
/// as `join("crates")` spells it, a walk over every crate: as a string with
/// a slash it is as often a prefix being cut off a path.
fn named(lines: &str, members: &[Member], tree: &Tree) -> Vec<Named> {
    let mut named = BTreeSet::new();
    for line in lines.lines() {
        let Some((file, text)) = line
            .split_once(':')
            .and_then(|(file, rest)| Some((file, rest.split_once(':')?.1)))
        else {
            continue;
        };
        let Some(by) = owner(members, file) else {
            continue;
        };
        let here = file.rsplit_once('/').map_or("", |(dir, _)| dir);
        // A string on a line of its own may be the argument of an `include`
        // a line above, so it counts as taken.
        let taken = text.contains("include")
            || text.contains("#[path")
            || text.trim_start().starts_with('"')
            || file.ends_with("/build.rs");
        // `walked`: the one spelling a walk over a directory around the crate
        // is taken from, so it passes the rule that leaves such a directory out.
        let mut add = |target: String, below: bool, walked: bool| {
            // Its own files are no reach, and a nested crate's are: ownership
            // decides, not the directory.
            let slashed = format!("{target}/");
            let own = owner(members, &slashed).is_some_and(|own| own.name == by.name);
            if walked || (!own && !by.dir.starts_with(&slashed)) {
                named.insert(Named {
                    target,
                    below,
                    taken,
                    by: by.name.clone(),
                });
            }
        };
        for (literal, joined) in literals(text) {
            if !literal.contains('/') {
                if joined && tree.dirs.contains(literal) {
                    add(literal.to_owned(), true, true);
                }
                continue;
            }
            let bases = [here, by.dir.trim_end_matches('/'), ""];
            for path in bases.iter().filter_map(|base| resolve(base, literal)) {
                if tree.dirs.contains(&path) {
                    add(path, true, false);
                } else if tree.files.contains(&path) {
                    match path.rsplit_once('/') {
                        Some((dir, name)) if is_rust(name) => add(dir.to_owned(), true, false),
                        _ => add(path, false, false),
                    }
                }
            }
        }
    }
    named.into_iter().collect()
}

/// Whether a file name is a Rust source's.
fn is_rust(name: &str) -> bool {
    Path::new(name).extension().is_some_and(|ext| ext == "rs")
}

/// The strings of a source line, each with whether `join(` stands before it.
/// Split at the quotes: an escaped quote misplaces the rest of its line,
/// which then resolves to no path.
fn literals(text: &str) -> impl Iterator<Item = (&str, bool)> {
    let mut pieces = text.split('"');
    let mut before = pieces.next().unwrap_or("");
    std::iter::from_fn(move || {
        let literal = pieces.next()?;
        let joined = before.ends_with("join(");
        before = pieces.next().unwrap_or("");
        Some((literal, joined))
    })
}

/// `literal` read as a path from `base`, both from the root; `None` when it
/// leaves the checkout or names the root itself.
fn resolve(base: &str, literal: &str) -> Option<String> {
    let mut parts: Vec<&str> = base.split('/').filter(|part| !part.is_empty()).collect();
    for part in literal.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// One git question about the checkout, as text. `git grep` finding nothing
/// exits 1 with nothing printed, which is an answer and not a failure.
fn git(root: &Path, args: &[&str]) -> Result<String> {
    let out = command("git", args)
        .current_dir(root)
        .output()
        .with_context(|| format!("running git {}", args[0]))?;
    ensure!(
        out.status.success() || (out.status.code() == Some(1) && out.stdout.is_empty()),
        "git {} failed",
        args[0]
    );
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// `cargo dev gate reach`: what the gate would test for these changed
/// paths, why, and the nextest filterset that selects it.
pub fn print(changed: Option<&[String]>) -> Result<i32> {
    let Some(changed) = changed else {
        println!("reach: cannot diff against origin/main - the gate tests every crate");
        return Ok(0);
    };
    let reach = Workspace::load(Path::new("."))?.reach(changed);
    match &reach {
        Reach::Everything(why) => println!("reach: every crate - {why}"),
        Reach::Crates(crates, of) => {
            println!("reach: {} of {of} crates", crates.len());
            for (name, why) in crates {
                println!("  {name}  ({why})");
            }
        }
    }
    if let Some(set) = reach.filterset() {
        println!("filterset: {set}");
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(name: &str, dir: &str, deps: &[&str]) -> Member {
        Member {
            name: name.into(),
            dir: dir.into(),
            deps: deps.iter().map(|&dep| dep.to_owned()).collect(),
        }
    }

    /// A small workspace: `core` under `app` and, as a dev-dependency would
    /// be, under `tool`; `cards` holding `base` in its own directory.
    fn workspace(lines: &str, files: &[&str]) -> Workspace {
        let members = vec![
            member("core", "crates/core/", &[]),
            member("app", "crates/app/", &["core"]),
            member("tool", "crates/tool/", &["core"]),
            member("lone", "crates/lone/", &[]),
            member("cards", "crates/cards/", &["base"]),
            member("base", "crates/cards/base/", &[]),
            member("baylee-agent", "crates/baylee-agent/", &[]),
            member("baylee-gateway", "crates/baylee-gateway/", &[]),
        ];
        let named = named(lines, &members, &Tree::new(files.iter().copied()));
        Workspace { members, named }
    }

    fn reached(workspace: &Workspace, changed: &[&str]) -> Vec<String> {
        let changed: Vec<String> = changed.iter().map(|&path| path.to_owned()).collect();
        match workspace.reach(&changed) {
            Reach::Crates(crates, _) => crates.into_keys().collect(),
            Reach::Everything(why) => vec![format!("everything: {why}")],
        }
    }

    #[test]
    fn a_string_is_read_as_a_path_from_a_directory() {
        assert_eq!(
            resolve("crates/a/src", "../../../docs/x.md").as_deref(),
            Some("docs/x.md")
        );
        assert_eq!(
            resolve("crates/a", "/../b/src").as_deref(),
            Some("crates/b/src")
        );
        assert_eq!(resolve("", "./docs//x.md").as_deref(), Some("docs/x.md"));
        assert_eq!(
            resolve("crates/a", "../../.."),
            None,
            "it leaves the checkout"
        );
        assert_eq!(
            resolve("crates/a", "../.."),
            None,
            "the root is no path to name"
        );
    }

    #[test]
    fn a_lines_strings_know_a_join_before_them() {
        let found: Vec<_> = literals(r#"root.join("crates").join(name), "a/b", x("c")"#).collect();
        assert_eq!(found, [("crates", true), ("a/b", false), ("c", false)]);
    }

    #[test]
    fn a_change_reaches_its_crate_and_what_depends_on_it() {
        let ws = workspace("", &[]);
        assert_eq!(
            reached(&ws, &["crates/core/src/lib.rs"]),
            ["app", "core", "tool"]
        );
        assert_eq!(reached(&ws, &["crates/app/src/lib.rs"]), ["app"]);
        // The nearest crate owns a file, and a crate around it does not.
        assert_eq!(
            reached(&ws, &["crates/cards/base/src/lib.rs"]),
            ["base", "cards"]
        );
        assert_eq!(reached(&ws, &["crates/cards/src/lib.rs"]), ["cards"]);
    }

    #[test]
    fn a_file_a_source_names_reaches_the_crate_that_names_it() {
        let ws = workspace(
            concat!(
                "crates/app/src/terms.rs:7:    include_str!(\"../../../docs/terms.md\"),\n",
                "crates/tool/src/main.rs:9:    root.join(\"docs/release/notes.md\")\n",
                "crates/lone/src/lib.rs:3:    concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../../data/sets\")\n",
                "crates/lone/src/lib.rs:4:    // see \"docs/nothing/there.md\"\n",
            ),
            &[
                "docs/terms.md",
                "docs/other.md",
                "docs/release/notes.md",
                "data/sets/a.json",
                "data/loose.json",
            ],
        );
        assert_eq!(reached(&ws, &["docs/terms.md"]), ["app"]);
        assert_eq!(reached(&ws, &["docs/release/notes.md"]), ["tool"]);
        assert_eq!(
            reached(&ws, &["data/sets/a.json"]),
            ["lone"],
            "a directory, below it"
        );
        // A doc nothing names is read by no test; alone it reaches no crate,
        // and beside a crate's change it adds nothing.
        assert_eq!(
            reached(&ws, &["docs/other.md"]),
            ["everything: the diff reaches no crate"]
        );
        assert_eq!(
            reached(&ws, &["docs/other.md", "crates/app/src/a.rs"]),
            ["app"]
        );
        // Data is compiled or packaged somewhere: unnamed, it is everyone's.
        assert_eq!(
            reached(&ws, &["data/loose.json"]),
            ["everything: data/loose.json lies in no crate and no source names it"]
        );
    }

    #[test]
    fn a_file_read_by_tests_stops_at_its_crate_and_one_compiled_in_goes_on() {
        let ws = workspace(
            concat!(
                "crates/core/tests/docs.rs:5:    root.join(\"docs/read.md\")\n",
                "crates/core/src/lib.rs:2:    include_str!(\"../../../docs/taken.md\")\n",
                "crates/core/src/lib.rs:8:        \"../../../docs/long/name/on/its/own/line.md\"\n",
            ),
            &[
                "docs/read.md",
                "docs/taken.md",
                "docs/long/name/on/its/own/line.md",
            ],
        );
        assert_eq!(reached(&ws, &["docs/read.md"]), ["core"]);
        assert_eq!(reached(&ws, &["docs/taken.md"]), ["app", "core", "tool"]);
        assert_eq!(
            reached(&ws, &["docs/long/name/on/its/own/line.md"]),
            ["app", "core", "tool"]
        );
        // The tests that start a binary are reached, their dependents are not.
        let started = Workspace {
            members: vec![
                member("baylee-agent", "crates/baylee-agent/", &[]),
                member("baylee-gateway", "crates/baylee-gateway/", &[]),
                member("above", "crates/above/", &["baylee-gateway"]),
            ],
            named: Vec::new(),
        };
        assert_eq!(
            reached(&started, &["crates/baylee-agent/src/main.rs"]),
            ["baylee-agent", "baylee-gateway"]
        );
    }

    #[test]
    fn a_module_taken_by_its_path_reaches_the_crate_that_takes_it() {
        let ws = workspace(
            "crates/cards/base/src/lib.rs:15:#[path = \"../../src/filters.rs\"]\n",
            &[
                "crates/cards/src/filters.rs",
                "crates/cards/src/filters/kinds.rs",
            ],
        );
        // `cards` depends on `base`, so Cargo carries nothing this way round.
        assert_eq!(
            reached(&ws, &["crates/cards/src/filters/kinds.rs"]),
            ["base", "cards"]
        );
    }

    #[test]
    fn a_path_inside_the_crate_or_around_it_is_no_reach() {
        let ws = workspace(
            concat!(
                "crates/app/src/lib.rs:1:    include_str!(\"../assets/a.txt\")\n",
                "crates/app/src/lib.rs:2:    at.strip_prefix(\"crates/\")\n",
                "crates/tool/src/lib.rs:3:    root.join(\"crates\").join(name)\n",
            ),
            &["crates/app/assets/a.txt", "crates/lone/src/lib.rs"],
        );
        assert_eq!(
            reached(&ws, &["crates/lone/src/lib.rs"]),
            ["lone", "tool"],
            "the walk over every crate reads it, the cut-off prefix does not"
        );
    }

    #[test]
    fn a_binary_started_by_its_path_reaches_the_tests_that_start_it() {
        let ws = workspace("", &[]);
        assert_eq!(
            reached(&ws, &["crates/baylee-agent/src/main.rs"]),
            ["baylee-agent", "baylee-gateway"]
        );
    }

    #[test]
    fn what_builds_every_crate_reaches_every_crate() {
        let ws = workspace("", &[]);
        for path in [
            "Cargo.lock",
            "crates/app/Cargo.toml",
            "rust-toolchain.toml",
            ".cargo/config.toml",
            ".config/nextest.toml",
        ] {
            assert_eq!(
                reached(&ws, &[path, "crates/app/src/lib.rs"]),
                [format!(
                    "everything: {path} changes the build of every crate"
                )]
            );
        }
        assert_eq!(reached(&ws, &[]), ["everything: the diff reaches no crate"]);
    }

    #[test]
    fn the_filterset_names_the_reached_crates_and_nothing_when_that_is_all() {
        let some = Reach::Crates(
            [("a".into(), String::new()), ("b".into(), String::new())].into(),
            3,
        );
        assert_eq!(
            some.filterset().as_deref(),
            Some("package(=a) | package(=b)")
        );
        let all = Reach::Crates([("a".into(), String::new())].into(), 1);
        assert_eq!(all.filterset(), None);
        assert_eq!(Reach::Everything(String::new()).filterset(), None);
    }

    /// This checkout, from the crate's own directory: a test changes no
    /// working directory.
    fn root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// The checkout's own hidden edges, the ones a narrow gate would
    /// otherwise skip: each was found by hand on 13.11.2026.
    #[test]
    fn this_checkout_carries_its_hidden_edges() {
        let ws = Workspace::load(&root()).unwrap();
        let reaches = |path: &str, name: &str| {
            let reach = ws.reach(&[path.to_owned()]);
            assert!(
                matches!(&reach, Reach::Crates(crates, _) if crates.contains_key(name)),
                "{path} should reach {name}: {reach:?}"
            );
        };
        // A module taken by `#[path]` against the dependency's direction.
        reaches("crates/cards/src/filters.rs", "baylee-cards-base");
        // A document a build reads, and one a test reads at run time.
        reaches("docs/card-dsl.md", "baylee-cards-dsl");
        reaches("docs/releasing.md", "baylee-ops");
        reaches("docs/gateway-legal/gateway-terms.de.md", "baylee-gateway");
        // A binary started by its path, and a walk over every crate.
        reaches("crates/baylee-agent/src/lib.rs", "baylee-gateway");
        reaches("crates/baylee-view/src/lib.rs", "baylee-tools");
        // A dev-dependency carries as a dependency does.
        reaches("crates/baylee-seathost/src/lib.rs", "baylee-gateway");
    }

    /// Every `include_str!`, `include_bytes!` and `#[path]` that leaves its
    /// crate is an edge the reach has: the file it takes reaches the crate
    /// that takes it.
    #[test]
    fn every_file_taken_from_outside_a_crate_reaches_it() {
        let ws = Workspace::load(&root()).unwrap();
        let files = git(&root(), &["ls-files"]).unwrap();
        let tree = Tree::new(files.lines());
        let taking = r#"(include_(str|bytes)!\(|#\[path *= *)"\.\./"#;
        let lines = git(&root(), &["grep", "-n", "-E", taking, "--", "*.rs"]).unwrap();
        let mut seen = 0;
        for line in lines.lines() {
            let (file, text) = line.split_once(':').unwrap();
            // A comment that shows such a line takes nothing.
            let code = text.split_once(':').map_or(text, |(_, code)| code);
            if code.trim_start().starts_with("//") {
                continue;
            }
            let Some(by) = owner(&ws.members, file) else {
                continue;
            };
            let here = file.rsplit_once('/').map_or("", |(dir, _)| dir);
            let from = text.find("\"../").unwrap() + 1;
            let literal = text[from..].split('"').next().unwrap();
            let taken = resolve(here, literal).filter(|path| tree.files.contains(path));
            let taken = taken.unwrap_or_else(|| panic!("{file} takes {literal}, which is no file"));
            if taken.starts_with(&by.dir) {
                continue;
            }
            seen += 1;
            // Every crate is reached where the file is everyone's (a manifest).
            let reach = ws.reach(std::slice::from_ref(&taken));
            assert!(
                match &reach {
                    Reach::Crates(crates, _) => crates.contains_key(&by.name),
                    Reach::Everything(_) => true,
                },
                "{taken} is taken by {} ({file}) and does not reach it: {reach:?}",
                by.name
            );
        }
        assert!(
            seen >= 10,
            "only {seen} such lines: the search found too little"
        );
    }

    /// The table is not allowed to outlive what it describes.
    #[test]
    fn the_tests_that_start_a_binary_by_path_still_name_it() {
        let ws = Workspace::load(&root()).unwrap();
        for (binary, by) in STARTED_BY_PATH {
            let dir = &ws.members.iter().find(|m| m.name == *by).unwrap().dir;
            let named = format!("\"{binary}\"");
            let found = git(&root(), &["grep", "-l", "-F", &named, "--", dir]).unwrap();
            assert!(!found.is_empty(), "{by} no longer names {binary}");
        }
    }
}
