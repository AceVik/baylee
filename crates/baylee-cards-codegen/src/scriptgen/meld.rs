//! Lowering Forge `MeldPair:` / `AlternateMode:Meld` cards and `AB$ Meld` /
//! `DB$ Meld` effects (`CR 701.42`).

use super::{CardScript, Params, SubtypeCatalogs, TokenLookup, Tx, card_type_const, color_word};
use std::fmt::Write as _;

/// Name of the file-local melded-back `TokenDef` static emitted above a
/// primary meld card's `CardDef`.
pub(super) const MELDED_STATIC: &str = "MELDED_FACE";

/// Renders `static MELDED_FACE: TokenDef = TokenDef { ... };` (plus any helper
/// statics created while transcoding the melded back face's rules). Returns
/// `None` if any ability or header on the melded face cannot be transcoded.
pub(super) fn build_melded_token_static(
    melded: &CardScript,
    cats: &SubtypeCatalogs,
    tokens: Option<&TokenLookup>,
) -> Option<String> {
    let body = super::transcode_face(melded, cats, tokens, String::new(), true, None)?;
    if melded.script_name.trim().is_empty() || has_unsupported_melded_fields(&body) {
        return None;
    }

    let mut type_bits: Vec<&'static str> = Vec::new();
    let mut super_bits: Vec<&'static str> = Vec::new();
    let mut subtypes: Vec<String> = Vec::new();
    for word in melded.type_line.split_whitespace() {
        if let Some(t) = card_type_const(word) {
            type_bits.push(t);
        } else if let Some(s) = supertype_const(word) {
            super_bits.push(s);
        } else {
            let sub = cats.const_path(word)?;
            subtypes.push(sub);
        }
    }
    if type_bits.is_empty() {
        return None;
    }
    let types_expr = join_const_union(&type_bits, "TypeSet::EMPTY");
    let supers_expr = join_const_union(&super_bits, "SupertypeSet::EMPTY");

    let mut color_bits: Vec<&'static str> = Vec::new();
    if let Some(raw_colors) = melded.colors.as_deref() {
        for word in raw_colors
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            color_bits.push(color_word(word)?);
        }
    }
    let colors_expr = if color_bits.is_empty() {
        "ColorSet::EMPTY".to_string()
    } else {
        format!("ColorSet::from_slice(&[{}])", color_bits.join(", "))
    };

    let (power, toughness) = match melded.pt.as_deref() {
        None => ("None".to_string(), "None".to_string()),
        Some(raw) => {
            let (p, t) = raw.split_once('/')?;
            (parse_pt_field(p)?, parse_pt_field(t)?)
        }
    };
    let loyalty = match melded.loyalty.as_deref() {
        None => "None".to_string(),
        Some(raw) => format!("Some({})", raw.trim().parse::<u16>().ok()?),
    };
    let keywords_expr = if body.keywords.is_empty() {
        "KeywordSet::EMPTY".to_string()
    } else {
        let mut expr = body.keywords[0].clone();
        for k in &body.keywords[1..] {
            expr = format!("{expr}.union({k})");
        }
        expr
    };

    let mut out = body.statics;
    let _ = writeln!(out, "static {MELDED_STATIC}: TokenDef = TokenDef {{");
    let _ = writeln!(out, "    name: {:?},", melded.script_name.trim());
    if colors_expr != "ColorSet::EMPTY" {
        let _ = writeln!(out, "    colors: {colors_expr},");
    }
    let _ = writeln!(out, "    types: {types_expr},");
    if supers_expr != "SupertypeSet::EMPTY" {
        let _ = writeln!(out, "    supertypes: {supers_expr},");
    }
    if !subtypes.is_empty() {
        let _ = writeln!(out, "    subtypes: &[{}],", subtypes.join(", "));
    }
    if power != "None" {
        let _ = writeln!(out, "    power: {power},");
    }
    if toughness != "None" {
        let _ = writeln!(out, "    toughness: {toughness},");
    }
    if loyalty != "None" {
        let _ = writeln!(out, "    loyalty: {loyalty},");
    }
    if keywords_expr != "KeywordSet::EMPTY" {
        let _ = writeln!(out, "    keywords: {keywords_expr},");
    }
    if !body.abilities.is_empty() {
        let _ = writeln!(out, "    abilities: &[");
        for ab in &body.abilities {
            let _ = writeln!(out, "        {ab},");
        }
        let _ = writeln!(out, "    ],");
    }
    let _ = writeln!(out, "    ..TokenDef::DEFAULT");
    let _ = writeln!(
        out,
        "}};
"
    );
    Some(out)
}

