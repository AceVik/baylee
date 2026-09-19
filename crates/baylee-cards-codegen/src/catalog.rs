//! Scryfall subtype catalogs → generated `subtypes.rs`.
//!
//! Deterministic, and **append-only**: every catalog is sorted
//! alphabetically, a name keeps whatever id it was first given, and a name
//! the catalogs have never carried takes the next free number. Ids were a
//! running index in kind order (creature, artifact, enchantment, land,
//! planeswalker, spell, battle) until #43, which is why `Battle` had to be
//! added last — appending a *kind* renumbered nothing, but appending a
//! creature type moved every id of every other kind, and a `SubtypeSet`
//! travels on the wire. Nothing renumbers anything now, so the order of
//! `ordered()` decides only where a new name is emitted, not what it means.

// One-shot string rendering; the allocation lint adds noise, not value.
#![allow(clippy::format_push_string)]

use std::collections::BTreeMap;

use baylee_core::types::SubtypeKind;
use serde::{Deserialize, Serialize};

/// All subtype catalogs, sorted and deduplicated.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubtypeCatalogs {
    /// Creature subtypes.
    pub creature: Vec<String>,
    /// Artifact subtypes.
    pub artifact: Vec<String>,
    /// Enchantment subtypes.
    pub enchantment: Vec<String>,
    /// Land subtypes.
    pub land: Vec<String>,
    /// Planeswalker subtypes.
    pub planeswalker: Vec<String>,
    /// Spell (instant/sorcery) subtypes.
    pub spell: Vec<String>,
    /// Battle subtypes.
    pub battle: Vec<String>,
}

impl SubtypeCatalogs {
    /// Sorts and deduplicates every catalog.
    pub fn normalize(&mut self) {
        for (_, list) in self.ordered_mut() {
            list.sort();
            list.dedup();
        }
    }

    /// Iterates `(kind, names)` in id-assignment order.
    pub fn ordered(&self) -> [(SubtypeKind, &Vec<String>); 7] {
        [
            (SubtypeKind::Creature, &self.creature),
            (SubtypeKind::Artifact, &self.artifact),
            (SubtypeKind::Enchantment, &self.enchantment),
            (SubtypeKind::Land, &self.land),
            (SubtypeKind::Planeswalker, &self.planeswalker),
            (SubtypeKind::Spell, &self.spell),
            (SubtypeKind::Battle, &self.battle),
        ]
    }

    /// The same order, for filling the catalogs rather than reading them.
    ///
    /// Two lists of the seven kinds is two more than nobody would like, and
    /// it is the fewest safe Rust allows: a shared and an exclusive borrow of
    /// the same seven fields cannot be one function. Everything else that
    /// enumerated them — `normalize`, the Scryfall fetch, the generated
    /// `kind()` — goes through one of these two, so adding an eighth kind is
    /// the field, the variant and these two lines.
    pub fn ordered_mut(&mut self) -> [(SubtypeKind, &mut Vec<String>); 7] {
        [
            (SubtypeKind::Creature, &mut self.creature),
            (SubtypeKind::Artifact, &mut self.artifact),
            (SubtypeKind::Enchantment, &mut self.enchantment),
            (SubtypeKind::Land, &mut self.land),
            (SubtypeKind::Planeswalker, &mut self.planeswalker),
            (SubtypeKind::Spell, &mut self.spell),
            (SubtypeKind::Battle, &mut self.battle),
        ]
    }

    /// The generated constant path for a subtype name, e.g.
    /// `subtypes::land::FOREST`.
    ///
    /// The catalogs are searched in id order, so a name printed by two kinds
    /// answers with the earlier one — creature before land. Use
    /// [`Self::const_path_of`] wherever the kind is already known from the
    /// sentence being read.
    #[must_use]
    pub fn const_path(&self, name: &str) -> Option<String> {
        self.ordered()
            .into_iter()
            .find_map(|(kind, names)| Self::lookup(kind, names, name))
    }

    /// The same, restricted to one kind.
    ///
    /// A reader that already knows which kind it is looking at has to say so.
    /// A checkland's "unless you control a Swamp" is a *land* subtype, and
    /// the filter it becomes says `Filter::LAND` beside the subtype clause —
    /// so a name that resolved to a creature constant would build a filter
    /// nothing can ever match, and the card would be generated as
    /// `Implemented` while entering tapped for the rest of its life. Refusing
    /// it is the honest-stub rule: an unread word leaves a stub.
    #[must_use]
    pub fn const_path_of(&self, kind: SubtypeKind, name: &str) -> Option<String> {
        self.ordered()
            .into_iter()
            .find(|(k, _)| *k == kind)
            .and_then(|(k, names)| Self::lookup(k, names, name))
    }

