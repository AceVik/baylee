//! Where a card's file lies: `crates/cards/sets/<group>/src/<set>/<slug>.rs`.
//!
//! The group is the time the card was first printed: the release year of the
//! set the `CardIndex` ledger froze for it (`Row::set`, its first appearance
//! over the whole corpus, `baylee_cards_index::released`), and from 2021 on
//! the quarter (`2021-q1`). Up to 2020 a year held at most 1,276 cards of the
//! corpus; since 2021, when Wizards began printing about two thousand new
//! cards a year, 2,000 to 2,340. The rule reads no count, so a group never
//! splits after the fact and a card never moves because others were added.
//!
//! A set code or slug that Windows reserves for a device gains an underscore
//! (Conflux's `con` is `con_`, [`windows_safe`]): Git for Windows cannot
//! check such a path out.
//!
//! Each group is a crate (`baylee-cards-<group>`): measured on 10.11.2026, one
//! compiles in about 0.4 s plus 3.5 ms a card, against 54 s for all the cards
//! in one crate, and a changed card rebuilds only its own group.
//! `docs/cards.md` §"Where a card compiles".

use crate::error::CodegenError;
use std::collections::BTreeSet;
use std::path::Path;

/// Where the groups lie, relative to the repository root.
pub const SETS_DIR: &str = "crates/cards/sets";

/// Where the card tests lie, relative to the repository root. They are filed
/// by subject rather than by group, and a card's own test is the file named
/// after its slug ([`tested_slugs`]).
pub const CARD_TESTS_DIR: &str = "crates/cards/testkit/card_tests";

/// The first year that is split into quarters.
pub const QUARTERS_FROM: u32 = 2021;

/// The group a card first printed on `released` (`YYYY-MM-DD`) belongs to:
/// `1993`, or `2021-q1` from [`QUARTERS_FROM`] on. `None` for a day that is
/// not one.
#[must_use]
pub fn group_of(released: &str) -> Option<String> {
    let mut parts = released.split('-');
    let year: u32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1993..=9999).contains(&year) {
        return None;
    }
    Some(if year < QUARTERS_FROM {
        year.to_string()
    } else {
        format!("{year}-q{}", (month - 1) / 3 + 1)
    })
}

/// The file of the card named `card`, first printed in `set`, relative to
/// [`SETS_DIR`]: `<group>/src/<set>/<slug>.rs`.
///
/// # Errors
/// [`CodegenError::UnknownSet`] when the set has no release day in
/// `baylee_cards_index::SETS` (`cargo xtask sets` adds it).
pub fn path_for(card: &str, set: &str, slug: &str) -> Result<String, CodegenError> {
    let group = baylee_cards_index::released(set)
        .and_then(group_of)
        .ok_or_else(|| CodegenError::UnknownSet {
            card: card.to_owned(),
            set: set.to_owned(),
        })?;
    Ok(format!("{group}/src/{}/{slug}.rs", windows_safe(set)))
}

/// Whether `component`, one name in a path, is a device Windows reserves:
/// `CON`, `PRN`, `AUX`, `NUL`, `CONIN$`, `CONOUT$`, `COM0`–`COM9` and
/// `LPT0`–`LPT9` (with the superscripts `¹²³`), in any case, alone or before
/// an extension (`con.rs` is the console as much as `con`). Windows cannot
/// create such a file, and Git for Windows refuses to check one out
/// ("invalid path"): Conflux's set code `con` made `2009/src/con/` one on
/// 10.11.2026, and no Windows runner could clone the repository.
#[must_use]
pub fn is_windows_reserved(component: &str) -> bool {
    let stem = component
        .split(['.', ':'])
        .next()
        .unwrap_or(component)
        .trim_end_matches(' ')
        .to_ascii_uppercase();
    if matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) {
        return true;
    }
    let numbered = stem
        .strip_prefix("COM")
        .or_else(|| stem.strip_prefix("LPT"));
    numbered.is_some_and(|n| {
        let mut chars = n.chars();
        chars
            .next()
            .is_some_and(|c| c.is_ascii_digit() || matches!(c, '¹' | '²' | '³'))
            && chars.next().is_none()
    })
}

