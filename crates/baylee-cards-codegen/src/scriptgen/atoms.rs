//! The small pieces a script line is built from: amounts, colours, stats,
//! counts, and the atoms and token stems a card's script names.

use super::{BTreeMap, CLUE_TOKEN_SCRIPT, CardScript, PROSE_KEYS, Params, keyword_const};

/// A whole number, or an `SVar` that resolves to one.
pub(super) fn amount(raw: &str, svars: &BTreeMap<String, String>, has_x: bool) -> Option<String> {
    let raw = raw.trim().trim_start_matches('+');
    if let Ok(n) = raw.parse::<i64>() {
        return Some(format!("Amount::Fixed({n})"));
    }
    // `X` is the number the player announced, and `Amount::X` is how the
    // engine reads it back — off the spell for a cast and off
    // `Engine::activation_x` for an activation. Two things have to be true
    // before that is the right reading, and both are checked because
    // neither is visible at the use site.
    //
    // The corpus's `X` is only *sometimes* that number: of the 356 scripts
    // that write `TokenAmount$ X`, 58 define `SVar:X:Count$xPaid` and the
    // rest count something — damage dealt, opponents, creatures in a
    // graveyard. So the definition is demanded rather than assumed.
    //
    // And `has_x` is the other half: a **triggered** ability announces no
    // number, so `Amount::X` would evaluate to `x.unwrap_or(0)` — a card
    // that compiles, claims `Implemented` and makes nothing at all, which
    // is exactly the outcome the honest-stub rule exists to prevent.
    // "For each creature that died this turn" (Scavenging Ghoul), counted as
    // the effect applies. Asked before the `X` rule, because `X` is the
    // letter the reference writes it under.
    if svars.get(raw).map(|def| def.trim())
        == Some("Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature")
    {
        return Some("Amount::CreaturesDiedThisTurn".to_string());
    }
    // "The damage dealt to you this turn" (Simulacrum, Discordant Spirit),
    // under the same letter and asked before it for the same reason.
    if svars.get(raw).map(|def| def.trim()) == Some("PlayerCountPropertyYou$DamageThisTurn") {
        return Some("Amount::DamageDealtToYouThisTurn".to_string());
    }
    if raw == "X" {
        return (has_x && svars.get("X").map(String::as_str) == Some("Count$xPaid"))
            .then(|| "Amount::X".to_string());
    }
    let resolved = svars.get(raw)?;
    let n = resolved.trim().parse::<i64>().ok()?;
    Some(format!("Amount::Fixed({n})"))
}

/// `W` as `ManaColor::White`, for a reader that is not inside a closure.
pub(super) fn mana_color_const(word: &str) -> Option<&'static str> {
    Some(match word {
        "W" => "ManaColor::White",
        "U" => "ManaColor::Blue",
        "B" => "ManaColor::Black",
        "R" => "ManaColor::Red",
        "G" => "ManaColor::Green",
        "C" => "ManaColor::Colorless",
        _ => return None,
    })
}

/// `UR` as `{U/R}`, or `None` when the token is not a hybrid pair.
///
/// The reference runs the two letters together and the printed card puts a
/// slash between them; `ColorPair` keeps the printed order — `{G/U}`, not
/// `{U/G}` — so the pair is written the way it is read.
pub(super) fn hybrid_pair(token: &str) -> Option<String> {
    let mut letters = token.chars();
    let (Some(a), Some(b), None) = (letters.next(), letters.next(), letters.next()) else {
        return None;
    };
    let colored = |c: char| "WUBRG".contains(c);
    (colored(a) && colored(b) && a != b).then(|| format!("{{{a}/{b}}}"))
}

/// A `NumAtt`/`NumDef` value as an `Amount`.
///
/// Separate from [`amount`] because a pump is the one place a *negative*
/// constant is ordinary, and `Amount::Fixed` holds a `u32` — the sign
/// lives in the variant, not in the number. Everything else it asks is the
/// same, and it asked none of it until a batch of six hundred cards walked
/// seven finished pumps into the pool that pump by nothing at all.
pub(super) fn pump_amount(
    raw: &str,
    svars: &BTreeMap<String, String>,
    has_x: bool,
) -> Option<String> {
    let raw = raw.trim();
    // `+X/+X` is the second commonest pump printed, and the two questions
    // [`amount`] asks of an `X` are asked here for the same two reasons.
    //
    // The letter is not the number. 207 scripts in the reference pump by
    // `X`, every one of them defines `SVar:X`, and only **44** define it as
    // `Count$xPaid` — the X the player announced, which `Amount::X` reads
    // straight back off the spell. The other 163 are *counts*:
    // `Count$Domain`, `Count$Valid Artifact.YouCtrl`, the greatest mana
    // value among permanents you control, and the DSL has no way to say any
    // of them. Written as `Amount::X` regardless, Gaea's Might came out
    // claiming `Implemented` and giving +0/+0, and six more with it.
    //
    // And `has_x` is the other half, because a triggered ability announces
    // no number at all: there `Amount::X` is `x.unwrap_or(0)`.
    match raw {
        "X" | "+X" | "-X" => {
            if !has_x || svars.get("X").map(String::as_str) != Some("Count$xPaid") {
                return None;
            }
            return Some(
                if raw == "-X" {
                    "Amount::NegX"
                } else {
                    "Amount::X"
                }
                .to_string(),
            );
        }
        _ => {}
    }
    let n = raw.parse::<i64>().ok().or_else(|| {
        svars
            .get(raw.trim_start_matches('+'))?
            .trim()
            .parse::<i64>()
            .ok()
    })?;
    Some(if n < 0 {
        format!("Amount::NegXFixed({})", n.unsigned_abs())
    } else {
        format!("Amount::Fixed({n})")
    })
}

