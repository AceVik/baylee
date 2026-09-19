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
//!   kindred (CR 205.2a lists the types, not an order — this one is ours).
//! - **The front face decides**, whatever the layout. A transforming back is
//!   not a card a player ever holds, and a modal back is the same card seen
//!   from the other side (CR 712.2); filing by either would give Westvale
//!   Abbey two homes.
//! - **A `mv_` level ends every branch but lands**, with `{X}` counting 0 —
//!   which is what [`ManaCost::cmc`] already answers (CR 202.3).
//! - **Lands take a semantic level instead**, because a land's type line says
//!   almost nothing: 888 of the 1124 in the pool print no subtype at all.
//!   Six sources are asked in order, most trustworthy first, and the first
//!   one that answers wins:
//!
//!   | # | source | example door |
//!   |---|--------|--------------|
//!   | 0 | the `Basic` supertype | `lands/basic` |
//!   | 1 | a second card type or defining subtype | `lands/creatures` |
//!   | 2 | the hand-kept cycle map ([`LandCycles`]) | `lands/fetch` |
//!   | 3 | a printed nonbasic land subtype (CR 205.3i) | `lands/deserts` |
//!   | 4 | what the printed text *does* (`land_role`) | `lands/utility` |
//!   | 5 | how many basic land types it prints | `lands/dual` |
//!
//!   Steps 2 and 4 are the two halves of "semantic". A cycle is an assertion
//!   no card prints — nothing about Scalding Tarn's text says "fetchland" —
//!   so it is hand-kept and additive: a land missing from the map is filed
//!   one level shallower, never misfiled. A *role* is the opposite; it is
//!   read straight off the card, because "enters tapped unless you control
//!   two or fewer other lands" is a fastland whoever printed it. The map is
//!   asked first so a shockland stays a shockland rather than becoming one
//!   more conditional tapland.
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
    ///
    /// A double-faced card arrives here under its joined name — Scryfall
    /// calls a pathway `Barkchannel Pathway // Tidechannel Pathway` — so the
    /// front face is tried as well, which is the rule the rest of this module
    /// already follows. Ten pathways sat unfiled for exactly that reason.
    #[must_use]
    pub fn of(&self, name: &str) -> Option<&str> {
        self.0
            .get(name)
            .or_else(|| self.0.get(name.split(" // ").next().unwrap_or(name)))
            .map(String::as_str)
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
    let (type_line, mana_cost, oracle_text) = front_face(card);
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
        // A land's own level, asking the six sources of the module header in
        // order and taking the first that answers.
        let level = if has_word(left, "Basic") {
            Some("basic".to_string())
        } else {
            second
                .or_else(|| cycles.of(&card.name).map(str::to_string))
                .or_else(|| land_subtype(right).map(str::to_string))
                .or_else(|| land_role(&oracle_text).map(str::to_string))
                .or_else(|| land_shape(right).map(str::to_string))
        };
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

/// The front face's type line, mana cost and printed text — the only face
/// that places a card, whichever layout it was printed in.
fn front_face(card: &ScryfallCard) -> (String, String, String) {
    if let Some(faces) = &card.card_faces
        && let Some(front) = faces.first()
        && faces.len() >= 2
    {
        return (
            front.type_line.clone().unwrap_or_default(),
            front.mana_cost.clone().unwrap_or_default(),
            front.oracle_text.clone().unwrap_or_default(),
        );
    }
    (
        card.type_line.clone().unwrap_or_default(),
        card.mana_cost.clone().unwrap_or_default(),
        card.oracle_text.clone().unwrap_or_default(),
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

/// A printed nonbasic land subtype that earns a door, and only one the table
/// names.
///
/// An unknown subtype stays flat rather than opening an `other/` door, which
/// would be a bucket that says nothing — the thing this taxonomy exists to
/// avoid. This is asked before [`land_role`] because a Gate or a Locus is an
/// identity a deck is built around, while a role is only what the card does.
fn land_subtype(right: &str) -> Option<&'static str> {
    right
        .split_whitespace()
        .find_map(|w| {
            LAND_SUBTYPE_DIRS
                .iter()
                .find(|(sub, _)| sub.eq_ignore_ascii_case(w))
        })
        .map(|(_, dir)| *dir)
}

/// A land's last resort: how many basic land types it prints (CR 305.6).
fn land_shape(right: &str) -> Option<&'static str> {
    match right
        .split_whitespace()
        .filter(|w| BASIC_LAND_TYPES.contains(w))
        .count()
    {
        2 => Some("dual"),
        n if n >= 3 => Some("triple"),
        _ => None,
    }
}

/// What a land *does*, read from its printed text.
///
/// Every door here is a cycle players already have a word for, which is what
/// makes the reader worth having: 729 of the 872 lands that had no cycle, no
/// second type and no printed subtype answer one of these. The order is the
/// whole design — the first pattern that fires wins, so Barren Moor is a
/// cycling land rather than one more tapland, and Bojuka Bog is a utility
/// land rather than one more land that enters tapped.
///
/// The last two doors are the shapes that are left when no named cycle
/// claims the card: `utility` for a land that does something other than make
/// mana, `tapland` for one whose only text is that it comes in tapped. A land
/// whose text says nothing but "add mana" answers `None` and stays flat in
/// `lands/`, which is the honest place for it.
fn land_role(text: &str) -> Option<&'static str> {
    let t = text.to_lowercase();

    // Named cycles, in the order a more specific reading beats a vaguer one.
    if t.contains("search your library for a") && t.contains("land") && t.contains("sacrifice") {
        return Some("fetch");
    }
    // `manlands`, not `creature`: `lands/creatures` already means a land that
    // *is* one on the type line (Dryad Arbor), and two doors a letter apart
    // would be a browsing trap.
    if t.contains("becomes a") && t.contains("creature") && t.contains("until end of turn") {
        return Some("manlands");
    }
    if t.contains("cycling") {
        return Some("cycling");
    }
    if t.contains("return a land you control to its owner's hand") {
        return Some("bounce");
    }
    if t.contains("storage counter") {
        return Some("storage");
    }
    if t.contains("pay 1 life") && t.contains("sacrifice") && t.contains("draw a card") {
        return Some("horizon");
    }
    if is_filter(&t) {
        return Some("filter");
    }
    if t.contains("damage to you") && t.contains("}: add") {
        return Some("pain");
    }
    if t.contains("you may pay 2 life") && t.contains("enters tapped") {
        return Some("shock");
    }

    // The `enters tapped unless …` family, which is one printed sentence and
    // a dozen different cycles hanging off its condition.
    if let Some(cond) = between(&t, "enters tapped unless ", ".") {
        if let Some(role) = conditional_role(cond) {
            return Some(role);
        }
    } else if t.contains("enters tapped") {
        // A tapland with a rider is named after the rider.
        for (mark, role) in [
            ("scry 1", "scry"),
            ("surveil 1", "surveil"),
            ("gain 2 life", "refuge"),
            ("gain 1 life", "gain"),
        ] {
            if t.contains(mark) {
                return Some(role);
            }
        }
    }

    // Two families of pure mana land that a player still picks out by name.
    // Both are named after what they print rather than after a nickname:
    // "slow land" already means the Innistrad cycle above, and "verge" is
    // only six of the sixteen lands whose mana has a condition on it.
    if t.contains("doesn't untap during your next untap step") {
        return Some("no_untap");
    }
    if t.contains("activate only if") {
        return Some("restricted");
    }

    if does_more_than_make_mana(&t) {
        return Some("utility");
    }
    t.contains("enters tapped").then_some("tapland")
}

