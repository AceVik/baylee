//! The L4 firing recorder: `BAYLEE_ABILITY_LOG=<dir>` makes every test of
//! this crate write down which card abilities it saw fire.
//!
//! The card-verification ladder (`docs/verification-hooks.md`, the contract
//! the consumer builds against) credits a card with L4 when every ability it
//! prints has actually fired in some test. This module is how the engine
//! says so: one JSON line per ability that fired, deduplicated per test, in
//! `<dir>/<test name>.jsonl`.
//!
//! `#[cfg(test)]` and nothing else — the module and every call into it are
//! absent from any build that is not this crate's own test binary. It is
//! inert unless the variable is set, and it only ever *reads* the game:
//! nothing here writes engine state, the journal or anything a hash sees.
//!
//! # What fired means
//!
//! | kind          | logged when                                                        | where                          |
//! |---------------|--------------------------------------------------------------------|--------------------------------|
//! | `spell`       | a spell whose face lists a spell ability finished resolving        | `Engine::finish_resolution`, `finalize_spell`'s door |
//! | `activated`   | an activated or loyalty ability finished resolving off the stack   | `Engine::finish_resolution`    |
//! | `triggered`   | a triggered, modal-triggered or chapter ability finished resolving, or a triggered mana ability produced its mana | `Engine::finish_resolution`, `resolve_triggered_mana_abilities` |
//! | `mana`        | a mana ability's cost was paid and it produced its mana            | `start_activation`, the CR 305.6 taps |
//! | `static`      | the projection (or, for "may choose not to untap", the untap step) applied a static ability's effect to an object | `layers::recompute_with`, `untap_optional` |
//! | `replacement` | a replacement rule changed an event, or a clone entered as a copy  | `replacement.rs`, `trigger.rs`, `apply_copy_choice` |
//!
//! Only abilities printed on a card of the compiled pool are logged, under
//! the card whose list the ability came from: a Clone that copied Llanowar
//! Elves and tapped for mana fired the *Elves'* ability. A token's, an
//! emblem's, a granted and a synthesised ability (prowess, ward, a reflexive
//! trigger) have no `(card, index)` of their own and are not logged; what
//! granted them was, as a static.

use crate::effects::{ContinuousEffect, EffectOrigin};
use crate::object::{GameObject, PrintedFace};
use crate::state::{CardLookup, GameState, ReplacementEntry};
use crate::zone::Zone;
use baylee_cards_dsl::{AbilityDef, Modifier, ReplacementRule};
use baylee_core::ids::{AbilityRef, CardIndex, EffectId, ObjectId};
use baylee_core::mana::ManaColor;
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// The environment variable naming the directory.
pub(crate) const VAR: &str = "BAYLEE_ABILITY_LOG";

/// How an ability fired: the `kind` field of a line.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Kind {
    Spell,
    Activated,
    Triggered,
    Static,
    Replacement,
    Mana,
}

impl Kind {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Spell => "spell",
            Self::Activated => "activated",
            Self::Triggered => "triggered",
            Self::Static => "static",
            Self::Replacement => "replacement",
            Self::Mana => "mana",
        }
    }

    /// The kind an entry of a card's ability list fires as, or `None` for
    /// the variants no door here logs (`docs/verification-hooks.md` names
    /// them).
    pub(crate) const fn of(ability: &AbilityDef) -> Option<Self> {
        Some(match ability {
            AbilityDef::Spell { .. } | AbilityDef::ModalSpell { .. } => Self::Spell,
            AbilityDef::Activated {
                mana_ability: true, ..
            }
            | AbilityDef::ActivatedConditional {
                mana_ability: true, ..
            } => Self::Mana,
            AbilityDef::Activated { .. }
            | AbilityDef::ActivatedConditional { .. }
            | AbilityDef::Loyalty { .. } => Self::Activated,
            AbilityDef::Triggered { .. }
            | AbilityDef::ModalTriggered { .. }
            | AbilityDef::SagaChapter { .. } => Self::Triggered,
            AbilityDef::Static(_) => Self::Static,
            AbilityDef::Replacement(_)
            | AbilityDef::CopyOnEnter { .. }
            | AbilityDef::CopyOnEnterUntilEot { .. } => Self::Replacement,
            AbilityDef::Unimplemented
            | AbilityDef::Ward { .. }
            | AbilityDef::Toxic { .. }
            | AbilityDef::Prepared { .. }
            | AbilityDef::Echo { .. }
            | AbilityDef::Suspend { .. } => return None,
        })
    }
}