/// `withFlying` as `Filter::HasKeyword(KeywordSet::FLYING)`, `withoutFlying`
/// as its negation. The reference runs the keyword into the word, spaces
/// removed (`withFirst Strike` is never written; `withFirstStrike` is), so
/// the name is matched against each bit's printed spelling with its spaces
/// taken out.
pub(super) fn keyword_atom(atom: &str) -> Option<String> {
    let (negated, name) = match atom.strip_prefix("without") {
        Some(rest) => (true, rest),
        None => (false, atom.strip_prefix("with")?),
    };
    let printed = [
        "Flying",
        "First Strike",
        "Double Strike",
        "Deathtouch",
        "Haste",
        "Hexproof",
        "Indestructible",
        "Lifelink",
        "Menace",
        "Reach",
        "Trample",
        "Vigilance",
        "Defender",
    ]
    .into_iter()
    .find(|p| p.replace(' ', "") == name)?;
    let has = format!("Filter::HasKeyword({})", keyword_const(printed)?);
    Some(if negated {
        format!("Filter::Not(&{has})")
    } else {
        has
    })
}

/// "Creatures named Plague Rats": the name in a `named…` atom. A name is a
/// characteristic (CR 201.2), compared as the object carries it now. Only a
/// plain name: the reference also writes counts and limits after one
/// (`namedHedron Alignment/LimitMax`).
pub(super) fn named_atom(atom: &str) -> Option<&str> {
    atom.strip_prefix("named").filter(|name| {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || " '-".contains(c))
    })
}

/// A colour word in a valid-string, and whether it is negated: `Black` is
/// `(false, "Black")` and `nonBlack` is `(true, "Black")`, the second half
/// being the `Color` variant's name.
pub(super) fn color_atom(atom: &str) -> Option<(bool, &'static str)> {
    let (negated, word) = atom
        .strip_prefix("non")
        .map_or((false, atom), |rest| (true, rest));
    let color = match word {
        "White" => "White",
        "Blue" => "Blue",
        "Black" => "Black",
        "Red" => "Red",
        "Green" => "Green",
        _ => return None,
    };
    Some((negated, color))
}

/// A power or toughness compared with a fixed number (`powerLE2`,
/// `toughnessGE4`), as the `Filter` that asks it. The strict comparisons
/// become the inclusive ones a step over, because the filters compare
/// inclusively and a power is a whole number.
pub(super) fn stat_atom(atom: &str) -> Option<String> {
    let lower = atom.to_ascii_lowercase();
    let (stat, rest) = if let Some(rest) = lower.strip_prefix("power") {
        ("Power", rest)
    } else {
        ("Toughness", lower.strip_prefix("toughness")?)
    };
    let (cmp, number) = rest.split_at_checked(2)?;
    let n: i16 = number.parse().ok()?;
    let (bound, n) = match cmp {
        "le" => ("AtMost", n),
        "lt" => ("AtMost", n.checked_sub(1)?),
        "ge" => ("AtLeast", n),
        "gt" => ("AtLeast", n.checked_add(1)?),
        _ => return None,
    };
    Some(format!("Filter::{stat}{bound}({n})"))
}

pub(super) fn plain_number(raw: &str, svars: &BTreeMap<String, String>) -> Option<i64> {
    let raw = raw.trim().trim_start_matches('+');
    raw.parse::<i64>()
        .ok()
        .or_else(|| svars.get(raw)?.trim().parse::<i64>().ok())
}

/// A `PresentCompare$` as the bound a count condition says: `GE2` is at least
/// two, `EQ0` none ("if no creatures are on the battlefield"), `LT3` at most
/// two. An exact count above zero is two bounds at once, which one
/// `Condition` cannot say.
pub(super) fn count_bound(compare: &str) -> Option<Bound> {
    let (cmp, number) = compare.split_at_checked(2)?;
    let n: u16 = number.parse().ok()?;
    let fits = |n: u16| u8::try_from(n).is_ok().then_some(n);
    Some(match cmp {
        "GE" => Bound::AtLeast(fits(n)?),
        "GT" => Bound::AtLeast(fits(n.checked_add(1)?)?),
        "LE" => Bound::AtMost(fits(n)?),
        "LT" => Bound::AtMost(fits(n.checked_sub(1)?)?),
        "EQ" if n == 0 => Bound::AtMost(0),
        _ => return None,
    })
}

