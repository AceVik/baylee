//! card-script reference card script → `CardDef` abilities.
//!
//! A card script is a line-oriented rules encoding:
//!
//! ```text
//! Name:Lightning Bolt
//! ManaCost:R
//! Types:Instant
//! A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ …
//! ```
//!
//! Names, costs, types and P/T are not read here — those come from Scryfall,
//! which is the identity this project already keys on. What this module reads
//! is the rules half: `K:` keywords, `A:`/`T:`/`S:`/`R:` abilities, and the
//! `SVar:` sub-abilities they chain into.
//!
//! # Refusal is the feature
//!
//! [`transcode`] returns `Some` only when **every line, every effect in every
//! `SubAbility$` chain, and every parameter key** was consumed. An unknown
//! API, an unknown parameter, a computed `SVar` — any one of them and the
//! card is refused and stays a stub. That rule is what makes a generated
//! `Coverage::Generated` claim as much as a hand-written `Implemented`: an
//! ability quietly dropped because its `NoRegen$ True` was ignored would be
//! worse than no card at all, because the deckbuilder would offer it.
//!
//! That key is the worked example of why *consuming* a parameter is not the
//! same as reading one. It was consumed as vacuous for as long as this
//! engine had no regeneration — true then, and a card the reader would have
//! written wrongly the moment a shield existed. It is read now.
//!
//! The corpus is read as an automated lookup only; no file of it is copied into
//! this repository.

use crate::body::CardBody;
use crate::catalog::SubtypeCatalogs;
use crate::tokengen::TokenLookup;
use std::collections::BTreeMap;

mod activated;
mod alternative_cost;
mod animation;
mod archon_kismet;
mod atoms;
mod chains;
mod constants;
mod creature_type;
mod damage_pair;
mod dig;
mod divided_damage;
mod effects;
mod final_portals;
mod gond_gate;
mod guardian52;
mod horse_johan;
mod imprison;
mod keywords;
mod kicker;
mod mana;
mod prevention;
mod protection52;
mod regeneration;
mod riftstone_portal;
mod saga_ward_convoke;
mod song52;
mod statics;
mod support;
mod targets;
mod transform48;
mod triggers;
mod wall52;
mod wave27a;
mod wave27b;
mod wave27c;
mod wave27d;
mod wave27e;
mod wave28b;

use atoms::{
    Bound, amount, color_atom, count_bound, hybrid_pair, keyword_atom, mana_color_const,
    named_atom, plain_number, pump_amount, stat_atom,
};
pub use atoms::{atoms, token_stems};
use constants::{
    card_type_const, color_const, color_word, cost_parts, counter_kind, keyword_const,
    keyword_enter_modifier, keyword_static, on_the_battlefield,
};
pub use constants::{keyword_const_of, keyword_enter_modifier_of, keyword_static_of};
use support::{CLUE_TOKEN_SCRIPT, asks_for_an_object, generic_mana, object_cost, printed_mana};
pub use support::{SUPPORTED_APIS, apis_used, is_supported_api, is_vanilla, refusal_reason};

/// One parsed card script.
#[derive(Debug, Default)]
#[allow(clippy::struct_excessive_bools)] // independent script-header rule markers
pub struct CardScript {
    /// Whether the type line explicitly names Land, for land-only permissions.
    pub is_land: bool,
    /// Whether the type line explicitly names Room (CR 709.5 split rooms use
    /// card-level concatenated door abilities and `UnlockDoor`).
    pub is_room: bool,
    /// Verbatim `Types:` header, used for card-type keyword guards (e.g.
    /// `Suspend` haste/timing restrictions and permanent-only keywords).
    pub type_line: String,
    /// Whether the `ManaCost:` line contains a variable `X`.
    pub has_x_mana_cost: bool,
    /// `Oracle:` line, when present, for cross-checking target player vs. opponent.
    pub oracle: String,
    /// `PT:` line (e.g. `*/4`, `2/1+*`), used by single-sided P/T CDAs.
    pub pt: Option<String>,
    /// `AlternateMode:<mode>` header on multi-face scripts (`Split`,
    /// `Adventure`, `Modal`, `DoubleFaced`, …).
    pub alternate_mode: Option<String>,
    /// Verbatim `Name:` header on this face, used when resolving `CopyFaceFrom:`.
    pub script_name: String,
    /// `CopyFaceFrom:<card>` on a Prepare alternate face referencing a standalone script.
    pub copy_face_from: Option<String>,
    /// Whether Forge declared `Text:CARDNAME can be your commander.` (checked
    /// against Scryfall's Oracle in `stubgen`).
    pub has_commander_text: bool,
    /// Second face parsed after an `ALTERNATE` separator line.
    pub alternate: Option<Box<Self>>,
    /// `K:` lines, verbatim.
    pub keywords: Vec<String>,
    /// Rules lines as `(kind, body)` — kind is `A`, `T`, `S` or `R`.
    pub rules: Vec<(char, String)>,
    /// `SVar:<name>:<body>` definitions.
    pub svars: BTreeMap<String, String>,
    /// A line whose prefix this module does not model at all.
    pub unknown_lines: Vec<String>,
}

/// Line prefixes whose content this project takes from Scryfall instead, or
/// that are the corpus's own deckbuilding/AI hints and carry no rules.
const IGNORED_PREFIXES: &[&str] = &[
    "Defense",
    "Colors",
    "DeckHas",
    "DeckHints",
    "DeckNeeds",
    "AI",
    "Draft",
    "HandLifeModifier",
];

/// Splits a card script into its rules-bearing lines.
#[must_use]
pub fn parse(text: &str) -> CardScript {
    let mut front_lines = Vec::new();
    let mut back_lines: Option<Vec<&str>> = None;
    for line in text.lines() {
        let line = line.trim_end();
        if line == "ALTERNATE" && back_lines.is_none() {
            back_lines = Some(Vec::new());
            continue;
        }
        if let Some(back) = &mut back_lines {
            back.push(line);
        } else {
            front_lines.push(line);
        }
    }
    let mut out = parse_face(&front_lines);
    if let Some(back) = back_lines {
        out.alternate = Some(Box::new(parse_face(&back)));
    }
    out
}

