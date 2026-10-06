//! Script spellings to DSL constants: costs, counters, colours, card types
//! and keywords.

use super::{BTreeMap, amount};

/// A `Cost$` value split into its parts.
///
/// Whitespace separates the parts of a cost — and it also appears *inside*
/// one, because the corpus's bracketed forms carry its own interface prose
/// as their last field: `Sac<1/CARDNAME/this artifact>`,
/// `Return<1/Forest/a Forest>`, `Discard<1/Card/a card>`. A plain
/// `split_whitespace` cuts those in half, and the half that is left over
/// then refuses the card under a cost part nobody wrote, reported as
/// "artifact>".
/// **731** of the reference's costs and 20 of its token scripts' contain such
/// a space, Treasure's among them, and every one of them was being read as
/// two parts.
///
/// So the split happens at bracket depth zero only. Nesting is counted
/// rather than merely flagged, because a cost's brackets do nest —
/// `tapXType<2/Creature.untapped/untapped creature>` is the shallow case and
/// a valid-string carrying its own `<…>` the deeper one — and a boolean
/// would close the first bracket on the innermost `>`.
pub(super) fn cost_parts(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut part = String::new();
    for c in raw.chars() {
        match c {
            '<' => {
                depth += 1;
                part.push(c);
            }
            '>' => {
                depth = depth.saturating_sub(1);
                part.push(c);
            }
            _ if c.is_whitespace() && depth == 0 => {
                if !part.is_empty() {
                    out.push(std::mem::take(&mut part));
                }
            }
            _ => part.push(c),
        }
    }
    if !part.is_empty() {
        out.push(part);
    }
    out
}

/// A filter expression with "and it is on the battlefield" added to it.
///
/// One `Filter::And` and never one inside another: an already-conjoined
/// expression is spliced open, so `Card.Self+YouCtrl` writes three clauses
/// in a row rather than a pair holding a pair. The splice is safe on the
/// text because `filter_expr` returns one complete expression, so a leading
/// `Filter::And(&[` is closed by the trailing `])` and by nothing else.
pub(super) fn on_the_battlefield(expr: &str) -> String {
    const ZONE: &str = "Filter::InZone(ZoneRef::Battlefield)";
    match expr
        .strip_prefix("Filter::And(&[")
        .and_then(|rest| rest.strip_suffix("])"))
    {
        Some(clauses) => format!("Filter::And(&[{ZONE}, {clauses}])"),
        None => format!("Filter::And(&[{ZONE}, {expr}])"),
    }
}

/// A reference-script counter code as the `CounterKind` spelling for it.
///
/// Read by two rules that must not disagree: the `PutCounter` *effect* and
/// the `AddCounter<n/KIND>` *cost*. Written twice, `M1M1` could end up
/// meaning one thing on a card's ability and another on its cost, and
/// nothing in the build would notice — the two never meet.
///
/// Only the kinds the rules know are written out here — the nine named ones
/// plus the P/T family below. Every other counter Magic prints is a
/// `CounterKind::Custom` id, and assigning one is a decision with a printed
/// word behind it rather than something a reader may do on the way past. So
/// the ids are **not** assigned here: `baylee_cards_dsl::counters::ASSIGNED`
/// is the registry, this reads it, and a code that is in neither place
/// refuses the card. Deriving the table instead of retyping it is the
/// difference between a registry and two lists of numbers that happen to
/// agree — and the registry is on the other side of the seam anyway, since
/// what a generated card writes is the *constant*, `counters::STORAGE`.
///
/// `Lifelink` is spelled the way it looks. Every other named code here is
/// shouted and a *keyword* counter is written as the keyword (`Flying`,
/// `Indestructible`, and lifelink, which is the one of those the engine reads
/// — `layers.rs` grants the keyword from it). The corpus is not consistent
/// about this beyond the two groups: it prints `Stun` 72 times and `STUN` 22.
/// So a code is matched exactly as the script spells it, and one spelled the
/// other way refuses the card rather than being guessed at.
///
/// `PxPy` and `MxMy` are read by [`pt_counter`] instead of by name, because
/// CR 122.1a is one rule over an open-ended set of pairs: the corpus prints
/// eleven of them and a table of names would go silent on the twelfth.
pub(super) fn counter_kind(code: &str) -> Option<String> {
    if let Some(pt) = pt_counter(code) {
        return Some(pt);
    }
    Some(
        match code {
            "LOYALTY" => "CounterKind::Loyalty",
            "LORE" => "CounterKind::Lore",
            "TIME" => "CounterKind::Time",
            "CHARGE" => "CounterKind::Charge",
            "POISON" => "CounterKind::Poison",
            "ENERGY" => "CounterKind::Energy",
            "RAD" => "CounterKind::Rad",
            "LEVEL" => "CounterKind::Level",
            "Lifelink" => "CounterKind::Lifelink",
            _ => return assigned_counter(code),
        }
        .to_string(),
    )
}