/// Which side of a count an enters-tapped clause is on, once the reference's
/// comparator has been read.
///
/// The reference states when the land comes down **tapped** and the card
/// prints when it does not, so every comparator [`Tx::enters_tapped_unless`]
/// sees is already the opposite of what a player reads. Naming the two
/// directions keeps that inversion in one place: the arithmetic happens where
/// the comparator is parsed, and the emitter picks its variant from a word
/// instead of re-deriving a direction from a number it was handed.
#[derive(Clone, Copy)]
pub(super) enum Bound {
    /// "unless you control N or more" — a slow land, a battle land, and at
    /// one a checkland.
    AtLeast(u16),
    /// "unless you control N or fewer" — a fast land, and the same predicate
    /// the manlands print as "if you control N+1 or more, it enters tapped".
    AtMost(u16),
}

/// Every token script this card names, by the stem its `TokenScript$` gives.
///
/// Read off the raw text of the rules lines and the `SVar` bodies rather than
/// from a parsed effect, because it is asked **before** the card is
/// transcoded: the ledger has to have given the token an id before a card may
/// name the constant it sits at, and a card is refused for a dozen reasons
/// that have nothing to do with its token. Naming a stem here is therefore
/// not a claim that the card will be read — it is a claim that *this token*
/// is one a card in this pool reaches for.
///
/// Every stem in the corpus is `[A-Za-z0-9_]`, so the scan ends at the first
/// character outside that set and needs no `|` splitting.
#[must_use]
pub fn token_stems(script: &CardScript) -> Vec<String> {
    let mut out = Vec::new();
    let bodies = script
        .rules
        .iter()
        .map(|(_, body)| body.as_str())
        .chain(script.svars.values().map(String::as_str));
    for body in bodies {
        for at in body.match_indices("TokenScript$").map(|(i, _)| i) {
            let rest = body[at + "TokenScript$".len()..].trim_start();
            let stem: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !stem.is_empty() && !out.contains(&stem) {
                out.push(stem);
            }
        }
        // The token a line names by not naming it. `Investigate` writes no
        // `TokenScript$` at all — CR 701.16a supplies which token it is —
        // so a scan for that key alone tells the ledger the card needs no
        // token, while [`Tx::investigate_effect`] is about to emit the
        // constant the ledger was never asked to assign. The emitter and
        // the ledger have to name the same tokens or one of them is wrong,
        // and today it survives only because another card happens to name
        // the Clue outright.
        //
        // Read through [`Params::parse`] and never for the word: every one
        // of these lines also *prints* it, and `SpellDescription$
        // Investigate. (Create a Clue token…)` is on 21 of the corpus's
        // 112.
        if Params::parse(body).is_some_and(|(api, _)| api == "Investigate")
            && !out.iter().any(|stem| stem == CLUE_TOKEN_SCRIPT)
        {
            out.push(CLUE_TOKEN_SCRIPT.to_string());
        }
    }
    out
}

/// Every distinct mechanic a script touches, as flat strings.
///
/// This is the unit a coverage plan is built out of: `api:Token`,
/// `param:Pump.Duration`, `kw:Equip`, `line:S`. It is deliberately *not*
/// a list of what the transcoder refused — it names what the script
/// *uses*, so that atoms already appearing in scripts the transcoder reads
/// in full can be subtracted as known. That subtraction is what keeps the
/// plan honest without restating each rule's parameter list here, where
/// the copy would rot the first time a rule learned a new key.
#[must_use]
pub fn atoms(script: &CardScript) -> Vec<String> {
    let mut out = Vec::new();
    for line in &script.keywords {
        let head = line.split(':').next().unwrap_or(line);
        let head = head.split(' ').next().unwrap_or(head);
        out.push(format!("kw:{head}"));
    }
    for line in &script.unknown_lines {
        let head = line.split(':').next().unwrap_or(line);
        out.push(format!("line:{head}"));
    }
    for (kind, spec) in &script.rules {
        if matches!(kind, 'S' | 'R') {
            out.push(format!("line:{kind}"));
        }
        let mut queue = vec![spec.clone()];
        let mut seen = 0usize;
        while let Some(spec) = queue.pop() {
            seen += 1;
            if seen > 32 {
                break; // a malformed chain must not spin here
            }
            let Some((api, params)) = Params::parse(&spec) else {
                continue;
            };
            out.push(format!("api:{api}"));
            for (key, _) in &params.entries {
                if !PROSE_KEYS.contains(&key.as_str()) {
                    out.push(format!("param:{api}.{key}"));
                }
            }
            for part in spec.split(" | ") {
                for prefix in ["SubAbility$ ", "Execute$ "] {
                    if let Some(name) = part.strip_prefix(prefix)
                        && let Some(body) = script.svars.get(name.trim())
                    {
                        queue.push(body.clone());
                    }
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}
