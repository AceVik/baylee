//! Writing generated files, or checking them: rustfmt, the card tree, and
//! the two ways a file is written.

use crate::{BTreeMap, Path, PathBuf, fs};

/// Formats Rust source with the toolchain's rustfmt so generated files are
/// fmt-stable (`cargo fmt --check` and `codegen --check` never conflict).
pub(crate) fn format_rust(content: &str) -> anyhow::Result<String> {
    use std::io::Write as _;
    use std::process::{Command, Stdio};
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2024"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    child
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(content.as_bytes())?;
    let out = child.wait_with_output()?;
    if out.status.success() {
        Ok(String::from_utf8(out.stdout)?)
    } else {
        // Unparseable generated code should fail at compile time anyway;
        // keep the raw text so the error points at the real file.
        Ok(content.to_string())
    }
}

/// Every card file under `cards/`, wherever the taxonomy has put it, keyed by
/// slug.
///
/// The slug is the file stem and the module name both, so two files claiming
/// one slug is not a layout question but a broken tree — `cards/mod.rs` could
/// only declare one of them, and which one would depend on directory order.
/// `mod.rs` itself is the tree's own bookkeeping and is skipped at every
/// level.
pub(crate) fn card_files(dir: &Path) -> anyhow::Result<BTreeMap<String, PathBuf>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(here) = stack.pop() {
        let Ok(entries) = fs::read_dir(&here) else {
            continue;
        };
        for entry in entries {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if stem == "mod" {
                continue;
            }
            if let Some(first) = out.insert(stem.to_string(), path.clone()) {
                anyhow::bail!(
                    "two files claim the slug {stem}: {} and {}",
                    first.display(),
                    path.display()
                );
            }
        }
    }
    Ok(out)
}

pub(crate) fn write_or_check(
    check: bool,
    path: &Path,
    content: &str,
    changed: &mut Vec<PathBuf>,
) -> anyhow::Result<()> {
    let owned;
    let content = if path.extension().is_some_and(|e| e == "rs") {
        owned = format_rust(content)?;
        owned.as_str()
    } else {
        content
    };
    write_verbatim(check, path, content, changed)
}

/// The same, for generated text that is already in the form rustfmt would
/// leave it in.
///
/// The `CardIndex` tree is 383 files of one-line constants, and running
/// rustfmt over each of them costs a process apiece on every `codegen` and
/// every CI `--check` to change nothing. `the_index_tree_is_already_
/// formatted` in `xtask` is what holds the claim: if the renderer ever emits
/// something rustfmt would rewrite, `cargo fmt --all` and `codegen --check`
/// would disagree forever, each undoing the other.
pub(crate) fn write_verbatim(
    check: bool,
    path: &Path,
    content: &str,
    changed: &mut Vec<PathBuf>,
) -> anyhow::Result<()> {
    let existing = fs::read_to_string(path).unwrap_or_default();
    if existing == content {
        return Ok(());
    }
    if check {
        changed.push(path.to_path_buf());
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        println!("wrote {}", path.display());
    }
    Ok(())
}

/// A card file's path as it reads in a message: relative to `cards/`.
pub(crate) fn relative(path: &Path, cards_dir: &Path) -> String {
    path.strip_prefix(cards_dir)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Extracts the first `"`-quoted value after `key` (e.g. `name = "…"`).
/// What a card file writes after `<field> =`, with the line break rustfmt
/// may have put in between skipped.
///
/// Card files are ordinary rustfmt output since the macros moved to
/// parentheses, which means a value too long for its line is wrapped onto the
/// next one: Nesting Dovehawk's `Coverage::Partial("…")` is written
/// `coverage =\n        Coverage::Partial(…)`. A reader matching the literal
/// `"coverage = Coverage::"` read that as a card with no coverage flag at all
/// — the seventh textual reader of the pool to answer a question it could not
/// see the answer to. Anything asking a card file what a knob says goes
/// through here.
pub(crate) fn knob<'a>(content: &'a str, field: &str) -> Option<&'a str> {
    let at = content.find(&format!("{field} ="))?;
    Some(content[at + field.len() + 2..].trim_start())
}

/// The Rust string literal that follows `key`, with its escapes undone.
///
/// It walks `\\` rather than stopping at the first quote, because a card is
/// allowed a quotation mark in its own **name**: the ledger holds seven of
/// them — `Kongming, "Sleeping Dragon"`, `Henzie "Toolbox" Torre`,
/// `"Name Sticker" Goblin` — and the pool reached the first one in §E8's
/// third batch. Stopping at the first quote read Kongming's code name as
/// `Kongming, \\` and reported the header it agrees with as a mismatch,
/// which is the twelfth textual reader of this pool caught answering a
/// question it could not see.
pub(crate) fn quoted_value(content: &str, key: &str) -> Option<String> {
    let start = content.find(key)? + key.len();
    let mut out = String::new();
    let mut escaped = false;
    for c in content[start..].chars() {
        if escaped {
            out.push(match c {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '0' => '\0',
                other => other,
            });
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return Some(out);
        } else {
            out.push(c);
        }
    }
    None
}