fn parse_face(lines: &[&str]) -> CardScript {
    const AFTER_BLOCKERS_TEXT: &str =
        "Text:Cast CARDNAME only during combat after blockers are declared.";

    let mut out = CardScript::default();
    let mut cast_only_after_blockers = false;
    for &line in lines {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((prefix, rest)) = line.split_once(':') else {
            out.unknown_lines.push(line.to_string());
            continue;
        };
        match prefix {
            "Name" => out.script_name = rest.trim().to_string(),
            "CopyFaceFrom" => {
                if out.copy_face_from.is_none() {
                    out.copy_face_from = Some(rest.trim().to_string());
                } else {
                    out.unknown_lines.push(line.to_string());
                }
            }
            "Types" => {
                out.is_land = rest.split_whitespace().any(|word| word == "Land");
                out.is_room = rest.split_whitespace().any(|word| word == "Room");
                out.type_line = rest.to_string();
            }
            "ManaCost" => out.has_x_mana_cost = rest.split_whitespace().any(|word| word == "X"),
            "PT" => out.pt = Some(rest.trim().to_string()),
            "Oracle" => out.oracle = rest.to_string(),
            "AlternateMode" => {
                if out.alternate_mode.is_none() {
                    out.alternate_mode = Some(rest.trim().to_string());
                } else {
                    out.unknown_lines.push(line.to_string());
                }
            }
            "Variant" if rest.starts_with("UniversesWithin:FlavorName:") => {}
            "Loyalty" if rest.trim().parse::<u32>().is_ok() => {}
            "Text" if rest.trim() == "CARDNAME can be your commander." => {
                out.has_commander_text = true;
            }
            "Text" if line == AFTER_BLOCKERS_TEXT => cast_only_after_blockers = true,
            "K" => out.keywords.push(rest.to_string()),
            "A" | "T" | "S" | "R" => out
                .rules
                .push((prefix.chars().next().unwrap_or('A'), rest.to_string())),
            "SVar" => {
                if let Some((name, body)) = rest.split_once(':') {
                    out.svars.insert(name.to_string(), body.to_string());
                } else {
                    out.unknown_lines.push(line.to_string());
                }
            }
            _ if IGNORED_PREFIXES.contains(&prefix) => {}
            _ => out.unknown_lines.push(line.to_string()),
        }
    }
    if cast_only_after_blockers {
        let spells: Vec<&str> = out
            .rules
            .iter()
            .filter(|(kind, spec)| *kind == 'A' && spec.trim_start().starts_with("SP$"))
            .map(|(_, spec)| spec.as_str())
            .collect();
        if spells.is_empty()
            || !spells.iter().all(|spec| {
                spec.split(" | ").any(|part| {
                    part.trim_start() == "ActivationPhases$ Declare Blockers->EndCombat"
                })
            })
        {
            out.unknown_lines.push(AFTER_BLOCKERS_TEXT.to_string());
        }
    }
    out
}

/// Parameter keys that are pure prose or AI hints: they change no rule, so
/// consuming them silently is safe. Everything not on this list must be
/// claimed by a rule or the card is refused.
const PROSE_KEYS: &[&str] = &[
    "SpellDescription",
    "StackDescription",
    "Description",
    "TriggerDescription",
    "TgtPrompt",
    "AILogic",
    "AINoRecursiveCheck",
    "AICheckSVar",
    "AISVarCompare",
    "AIPreference",
    // Which mana the reference's own AI should rather spend — twelve
    // scripts, and every value it takes is a colour, `Treasure` or
    // `NotSameCard` (measured 2026-09-16). It changes no rule, which is
    // the only thing that puts a key on this list; it is here because
    // Basalt Monolith prints it beside `AILogic`, not to move a number.
    "AIManaPref",
    "AICurse",
    "AITgts",
    "PrecostDesc",
    "CostDesc",
    "References",
    "SpellDescriptionSVar",
];

/// An ordered `Key$ Value` list that records what has been read.
#[derive(Debug, Default, Clone)]
struct Params {
    entries: Vec<(String, String)>,
    opponents_only_lure: bool,
}

impl Params {
    /// Splits `"DealDamage | ValidTgts$ Any | NumDmg$ 3"` into the API name
    /// and its parameters.
    fn parse(spec: &str) -> Option<(String, Self)> {
        let mut parts = spec.split(" | ");
        let head = parts.next()?.trim();
        // `AB$ DealDamage`, `DB$ …`, `SP$ …`, `ST$ …`, or a bare `Mode$ …`.
        // Every one of those has a `$`, and a head without one is none of
        // them: an `SVar` body is very often a bare number (`SVar:SacMe:1`,
        // `SVar:X:3`), which this read as an API called "1" with no
        // parameters and then removed a leading entry that was never
        // pushed. It panicked rather than refusing, and held only because
        // nothing walked *every* `SVar` — every caller asked about a body a
        // rules line had named. `token_stems` walks all of them.
        let api = head.split_once('$')?.1.trim().to_string();
        let mut entries = Vec::new();
        for part in parts {
            let (key, value) = part.split_once('$')?;
            entries.push((key.trim().to_string(), value.trim().to_string()));
        }
        let opponents_only_lure = entries.iter().any(|(k, v)| {
            k == "SpellDescription"
                && v.to_ascii_lowercase()
                    .contains("creatures your opponents control able to block")
        });
        Some((
            api,
            Self {
                entries,
                opponents_only_lure,
            },
        ))
    }

    fn take(&mut self, key: &str) -> Option<String> {
        let i = self.entries.iter().position(|(k, _)| k == key)?;
        Some(self.entries.remove(i).1)
    }