    fn lookup(kind: SubtypeKind, names: &[String], name: &str) -> Option<String> {
        names
            .iter()
            .any(|n| n.eq_ignore_ascii_case(name))
            .then(|| format!("subtypes::{}::{}", module_name(kind), const_name(name)))
    }
}

/// Rust module name for a kind.
#[must_use]
pub const fn module_name(kind: SubtypeKind) -> &'static str {
    match kind {
        SubtypeKind::Creature => "creature",
        SubtypeKind::Artifact => "artifact",
        SubtypeKind::Enchantment => "enchantment",
        SubtypeKind::Land => "land",
        SubtypeKind::Planeswalker => "planeswalker",
        SubtypeKind::Spell => "spell",
        SubtypeKind::Battle => "battle",
    }
}

/// `SCREAMING_SNAKE` constant identifier for a subtype name.
#[must_use]
pub fn const_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_uppercase());
        } else {
            out.push('_');
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    let trimmed = out.trim_matches('_');
    if trimmed.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return format!("S_{trimmed}");
    }
    trimmed.to_string()
}

/// What the committed table has already assigned, so codegen can append to it
/// rather than renumber it.
///
/// Subtype ids used to be a running index over one sorted range partitioned by
/// kind, which meant a single new *creature* type moved every artifact,
/// enchantment, land, planeswalker, spell and battle subtype by one. A
/// `SubtypeSet` travels on the wire, so an older client read the shifted bits
/// as other subtypes — wrong ones, not missing ones — and `VIEW_VERSION` could
/// not catch it, because the struct did not change (#43).
///
/// So the generated table is now the ledger, the way `baylee-cards-index` is
/// the ledger for `CardIndex`: codegen reads the assignment it is about to
/// rewrite and only ever appends. Reading the *compiled* table rather than
/// parsing the file is the same choice made there — nobody has to keep a
/// parser in step with an emitter, and what comes out is checked by the build.
#[derive(Debug, Default, Clone)]
pub struct PriorSubtypes {
    /// `(kind's module name, lowercased subtype name)` → the id it was first
    /// given, and the spelling to print. Keyed on the module rather than on
    /// `SubtypeKind` because a kind is not an ordered thing and giving it an
    /// `Ord` to satisfy a map would invent one.
    assigned: BTreeMap<(&'static str, String), (u16, String)>,
    /// One past the highest assigned id: where new names start.
    next: u16,
}

impl PriorSubtypes {
    /// The assignment this build compiled, which is the committed one.
    #[must_use]
    pub fn from_compiled_table() -> Self {
        use baylee_core::generated::subtypes;
        let mut assigned = BTreeMap::new();
        for raw in 0..subtypes::COUNT {
            let id = baylee_core::ids::SubtypeId::new(raw);
            let (Some(name), Some(kind)) = (subtypes::name(id), subtypes::kind(id)) else {
                // Dense by construction: `name` is an index into `NAMES` and
                // `kind` reads the masks the same ids are in. A gap means the
                // committed table is not the one this crate compiled against,
                // and appending to it would assign an id twice.
                panic!("subtype id {raw} is in the table without a name or a kind");
            };
            assigned.insert(
                (module_name(kind), name.to_ascii_lowercase()),
                (raw, name.to_string()),
            );
        }
        Self {
            assigned,
            next: subtypes::COUNT,
        }
    }

    /// No history: every name is new. What the unit tests below render with,
    /// and what the very first run would have had.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    fn id(&self, kind: SubtypeKind, name: &str) -> Option<u16> {
        self.assigned
            .get(&(module_name(kind), name.to_ascii_lowercase()))
            .map(|(id, _)| *id)
    }

    /// The names this kind holds that the catalog no longer prints.
    ///
    /// They keep their ids and stay in the table. Scryfall retiring a subtype
    /// must not renumber the ones after it, and an id that vanished would be a
    /// hole in `NAMES`, which is indexed by id.
    fn retired(&self, kind: SubtypeKind, current: &[String]) -> Vec<String> {
        self.assigned
            .iter()
            .filter(|((k, _), _)| *k == module_name(kind))
            .filter(|((_, lower), _)| !current.iter().any(|n| n.to_ascii_lowercase() == *lower))
            .map(|(_, (_, display))| display.clone())
            .collect()
    }
}

