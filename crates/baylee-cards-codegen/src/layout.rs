//! Where a card's file lives.
//!
//! `cards/` is arranged as a taxonomy a person can browse rather than as a
//! flat list of 1365 slugs, and every part of it is computed from what the
//! card *prints* — one card, one home:
//!
//! ```text
//! <card type>/<second type or defining subtype>/mv_<mana value>/<slug>.rs
//! ```
//!
//! Four rules carry the whole thing.
//!
//! - **A total order over card types picks the door** ([`PRECEDENCE`]), so an
//!   Artifact Creature is a creature that happens to be an artifact and lives
//!   in exactly one place. Land comes first because a land prints no mana
//!   cost, which keeps every mana-less card inside the one branch that has no
//!   `mv_` level; Kindred comes last so Crib Swap is an instant rather than a
//!   kindred (CR 205.1a lists the types, not an order — this one is ours).
//! - **The front face decides**, whatever the layout. A transforming back is
//!   not a card a player ever holds, and a modal back is the same card seen
//!   from the other side (CR 712.2); filing by either would give Westvale
//!   Abbey two homes.
//! - **A `mv_` level ends every branch but lands**, with `{X}` counting 0 —
//!   which is what [`ManaCost::cmc`] already answers (CR 202.3).
//! - **Lands take a semantic level instead**, because a land's type line says
//!   almost nothing: 888 of the 1124 in the pool print no subtype at all. A
//!   hand-kept cycle map ([`LandCycles`]) names what players name — fetch,
//!   shock, triome, pathway — and what it does not cover falls back to the
//!   printed subtype, then to `lands/` itself.
//!
//! The path is a *file* location and never a module path: `cards/mod.rs`
//! declares every card with `#[path = …]`, so `cards::baleful_strix` resolves
//! exactly as it did when the directory was flat, and re-filing a card whose
//! type line was corrected costs one `git mv` and one generated line.

use crate::scryfall::ScryfallCard;
use baylee_core::mana::ManaCost;
use std::collections::BTreeMap;

/// Card types in placement order, each with the directory it opens.
///
/// The order is a judgement and the reason for each position is in the module
/// header; `Tribal` is the retired spelling of `Kindred` and shares its door.
const PRECEDENCE: &[(&str, &str)] = &[
    ("Land", "lands"),
    ("Planeswalker", "planeswalkers"),
    ("Creature", "creatures"),
    ("Artifact", "artifacts"),
    ("Enchantment", "enchantments"),
    ("Battle", "battles"),
    ("Instant", "instants"),
    ("Sorcery", "sorceries"),
    ("Kindred", "kindred"),
    ("Tribal", "kindred"),
];

/// Subtypes that say what a card *is* rather than what it is about.
///
/// Deliberately short, and deliberately no creature or land subtypes: a
/// tribe is not a kind of card, and `creatures/` would open into a thousand
/// doors holding one card each.
const DEFINING: &[(&str, &str)] = &[
    ("Equipment", "equipment"),
    ("Vehicle", "vehicles"),
    ("Fortification", "fortifications"),
    ("Station", "stations"),
    ("Attraction", "attractions"),
    ("Aura", "auras"),
    ("Saga", "sagas"),
    ("Class", "classes"),
    ("Curse", "curses"),
    ("Shrine", "shrines"),
    ("Room", "rooms"),
    ("Background", "backgrounds"),
    ("Cartouche", "cartouches"),
    ("Rune", "runes"),
];

/// The five basic land types, which are what makes a land a dual (CR 305.6).
const BASIC_LAND_TYPES: [&str; 5] = ["Plains", "Island", "Swamp", "Mountain", "Forest"];

/// The hand-kept map from a card's printed name to the cycle players call it.
///
/// Read from `data/land-cycles.tsv`. It is *additive*: a land missing from it
/// is filed one level shallower, never misfiled, so a stale map costs
/// browsing and never correctness. Nothing derives it, because "fetchland"
/// and "shockland" are printed on no card.
#[derive(Debug, Default, Clone)]
pub struct LandCycles(BTreeMap<String, String>);