    /// Whether a parameter is present, without claiming it.
    fn has(&self, key: &str) -> bool {
        self.entries.iter().any(|(k, _)| k == key)
    }

    /// A parameter's value, without claiming it: for a rule that has to
    /// know one key before it can read another.
    fn peek(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Claims a label that only restates the filter standing beside it.
    ///
    /// **Not a [`PROSE_KEYS`] entry, and the difference is the guard.** A key
    /// on that list is prose wherever it appears; this one is prose only
    /// while the parameter it describes is there to carry the meaning — so
    /// the same word would be refused on a line that wrote the label and no
    /// filter, where it would be the only thing said about what is being
    /// found.
    ///
    /// Measured before it was written, which is what the rule about that
    /// list asks for: `ChangeTypeDesc$` appears on 369 lines of the
    /// reference, every one of them a `ChangeZone`, and **not one** of them
    /// without a `ChangeType$` beside it. Its commonest value is `basic
    /// land` (257), which is `Land.Basic` spelled for a human.
    fn claim_label(&mut self, label: &str, filter: &str) {
        if self.has(filter) {
            self.take(label);
        }
    }

    fn drop_prose(&mut self) {
        self.entries
            .retain(|(k, _)| !PROSE_KEYS.contains(&k.as_str()));
    }

    /// The first parameter still unclaimed, for reporting.
    fn first_key(&self) -> Option<&str> {
        self.entries.first().map(|(k, _)| k.as_str())
    }

    /// True when every parameter has been claimed.
    fn exhausted(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A chain of effects and its independently announced target instances.
#[derive(Debug, Default)]
#[allow(clippy::struct_excessive_bools)]
struct Chain {
    effects: Vec<String>,
    target: Option<String>,
    target_is_x: bool,
    target_up_to: Option<u8>,
    target_exact: Option<u8>,
    target_unique: bool,
    target_constraint: Option<&'static str>,
    second_target: Option<String>,
    second_target_up_to: Option<u8>,
    second_target_constraint: Option<&'static str>,
    third_target: Option<String>,
    third_target_up_to: Option<u8>,
    third_target_constraint: Option<&'static str>,
    could_add_mana: bool,
    moves_library: bool,
}

/// "… unless <a player> pays <a price>" (CR 118.12a), read off one line.
struct Unless {
    /// Who is asked, as a `PlayerRel`.
    payer: &'static str,
    price: Price,
    /// `UnlessSwitched$ True`: the line's effect is what paying *buys*
    /// ("you may pay {1}. If you do, …"), not what refusing costs.
    switched: bool,
    /// `UnlessResolveSubs$ WhenNotPaid`: the line's `SubAbility$` chain is
    /// also gated behind the same refusal.
    resolve_subs: bool,
}

/// A price a player may pay as an ability resolves, in the shape the effect
/// that charges it carries.
enum Price {
    /// Generic mana, as an `Amount` expression: `PlayerMayPayOr`/`Then`.
    Generic(String),
    /// A printed cost with colour in it, spelled `{G}{G}`:
    /// `PlayerMayPayManaOr`/`Then`.
    Printed(String),
    /// One cost part the player pays by naming an object:
    /// `PlayerMayPayCostOr`.
    Part(String),
}

/// Which side of a block a `T:Mode$ AttackerBlockedByCreature` line puts
/// its source on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BlockRole {
    /// `ValidBlocker$ Card.Self`: "whenever this creature blocks …".
    Blocks,
    /// `ValidCard$ Card.Self`: "whenever this creature becomes blocked by …".
    Blocked,
}

/// One half of a "blocks or becomes blocked by" trigger, read and waiting
/// for its mirror ([`Tx::block_trigger`]).
struct BlockHalf {
    role: BlockRole,
    /// The valid-string of the other creature.
    other: String,
    /// Whether this half carried `Secondary$ True`.
    secondary: bool,
    /// Whether this half is the "one or more" form (`Blocks` + `AttackerBlocked`).
    one_or_more: bool,
    /// The ability this half wrote.
    ability: String,
}

#[allow(clippy::struct_excessive_bools)]
struct Tx<'a> {
    svars: &'a BTreeMap<String, String>,
    script_name: &'a str,
    oracle: &'a str,
    type_line: &'a str,
    cats: &'a SubtypeCatalogs,
    /// The reference's token scripts, when a checkout is at hand. `None` is
    /// a run with no token corpus and not a gap in the DSL, which is why the
    /// refusal it produces says so in those words — a report that ranked a
    /// missing directory as the top blocker would send somebody to write a
    /// rule that already exists.
    tokens: Option<&'a TokenLookup>,
    /// Whether the rules line being read announces an `X` of its own.
    ///
    /// True for `A:` — a spell's X is chosen on the stack (CR 601.2b) and an
    /// activated ability's on activation, and the engine reads both back as
    /// `Amount::X`. False for `T:`, `S:` and `R:`, where nothing announced a
    /// number and `Amount::X` would silently evaluate to nought. Set per
    /// line rather than per script, because one card writes both.
    has_x: bool,
    /// Whether the rules line being read is a spell (`A:SP$`) rather than an
    /// activated ability or a trigger. A spell has no "itself" to act on
    /// once it resolves, so a clause that defaults to the source means
    /// nothing on one.
    on_a_spell: bool,
    /// The mode of the `T:` line being read (`Phase`, `ChangesZone`, …), and
    /// `None` on every other kind of line: what "that player" means is the
    /// trigger's to say.
    trigger_mode: Option<String>,
    /// Which side of a block the `T:` line being read puts its source on,
    /// and the half it read ([`Tx::block_trigger`]); `None` on every other
    /// line.
    block_line: Option<(BlockRole, String, bool)>,
    /// Whether `block_line` came from the one-or-more block trigger pair
    /// (`Blocks` + `AttackerBlocked`) rather than `AttackerBlockedByCreature`.
    block_line_one_or_more: bool,
    /// The first half of a "blocks or becomes blocked by" trigger, while its
    /// mirror has not been read. A script that ends with one here printed a
    /// sentence this reader does not say, and is refused.
    block_half: Option<BlockHalf>,
    /// Whether the chain being read is a delayed trigger's `Execute$`, where
    /// `Defined$ DelayTriggerRememberedLKI` is the object it remembers.
    in_delayed: bool,
    /// An earlier unconditional effect in this resolution chain chose a number.
    has_resolution_chosen_number: bool,
    /// This permanent or an earlier effect in this resolution chain chose a color.
    has_resolution_chosen_color: bool,
    /// An earlier effect in this resolution chain chose a card name.
    has_resolution_chosen_name: bool,
    /// Whether the activated ability being read sacrifices `CARDNAME` in its cost.
    cost_sacrifices_self: bool,
    /// Whether the activated ability being read sacrifices another object in its cost.
    cost_sacrifices_other: bool,
    /// Whether the spell/ability cost discards a single card (`Discard<1/...>`).
    cost_discards_single_for_cost: bool,
    /// Whether the active damage trigger binds `event_object` to the damage source.
    damage_trigger_binds_source: bool,
    /// Whether the active damage trigger's `ValidSource$` is the attached permanent.
    damage_trigger_source_is_attached: bool,
    /// Whether the active damage trigger is a combat-damage trigger.
    damage_trigger_is_combat: bool,
    /// Whether the active trigger/chapter body runs inside an optional `DiscardHand` payment.
    in_discard_hand_may: bool,
    /// Whether the active trigger/chapter body runs inside a `ChooseNumberEx` `X` payment.
    in_resolution_chosen_x: bool,
    /// Whether an earlier instruction in this chain emitted `Effect::LoseLife`.
    chain_prev_lose_life: bool,
    /// Whether the line currently being lowered has a single object target.
    single_object_target: bool,
    /// Whether the script's `ManaCost:` line contains a variable `X`.
    has_x_mana_cost: bool,
    /// Whether the script carries `K:Flash`, for timing-sensitive keyword guards.
    has_flash: bool,
    /// Whether the script declares exactly one single-cost `K:Kicker:` line
    /// (no dual kicker and no `K:Multikicker:`), so `times_kicked` is strictly
    /// 0 or 1.
    has_single_kicker: bool,
    /// Maps 1-based script kicker slot (`1` -> `[0]`, `2` -> `[1]`) to the
    /// 0-based Oracle-ordered `FaceDef::additional_costs` index for dual
    /// kicker (`K:Kicker:<c1>:<c2>`, CR 702.33a–b). `None` on non-dual cards.
    dual_kicker_remap: Option<[u8; 2]>,
    /// Whether the triggered ability being read triggers on `Card.Self`.
    trigger_is_self: bool,
    /// Whether the cost being read belongs to `S: Mode$ AlternativeCost`,
    /// where `cast_wizard` pays `ExileFromHand`, `TapN` and multi-return
    /// parts that an activated ability does not.
    in_alternative_cost: bool,
    /// Whether the script declares `K:Evoke:`, which claims `o.alt_cast` for
    /// `Trigger::EntersBattlefieldEvoked` and must not coexist with another
    /// `AlternativeCost`.
    has_evoke: bool,
    /// Whether the triggered ability being read is `Trigger::ETB` (exact
    /// `ValidCard$ Card.Self`), the only trigger whose stack object carries
    /// `times_kicked` for `Effect::IfKicked`.
    is_etb_self_trigger: bool,
    /// Whether the triggered ability being read is `Trigger::BlocksCreature`,
    /// whose `EventObject` is the blocked attacker (`TriggeredAttacker`).
    trigger_blocks_creature: bool,
    /// Set by `changes_zone_trigger` when `ValidCard$` is `Card.Self+kicked`,
    /// so `triggered()` still sets `trigger_is_self = true` even though the
    /// filter was hoisted into a static.
    trigger_self_etb_kicked: bool,
    /// Set by `changes_zone_trigger` when `ValidCard$` is
    /// `Card.Self+kicked <1|2>` on a dual-kicker permanent, carrying the
    /// remapped 0-based Oracle kicker index for `Condition::KickedWith`.
    trigger_self_etb_kicked_with: Option<u8>,
    /// While splitting a line whose numeric parameter references a
    /// `Count$Kicked.<A>.<B>` `SVar` into `Effect::IfKicked { then, otherwise }`,
    /// holds `Some(true)` for the kicked arm and `Some(false)` for the
    /// unkicked arm; `None` everywhere else.
    kicked_branch: Option<bool>,
    /// Set when `kicked_branch` actually resolves a `Count$Kicked.<A>.<B>`
    /// parameter during line evaluation.
    kicked_branch_used: std::cell::Cell<bool>,
    /// The `PutCounter` line being read names the source while a later line of
    /// its chain declares the target (Sphinx-Bone Wand).
    counter_on_source: bool,
    /// Whether the chain being read follows `ChooseType | Type$ Creature`,
    /// enabling `ChosenType` filter atoms for resolution-chosen creature types.
    in_chosen_creature_type: bool,
    /// Whether the activated cost being read is activated from the graveyard
    /// (`ActivationZone$ Graveyard`), allowing `ExileFromGrave<1/CARDNAME>`.
    pub(super) in_graveyard_activation: bool,
    /// Whether the activated cost being read is activated from the hand
    /// (`ActivationZone$ Hand`), allowing self-discard and self-exile costs.
    pub(super) in_hand_activation: bool,
    /// Whether this face is the front half of a `Split` card (`AlternateMode$ Split`).
    is_split_front: bool,
    /// When transcoding a Room split card (CR 709.5), the door index of this
    /// face (`Some(0)` left, `Some(1)` right); `None` on all non-Room cards.
    room_door: Option<u8>,
    /// Whether this face is the front face of a `DoubleFaced` transform card
    /// (`AlternateMode$ DoubleFaced`).
    is_double_faced_front: bool,
    /// `PT:` line on this face, for single-sided characteristic-defining P/T.
    pt: Option<&'a str>,
    /// Cached `(keywords_expr, grant_abilities_ref)` for repeated `K:Backup:<N>:<SVar>` lines.
    backup_grants: BTreeMap<String, (String, String)>,
    /// Next Class level bar expected in script order.
    class_next_level: u8,
    /// Stashed `K:Overload:<mana>` cost for post-rule modal rewriting in `apply_overload`.
    overload_cost: Option<String>,
    /// Active Wave 27c `Effect::CaptureAffected::then` scope, if any.
    capture_scope: Option<wave27c::CaptureScope>,
    body: CardBody,
    /// The first `Api.Key` no rule claimed, if that is why this
    /// script was refused. Recorded rather than derived, because a
    /// second list of each rule's keys would rot the first time a
    /// rule learned a new one.
    unclaimed: std::cell::RefCell<Option<String>>,
}

impl<'a> Tx<'a> {
    /// Whether `ChosenType` in a filter refers to a creature type chosen either
    /// as this permanent entered (`EnterModifier::ChooseSubtype`) or earlier in
    /// the resolving effect chain (`Effect::ChooseCreatureType`).
    pub(super) fn has_chosen_creature_type(&self) -> bool {
        self.in_chosen_creature_type
            || self
                .body
                .enter_modifiers
                .iter()
                .any(|m| m == "EnterModifier::ChooseSubtype")
    }