/// The directory, when the variable names one; read once per process.
pub(crate) fn dir() -> Option<&'static PathBuf> {
    static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = PathBuf::from(std::env::var_os(VAR).filter(|v| !v.is_empty())?);
        std::fs::create_dir_all(&dir)
            .unwrap_or_else(|e| panic!("{VAR}={}: cannot create it: {e}", dir.display()));
        Some(dir)
    })
    .as_ref()
}

/// Whether the recorder is on at all: the one question every door asks
/// first, so a run without the variable pays a load of a `OnceLock`.
pub(crate) fn enabled() -> bool {
    dir().is_some()
}

/// The test this thread is running: libtest and nextest both name a test's
/// thread after the test (`engine::card_tests::…::name`). A thread a test
/// spawned itself is unnamed unless the test passed the name on
/// (`testkit::spawn_named`); its lines go under [`UNNAMED`].
fn test_name() -> String {
    std::thread::current()
        .name()
        .map_or_else(|| UNNAMED.to_owned(), str::to_owned)
}

/// Where the lines of an unnamed thread go.
pub(crate) const UNNAMED: &str = "unnamed-thread";

/// What this process has written, so each test gets each line once and its
/// file is started afresh on its first line.
#[derive(Default)]
struct Written {
    lines: HashSet<(String, u32, u32, Kind)>,
    files: HashSet<String>,
}

fn written() -> &'static Mutex<Written> {
    static WRITTEN: OnceLock<Mutex<Written>> = OnceLock::new();
    WRITTEN.get_or_init(Mutex::default)
}

/// The file a test's lines go to: the test's name, which is a Rust path and
/// so safe on every file system but one — Windows refuses `:` in a name.
/// The one place a name is made: a reader asks here too.
pub(crate) fn file_name(test: &str) -> String {
    let stem: String = if cfg!(windows) {
        test.replace(':', "_")
    } else {
        test.to_owned()
    };
    format!("{stem}.jsonl")
}

/// A JSON string literal.
pub(crate) fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Records that ability `index` of `card` fired as `kind`.
fn fired(card: CardIndex, index: u32, kind: Kind) {
    let Some(dir) = dir() else {
        return;
    };
    let test = test_name();
    let mut written = written()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !written
        .lines
        .insert((test.clone(), card.get(), index, kind))
    {
        return;
    }
    let first = written.files.insert(test.clone());
    let path = dir.join(file_name(&test));
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(!first)
        .truncate(first)
        .open(&path);
    let line = format!(
        "{{\"test\":{},\"card\":{},\"index\":{index},\"kind\":\"{}\"}}\n",
        json_str(&test),
        card.get(),
        kind.name()
    );
    file.and_then(|mut f| f.write_all(line.as_bytes()))
        .unwrap_or_else(|e| panic!("{VAR}: cannot write {}: {e}", path.display()));
}

/// Whether `card` is the compiled pool's card, as the engine's lookup has it.
///
/// A test may play a card it made up under an index the pool also uses
/// (`m2_tests`, `synthetic`); crediting the pool's card with what the made-up
/// one did would be evidence for the wrong card.
fn from_pool(lookup: &impl CardLookup, card: CardIndex) -> bool {
    match (lookup.card(card), baylee_cards::by_index(card)) {
        (Some(ours), Some(pool)) => std::ptr::eq(ours, pool),
        _ => false,
    }
}

/// Logs `(card, index)` as `kind` if the card is the pool's.
fn fired_from(lookup: &impl CardLookup, face: Option<PrintedFace>, index: u32, kind: Kind) {
    if let Some(face) = face
        && from_pool(lookup, face.card())
    {
        fired(face.card(), index, kind);
    }
}

/// The list an object's abilities come from and the face it is printed on:
/// what `GameObject::printed_abilities` answers and what
/// `GameObject::printed_face` names beside it.
fn printed(obj: &GameObject, lookup: &impl CardLookup) -> crate::object::AbilityList {
    obj.printed_ability_list(lookup)
}