/// The cycle a conditional tapland belongs to, read from its condition.
///
/// The community names are the doors, because they are what a person types
/// into a search box; `unlucky` is the Duskmourn cycle, `crowd` the Battlebond
/// one, `saddle` the Aetherdrift one. A condition none of these claims falls
/// through to the rules after it rather than opening a `conditional/` door.
fn conditional_role(cond: &str) -> Option<&'static str> {
    if cond.contains("two or fewer other lands") {
        return Some("fast");
    }
    if cond.contains("two or more other lands") {
        return Some("slow");
    }
    if cond.contains("two or more basic lands") {
        return Some("battle");
    }
    if cond.contains("two or more opponents") {
        return Some("crowd");
    }
    if cond.contains("13 or less life") {
        return Some("unlucky");
    }
    if cond.contains("legendary creature") {
        return Some("legendary");
    }
    if cond.contains("mount or vehicle") {
        return Some("saddle");
    }
    if cond.contains("reveal") {
        return Some("reveal");
    }
    if cond.contains("basic land")
        || BASIC_LAND_TYPES
            .iter()
            .any(|b| has_word(cond, &b.to_lowercase()))
    {
        return Some("check");
    }
    None
}

/// Whether a filter land's activation is printed here: a tap cost paid with a
/// hybrid or generic pip that answers with two or more coloured symbols.
///
/// The two coloured symbols are what separates Graven Cairns from a land that
/// taps for one mana of any colour, which prints the same shape of line.
fn is_filter(text: &str) -> bool {
    text.lines().any(|line| {
        let Some((cost, gives)) = line.split_once(':') else {
            return false;
        };
        let gives = gives.trim();
        cost.contains("{t}")
            && (cost.contains('/') || cost.contains("{1}"))
            && gives.starts_with("add")
            && gives.matches('{').count() >= 2
            && !gives.contains("any color")
    })
}

/// Whether the card has an ability that is not a mana ability — an activated
/// one whose answer is not `Add`, or any trigger at all.
///
/// A trigger counts because the classic utility land is Bojuka Bog, whose
/// whole text is that it enters tapped and exiles a graveyard.
fn does_more_than_make_mana(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim();
        if let Some((cost, gives)) = line.split_once(':') {
            return !cost.is_empty() && !gives.trim_start().starts_with("add");
        }
        line.starts_with("when ") || line.starts_with("whenever ") || line.starts_with("at the ")
    })
}