/// `name` as a folder or file stem every platform can hold: a name Windows
/// reserves ([`is_windows_reserved`]) gains a trailing underscore, which no
/// set code and no other slug ends in, so it cannot meet a real one. Conflux
/// is `2009/src/con_/`. Every other name is itself.
#[must_use]
pub fn windows_safe(name: &str) -> String {
    if is_windows_reserved(name) {
        format!("{name}_")
    } else {
        name.to_owned()
    }
}

/// The group a path from [`path_for`] lies in: its first component.
#[must_use]
pub fn group_of_path(path: &str) -> &str {
    path.split('/').next().unwrap_or(path)
}

/// The package of a group: `baylee-cards-1993`, `baylee-cards-2021-q1`.
#[must_use]
pub fn crate_name(group: &str) -> String {
    format!("baylee-cards-{group}")
}

/// The same as a Rust path: `baylee_cards_2021_q1`.
#[must_use]
pub fn crate_ident(group: &str) -> String {
    crate_name(group).replace('-', "_")
}

/// Whether `path`, a file under [`SETS_DIR`], is a card's: a `.rs` file
/// other than a group's own `src/lib.rs` (or a `mod.rs`).
#[must_use]
pub fn is_card_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let in_src = path
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|d| d == "src");
    path.extension().is_some_and(|e| e == "rs") && name != "mod.rs" && !(name == "lib.rs" && in_src)
}