fn fired_ability(
    lookup: &impl CardLookup,
    list: &crate::object::AbilityList,
    index: u32,
    kind: Kind,
) {
    let origin = list.origin(index as usize);
    fired_from(
        lookup,
        origin
            .origin
            .and_then(crate::object::AbilityOrigin::printed),
        origin.index,
        kind,
    );
}

// --- spells, activated and triggered abilities --------------------------------

thread_local! {
    /// Spells that began to resolve, with the line each would log, read as
    /// it began: a spell whose own effect moves it off the stack (Temporal
    /// Mastery's "Exile Temporal Mastery") is no longer there to be read
    /// when its resolution finishes.
    static RESOLVING: RefCell<Vec<(ObjectId, Option<PrintedFace>)>> = const { RefCell::new(Vec::new()) };
}

/// A spell on top of the stack begins to resolve: what it would log is
/// read now, for [`resolved`] to log if the spell has left the stack by the
/// time it finishes. Only a resolution that finishes logs it; one CR 608.2b
/// removes never reaches here.
pub(crate) fn resolving(state: &GameState, lookup: &impl CardLookup, on_stack: ObjectId) {
    if !enabled() {
        return;
    }
    let Some(obj) = state.object(on_stack) else {
        return;
    };
    if obj.zone != Zone::Stack || obj.ability.is_some() {
        return;
    }
    let face = spell_face(obj, lookup);
    RESOLVING.with_borrow_mut(|resolving| {
        resolving.retain(|(id, _)| *id != on_stack);
        resolving.push((on_stack, face));
    });
}

/// The face a resolving spell credits, when its list has a spell ability.
fn spell_face(obj: &GameObject, lookup: &impl CardLookup) -> Option<PrintedFace> {
    let list = printed(obj, lookup);
    list.abilities
        .iter()
        .any(|a| matches!(a, AbilityDef::Spell { .. } | AbilityDef::ModalSpell { .. }))
        .then_some(list.printed)
        .flatten()
}

/// A spell or an ability on the stack has finished resolving: the first line
/// of `Engine::finish_resolution`, and the door a spell with no spell
/// effects of its own (an Aura) leaves the stack by.
pub(crate) fn resolved(state: &GameState, lookup: &impl CardLookup, on_stack: ObjectId) {
    if !enabled() {
        return;
    }
    let began = RESOLVING.with_borrow_mut(|resolving| {
        let at = resolving.iter().position(|(id, _)| *id == on_stack)?;
        Some(resolving.remove(at).1)
    });
    let on_the_stack = state.object(on_stack).filter(|obj| obj.zone == Zone::Stack);
    let Some(obj) = on_the_stack else {
        // Gone during its own resolution: the line read as it began.
        if let Some(face) = began {
            fired_from(lookup, face, AbilityRef::SPELL, Kind::Spell);
        }
        return;
    };
    if let Some(loc) = obj.ability {
        // Reserved indices are synthesised abilities (prowess, ward, a
        // granted one, a reflexive trigger): no entry of any list.
        if !AbilityRef::new(CardIndex::new(0), loc.index).is_listed_ability() {
            return;
        }
        // The list captured as it was put on the stack, else its source's.
        let list = if obj.own_abilities.is_some() {
            printed(obj, lookup)
        } else if let Some(source) = state.object(loc.source) {
            printed(source, lookup)
        } else {
            return;
        };
        if let Some(kind) = list.abilities.get(loc.index as usize).and_then(Kind::of)
            && kind != Kind::Spell
        {
            fired_ability(lookup, &list, loc.index, kind);
        }
        return;
    }
    fired_from(
        lookup,
        spell_face(obj, lookup),
        AbilityRef::SPELL,
        Kind::Spell,
    );
}

// --- mana abilities ------------------------------------------------------------

thread_local! {
    /// Mana abilities whose resolution stopped to ask a colour, by source:
    /// they produce their mana when the answer finishes them.
    static MANA_ASKING: RefCell<Vec<(ObjectId, PrintedFace, u32, Kind)>> = const { RefCell::new(Vec::new()) };
}

