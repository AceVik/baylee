//! What the reader supports, and why it refuses a script it does not.

use super::{BTreeMap, CardBody, CardScript, Params, SubtypeCatalogs, TokenLookup, Tx, cost_parts};

/// Whether the script says nothing beyond what Scryfall supplies anyway: no
/// keyword, no ability and no line kind the parser does not model.
///
/// [`transcode`] refuses such a script, because an empty body is also what
/// a card it could not read at all would look like. A vanilla creature is
/// the other reading, and only the printing can tell the two apart, so
/// `stubgen` finishes the card when its printed text is empty as well; the
/// reports that walk scripts without a printing (`reach-list`,
/// `transcode-report`) count it as read on this alone, and `codegen` decides.
#[must_use]
pub fn is_vanilla(script: &CardScript) -> bool {
    script.keywords.is_empty() && script.rules.is_empty() && script.unknown_lines.is_empty()
}

/// Reads a script and, when it is refused over a parameter, names it.
///
/// `None` means the transcoder has nothing to say about this script: it was
/// read in full, or refused somewhere that records no reason.
///
/// The transcoder reports this itself rather than a second table listing
/// each rule's keys: such a list would rot the first time a rule learned a
/// new one, and a stale worklist is worse than none.
#[must_use]
pub fn refusal_reason(
    script: &CardScript,
    cats: &SubtypeCatalogs,
    tokens: Option<&TokenLookup>,
) -> Option<String> {
    if !script.unknown_lines.is_empty() {
        return None;
    }
    let mut tx = Tx {
        svars: &script.svars,
        cats,
        tokens,
        has_x: false,
        on_a_spell: false,
        trigger_mode: None,
        block_line: None,
        block_half: None,
        in_delayed: false,
        body: CardBody::default(),
        unclaimed: std::cell::RefCell::new(None),
    };
    for line in &script.keywords {
        if tx.keyword(line).is_none() {
            return tx.unclaimed.into_inner();
        }
    }
    for (kind, spec) in &script.rules {
        if tx.rule(*kind, spec).is_none() {
            return tx.unclaimed.into_inner();
        }
    }
    if tx.block_halves_paired().is_none() {
        return tx.unclaimed.into_inner();
    }
    // [`transcode`]'s last refusal, mirrored. A script that is read in full
    // and yields nothing is a card whose rules text this transcoder has no
    // rule for at all — usually a vanilla body, and never a defect — but it
    // is a *reason*, and leaving it out filed every one of them under "no
    // reason recorded".
    if tx.body.is_empty() {
        return Some("a script that reads as an empty card".to_string());
    }
    None
}

/// The reference's name for the Clue token, which CR 701.16a makes the whole
/// definition of investigating.
///
/// Named here rather than written into [`Scriptgen::investigate_effect`]
/// because it is a fact about the *corpus* and not about the rule: the rule
/// says "a Clue token" and this is the file that happens to hold one. It
/// goes through the same `TokenLookup` every `TokenScript$` goes through, so
/// a run with no token corpus refuses investigating for the same stated
/// reason it refuses every other token, instead of emitting a constant that
/// the ledger may not have assigned.
pub(super) const CLUE_TOKEN_SCRIPT: &str = "c_a_clue_draw";

/// The effect APIs [`transcode`] knows how to write.
///
/// Kept beside the match in [`Tx::chain`] so a report of what the corpus
/// still needs cannot drift from what the transcoder actually reads.
pub const SUPPORTED_APIS: &[&str] = &[
    "DealDamage",
    "GainLife",
    "LoseLife",
    "Draw",
    "Discard",
    "Mill",
    "Scry",
    "Surveil",
    "Mana",
    "Destroy",
    "DestroyAll",
    "DelayedTrigger",
    "DamageAll",
    "DamageResolve",
    "PreventDamage",
    "AddTurn",
    "Effect",
    "ChooseSource",
    "Fog",
    "Regenerate",
    "Tap",
    "TapOrUntap",
    "TapAll",
    "DrainMana",
    "Untap",
    "Counter",
    "PutCounter",
    "Sacrifice",
    "Pump",
    "ChangeZone",
    "ChangeZoneAll",
    "RearrangeTopOfLibrary",
    "Token",
    "Investigate",
];

/// A cost token that names an object, split into its kind and its body.
///
/// `Sac<1/CARDNAME…>` is deliberately not among them: it is matched one
/// branch earlier as `SacrificeSelf`, which asks nobody anything.
pub(super) fn object_cost(token: &str) -> Option<(&'static str, &str)> {
    for kind in ["Sac", "Discard", "tapXType", "Return", "ExileFromGrave"] {
        if let Some(body) = token
            .strip_prefix(kind)
            .and_then(|t| t.strip_prefix('<'))
            .and_then(|t| t.strip_suffix('>'))
        {
            return Some((kind, body));
        }
    }
    None
}

/// Whether a cost part is paid by naming an object.
///
/// The five `cost_wizard` puts a list up for, spelled as the emitter writes
/// them rather than as the engine matches them, because this side has a
/// string and not a `CostPart`. `SacrificeSelf` and `ReturnSelfToHand` are
/// deliberately not among them: they name the source and ask nothing.
pub(super) fn asks_for_an_object(part: &str) -> bool {
    [
        "Sacrifice(",
        "Discard(",
        "TapOther(",
        "ReturnToHand(",
        "ExileFromGraveyard(",
    ]
    .iter()
    .any(|kind| part.starts_with(kind))
}

/// `{N}` alone as its number: a price that is generic mana and nothing else.
pub(super) fn generic_mana(mana: &str) -> Option<u16> {
    mana.strip_prefix('{')?.strip_suffix('}')?.parse().ok()
}

/// A cost of mana symbols only (`W W`, `1 U`) in its printed spelling
/// (`{W}{W}`, `{1}{U}`); `None` when any part is something other than mana.
pub(super) fn printed_mana(cost: &str) -> Option<String> {
    let mut out = String::new();
    for part in cost_parts(cost) {
        let mana = part.chars().all(|c| c.is_ascii_digit())
            || matches!(part.as_str(), "W" | "U" | "B" | "R" | "G" | "C");
        if !mana || part.is_empty() {
            return None;
        }
        out.push('{');
        out.push_str(&part);
        out.push('}');
    }
    (!out.is_empty()).then_some(out)
}

/// Whether [`transcode`] has a rule for this effect API.
#[must_use]
pub fn is_supported_api(api: &str) -> bool {
    SUPPORTED_APIS.contains(&api)
}

/// Every effect API a rules line reaches, following `SubAbility$` chains.
#[must_use]
pub fn apis_used(spec: &str, svars: &BTreeMap<String, String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut queue = vec![spec.to_string()];
    let mut seen = 0usize;
    while let Some(spec) = queue.pop() {
        seen += 1;
        if seen > 32 {
            break; // a malformed chain must not spin here
        }
        let Some((api, _)) = Params::parse(&spec) else {
            continue;
        };
        for part in spec.split(" | ") {
            if let Some(name) = part.strip_prefix("SubAbility$ ")
                && let Some(body) = svars.get(name.trim())
            {
                queue.push(body.clone());
            }
            if let Some(name) = part.strip_prefix("Execute$ ")
                && let Some(body) = svars.get(name.trim())
            {
                queue.push(body.clone());
            }
        }
        if !matches!(
            api.as_str(),
            "ChangesZone" | "Phase" | "Attacks" | "Taps" | "AttackerBlockedByCreature"
        ) {
            out.push(api);
        }
    }
    out
}
