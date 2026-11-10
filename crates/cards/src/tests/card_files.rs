//! The card files as text, for the tests that read what a file says rather
//! than what it compiles to. Shared by `baylee-cards` and
//! `baylee-cards-base` (whose token test reads them too), so the path names
//! the cards from the workspace's `crates/` directory, which both reach.

/// Every card file, in whichever group and set folder it lies.
///
/// `crates/cards/sets` is a tree — `<group>/src/<set>/<slug>.rs` — and two
/// lints once read the card folder with a bare `read_dir`, which saw only
/// folders. Both had been passing on an empty worklist ever since the files
/// moved, which is the failure mode `docs/cards.md` names for anything that
/// reads card files: a non-recursive read of a tree finds nothing and
/// reports it as an answer.
///
/// So this walks, skips each group's own `src/lib.rs` (it declares the
/// cards and is none), and panics on an empty result rather than handing
/// back a list a caller would read as "nothing is wrong".
pub(crate) fn every_card_file() -> Vec<(String, String)> {
    // `baylee-cards` (crates/cards) and `baylee-cards-base` (crates/cards/base)
    // both read it: the first finds it beside its manifest, the second one up.
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = [manifest.join("sets"), manifest.join("../sets")]
        .into_iter()
        .find(|dir| dir.is_dir())
        .expect("crates/cards/sets beside this crate");
    let mut found = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(here) = stack.pop() {
        for entry in std::fs::read_dir(&here).expect("a cards directory") {
            let path = entry.expect("a directory entry").path();
            let group_root = path.file_name().is_some_and(|n| n == "lib.rs")
                && path
                    .parent()
                    .and_then(std::path::Path::file_name)
                    .is_some_and(|d| d == "src");
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") && !group_root {
                let name = path
                    .file_name()
                    .expect("a file name")
                    .to_string_lossy()
                    .into_owned();
                found.push((name, std::fs::read_to_string(&path).expect("read a card")));
            }
        }
    }
    assert!(
        found.len() > 1000,
        "only {} card files were found under {} — the sweep is not reaching the pool",
        found.len(),
        root.display()
    );
    found.sort();
    found
}