/// A mana ability of `source` at `index` had its cost paid and resolved
/// (CR 605.3b): logged now if it has produced its mana, or once the question
/// it stopped on is answered.
pub(crate) fn mana_activated(
    state: &GameState,
    lookup: &impl CardLookup,
    source: ObjectId,
    index: u32,
    produced: bool,
) {
    mana_resolved(state, lookup, source, index, Kind::Mana, produced);
}

/// A triggered mana ability of `source` at `index` (CR 605.1b, Badgermole
/// Cub's "add an additional {G}") resolved off the stack as it triggered
/// (CR 605.4a): `triggered`, as its entry is, logged when it has produced
/// its mana as [`mana_activated`] is.
pub(crate) fn triggered_mana(
    state: &GameState,
    lookup: &impl CardLookup,
    source: ObjectId,
    index: u32,
    produced: bool,
) {
    mana_resolved(state, lookup, source, index, Kind::Triggered, produced);
}

fn mana_resolved(
    state: &GameState,
    lookup: &impl CardLookup,
    source: ObjectId,
    index: u32,
    kind: Kind,
    produced: bool,
) {
    if !enabled() {
        return;
    }
    let Some(object) = state.object(source) else {
        return;
    };
    let origin = object.printed_ability_list(lookup).origin(index as usize);
    let Some(face) = origin
        .origin
        .and_then(crate::object::AbilityOrigin::printed)
    else {
        return;
    };
    let index = origin.index;
    if produced {
        fired_from(lookup, Some(face), index, kind);
    } else {
        MANA_ASKING.with_borrow_mut(|asking| asking.push((source, face, index, kind)));
    }
}

/// A mana ability's resolution finished after a question.
pub(crate) fn mana_finished(lookup: &impl CardLookup, source: ObjectId) {
    if !enabled() {
        return;
    }
    let asked = MANA_ASKING.with_borrow_mut(|asking| {
        let at = asking.iter().rposition(|(s, ..)| *s == source)?;
        Some(asking.remove(at))
    });
    if let Some((_, face, index, kind)) = asked {
        fired_from(lookup, Some(face), index, kind);
    }
}

/// A land tapped for `color` through CR 305.6's intrinsic ability.
///
/// Every land with a basic land type prints that ability as its own entry
/// (`landgen::intrinsic_mana_ability`, "({T}: Add {G}.)"), and the offer taps
/// it through the rule rather than through the entry. It is one ability, so
/// the entry that makes this colour for `{T}` is the one that fired.
pub(crate) fn intrinsic_mana(
    state: &GameState,
    lookup: &impl CardLookup,
    source: ObjectId,
    color: ManaColor,
) {
    if !enabled() {
        return;
    }
    let Some(obj) = state.object(source) else {
        return;
    };
    let list = obj.ability_list(lookup);
    let colors = crate::casting::intrinsic_mana_colors(state, source);
    let entry = position(&list.abilities, |a| {
        a.is_intrinsic_mana_ability() && colors.contains(&color)
    });
    if let Some(entry) = entry {
        fired_ability(lookup, &list, entry, Kind::Mana);
    }
}

// --- statics and replacement rules ----------------------------------------------

/// Which printed ability each registered static effect and replacement rule
/// came from, noted where the engine has a card lookup and read where it
/// has none (the projection, the replacement readers).
///
/// Per thread, because a test's games run on its thread. Keyed by the
/// effect's id, source and modifier and **not** its timestamp: attaching an
/// Equipment gives its grant a new timestamp (CR 613.7e) and keeps the
/// effect, and a key with the timestamp in it no longer found the note made
/// before the attach. Two games on one thread cannot lend each other a card
/// because a note is rewritten, not skipped, when the key is seen again.
struct Sources {
    statics: Vec<(StaticKey, CardIndex, u32)>,
    rules: Vec<(ObjectId, ReplacementRule, CardIndex, u32)>,
}

type StaticKey = (EffectId, ObjectId, Modifier);

thread_local! {
    static SOURCES: RefCell<Sources> = const {
        RefCell::new(Sources {
            statics: Vec::new(),
            rules: Vec::new(),
        })
    };
}

fn static_key(fx: &ContinuousEffect) -> Option<StaticKey> {
    (fx.origin == EffectOrigin::Static)
        .then_some(fx.source)
        .flatten()
        .map(|source| (fx.id, source, fx.modifier))
}