/// A counter word the DSL has already given a `Custom` id, as the constant
/// that names it.
///
/// The match is on the **word**, case-folded, because the two sides spell it
/// differently on purpose: the registry writes what a player says
/// (`"storage"`), the reference shouts a code (`STORAGE`), and the constant
/// is the word in capitals. `counters::ASSIGNED`'s own test keeps every word
/// lowercase ASCII, which is what makes that last step well defined rather
/// than a guess — and the generated card naming a constant that does not
/// exist would not compile, so the build is the second check.
pub(super) fn assigned_counter(code: &str) -> Option<String> {
    let word = code.to_ascii_lowercase();
    baylee_cards_dsl::counters::ASSIGNED
        .iter()
        .find(|(assigned, _)| *assigned == word)
        .map(|(assigned, _)| format!("counters::{}", assigned.to_ascii_uppercase()))
}

/// `P1P1`, `M0M1`, `P2P2` — a +X/+Y or -X/-Y counter (CR 122.1a).
///
/// The sign letter is the same on both halves in every one of the eleven
/// codes the corpus prints, which is the rule's own two forms; a mixed pair
/// is refused rather than guessed at, because `CounterKind` cannot say one.
pub(super) fn pt_counter(code: &str) -> Option<String> {
    let (sign, rest) = code.split_at_checked(1)?;
    let variant = match sign {
        "P" => "Plus",
        "M" => "Minus",
        _ => return None,
    };
    let (power, toughness) = rest.split_once(sign)?;
    let power: u8 = power.parse().ok()?;
    let toughness: u8 = toughness.parse().ok()?;
    // The two Magic prints everywhere are written as the constants that name
    // them. They are the same *value* as the general form — `CounterKind`
    // has one variant for all of them — so this is a spelling and not a
    // second meaning, and it keeps 2528 scripts' worth of cards reading
    // `CounterKind::P1P1` the way a player says it.
    Some(match (variant, power, toughness) {
        ("Plus", 1, 1) => "CounterKind::P1P1".to_string(),
        ("Minus", 1, 1) => "CounterKind::M1M1".to_string(),
        _ => format!("CounterKind::{variant} {{ power: {power}, toughness: {toughness} }}"),
    })
}

/// A script colour word as our `Color` constant.
///
/// `Colorless` is the empty set rather than a colour, and `ChosenColor` is
/// a choice a transcoder cannot make — both stay unread.
pub(super) fn color_const(word: &str) -> Option<&'static str> {
    Some(match word {
        "White" => "Color::White",
        "Blue" => "Color::Blue",
        "Black" => "Color::Black",
        "Red" => "Color::Red",
        "Green" => "Color::Green",
        _ => return None,
    })
}