impl Tx<'_> {
    /// Lowers `AB$ Meld` / `DB$ Meld` into `Effect::MeldPair`.
    pub(super) fn lower_meld(&mut self, p: &mut Params) -> Option<Vec<String>> {
        let (expected_melded_name, melded_static) = self.melded_face.clone()?;
        let primary = p.take("Primary")?;
        let secondary = p.take("Secondary")?;
        if let Some(name) = p.take("Name")
            && name.trim() != expected_melded_name
        {
            return None;
        }
        let partner_types = match p.take("SecondaryType") {
            None => "TypeSet::CREATURE".to_string(),
            Some(raw) => {
                let mut bits: Vec<&'static str> = Vec::new();
                for word in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                    bits.push(card_type_const(word)?);
                }
                if bits.is_empty() {
                    return None;
                }
                join_const_union(&bits, "TypeSet::CREATURE")
            }
        };
        let enters_tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let enters_attacking = match p.take("Attacking").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let mut require_attacking = false;
        if let Some(cond) = p.take("ConditionPresent") {
            match cond.trim() {
                "Card.Self+YouCtrl+YouOwn" => {}
                "Card.Self+YouCtrl+YouOwn+attacking" => require_attacking = true,
                _ => return None,
            }
        }
        if let Some(svar_name) = p.take("ConditionCheckSVar") {
            let expr = self.svars.get(svar_name.trim())?;
            let valid = expr.strip_prefix("Count$Valid ")?;
            if !valid.contains("YouCtrl")
                || !valid.contains("YouOwn")
                || !valid.contains(&format!("named{secondary}"))
            {
                return None;
            }
            if valid.contains("attacking") {
                require_attacking = true;
            }
        }
        Some(vec![format!(
            "Effect::MeldPair {{ source_name: {primary:?}, partner_name: {secondary:?}, partner_types: {partner_types}, require_attacking: {require_attacking}, enters_tapped: {enters_tapped}, enters_attacking: {enters_attacking}, melded: &{melded_static} }}"
        )])
    }
}

fn supertype_const(word: &str) -> Option<&'static str> {
    match word {
        "Legendary" => Some("SupertypeSet::LEGENDARY"),
        "Basic" => Some("SupertypeSet::BASIC"),
        "Snow" => Some("SupertypeSet::SNOW"),
        "World" => Some("SupertypeSet::WORLD"),
        _ => None,
    }
}

fn parse_pt_field(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if let Ok(v) = trimmed.parse::<i16>() {
        return Some(format!("Some({v})"));
    }
    if trimmed == "*" {
        return Some("Some(0)".to_string());
    }
    None
}

fn join_const_union(bits: &[&str], empty: &str) -> String {
    if bits.is_empty() {
        return empty.to_string();
    }
    let mut expr = bits[0].to_string();
    for bit in &bits[1..] {
        expr = format!("{expr}.union({bit})");
    }
    expr
}

/// Guard against silently dropping any non-`TokenDef` face fields when
/// lowering a melded back face (`CardBody` is destructured exhaustively so new
/// fields must be classified here).
#[allow(clippy::too_many_lines)] // Exhaustive CardBody destructuring pins all fields against silent drops.
fn has_unsupported_melded_fields(body: &crate::body::CardBody) -> bool {
    let crate::body::CardBody {
        statics: _,
        keywords: _,
        abilities: _,
        notes: _,
        enter_modifiers,
        alternative_costs,
        mandatory_additional_costs,
        convoke,
        delve,
        improvise,
        replicate,
        disguise,
        flashback,
        flashback_parts,
        madness,
        foretell,
        miracle,
        dash,
        blitz,
        morph,
        megamorph,
        morph_parts,
        bestow,
        prototype,
        bargain,
        emerge,
        jump_start,
        aftermath,
        disturb,
        escape,
        squad,
        offspring,
        fuse,
        behold_or_pay,
        mutate,
        plot,
        casualty,
        impending,
        gift,
        warp,
        mayhem,
        optional_cost,
        optional_cost_grants_flash,
        waterbend,
        cost_reduction,
        entwine,
        buyback,
        multikicker,
        additional_costs,
        kicked_targets,
        back,
        back_is_adventure,
        back_is_flip,
        back_is_omen,
        back_is_prepare,
        meld_partner,
        has_partner,
    } = body;
    let vecs = [
        enter_modifiers,
        alternative_costs,
        mandatory_additional_costs,
        flashback_parts,
        morph_parts,
        additional_costs,
    ];
    let flags = [
        *convoke,
        *delve,
        *improvise,
        *bargain,
        *jump_start,
        *aftermath,
        *fuse,
        *optional_cost_grants_flash,
        *waterbend,
        *back_is_adventure,
        *back_is_flip,
        *back_is_omen,
        *back_is_prepare,
        *has_partner,
        back.is_some(),
    ];
    let opts = [
        replicate,
        disguise,
        flashback,
        madness,
        foretell,
        miracle,
        dash,
        blitz,
        morph,
        megamorph,
        bestow,
        prototype,
        emerge,
        disturb,
        escape,
        squad,
        offspring,
        behold_or_pay,
        mutate,
        plot,
        casualty,
        impending,
        gift,
        warp,
        mayhem,
        optional_cost,
        cost_reduction,
        entwine,
        buyback,
        multikicker,
        kicked_targets,
        meld_partner,
    ];
    vecs.iter().any(|v| !v.is_empty())
        || flags.iter().any(|&f| f)
        || opts.iter().any(|o| o.is_some())
}