impl LandCycles {
    /// Parses the two-column `<cycle>\t<card name>` file; `#` starts a
    /// comment and blank lines are skipped.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut map = BTreeMap::new();
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let mut cols = line.splitn(2, '\t');
            let (Some(cycle), Some(name)) = (cols.next(), cols.next()) else {
                continue;
            };
            let (cycle, name) = (cycle.trim(), name.trim());
            if !cycle.is_empty() && !name.is_empty() {
                map.insert(name.to_string(), cycle.to_string());
            }
        }
        Self(map)
    }

    /// The cycle this card belongs to, if the map names one.
    #[must_use]
    pub fn of(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }

    /// Every card name the map claims, for the check that none has gone stale.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }
}

/// The file path for a card, relative to `cards/` — e.g.
/// `creatures/artifacts/mv_2/baleful_strix.rs`.
#[must_use]
pub fn path_for(card: &ScryfallCard, slug: &str, cycles: &LandCycles) -> String {
    let (type_line, mana_cost) = front_face(card);
    let (left, right) = split_type_line(&type_line);
    let doors = doors_of(left);

    let Some(top) = doors.first() else {
        // No card type the taxonomy knows (a plane, a scheme, a vanguard).
        // Flat at the root is the honest answer rather than a door invented
        // for one card.
        return format!("{slug}.rs");
    };

    let second = doors
        .get(1)
        .copied()
        .or_else(|| defining_subtype(right))
        .map(str::to_string);

    if *top == "lands" {
        // A land's own level, in the order the information is trustworthy:
        // a second card type is printed, a cycle is asserted by hand, a
        // subtype is printed but says less.
        let level = second
            .or_else(|| cycles.of(&card.name).map(str::to_string))
            .or_else(|| land_shape(left, right).map(str::to_string));
        return match level {
            Some(level) => format!("lands/{level}/{slug}.rs"),
            None => format!("lands/{slug}.rs"),
        };
    }

    let mv = mana_value(&mana_cost);
    match second {
        Some(second) => format!("{top}/{second}/mv_{mv}/{slug}.rs"),
        None => format!("{top}/mv_{mv}/{slug}.rs"),
    }
}

/// The front face's type line and mana cost — the only face that places a
/// card, whichever layout it was printed in.
fn front_face(card: &ScryfallCard) -> (String, String) {
    if let Some(faces) = &card.card_faces
        && let Some(front) = faces.first()
        && faces.len() >= 2
    {
        return (
            front.type_line.clone().unwrap_or_default(),
            front.mana_cost.clone().unwrap_or_default(),
        );
    }
    (
        card.type_line.clone().unwrap_or_default(),
        card.mana_cost.clone().unwrap_or_default(),
    )
}

/// Splits a type line at the em dash into (types and supertypes, subtypes).
fn split_type_line(line: &str) -> (&str, &str) {
    let mut parts = line.splitn(2, '\u{2014}');
    (
        parts.next().unwrap_or("").trim(),
        parts.next().unwrap_or("").trim(),
    )
}

/// The directories this card's types open, most significant first and with
/// `Kindred`/`Tribal` collapsed onto one.
fn doors_of(left: &str) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for (word, dir) in PRECEDENCE {
        if has_word(left, word) && !out.contains(dir) {
            out.push(dir);
        }
    }
    out
}

/// The first subtype that names what the card is, if it prints one.
fn defining_subtype(right: &str) -> Option<&'static str> {
    DEFINING
        .iter()
        .find(|(word, _)| has_word(right, word))
        .map(|(_, dir)| *dir)
}

/// A land's level when no cycle claims it: what its own type line says.
fn land_shape(left: &str, right: &str) -> Option<&'static str> {
    if has_word(left, "Basic") {
        return Some("basic");
    }
    match right
        .split_whitespace()
        .filter(|w| BASIC_LAND_TYPES.contains(w))
        .count()
    {
        2 => return Some("dual"),
        n if n >= 3 => return Some("triple"),
        _ => {}
    }
    // A printed non-basic subtype, and only one the table names. An unknown
    // one stays flat rather than opening an `other/` door, which would be a
    // bucket that says nothing — the thing this taxonomy exists to avoid.
    right
        .split_whitespace()
        .find_map(|w| {
            LAND_SUBTYPE_DIRS
                .iter()
                .find(|(sub, _)| sub.eq_ignore_ascii_case(w))
        })
        .map(|(_, dir)| *dir)
}