    pub(super) fn new(
        script: &'a CardScript,
        cats: &'a SubtypeCatalogs,
        tokens: Option<&'a TokenLookup>,
    ) -> Self {
        let mut single_kickers = 0usize;
        let mut multi_or_dual = false;
        for kw in &script.keywords {
            if let Some(rest) = kw.strip_prefix("Kicker:") {
                if rest.contains(':') {
                    multi_or_dual = true;
                } else {
                    single_kickers += 1;
                }
            } else if kw.starts_with("Multikicker:") {
                multi_or_dual = true;
            }
        }
        Self {
            svars: &script.svars,
            script_name: &script.script_name,
            oracle: &script.oracle,
            type_line: &script.type_line,
            cats,
            tokens,
            has_x: false,
            on_a_spell: false,
            trigger_mode: None,
            block_line: None,
            block_line_one_or_more: false,
            block_half: None,
            in_delayed: false,
            has_resolution_chosen_number: false,
            has_resolution_chosen_color: false,
            has_resolution_chosen_name: false,
            cost_sacrifices_self: false,
            cost_sacrifices_other: false,
            cost_discards_single_for_cost: false,
            damage_trigger_binds_source: false,
            damage_trigger_source_is_attached: false,
            damage_trigger_is_combat: false,
            in_discard_hand_may: false,
            in_resolution_chosen_x: false,
            chain_prev_lose_life: false,
            single_object_target: false,
            has_x_mana_cost: script.has_x_mana_cost,
            has_flash: script.keywords.iter().any(|k| k.trim() == "Flash"),
            has_single_kicker: !multi_or_dual && single_kickers == 1,
            dual_kicker_remap: None,
            trigger_is_self: false,
            in_alternative_cost: false,
            has_evoke: false,
            is_etb_self_trigger: false,
            trigger_blocks_creature: false,
            trigger_self_etb_kicked: false,
            trigger_self_etb_kicked_with: None,
            kicked_branch: None,
            kicked_branch_used: std::cell::Cell::new(false),
            counter_on_source: false,
            in_chosen_creature_type: false,
            in_graveyard_activation: false,
            in_hand_activation: false,
            is_split_front: script.alternate_mode.as_deref() == Some("Split"),
            room_door: None,
            is_double_faced_front: script.alternate_mode.as_deref() == Some("DoubleFaced"),
            pt: script.pt.as_deref(),
            backup_grants: BTreeMap::new(),
            class_next_level: 2,
            overload_cost: None,
            capture_scope: None,
            body: CardBody::default(),
            unclaimed: std::cell::RefCell::new(None),
        }
    }