/// The slugs a card test is filed under: the stem of every `.rs` file under
/// `dir` (the [`CARD_TESTS_DIR`]). A set, not a map: a stem may be there
/// twice (a subject module and a batch both naming one card), and either file
/// is a test of it.
///
/// A module root (`lands/pain.rs` beside `lands/pain/`) counts only when it
/// holds a `#[test]` of its own. Night Soil's test has a child module and is
/// the card's test; a land cycle's root only gathers the tests of the cards
/// inside it, and `pain` is also the slug of a card (Pain // Suffering).
///
/// # Errors
/// The first directory or module root that cannot be read.
pub fn tested_slugs(dir: &Path) -> std::io::Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(here) = stack.pop() {
        for entry in std::fs::read_dir(&here)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if path.extension().is_none_or(|e| e != "rs")
                || (path.with_extension("").is_dir()
                    && !std::fs::read_to_string(&path)?.contains("#[test]"))
            {
                continue;
            }
            out.insert(stem.to_owned());
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A year before 2021 is one group whatever the month; from 2021 each
    /// quarter is its own, January to March the first.
    #[test]
    fn a_year_is_a_group_until_the_quarters_begin() {
        assert_eq!(group_of("1993-08-05").as_deref(), Some("1993"));
        assert_eq!(group_of("2020-12-31").as_deref(), Some("2020"));
        assert_eq!(group_of("2021-01-01").as_deref(), Some("2021-q1"));
        assert_eq!(group_of("2021-03-31").as_deref(), Some("2021-q1"));
        assert_eq!(group_of("2021-04-01").as_deref(), Some("2021-q2"));
        assert_eq!(group_of("2026-12-04").as_deref(), Some("2026-q4"));
        assert_eq!(group_of("2026-13-01"), None);
        assert_eq!(group_of("soon"), None);
    }

    /// The path names the group, the set and the slug, and the crate of a
    /// quarter is a valid package and Rust name.
    #[test]
    fn a_card_lies_in_its_first_printings_group_and_set() {
        assert_eq!(
            path_for("Lightning Bolt", "lea", "lightning_bolt").unwrap(),
            "1993/src/lea/lightning_bolt.rs"
        );
        assert_eq!(group_of_path("1993/src/lea/lightning_bolt.rs"), "1993");
        assert_eq!(crate_ident("2021-q1"), "baylee_cards_2021_q1");
        assert!(matches!(
            path_for("Nobody", "no-such-set", "nobody"),
            Err(CodegenError::UnknownSet { .. })
        ));
    }

    /// Windows' device names, in any case and before any extension, and
    /// nothing that merely begins like one.
    #[test]
    fn windows_reserved_names_are_known() {
        for reserved in [
            "con",
            "CON",
            "Con.rs",
            "prn",
            "aux.rs",
            "nul",
            "com1",
            "COM9.txt",
            "lpt3",
            "lpt¹",
            "conin$",
            "CONOUT$",
            "nul .rs",
            "aux:stream",
        ] {
            assert!(is_windows_reserved(reserved), "{reserved}");
        }
        for fine in [
            "conflux",
            "con_",
            "com",
            "com10",
            "lpt",
            "auxiliary",
            "null",
            "c18",
            "m10",
            "",
        ] {
            assert!(!is_windows_reserved(fine), "{fine}");
        }
        assert_eq!(windows_safe("con"), "con_");
        assert_eq!(windows_safe("zen"), "zen");
    }

    /// Conflux's set code is the console on Windows; its cards lie in
    /// `con_` instead.
    #[test]
    fn a_reserved_set_code_gets_a_folder_windows_can_hold() {
        assert_eq!(
            path_for("Ancient Ziggurat", "con", "ancient_ziggurat").unwrap(),
            "2009/src/con_/ancient_ziggurat.rs"
        );
    }

    /// No folder or file under `crates/cards` is one Windows cannot hold:
    /// one such name and Git for Windows cannot check the repository out at
    /// all (every Windows job of CI from 10.11.2026, `2009/src/con/`).
    #[test]
    fn nothing_under_the_cards_is_a_name_windows_reserves() {
        let cards = Path::new(env!("CARGO_MANIFEST_DIR")).join("../cards");
        let mut stack = vec![cards];
        let mut seen = 0_usize;
        let mut reserved = Vec::new();
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let entry = entry.unwrap();
                let name = entry.file_name().to_string_lossy().into_owned();
                if is_windows_reserved(&name) {
                    reserved.push(entry.path());
                }
                if entry.file_type().unwrap().is_dir() {
                    stack.push(entry.path());
                }
                seen += 1;
            }
        }
        assert!(seen > 10_000, "only {seen} entries: not the cards' tree");
        assert!(reserved.is_empty(), "names Windows reserves: {reserved:?}");
    }

    /// A group's own root is no card; any file below a set folder is.
    #[test]
    fn a_groups_lib_is_no_card() {
        assert!(is_card_file(Path::new("1993/src/lea/lightning_bolt.rs")));
        assert!(!is_card_file(Path::new("1993/src/lib.rs")));
        assert!(!is_card_file(Path::new("1993/Cargo.toml")));
        assert!(
            is_card_file(Path::new("1993/src/lea/lib.rs")),
            "only the group's root is skipped"
        );
    }

    /// A card test is found by the card's slug wherever it is filed, and a
    /// module root counts only when it plays something itself.
    #[test]
    fn a_card_test_is_found_by_its_slug() {
        let dir = tempfile::tempdir().unwrap();
        let write = |rel: &str, text: &str| {
            let path = dir.path().join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write("instants/mv_1/lightning_bolt.rs", "#[test] fn bolt() {}");
        write("lands/pain.rs", "mod sulfurous_springs;");
        write("lands/pain/sulfurous_springs.rs", "#[test] fn springs() {}");
        write(
            "enchantments/night_soil.rs",
            "#[test] fn soil() {}\nmod selection;",
        );
        write(
            "enchantments/night_soil/selection.rs",
            "#[test] fn pick() {}",
        );
        write("notes.txt", "pain");
        let slugs: Vec<String> = tested_slugs(dir.path()).unwrap().into_iter().collect();
        assert_eq!(
            slugs,
            [
                "lightning_bolt",
                "night_soil",
                "selection",
                "sulfurous_springs"
            ]
        );
    }
}