/// A type word as the `TypeSet` constant for it, or `None` when the
/// word is a subtype (or a type the engine has no bit for).
pub(super) fn card_type_const(word: &str) -> Option<&'static str> {
    Some(match word {
        "Artifact" => "TypeSet::ARTIFACT",
        "Creature" => "TypeSet::CREATURE",
        "Enchantment" => "TypeSet::ENCHANTMENT",
        "Instant" => "TypeSet::INSTANT",
        "Kindred" | "Tribal" => "TypeSet::KINDRED",
        "Land" => "TypeSet::LAND",
        "Planeswalker" => "TypeSet::PLANESWALKER",
        "Sorcery" => "TypeSet::SORCERY",
        "Battle" => "TypeSet::BATTLE",
        _ => return None,
    })
}

/// A keyword line → the bit in our `KeywordSet`, for the keywords that
/// are text-independent (CR 702). Parameterized keywords are data, not bits,
/// and are refused here on purpose.
///
/// So are the keywords **no engine rule reads**, which is the same honesty
/// rule one step earlier. A bit the layer system carries and combat never
/// asks about is worse than a stub: the card says `Coverage::Implemented`,
/// the view draws the sheath, the deckbuilder offers it as playable, and the
/// creature is blocked as if it had nothing. The table therefore lists only
/// what `keyword_tests::ENFORCED` lists, and the gate that found this —
/// `no_card_claims_a_keyword_the_engine_ignores` — is what fires when the
/// two drift. Ten entries came off it for that reason: fear, intimidate,
/// shadow, horsemanship, infect, wither, persist, undying, skulk and
/// flanking. Four of them were already on cards; the other six were waiting
/// for the pool to grow into them. A keyword returns here on the commit that
/// gives it a rule, not before.
/// A keyword line → a `static_ability!` on this card, for the one printed
/// sentence the reference files as a keyword and the rules make a static
/// ability.
///
/// "You may choose not to untap CARDNAME during your untap step" is not a
/// keyword at all (CR 702 lists none like it); the reference keeps it on a
/// `K:` line because it has no parameters, which is also why it can be read
/// by matching the whole sentence. It is written exactly one way across the
/// 45 scripts that print it, so the match is the literal string and not a
/// pattern — anything else that ever lands on a `K:` line is a keyword or a
/// refusal, as before.
pub(super) fn keyword_static(line: &str) -> Option<&'static str> {
    match line.trim() {
        "You may choose not to untap CARDNAME during your untap step." => {
            Some("Modifier::MayChooseNotToUntap")
        }
        _ => None,
    }
}

pub(super) fn keyword_const(line: &str) -> Option<&'static str> {
    Some(match line.trim() {
        "Flying" => "KeywordSet::FLYING",
        "First Strike" => "KeywordSet::FIRST_STRIKE",
        "Double Strike" => "KeywordSet::DOUBLE_STRIKE",
        "Deathtouch" => "KeywordSet::DEATHTOUCH",
        "Haste" => "KeywordSet::HASTE",
        "Hexproof" => "KeywordSet::HEXPROOF",
        "Indestructible" => "KeywordSet::INDESTRUCTIBLE",
        "Lifelink" => "KeywordSet::LIFELINK",
        "Menace" => "KeywordSet::MENACE",
        "Reach" => "KeywordSet::REACH",
        "Trample" => "KeywordSet::TRAMPLE",
        "Vigilance" => "KeywordSet::VIGILANCE",
        "Defender" => "KeywordSet::DEFENDER",
        "Flash" => "KeywordSet::FLASH",
        "Shroud" => "KeywordSet::SHROUD",
        "Shadow" => "KeywordSet::SHADOW",
        "Fear" => "KeywordSet::FEAR",
        "Landwalk:Plains" => "KeywordSet::PLAINSWALK",
        "Landwalk:Island" => "KeywordSet::ISLANDWALK",
        "Landwalk:Swamp" => "KeywordSet::SWAMPWALK",
        "Landwalk:Mountain" => "KeywordSet::MOUNTAINWALK",
        "Landwalk:Forest" => "KeywordSet::FORESTWALK",
        "Prowess" => "KeywordSet::PROWESS",
        "Changeling" => "KeywordSet::CHANGELING",
        "Banding" => "KeywordSet::BANDING",
        _ => return None,
    })
}