    /// Whether `Effect::IfKicked` reads a live kicked flag from `res.on_stack`.
    ///
    /// The engine populates `times_kicked` on the stack object for spells
    /// (`A:SP$`) and for `Trigger::ETB` (`ValidCard$ Card.Self`) only;
    /// activated abilities, delayed triggers, and non-ETB triggers have
    /// `times_kicked == 0` on their stack objects and must never emit
    /// `Effect::IfKicked`.
    pub(super) fn can_check_kicked(&self) -> bool {
        !self.in_delayed && (self.on_a_spell || self.is_etb_self_trigger)
    }

    /// Whether `Condition::OptionalCostPaid` can read a live optional-cost flag
    /// for this face (`FaceDef::optional_cost`, CR 118.8b).
    pub(super) fn can_check_optional_cost(&self) -> bool {
        !self.in_delayed && self.body.optional_cost.is_some()
    }

    /// Lowers a self-spell `S: Mode$ OptionalCost` static ability into
    /// `FaceDef::optional_cost` (`CR 118.8b`).
    fn lower_optional_cost_static(&mut self, spec: &str, is_land: bool) -> Option<()> {
        if is_land {
            return self.deny("`S: Mode$ OptionalCost` on a land".to_string());
        }
        if self.body.optional_cost.is_some() {
            return self.deny("a second `S: Mode$ OptionalCost` on one face".to_string());
        }
        let Some((mode, mut p)) = Params::parse(spec) else {
            return self.deny("an `S:` line with no `$` in it".to_string());
        };
        if mode != "OptionalCost" {
            return self.deny(format!("static ability `S: Mode$ {mode}`"));
        }
        p.drop_prose();
        if p.take("EffectZone").as_deref() != Some("All")
            || p.take("ValidCard").as_deref() != Some("Card.Self")
            || p.take("ValidSA").as_deref() != Some("Spell")
        {
            return self.deny("static ability `S: Mode$ OptionalCost`".to_string());
        }
        let Some(cost_raw) = p.take("Cost") else {
            return self.deny("`S: Mode$ OptionalCost` without `Cost$`".to_string());
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `OptionalCost.{key}`"));
        }
        let (cost_expr, uses_x) = self.parse_kicker_cost_expr(&cost_raw)?;
        if uses_x {
            return self.deny("a variable-X `S: Mode$ OptionalCost`".to_string());
        }
        self.body.optional_cost = Some(format!("Some({cost_expr})"));
        self.body.notes.push("optional additional cost".to_string());
        Some(())
    }