/// The text between a marker and the next occurrence of `end`, if both are
/// there.
fn between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let rest = &text[text.find(start)? + start.len()..];
    Some(&rest[..rest.find(end).unwrap_or(rest.len())])
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
            image_uris: None,
            image_status: None,
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
            image_uris: None,
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

    /// The printed text carries every land cycle nobody has to assert by
    /// hand, which is what keeps `data/land-cycles.tsv` down to the four
    /// things a card genuinely does not print.
    #[test]
    fn a_land_cycle_is_read_off_the_card_when_the_card_says_it() {
        let cases = [
            (
                "Blackcleave Cliffs enters tapped unless you control two or fewer other lands.",
                "fast",
            ),
            (
                "Deserted Beach enters tapped unless you control two or more other lands.",
                "slow",
            ),
            (
                "Glacial Fortress enters tapped unless you control a Plains or an Island.",
                "check",
            ),
            (
                "Sea of Clouds enters tapped unless you have two or more opponents.",
                "crowd",
            ),
            (
                "Etched Cornfield enters tapped unless a player has 13 or less life.",
                "unlucky",
            ),
            (
                "Minas Tirith enters tapped unless you control a legendary creature.",
                "legendary",
            ),
            (
                "Country Roads enters tapped unless you control a Mount or Vehicle.",
                "saddle",
            ),
            (
                "Temple of Enlightenment enters tapped.\nWhen this land enters, scry 1.",
                "scry",
            ),
            (
                "Akoum Refuge enters tapped.\nWhen this land enters, you gain 1 life.",
                "gain",
            ),
            (
                "{4}{G}{W}: Stirring Wildwood becomes a 3/4 green and white Elemental \
                 creature with reach until end of turn. It's still a land.",
                "manlands",
            ),
            (
                "{T}: Add {C}.\n{W/U}, {T}: Add {W}{W}, {W}{U}, or {U}{U}.",
                "filter",
            ),
            (
                "{T}: Add {C}.\n{T}: Add {W} or {U}. This land deals 1 damage to you.",
                "pain",
            ),
            (
                "Bojuka Bog enters tapped.\nWhen Bojuka Bog enters, exile target player's \
                 graveyard.",
                "utility",
            ),
            (
                "{T}: Add {B}.\n{T}: Add {R}. Activate only if you control a Swamp.",
                "restricted",
            ),
            (
                "{T}: Add {C}.\n{T}: Add {B} or {R}. This land doesn't untap during your \
                 next untap step.",
                "no_untap",
            ),
            (
                "Jungle Hollow enters tapped.\n{T}: Add {B} or {G}.",
                "tapland",
            ),
        ];
        for (text, role) in cases {
            assert_eq!(land_role(text), Some(role), "reading {text:?}");
        }
    }

    /// A land whose whole text is a mana ability has no role, and a role that
    /// is not read is a level the card simply does not get — never a wrong
    /// one, and never an `other/` bucket.
    #[test]
    fn a_plain_mana_land_has_no_role_and_stays_flat() {
        assert_eq!(
            land_role("{T}: Add one mana of any color in your commander's color identity."),
            None
        );
        assert_eq!(
            land_role("{T}: Add {G} for each creature you control."),
            None
        );
    }

    /// The map is asked before the reader, so an assertion nobody prints wins
    /// over a sentence everybody does. A triome prints cycling and the reader
    /// would file it with the cycling lands, which is true and not what a
    /// player is looking for.
    #[test]
    fn the_cycle_map_outranks_what_the_card_prints() {
        let cycles = LandCycles::parse("triome\tIndatha Triome\n");
        let mut card = card("Indatha Triome", "Land \u{2014} Plains Swamp Forest", "");
        card.oracle_text = Some("Indatha Triome enters tapped.\nCycling {3}".to_string());
        assert_eq!(
            path_for(&card, "indatha_triome", &cycles),
            "lands/triome/indatha_triome.rs"
        );
        // Without the map it is still filed, by what it prints — the map is
        // additive, so a stale one costs browsing and never correctness.
        assert_eq!(
            path_for(&card, "indatha_triome", &LandCycles::default()),
            "lands/cycling/indatha_triome.rs"
        );
    }

    /// Scryfall names a double-faced card with both faces joined, and the map
    /// is keyed on the printed front. Ten pathways went unfiled for a whole
    /// commit because this lookup only tried the joined name.
    #[test]
    fn the_cycle_map_finds_a_double_faced_card_by_its_front() {
        let cycles = LandCycles::parse("pathway\tBarkchannel Pathway\n");
        assert_eq!(
            cycles.of("Barkchannel Pathway // Tidechannel Pathway"),
            Some("pathway")
        );
    }
}
