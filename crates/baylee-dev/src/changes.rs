//! What a change touches, for `ci.yml`'s `changes` job: changed paths on
//! stdin, one per line, out come `key=true|false` lines for `$GITHUB_OUTPUT`.
//!
//! - `code`: anything compiled, packaged or run by a Rust job: crates, xtask,
//!   Cargo files, data, LICENSE/NOTICE (both are packaged), ...
//! - `docs`: Markdown and `docs/`. Not inert: tests `include_str!`
//!   `docs/card-dsl.md`, `docs/llm-seat.md` and `docs/terms-placeholder*`, and
//!   the gateway reads `docs/` at test time, so a docs change still runs the
//!   debug suite.
//! - `web`: `web/feedback/`, the feedback UI, which only its own job builds.
//! - `ci`: the workflows, their actions and this classifier: a CI change runs
//!   all of CI, because that is the only way to see it work.
//!
//! Paths no job reads (agent scratch directories, the other `web/` pages, and
//! the training area: `baylee-train`, `baylee-train-ops` and `tools/trainer`,
//! which are checked locally and not by CI, owner 30.10.2026) set nothing. An empty list, or a diff that could not be computed, is the
//! caller's to treat as "everything" (`ci.yml` does).
//!
//! Locally: `git diff --name-only origin/main | cargo dev ci-changes`
use std::{
    io::{self, BufRead},
    path::Path,
};

/// The four answers, in the order `ci.yml` reads them: four independent
/// outputs, not a state, which is why they are four bools.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Classes {
    code: bool,
    docs: bool,
    web: bool,
    ci: bool,
}

/// This file's own path: editing the classifier is a CI change.
const SELF: &str = "crates/baylee-dev/src/changes.rs";

/// The cargo aliases CI builds with (`build-client`, `check-client`,
/// `build-servers`): changing one changes what the packages are.
const ALIASES: &str = ".cargo/config.toml";

/// The training area, which CI does not build, lint or test (owner,
/// 30.10.2026; `docs/releasing.md` §"CI"). A trailing slash each, so a crate
/// whose name merely starts the same is still code.
const TRAINING: [&str; 3] = [
    "crates/baylee-train/",
    "crates/baylee-train-ops/",
    "tools/trainer/",
];

/// Classifies paths; the first matching rule wins, as the script's `case` did.
pub fn classify<'a>(paths: impl IntoIterator<Item = &'a str>) -> Classes {
    let mut classes = Classes::default();
    for path in paths.into_iter().map(str::trim).filter(|p| !p.is_empty()) {
        if path.starts_with(".github/") || path == SELF || path == ALIASES {
            classes.ci = true;
        } else if path.starts_with("web/feedback/") {
            classes.web = true;
        } else if [".claude/", ".agents/", ".codex/", "web/"]
            .iter()
            .chain(&TRAINING)
            .any(|prefix| path.starts_with(prefix))
        {
        } else if path.starts_with("docs/")
            || Path::new(path).extension().is_some_and(|e| e == "md")
        {
            classes.docs = true;
        } else {
            classes.code = true;
        }
    }
    classes
}

impl std::fmt::Display for Classes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "code={}", self.code)?;
        writeln!(f, "docs={}", self.docs)?;
        writeln!(f, "web={}", self.web)?;
        writeln!(f, "ci={}", self.ci)
    }
}

/// Reads stdin, prints the classes.
pub fn run() -> anyhow::Result<i32> {
    let lines: Vec<String> = io::stdin().lock().lines().collect::<Result<_, _>>()?;
    print!("{}", classify(lines.iter().map(String::as_str)));
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of(paths: &[&str]) -> Classes {
        classify(paths.iter().copied())
    }

    #[test]
    fn each_class_is_reached_by_its_own_paths() {
        assert_eq!(
            of(&["crates/baylee-engine/src/lib.rs"]),
            Classes {
                code: true,
                ..Classes::default()
            }
        );
        assert_eq!(
            of(&["Cargo.lock", "NOTICE"]),
            Classes {
                code: true,
                ..Classes::default()
            }
        );
        assert_eq!(
            of(&["docs/protocol.md", "README.md"]),
            Classes {
                docs: true,
                ..Classes::default()
            }
        );
        assert_eq!(
            of(&["web/feedback/src/App.tsx"]),
            Classes {
                web: true,
                ..Classes::default()
            }
        );
        assert_eq!(
            of(&[".github/workflows/ci.yml"]),
            Classes {
                ci: true,
                ..Classes::default()
            }
        );
        assert_eq!(
            of(&[SELF]),
            Classes {
                ci: true,
                ..Classes::default()
            }
        );
        assert_eq!(
            of(&[ALIASES]),
            Classes {
                ci: true,
                ..Classes::default()
            }
        );
    }

    #[test]
    fn scratch_directories_and_other_web_pages_set_nothing() {
        let quiet = [
            ".claude/x.md",
            ".agents/skills/x/SKILL.md",
            ".codex/hooks.json",
            "web/site/index.html",
            "",
            "  ",
        ];
        assert_eq!(of(&quiet), Classes::default());
    }

    /// The training area is local only: a change to it alone runs no CI
    /// job, its Markdown included, while a crate that only shares the name's
    /// start, or the training docs under `docs/`, still count.
    #[test]
    fn the_training_area_alone_sets_nothing() {
        let training = [
            "crates/baylee-train/src/lib.rs",
            "crates/baylee-train/Cargo.toml",
            "crates/baylee-train-ops/src/evaluation/publication.rs",
            "crates/baylee-train-ops/README.md",
            "tools/trainer/training4/bc_mixture/metadata.py",
        ];
        assert_eq!(of(&training), Classes::default());
        assert_eq!(
            of(&["crates/baylee-trainer/src/lib.rs", "tools/trainers.py"]),
            Classes {
                code: true,
                ..Classes::default()
            }
        );
        assert_eq!(
            of(&["docs/trained-ai.md"]),
            Classes {
                docs: true,
                ..Classes::default()
            }
        );
        let mut mixed = training.to_vec();
        mixed.push("crates/baylee-engine/src/lib.rs");
        assert!(of(&mixed).code);
    }

    #[test]
    fn markdown_inside_a_crate_is_docs_and_the_output_format_is_ci_ymls() {
        // `*.md` anywhere was a `case` arm after the ignore list, so a crate's
        // README is docs, not code: the debug suite still runs on docs.
        assert_eq!(
            of(&["crates/baylee-client/CLAUDE.md"]),
            Classes {
                docs: true,
                ..Classes::default()
            }
        );
        let all = of(&["crates/a.rs", "docs/b.md", "web/feedback/c", ".github/d"]);
        assert_eq!(all.to_string(), "code=true\ndocs=true\nweb=true\nci=true\n");
    }
}