    /// Parses `SVar:<name>:Count$Kicked.<when_kicked>.<when_unkicked>` when
    /// both arms are plain non-negative whole numbers.
    pub(super) fn kicked_pair(&self, raw: &str) -> Option<(u32, u32)> {
        let def = self.svars.get(raw.trim())?.trim();
        let rest = def.strip_prefix("Count$Kicked.")?;
        let (kicked, unkicked) = rest.split_once('.')?;
        if unkicked.contains('.') {
            return None;
        }
        Some((kicked.parse().ok()?, unkicked.parse().ok()?))
    }

    /// [`amount`], resolving a `Count$Kicked.<A>.<B>` `SVar` when evaluating a
    /// kicked/unkicked branch (refusing a 0 arm so non-pump effects never emit
    /// vacuous zero-amount instructions).
    pub(super) fn amount_num(&self, raw: &str) -> Option<String> {
        if let Some(kicked) = self.kicked_branch {
            let trimmed = raw.trim().trim_start_matches('+');
            if let Some((a, b)) = self.kicked_pair(trimmed) {
                let value = if kicked { a } else { b };
                if value == 0 {
                    return None;
                }
                self.kicked_branch_used.set(true);
                return Some(format!("Amount::Fixed({value})"));
            }
        }
        amount(raw, self.svars, self.has_x)
    }

    /// [`plain_number`], resolving a non-zero `Count$Kicked.<A>.<B>` `SVar` when
    /// evaluating a kicked/unkicked branch.
    pub(super) fn plain_num(&self, raw: &str) -> Option<i64> {
        if let Some(kicked) = self.kicked_branch {
            let trimmed = raw.trim().trim_start_matches('+');
            if let Some((a, b)) = self.kicked_pair(trimmed) {
                let value = if kicked { a } else { b };
                if value == 0 {
                    return None;
                }
                self.kicked_branch_used.set(true);
                return Some(i64::from(value));
            }
        }
        plain_number(raw, self.svars)
    }
}

impl Tx<'_> {
    /// Records the first reason this script was refused.
    ///
    /// A diagnostic side channel, which is why it is behind a `RefCell`:
    /// the refusal points are `&self` readers, and threading `&mut` through
    /// them to carry a message would put the report in the way of the rules.
    fn note(&self, what: String) {
        let mut slot = self.unclaimed.borrow_mut();
        if slot.is_none() {
            *slot = Some(what);
        }
    }

    /// Refuses, saying why.
    ///
    /// The spelling that makes a silent `?` visible at the point it happens.
    /// Every refusal that reaches [`refusal_reason`] with nothing recorded is
    /// reported as "no reason recorded", which is a worklist entry naming no
    /// work — the same fault [`crate::scriptgen`]'s own report was written to
    /// fix one level up.
    fn deny<T>(&self, what: String) -> Option<T> {
        self.note(what);
        None
    }

    /// The named constants on `Filter`, against what this reader would
    /// otherwise write out.
    ///
    /// Every pair is the **same bytes** — clause order included — which is
    /// what makes the substitution invisible to `pool-dump` and is why the
    /// table is a lookup rather than a second way of building a filter. A
    /// constant this reader cannot reach byte for byte is therefore absent
    /// rather than approximated: `BASIC_LAND` is supertype first, and
    /// `YOUR_BASIC_LAND` nests it, where a script filter reading
    /// `Land.Basic.YouCtrl` builds three flat clauses. `YOUR_LAND` is here
    /// because it is noun first, which is the order this reader writes.
    ///
    /// A valid-string names its clauses in an order of its own, and for a
    /// filter of two or more the corpus prints more than one — so a row is
    /// worth having only where the constant's order is the one the corpus
    /// predominantly writes. Measured over the reference corpus, in files,
    /// with a restriction counted under either joiner (`Land.nonBasic` and
    /// `Land+nonBasic` are one order written two ways):
    /// `Artifact,Creature` 247 against `Creature,Artifact` 80,
    /// `Artifact.YouCtrl` 432 against **nought** the other way,
    /// `Creature,Planeswalker` 247 against nought,
    /// `Artifact,Creature,Enchantment` 31 against `Artifact,Enchantment,
    /// Creature` 13, and `Land.nonBasic` 78 against nought.
    ///
    /// The commit that added these rows reported the artifact one as "432
    /// against `YouCtrl.Artifact` 27", and the 27 was a measurement with an
    /// unescaped dot: what it found was `YouCtrl,Artifact`, where the comma
    /// is the corpus's *or* and the string is a different filter entirely.
    /// No script in the corpus writes the control clause before the noun.
    ///
    /// `ANOTHER_CREATURE_YOU_CONTROL` is the one constant that fails the
    /// test, and the reason the rule is written down: its order is `your`
    /// before `another`, where the corpus writes `Creature.Other+YouCtrl`
    /// 381 times against `Creature.YouCtrl+Other` 170, so a row would name
    /// the rarer spelling and write the commoner one out — the table
    /// disagreeing with itself on one filter.
    const NAMED: &'static [(&'static str, &'static str)] = &[
        (
            "Filter::And(&[Filter::CREATURE, Filter::ControlledByYou])",
            "Filter::YOUR_CREATURE",
        ),
        (
            "Filter::And(&[Filter::CREATURE, Filter::ControlledByOpponent])",
            "Filter::OPPONENT_CREATURE",
        ),
        (
            "Filter::And(&[Filter::CREATURE, Filter::Another])",
            "Filter::ANOTHER_CREATURE",
        ),
        (
            "Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::IsToken)])",
            "Filter::NONTOKEN_CREATURE",
        ),
        (
            "Filter::And(&[Filter::CREATURE, Filter::HasSupertype(SupertypeSet::LEGENDARY)])",
            "Filter::LEGENDARY_CREATURE",
        ),
        (
            "Filter::And(&[Filter::CREATURE, Filter::Attacking])",
            "Filter::ATTACKING_CREATURE",
        ),
        (
            "Filter::And(&[Filter::LAND, Filter::ControlledByYou])",
            "Filter::YOUR_LAND",
        ),
        (
            "Filter::And(&[Filter::ARTIFACT, Filter::ControlledByYou])",
            "Filter::YOUR_ARTIFACT",
        ),
        (
            "Filter::And(&[Filter::LAND, Filter::Not(&Filter::HasSupertype(SupertypeSet::BASIC))])",
            "Filter::NONBASIC_LAND",
        ),
        (
            "Filter::Or(&[Filter::ARTIFACT, Filter::ENCHANTMENT])",
            "Filter::ARTIFACT_OR_ENCHANTMENT",
        ),
        (
            "Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE])",
            "Filter::ARTIFACT_OR_CREATURE",
        ),
        (
            "Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE, Filter::ENCHANTMENT])",
            "Filter::ARTIFACT_CREATURE_OR_ENCHANTMENT",
        ),
        (
            "Filter::Or(&[Filter::CREATURE, Filter::PLANESWALKER])",
            "Filter::CREATURE_OR_PLANESWALKER",
        ),
        (
            "Filter::Or(&[Filter::HasType(TypeSet::INSTANT), Filter::HasType(TypeSet::SORCERY)])",
            "Filter::INSTANT_OR_SORCERY",
        ),
    ];
}