/// The position of the first entry of `list` that `wanted` accepts.
fn position(
    list: impl Into<crate::copiable_abilities::AbilityDefs>,
    wanted: impl Fn(&AbilityDef) -> bool,
) -> Option<u32> {
    list.into()
        .iter()
        .position(wanted)
        .and_then(|at| u32::try_from(at).ok())
}

/// Notes the printed ability behind every static effect and replacement rule
/// the engine has registered: the last line of `sync_static_effects`, which
/// registers both.
pub(crate) fn note_sources(state: &GameState, lookup: &impl CardLookup) {
    if !enabled() {
        return;
    }
    SOURCES.with_borrow_mut(|sources| {
        for fx in state.effects.iter() {
            let Some(key) = static_key(fx) else {
                continue;
            };
            sources.statics.retain(|(k, ..)| *k != key);
            let Some(obj) = state.object(key.1) else {
                continue;
            };
            let is_it = |a: &AbilityDef| {
                matches!(a, AbilityDef::Static(sa) if sa.modifier == fx.modifier && sa.layer == fx.layer)
            };
            let list = printed(obj, lookup);
            let index = state.effect_text_overrides.iter().find_map(|(id, origin)| {
                if *id != fx.id { return None; }
                let crate::text_changes::TextOrigin::Ability { source, index, .. } = origin else { return None; };
                (source.object == obj.id && list.abilities.get(*index as usize).is_some_and(is_it)).then_some(*index)
            }).or_else(|| position(&list.abilities, is_it));
            let found = index.map(|index| {
                let origin = list.origin(index as usize);
                (origin.origin.and_then(crate::object::AbilityOrigin::printed), origin.index)
            });
            if let Some((Some(face), at)) = found
                && from_pool(lookup, face.card())
            {
                sources.statics.push((key, face.card(), at));
            }
        }
        for entry in &state.replacement_rules {
            if sources
                .rules
                .iter()
                .any(|(s, r, ..)| *s == entry.source && *r == entry.rule)
            {
                continue;
            }
            let Some(obj) = state.object(entry.source) else {
                continue;
            };
            let list = obj.ability_list(lookup);
            let origin = position(&list.abilities, |a| matches!(a, AbilityDef::Replacement(r) if *r == entry.rule))
                .map(|index| list.origin(index as usize));
            if let Some((at, face)) = origin.and_then(|origin| Some((origin.index, origin.origin?.printed()?)))
                && from_pool(lookup, face.card())
            {
                sources.rules.push((entry.source, entry.rule, face.card(), at));
            }
        }
    });
}

/// The projection applied `fx` to an object.
pub(crate) fn static_applied(fx: &ContinuousEffect) {
    if !enabled() {
        return;
    }
    let Some(key) = static_key(fx) else {
        return;
    };
    let noted = SOURCES.with_borrow(|sources| {
        sources
            .statics
            .iter()
            .find(|(k, ..)| *k == key)
            .map(|&(_, card, at)| (card, at))
    });
    if let Some((card, at)) = noted {
        fired(card, at, Kind::Static);
    }
}

/// `entry` changed an event.
pub(crate) fn replaced(entry: &ReplacementEntry) {
    if !enabled() {
        return;
    }
    let noted = SOURCES.with_borrow(|sources| {
        sources
            .rules
            .iter()
            .find(|(s, r, ..)| *s == entry.source && *r == entry.rule)
            .map(|&(.., card, at)| (card, at))
    });
    if let Some((card, at)) = noted {
        fired(card, at, Kind::Replacement);
    }
}

/// `copier` is entering as a copy of something: its "enter as a copy"
/// ability did what it says (CR 707.9, a replacement of how it enters).
pub(crate) fn copied_on_entry(state: &GameState, lookup: &impl CardLookup, copier: ObjectId) {
    if !enabled() {
        return;
    }
    let Some(obj) = state.object(copier) else {
        return;
    };
    let list = printed(obj, lookup);
    let at = position(&list.abilities, |a| {
        matches!(
            a,
            AbilityDef::CopyOnEnter { .. } | AbilityDef::CopyOnEnterUntilEot { .. }
        )
    });
    if let Some(at) = at {
        fired_ability(lookup, &list, at, Kind::Replacement);
    }
}