/// `etbCounter:<KIND>:<n>` as the `EnterModifier` it is.
///
/// The third of the three things a `K:` line can be, and the one that is no
/// ability at all: "this permanent enters with N counters on it" is a
/// replacement effect (CR 614.1c), which the DSL says on the face rather
/// than in `abilities`. The reference keeps it on a keyword line because it
/// has no ability body to write.
///
/// Two fields are read and everything else refuses the card. The corpus
/// prints 475 of these lines and this rule accepts 297:
///
/// - `n` is a plain number, or an `SVar` that resolves to one.
/// - `n` is `X` **and** the card's own `SVar:X` is `Count$xPaid` — the X
///   announced as the spell was cast (CR 107.3m), which is the one thing
///   `Amount::X` means. Reading every `X` as that one would be a wrong card
///   rather than a missing one: of the 59 scripts whose counter is `X` under
///   an explicit "no Condition", 57 mean a *count* — creatures in a
///   graveyard, colours of mana spent, lands you control — and Sautekh
///   Immortal means "for each creature that died this turn". Each of those
///   would have generated a body that enters with nothing.
/// - A third field is read only as the literal `no Condition`, which is the
///   reference's own way of saying there is none, and a fourth is the
///   reminder text. Anything else there is a condition the DSL cannot say
///   (`CheckSVar$ WasKicked`, `ValidCard$ Card.Self+escaped`, `Adamant`,
///   `Revolt`), so those lines stay honest stubs rather than becoming cards
///   that place the counter unconditionally. Sautekh Immortal is why the
///   field is matched and not merely counted: its third field is the
///   *description*, and a reader that skipped past it would have taken the
///   `X` beside it for the spell's.
///
/// The counter word goes through [`counter_kind`], so a card whose counter
/// the DSL has no id for refuses here exactly as it does on an ability.
/// A colour spelled as the reference writes it in prose (`Exclude$ white`).
pub(super) fn color_word(word: &str) -> Option<&'static str> {
    Some(match word.trim().to_ascii_lowercase().as_str() {
        "white" => "ManaColor::White",
        "blue" => "ManaColor::Blue",
        "black" => "ManaColor::Black",
        "red" => "ManaColor::Red",
        "green" => "ManaColor::Green",
        _ => return None,
    })
}

pub(super) fn keyword_enter_modifier(
    line: &str,
    svars: &BTreeMap<String, String>,
) -> Option<String> {
    let mut fields = line.strip_prefix("etbCounter:")?.split(':');
    let kind = counter_kind(fields.next()?.trim())?;
    let raw = fields.next()?.trim();
    if let Some(condition) = fields.next()
        && !condition.trim().eq_ignore_ascii_case("no condition")
    {
        return None;
    }
    // `true`: an `etbCounter` rides on the permanent's own spell, and a spell
    // announces its `X` — the same number `Amount::X` reads back.
    let amount = amount(raw, svars, true)?;
    Some(format!(
        "EnterModifier::WithCounters {{ kind: {kind}, amount: {amount} }}"
    ))
}

/// Whether a keyword line maps onto a `KeywordSet` bit.
#[must_use]
pub fn keyword_const_of(line: &str) -> Option<&'static str> {
    keyword_const(line)
}

/// The `K:` lines that are a static ability rather than a bit, for a
/// reporter that has to tell the two apart.
#[must_use]
pub fn keyword_static_of(line: &str) -> Option<&'static str> {
    keyword_static(line)
}

/// The `K:` lines that are an as-it-enters modifier on the face rather than
/// anything in `abilities`, for a reporter that has to tell them apart.
///
/// Takes the card's `SVar`s because the line's own amount is not enough to
/// say whether it was read: `etbCounter:P1P1:X` is a card under one `SVar:X`
/// and a refusal under every other.
#[must_use]
pub fn keyword_enter_modifier_of(line: &str, svars: &BTreeMap<String, String>) -> Option<String> {
    keyword_enter_modifier(line, svars)
}