pub(super) fn is_vanilla_face(script: &CardScript) -> bool {
    script.keywords.is_empty()
        && script.rules.is_empty()
        && script.unknown_lines.is_empty()
        && !script.is_land
        && !script.has_commander_text
        && script.oracle.trim().is_empty()
}

fn is_optional_cost_static(spec: &str) -> bool {
    Params::parse(spec).is_some_and(|(mode, _)| mode == "OptionalCost")
}

fn transcode_face(
    script: &CardScript,
    cats: &SubtypeCatalogs,
    tokens: Option<&TokenLookup>,
    statics: String,
    allow_vanilla: bool,
    room_door: Option<u8>,
) -> Option<CardBody> {
    if !script.unknown_lines.is_empty() {
        return None;
    }
    // Costless Mayhem is supported only as a land-play permission.
    if script.keywords.iter().any(|word| word == "Mayhem") && !script.is_land {
        return None;
    }
    if script.has_commander_text
        && !script.oracle.trim().is_empty()
        && !script
            .oracle
            .to_ascii_lowercase()
            .contains("can be your commander")
    {
        return None;
    }
    let mut tx = Tx::new(script, cats, tokens);
    tx.room_door = room_door;
    tx.body.statics = statics;
    if song52::song_candidate(script) {
        tx.song_whole_card(script)?;
        return Some(tx.body);
    }
    // Read `K:Kicker:` lines before other keywords so `dual_kicker_remap` is
    // already established when `K:ETBReplacement:` references `kicked 1`/`2`.
    for line in script
        .keywords
        .iter()
        .filter(|line| line.starts_with("Kicker:"))
    {
        tx.keyword(line)?;
    }
    for line in script
        .keywords
        .iter()
        .filter(|line| !line.starts_with("Kicker:"))
    {
        tx.keyword(line)?;
    }
    // Read `S: Mode$ OptionalCost` before other rules so `self.body.optional_cost`
    // is established before `A:`/`T:` lines check `Condition::OptionalCostPaid`.
    for (kind, spec) in &script.rules {
        if *kind == 'S' && is_optional_cost_static(spec) {
            tx.lower_optional_cost_static(spec, script.is_land)?;
        }
    }
    for (kind, spec) in &script.rules {
        if *kind == 'S' && is_optional_cost_static(spec) {
            continue;
        }
        tx.rule(*kind, spec)?;
    }
    tx.block_halves_paired()?;
    tx.align_dual_kicker_order();
    tx.apply_overload()?;
    if !(tx.body.has_own_rules() || allow_vanilla && is_vanilla_face(script)) {
        return None;
    }
    Some(tx.body)
}

/// Whether a face's `Types:` line names only instant/sorcery card types (plus
/// optional supertypes), matching `CardDef::is_split_spell_card`.
pub(super) fn is_pure_spell_face(type_line: &str) -> bool {
    use baylee_core::types::{SupertypeSet, TypeSet};

    let mut has_spell = false;
    for word in type_line.split_whitespace() {
        if let Some(t) = TypeSet::from_word(word) {
            if !matches!(t, TypeSet::INSTANT | TypeSet::SORCERY) {
                return false;
            }
            has_spell = true;
        } else if SupertypeSet::from_word(word).is_none() {
            return false;
        }
    }
    has_spell
}