/// Land subtypes that earn a door of their own, and the door's name.
const LAND_SUBTYPE_DIRS: &[(&str, &str)] = &[
    ("Desert", "deserts"),
    ("Gate", "gates"),
    ("Town", "towns"),
    ("Cave", "caves"),
    ("Sphere", "spheres"),
    ("Lair", "lairs"),
    ("Planet", "planets"),
    ("Locus", "loci"),
    ("Mine", "mines"),
    ("Tower", "towers"),
    ("PowerPlant", "power_plants"),
];

/// The card's mana value, `{X}` counting 0 (CR 202.3).
fn mana_value(cost: &str) -> u32 {
    if cost.is_empty() {
        return 0;
    }
    ManaCost::try_parse(cost).map_or(0, |c| c.cmc())
}

/// Whole-word match, so `Land` does not fire on `Island` and `Class` does
/// not fire on `Classic`.
fn has_word(haystack: &str, word: &str) -> bool {
    haystack
        .split(|c: char| !c.is_alphanumeric())
        .any(|w| w == word)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scryfall::ScryfallFace;

    fn card(name: &str, type_line: &str, mana_cost: &str) -> ScryfallCard {
        ScryfallCard {
            id: String::new(),
            oracle_id: None,
            name: name.to_string(),
            mana_cost: Some(mana_cost.to_string()),
            type_line: Some(type_line.to_string()),
            oracle_text: Some(String::new()),
            colors: None,
            color_identity: None,
            set: None,
            set_name: None,
            collector_number: None,
            rarity: None,
            layout: None,
            power: None,
            toughness: None,
            loyalty: None,
            card_faces: None,
        }
    }

    fn face(type_line: &str, mana_cost: &str) -> ScryfallFace {
        ScryfallFace {
            name: String::new(),
            mana_cost: Some(mana_cost.to_string()),
            type_line: Some(type_line.to_string()),
            oracle_text: Some(String::new()),
            power: None,
            toughness: None,
            loyalty: None,
        }
    }

    fn at(name: &str, type_line: &str, cost: &str, slug: &str) -> String {
        path_for(&card(name, type_line, cost), slug, &LandCycles::default())
    }

    /// The order is the whole design: the highest type a card has opens its
    /// door and the next one it has is the room inside.
    #[test]
    fn a_card_with_two_types_files_under_the_higher_one() {
        assert_eq!(
            at(
                "Baleful Strix",
                "Artifact Creature \u{2014} Bird",
                "{U}{B}",
                "baleful_strix"
            ),
            "creatures/artifacts/mv_2/baleful_strix.rs"
        );
        assert_eq!(
            at(
                "Urza's Saga",
                "Enchantment Land \u{2014} Urza's Saga",
                "",
                "urza_s_saga"
            ),
            "lands/enchantments/urza_s_saga.rs"
        );
        assert_eq!(
            at(
                "Crib Swap",
                "Kindred Instant \u{2014} Shapeshifter",
                "{2}{W}",
                "crib_swap"
            ),
            "instants/kindred/mv_3/crib_swap.rs"
        );
    }

    /// A subtype only takes the second level when no second card type does,
    /// and only a subtype that says what the card *is*.
    #[test]
    fn a_defining_subtype_is_the_second_level_when_there_is_no_second_type() {
        assert_eq!(
            at(
                "Dowsing Dagger",
                "Artifact \u{2014} Equipment",
                "{2}",
                "dowsing_dagger"
            ),
            "artifacts/equipment/mv_2/dowsing_dagger.rs"
        );
        assert_eq!(
            at(
                "Wizard Class",
                "Enchantment \u{2014} Class",
                "{U}",
                "wizard_class"
            ),
            "enchantments/classes/mv_1/wizard_class.rs"
        );
        // A tribe is not a kind of card.
        assert_eq!(
            at(
                "Orcish Bowmasters",
                "Creature \u{2014} Orc Archer",
                "{1}{B}",
                "orcish_bowmasters"
            ),
            "creatures/mv_2/orcish_bowmasters.rs"
        );
    }

    /// CR 202.3: `{X}` contributes nothing, which is the owner's rule too.
    #[test]
    fn the_mana_value_counts_x_as_zero() {
        assert_eq!(
            at(
                "Curse of the Swine",
                "Sorcery",
                "{X}{U}{U}",
                "curse_of_the_swine"
            ),
            "sorceries/mv_2/curse_of_the_swine.rs"
        );
    }

    /// A back face is never a card a player holds (CR 712.2), so it never
    /// places one — Westvale Abbey is a land however big its Demon is.
    #[test]
    fn the_front_face_places_a_double_faced_card() {
        let mut abbey = card("Westvale Abbey // Ormendahl, Profane Prince", "Land", "");
        abbey.card_faces = Some(vec![
            face("Land", ""),
            face("Legendary Creature \u{2014} Demon", ""),
        ]);
        assert_eq!(
            path_for(&abbey, "westvale_abbey", &LandCycles::default()),
            "lands/westvale_abbey.rs"
        );

        let mut guardian = card(
            "Golden Guardian // Gold-Forge Garrison",
            "Artifact Creature",
            "{4}",
        );
        guardian.card_faces = Some(vec![
            face("Artifact Creature \u{2014} Golem", "{4}"),
            face("Land", ""),
        ]);
        assert_eq!(
            path_for(&guardian, "golden_guardian", &LandCycles::default()),
            "creatures/artifacts/mv_4/golden_guardian.rs"
        );
    }

    /// A land takes a semantic level and never an `mv_` one — it prints no
    /// cost, so every land in the pool would land in `mv_0` together.
    #[test]
    fn a_land_is_filed_by_its_cycle_and_never_by_mana_value() {
        let cycles = LandCycles::parse("fetch\tArid Mesa\n# a comment\nshock\tBlood Crypt\n");
        assert_eq!(
            path_for(&card("Arid Mesa", "Land", ""), "arid_mesa", &cycles),
            "lands/fetch/arid_mesa.rs"
        );
        assert_eq!(
            path_for(
                &card("Blood Crypt", "Land \u{2014} Swamp Mountain", ""),
                "blood_crypt",
                &cycles
            ),
            "lands/shock/blood_crypt.rs"
        );
    }

    /// The map is additive: a land it does not name falls back to what the
    /// type line prints, and then to `lands/` itself. It can never misfile.
    #[test]
    fn an_unmapped_land_falls_back_to_what_it_prints() {
        assert_eq!(
            at("Taiga", "Land \u{2014} Mountain Forest", "", "taiga"),
            "lands/dual/taiga.rs"
        );
        assert_eq!(
            at(
                "Indatha Triome",
                "Land \u{2014} Plains Swamp Forest",
                "",
                "indatha_triome"
            ),
            "lands/triple/indatha_triome.rs"
        );
        assert_eq!(
            at("Plains", "Basic Land \u{2014} Plains", "", "plains"),
            "lands/basic/plains.rs"
        );
        assert_eq!(
            at(
                "Desert of the True",
                "Land \u{2014} Desert",
                "",
                "desert_of_the_true"
            ),
            "lands/deserts/desert_of_the_true.rs"
        );
        assert_eq!(
            at("Wasteland", "Land", "", "wasteland"),
            "lands/wasteland.rs"
        );
    }

    /// `Land` must not fire on `Island`, nor `Class` on a card whose subtype
    /// merely starts with it.
    #[test]
    fn a_type_is_matched_as_a_whole_word() {
        assert!(has_word("Basic Land", "Land"));
        assert!(!has_word("Island Sanctuary", "Land"));
        assert!(!has_word("Enchantment \u{2014} Classic", "Class"));
    }
}
