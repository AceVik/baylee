//! Two rules about the shell's sizes, held as tests over the source tree
//! (the shell design, §8).
//!
//! 1. A shell module writes no bare numeric `px(…)`: every length is either
//!    scaled by the text step (`m.px(n)`) or says it is not
//!    (`px_fixed(n)`). [`SHELL_MODULES`] lists the modules the rule covers;
//!    each screen's package adds its module when it moves onto the shell,
//!    until the list is the lobby's whole.
//! 2. The table's interface never reads the shell's metrics: the table's
//!    sizes are x-height corrections of their own (`hud.rs`, `UI_SCALE`)
//!    and do not follow the shell's text step.

/// The modules (relative to `src/`) whose lengths follow the text step.
pub const SHELL_MODULES: &[&str] = &["shellkit"];

/// The lines (1-based) of `source` that write a bare numeric `px(`.
///
/// `m.px(…)` (a method) and `px_fixed(…)` are not bare; a comment is
/// ignored.
#[must_use]
pub fn bare_px_literals(source: &str) -> Vec<usize> {
    let mut found = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let code = line.split("//").next().unwrap_or("");
        let bytes = code.as_bytes();
        let mut from = 0;
        while let Some(at) = code[from..].find("px(") {
            let start = from + at;
            from = start + 3;
            let before = start.checked_sub(1).map(|i| bytes[i]);
            // A method call (`.px(`) or a longer name (`ui_px(`) is not ours.
            if before.is_some_and(|b| b == b'.' || b == b'_' || b.is_ascii_alphanumeric()) {
                continue;
            }
            let rest = code[from..].trim_start();
            if rest.starts_with(|c: char| c.is_ascii_digit() || c == '-') {
                found.push(index + 1);
                break;
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn sources(under: &Path, out: &mut Vec<PathBuf>) {
        if under.is_file() {
            out.push(under.to_path_buf());
            return;
        }
        let Ok(entries) = std::fs::read_dir(under) else {
            return;
        };
        let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                sources(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    fn module_files(module: &str) -> Vec<PathBuf> {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut out = Vec::new();
        let file = src.join(format!("{module}.rs"));
        if file.exists() {
            out.push(file);
        }
        sources(&src.join(module), &mut out);
        out
    }

    #[test]
    fn no_shell_module_writes_a_bare_numeric_px() {
        let mut files = 0;
        for module in SHELL_MODULES {
            for path in module_files(module) {
                files += 1;
                let source = std::fs::read_to_string(&path).expect("a source file");
                // A file's own tests may spell the forbidden shape on purpose
                // (this one does, to prove the lint bites).
                let code = source.split("#[cfg(test)]").next().unwrap_or("");
                let lines = bare_px_literals(code);
                assert!(
                    lines.is_empty(),
                    "{}: bare px( on lines {lines:?} — use m.px(n) or px_fixed(n)",
                    path.display()
                );
            }
        }
        // Not blind: the kit is more than one file.
        assert!(files >= 8, "the lint read only {files} files");
    }

    /// The lint is red on an injected literal, and quiet on the two spellings
    /// that are allowed.
    #[test]
    fn the_px_lint_finds_an_injected_literal() {
        let injected = "fn f() {\n    let a = Node { width: px(12), ..default() };\n}\n";
        assert_eq!(bare_px_literals(injected), vec![2]);
        assert_eq!(bare_px_literals("let a = px(-3.0);"), vec![1]);
        assert_eq!(bare_px_literals("let a = (px( 4.0), 1);"), vec![1]);
        for allowed in [
            "let a = m.px(12.0);",
            "let a = kit.m.px(12.0);",
            "let a = px_fixed(1.0);",
            "let a = px(m.gap);",
            "// px(12) in a comment",
            "let a = ui_px(3);",
        ] {
            assert!(bare_px_literals(allowed).is_empty(), "{allowed}");
        }
    }

    /// The 3D table and its felt never read the shell's metrics: they are
    /// laid out in world units, and a text step must not move a card. The
    /// table's HUD may (the table design, §9: text steps reach it with
    /// floors), so `hud/**` is not held to this.
    #[test]
    fn the_table_never_reads_the_shell_s_metrics() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = module_files("table");
        files.extend(
            ["feltmat.rs", "tabletop.rs", "cardmat.rs", "shellmat.rs"]
                .iter()
                .map(|f| src.join(f))
                .filter(|p| p.exists()),
        );
        assert!(
            files.len() > 10,
            "the table's files not found: {}",
            files.len()
        );
        for path in files {
            let source = std::fs::read_to_string(&path).expect("a source file");
            for needle in ["shellkit", "ShellMetrics"] {
                assert!(
                    !source.contains(needle),
                    "{} reads the shell's metrics ({needle})",
                    path.display()
                );
            }
        }
    }
}