/// Renders the complete `crates/baylee-core/src/generated/subtypes.rs`.
///
/// `prior` is the assignment to keep; see [`PriorSubtypes`]. Pass
/// [`PriorSubtypes::empty`] to number a table from nothing.
#[must_use]
pub fn render_subtypes_rs(cats: &SubtypeCatalogs, prior: &PriorSubtypes) -> String {
    let mut out = String::with_capacity(96 * 1024);
    out.push_str(
        "// GENERATED by `cargo xtask codegen` — do not edit by hand.\n\
         // Source: Scryfall subtype catalogs (sorted per kind, ids append-only).\n\
         //\n\
         // An id is assigned once and never moves. A subtype Wizards print\n\
         // tomorrow takes the next free number wherever its name sorts, because a\n\
         // `SubtypeSet` travels on the wire and a renumbering would be silent: an\n\
         // older client would read the *wrong* subtypes rather than missing ones,\n\
         // and `VIEW_VERSION` cannot see it because the struct does not change.\n\n\
         #![allow(missing_docs, unused_imports, dead_code, clippy::all, clippy::pedantic)]\n\n\
         use crate::ids::SubtypeId;\n\
         use crate::types::{SubtypeKind, SubtypeSet};\n\n",
    );

    // Assignment first, emission second: every table below is a different
    // reading of this one list, so none of them can disagree with another.
    let mut next = prior.next;
    let mut members: Vec<(SubtypeKind, Vec<(String, String, u16)>)> = Vec::new();
    for (kind, names) in cats.ordered() {
        let mut all: Vec<String> = names.clone();
        all.extend(prior.retired(kind, names));
        all.sort();
        all.dedup();
        let mut assigned = Vec::with_capacity(all.len());
        for name in all {
            let id = prior.id(kind, &name).unwrap_or_else(|| {
                let id = next;
                next += 1;
                id
            });
            assigned.push((const_name(&name), name, id));
        }
        members.push((kind, assigned));
    }
    let count = next;

    // `NAMES` is indexed by id, so the ids have to be dense. They are by
    // construction — every one is either kept from the table or the next free
    // number — and a hole would mislabel every type line past it.
    let mut display_by_id: Vec<Option<String>> = vec![None; count as usize];
    for (_, assigned) in &members {
        for (_, name, id) in assigned {
            assert!(
                display_by_id[*id as usize].is_none(),
                "two subtypes at id {id}"
            );
            display_by_id[*id as usize] = Some(name.replace('"', "\\\""));
        }
    }

    for (kind, assigned) in &members {
        out.push_str(&format!(
            "pub mod {} {{\n    use super::SubtypeId;\n",
            module_name(*kind)
        ));
        for (cname, _, id) in assigned {
            out.push_str(&format!(
                "    pub const {cname}: SubtypeId = SubtypeId::new({id});\n"
            ));
        }
        out.push_str("}\n");
    }

    out.push_str(&format!("\npub const COUNT: u16 = {count};\n"));
    out.push_str(
        "\n// The bitmap has to have room for every id assigned above. A `SubtypeSet`\n\
         // is a fixed number of words, so this is the whole bound: outgrowing it is\n\
         // a compile error here rather than an index panic at a call site.\n\
         const _: () = assert!(COUNT <= SubtypeSet::CAPACITY);\n",
    );

    out.push_str(
        "\n// Whole-kind masks. Ids are no longer contiguous per kind — an appended\n\
         // subtype sits past every block — so each mask lists what belongs to it,\n\
         // and \"every creature type\" (changeling, CR 702.73) stays one compile-time\n\
         // bitmap rather than a scan with a `kind()` call per id.\n",
    );
    for (kind, assigned) in &members {
        let module = module_name(*kind);
        out.push_str(&format!(
            "pub const ALL_{}_TYPES: SubtypeSet = SubtypeSet::from_slice(&[\n",
            module.to_ascii_uppercase()
        ));
        for (cname, _, _) in assigned {
            out.push_str(&format!("    {module}::{cname},\n"));
        }
        out.push_str("]);\n");
    }

    // kind(), derived from the masks rather than from a range. The ranges were
    // what made a new creature type renumber every other kind, and a list
    // written out a second time here is a list that can drift from the first:
    // `ordered()` is the one place the kinds are enumerated, exactly as it was
    // when a hard-coded tail would have reported an eighth kind as a Spell.
    out.push_str(
        "\n// Derived from the masks above rather than from a range: the ranges were\n\
         // what made a new creature type renumber every other kind. One list, read\n\
         // two ways, so a kind and its members cannot drift apart.\n\
         pub const fn kind(id: SubtypeId) -> Option<SubtypeKind> {\n",
    );
    for (kind, _) in &members {
        out.push_str(&format!(
            "    if ALL_{}_TYPES.contains(id) {{\n        return Some(SubtypeKind::{kind:?});\n    }}\n",
            module_name(*kind).to_ascii_uppercase()
        ));
    }
    out.push_str(
        "    // An id this build has no name for: a table written after this one\n\
         \x20   // assigned it. Saying so is the point — the old answer was the last\n\
         \x20   // kind in the list, which is a wrong kind rather than no kind.\n\
         \x20   None\n}\n",
    );

    // by_name(), in id order — which is what makes a name two kinds print
    // answer with the earlier kind, the promise `const_path` makes above.
    out.push_str(
        "\npub fn by_name(name: &str) -> Option<SubtypeId> {\n    Some(match name.to_ascii_lowercase().as_str() {\n",
    );
    let mut arms: Vec<(u16, String)> = Vec::new();
    for (kind, assigned) in &members {
        for (cname, name, id) in assigned {
            arms.push((
                *id,
                format!(
                    "        \"{}\" => {}::{},",
                    name.to_ascii_lowercase().replace('"', "\\\""),
                    module_name(*kind),
                    cname
                ),
            ));
        }
    }
    arms.sort_by_key(|(id, _)| *id);
    for (_, arm) in arms {
        out.push_str(&arm);
        out.push('\n');
    }
    out.push_str("        _ => return None,\n    })\n}\n");

    // NAMES / name(): the way back from an id to printable text.
    //
    // `by_name` alone is not enough for a client: a type line is built from
    // the *projected* subtypes of an object (an animated land really is a
    // Creature — Elemental), and those arrive as ids, never as strings.
    out.push_str("\npub static NAMES: &[&str] = &[\n");
    for (id, name) in display_by_id.iter().enumerate() {
        let name = name
            .as_deref()
            .unwrap_or_else(|| panic!("no subtype at id {id}: the ids are not dense"));
        out.push_str(&format!("    \"{name}\",\n"));
    }
    out.push_str("];\n");
    out.push_str(
        "\npub fn name(id: SubtypeId) -> Option<&'static str> {\n    \
         NAMES.get(id.get() as usize).copied()\n}\n",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_names() {
        assert_eq!(const_name("Assembly-Worker"), "ASSEMBLY_WORKER");
        assert_eq!(const_name("Urza's"), "URZA_S");
        assert_eq!(const_name("Forest"), "FOREST");
    }

    #[test]
    fn renders_deterministically() {
        let mut cats = SubtypeCatalogs {
            creature: vec!["Wizard".into(), "Ally".into()],
            land: vec!["Forest".into()],
            ..SubtypeCatalogs::default()
        };
        cats.normalize();
        let a = render_subtypes_rs(&cats, &PriorSubtypes::empty());
        let b = render_subtypes_rs(&cats, &PriorSubtypes::empty());
        assert_eq!(a, b);
        assert!(a.contains("pub const ALLY: SubtypeId = SubtypeId::new(0);"));
        assert!(a.contains("pub const FOREST: SubtypeId = SubtypeId::new(2);"));
        assert_eq!(
            cats.const_path("forest").as_deref(),
            Some("subtypes::land::FOREST")
        );
    }

    /// A name two kinds print resolves to the earlier kind, and a reader that
    /// knows which one it wants can say so.
    ///
    /// Scryfall prints no such collision today, which is exactly why this is
    /// asserted on a catalog built by hand: the trap is one printing away and
    /// would be silent when it arrived — a checkland whose filter says
    /// `Filter::LAND` beside `HasSubtype(creature::CAVE)` matches nothing,
    /// and the land enters tapped forever while claiming to be implemented.
    #[test]
    fn a_name_two_kinds_share_is_reached_by_naming_the_kind() {
        let mut cats = SubtypeCatalogs {
            creature: vec!["Cave".into()],
            land: vec!["Cave".into()],
            ..SubtypeCatalogs::default()
        };
        cats.normalize();
        assert_eq!(
            cats.const_path("cave").as_deref(),
            Some("subtypes::creature::CAVE"),
            "the flat lookup answers with the earlier kind, id order"
        );
        assert_eq!(
            cats.const_path_of(SubtypeKind::Land, "cave").as_deref(),
            Some("subtypes::land::CAVE")
        );
        assert_eq!(
            cats.const_path_of(SubtypeKind::Land, "wizard").as_deref(),
            None,
            "a name of another kind is not a land subtype and must not answer"
        );
    }

    /// The committed table is what this emitter writes for the catalog the
    /// table itself describes. Rendering it again has to come out byte for
    /// byte, and that is two claims at once: the file really is generated
    /// output, and a run that changes nothing renumbers nothing.
    ///
    /// It needs no network, because the catalog is read back out of the
    /// compiled table — which is also why it keeps holding after Scryfall
    /// prints a new subtype and codegen appends it.
    #[test]
    fn the_committed_table_is_what_the_emitter_writes_for_it() {
        let cats = catalogs_from_the_compiled_table();
        let rendered = render_subtypes_rs(&cats, &PriorSubtypes::from_compiled_table());
        let committed = include_str!("../../baylee-core/src/generated/subtypes.rs");
        assert_eq!(
            rendered, committed,
            "the generated subtype table is not what codegen would write"
        );
    }

    /// The whole of #43: a new subtype takes the next free id and moves
    /// nothing. Asserted against the *real* table, because the failure it
    /// guards against is a creature type inserted alphabetically shifting
    /// every id after it — which on a two-name toy catalog is invisible.
    #[test]
    fn a_new_subtype_appends_and_renumbers_nothing() {
        use baylee_core::generated::subtypes;

        let prior = PriorSubtypes::from_compiled_table();
        let mut cats = catalogs_from_the_compiled_table();
        // Alphabetically first among creature types, so under the old scheme
        // it would have taken id 0 and pushed all 507 along by one.
        cats.creature.push("Aardvark".into());
        // And a land type Scryfall stopped printing, to prove a name leaving
        // the catalog does not close the gap behind it either.
        let dropped = cats.land.remove(0);
        cats.normalize();
        let out = render_subtypes_rs(&cats, &prior);

        assert!(
            out.contains(&format!(
                "    pub const AARDVARK: SubtypeId = SubtypeId::new({});\n",
                subtypes::COUNT
            )),
            "a new name takes the next free id"
        );
        assert!(
            out.contains(&format!("pub const COUNT: u16 = {};\n", subtypes::COUNT + 1))
        );
        for raw in 0..subtypes::COUNT {
            let id = baylee_core::ids::SubtypeId::new(raw);
            let name = subtypes::name(id).expect("every id is named");
            let module = module_name(subtypes::kind(id).expect("every id has a kind"));
            assert!(
                out.contains(&format!(
                    "    pub const {}: SubtypeId = SubtypeId::new({raw});\n",
                    const_name(name)
                )),
                "{module}::{name} moved off id {raw}"
            );
        }
        assert!(
            out.contains(&format!("    \"{dropped}\",\n")),
            "a retired subtype keeps its id and its row in NAMES"
        );
    }

    /// The catalogs the committed table stands for: one list per kind, read
    /// back out of the table itself.
    fn catalogs_from_the_compiled_table() -> SubtypeCatalogs {
        use baylee_core::generated::subtypes;
        let mut cats = SubtypeCatalogs::default();
        for raw in 0..subtypes::COUNT {
            let id = baylee_core::ids::SubtypeId::new(raw);
            let name = subtypes::name(id).expect("every id is named").to_string();
            let kind = subtypes::kind(id).expect("every id has a kind");
            for (k, list) in cats.ordered_mut() {
                if k == kind {
                    list.push(name);
                    break;
                }
            }
        }
        cats.normalize();
        cats
    }

    /// `NAMES` is indexed by the same sequential id the constants above get,
    /// so a drift between the two tables would silently mislabel every type
    /// line. Pinning the order here is what turns that into a test failure.
    #[test]
    fn names_are_indexed_by_id_and_keep_their_printed_spelling() {
        let mut cats = SubtypeCatalogs {
            creature: vec!["Wizard".into(), "Assembly-Worker".into()],
            land: vec!["Forest".into()],
            ..SubtypeCatalogs::default()
        };
        cats.normalize();
        let out = render_subtypes_rs(&cats, &PriorSubtypes::empty());
        // normalize() sorts: Assembly-Worker = 0, Wizard = 1, Forest = 2.
        assert!(out.contains(
            "pub static NAMES: &[&str] = &[\n    \"Assembly-Worker\",\n    \"Wizard\",\n    \"Forest\",\n];"
        ));
    }
}