/// Validates cross-face invariants (`Fuse`, `Disturb`) after both halves of a
/// two-faced card have been transcoded, returning an honest refusal reason if
/// invalid and applying any cross-face propagation (e.g. `disturb` mana).
pub(super) fn finalize_two_face_bodies(
    mode: &str,
    script: &CardScript,
    back_script: &CardScript,
    front: &CardBody,
    back: &mut CardBody,
) -> Option<String> {
    if script.is_room || back_script.is_room {
        if mode != "Split" || !script.is_room || !back_script.is_room {
            return Some("Room card without two Room split halves".to_string());
        }
        if front.fuse
            || back.fuse
            || front.aftermath
            || back.aftermath
            || front.disturb.is_some()
            || back.disturb.is_some()
        {
            return Some("Room split card with incompatible split/DFC mechanic".to_string());
        }
    }
    if front.fuse {
        if mode != "Split" || front.aftermath || back.aftermath || back.fuse {
            return Some(
                "`Fuse` combined with non-split or incompatible aftermath halves".to_string(),
            );
        }
        if !is_pure_spell_face(&script.type_line) || !is_pure_spell_face(&back_script.type_line) {
            return Some("`Fuse` on a split card with non-instant/non-sorcery half".to_string());
        }
    }
    if let Some(disturb) = front.disturb.clone() {
        let back_has_exile = back
            .abilities
            .iter()
            .any(|a| a.contains("ReplacementRule::ExileSelfInsteadOfGraveyard"));
        let back_is_permanent = !back_script.type_line.is_empty()
            && !back_script
                .type_line
                .split_whitespace()
                .any(|w| matches!(w, "Land" | "Instant" | "Sorcery"));
        if mode != "DoubleFaced" || back.disturb.is_some() || !back_has_exile || !back_is_permanent
        {
            return Some(
                "`Disturb` whose back face is not a non-land permanent with exile replacement"
                    .to_string(),
            );
        }
        back.disturb = Some(disturb);
    }
    None
}

/// Reads a whole card script, or refuses it.
///
/// # Errors
/// Never errors; an unreadable script is `None`, which is what keeps a
/// generated card honest.
#[must_use]
pub fn transcode(
    script: &CardScript,
    cats: &SubtypeCatalogs,
    tokens: Option<&TokenLookup>,
) -> Option<CardBody> {
    if script.alternate_mode.is_none() && script.alternate.is_none() {
        if script.is_room {
            return None;
        }
        let mut body = transcode_face(script, cats, tokens, String::new(), false, None)?;
        body.prune_unused_statics();
        body.notes.push("transcoded from the card's rules".into());
        return Some(body);
    }
    let mode = script.alternate_mode.as_deref()?;
    if !matches!(
        mode,
        "Split" | "Adventure" | "Modal" | "DoubleFaced" | "Prepare"
    ) {
        return None;
    }
    let back_script = script.alternate.as_deref()?;
    let is_room = script.is_room || back_script.is_room;
    if (is_room && (mode != "Split" || !script.is_room || !back_script.is_room))
        || back_script.copy_face_from.is_some()
        || back_script.alternate_mode.is_some()
        || back_script.alternate.is_some()
    {
        return None;
    }
    let (front_door, back_door) = if is_room {
        (Some(0), Some(1))
    } else {
        (None, None)
    };
    let mut front = transcode_face(script, cats, tokens, String::new(), true, front_door)?;
    let statics = std::mem::take(&mut front.statics);
    let mut back = transcode_face(back_script, cats, tokens, statics, true, back_door)?;
    front.statics = std::mem::take(&mut back.statics);
    if finalize_two_face_bodies(mode, script, back_script, &front, &mut back).is_some() {
        return None;
    }
    if !front.has_own_rules() && !back.has_own_rules() {
        return None;
    }
    front.back_is_adventure = mode == "Adventure";
    front.back_is_prepare = mode == "Prepare";
    front.back = Some(Box::new(back));
    front.prune_unused_statics();
    front.notes.push("transcoded from the card's rules".into());
    Some(front)
}

#[cfg(test)]
mod tests;

/// A card-script reference checkout, resolved by card name.
///
/// Held by `codegen` so a stub can be transcoded from the rules reference
/// when one is available locally, and generated exactly as before when it is
/// not — the checkout is never part of the build.
#[derive(Debug)]
pub struct ScriptLookup {
    root: std::path::PathBuf,
    index: BTreeMap<String, String>,
}

impl ScriptLookup {
    /// Wraps a cardsfolder and the name → relative-path index `codegen`
    /// already builds.
    #[must_use]
    pub fn new(root: std::path::PathBuf, index: BTreeMap<String, String>) -> Self {
        Self { root, index }
    }

    fn raw_script(&self, name: &str) -> Option<CardScript> {
        let key = self
            .index
            .get(name)
            .or_else(|| {
                let front = name.split(" // ").next()?;
                self.index.get(front)
            })
            .or_else(|| {
                self.index
                    .iter()
                    .find(|(k, _)| k.split(" // ").any(|half| half == name))
                    .map(|(_, path)| path)
            })?;
        let text = std::fs::read_to_string(self.root.join(key)).ok()?;
        Some(parse(&text))
    }

    /// The script for a card, by its Scryfall name, resolving single-hop
    /// `CopyFaceFrom:<card>` links on `Prepare` alternate faces.
    #[must_use]
    pub fn script(&self, name: &str) -> Option<CardScript> {
        let mut out = self.raw_script(name)?;
        if out.alternate_mode.as_deref() == Some("Prepare")
            && let Some(alt) = out.alternate.as_mut()
            && let Some(source_name) = alt.copy_face_from.clone()
        {
            let mut copied = self.raw_script(&source_name)?;
            let mut resolved = if copied.script_name.eq_ignore_ascii_case(&source_name) {
                copied.alternate = None;
                copied.alternate_mode = None;
                copied
            } else if let Some(alt_face) = copied.alternate.take()
                && alt_face.script_name.eq_ignore_ascii_case(&source_name)
            {
                *alt_face
            } else {
                return None;
            };
            if resolved.copy_face_from.is_some() {
                return None;
            }
            resolved.alternate = None;
            resolved.alternate_mode = None;
            resolved.copy_face_from = None;
            **alt = resolved;
        }
        Some(out)
    }
}
