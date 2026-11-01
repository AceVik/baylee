//! Per-card source rendering.

use super::{
    CardBody, CodegenError, HELD_MARKER, IndexLedger, LedgerEntry, OWNED_MARKER, STUB_MARKER,
    ScryfallCard, StubInfo, SubtypeCatalogs, doc_lines, faces_of, render_card_literal, render_face,
    set_line, slug, transcode_card, vanilla,
};

/// Renders one stub file, and says where under `cards/` it belongs.
///
/// `tested` answers whether a card test exists at a path relative to
/// `cards/` (the same relative path under `card_tests/`); it decides
/// between `Implemented` and `Generated` for a card read in full.
///
/// # Errors
/// [`CodegenError::Mana`] when a mana cost fails validation.
// One card written out end to end, with the reason for each field beside it.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub fn render_stub(
    card: &ScryfallCard,
    row: &LedgerEntry,
    ledger: &IndexLedger,
    cats: &SubtypeCatalogs,
    scripts: Option<&crate::scriptgen::ScriptLookup>,
    tokens: Option<&crate::tokengen::TokenLookup>,
    cycles: &crate::layout::LandCycles,
    tested: &dyn Fn(&str) -> bool,
) -> Result<(StubInfo, String), CodegenError> {
    // Multi-face cards slug by their front face ("Brightclimb Pathway // …"
    // → "brightclimb_pathway").
    let faces = faces_of(card);
    // A card is written out finished only when a reader understood the whole
    // of it; one clause left over and it stays an ordinary stub. Its own
    // printed text is tried first, because a land's intrinsic mana comes from
    // its type line (CR 305.6) and no reference script restates it.
    let land = crate::landgen::recognize(card, cats)
        .or_else(|| {
            let script = scripts?.script(&card.name)?;
            transcode_card(&script, &faces[0].type_line, cats, tokens)
        })
        .or_else(|| vanilla(&faces, &scripts?.script(&card.name)?));

    render_card(card, row, ledger, cats, cycles, land.as_ref(), tested, None)
}

/// Render an explicitly reviewed card as an unimplemented stub, bypassing
/// both ability readers even for lands they would ordinarily finish.
///
/// `reason` is recorded in the generated source, one comment per line.
/// No ability, keyword or test-derived coverage is retained.
///
/// # Errors
/// [`CodegenError::Mana`] when a printed mana cost fails validation.
pub fn render_unimplemented(
    card: &ScryfallCard,
    row: &LedgerEntry,
    ledger: &IndexLedger,
    cats: &SubtypeCatalogs,
    cycles: &crate::layout::LandCycles,
    reason: &str,
) -> Result<(StubInfo, String), CodegenError> {
    render_card(
        card,
        row,
        ledger,
        cats,
        cycles,
        None,
        &|_| false,
        Some(reason),
    )
}

// Shared canonical rendering for ordinary generation and explicit review holds.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
fn render_card(
    card: &ScryfallCard,
    row: &LedgerEntry,
    ledger: &IndexLedger,
    cats: &SubtypeCatalogs,
    cycles: &crate::layout::LandCycles,
    land: Option<&CardBody>,
    tested: &dyn Fn(&str) -> bool,
    held: Option<&str>,
) -> Result<(StubInfo, String), CodegenError> {
    let slug = slug(card.name.split(" // ").next().unwrap_or(&card.name));
    let faces = faces_of(card);
    let oracle_id = card.oracle_id.clone().unwrap_or_default();
    let path = crate::layout::path_for(card, &slug, cycles);
    let mut out = String::with_capacity(4096);
    // Human-verifiable header (docs/card-dsl.md).
    out.push_str(&format!("//! {} \u{2014} ", card.name));
    // The front face's cost, not the card's: a modal double-faced card
    // carries no top-level `mana_cost`, so reading it there printed
    // "(no cost)" over a `FaceDef` that plainly had one — which `xtask
    // validate` compares, and which stayed invisible while the pool held no
    // two-faced cards.
    let front_cost = faces.first().map_or("", |f| f.mana_cost.as_str());
    out.push_str(if front_cost.is_empty() {
        "(no cost)"
    } else {
        front_cost
    });
    out.push_str(&format!(
        " \u{2014} {}\n",
        card.type_line.as_deref().unwrap_or("")
    ));
    // Every face's printed text, not the card's. Scryfall carries no
    // top-level `oracle_text` for a double-faced card — it is per face —
    // so reading it there left the Oracle header of all 96 two-faced files
    // *empty*, which is the human-verification surface missing on exactly
    // the cards that are hardest to check by eye. The faces are joined in
    // printed order, which is the order `xtask validate` compares against.
    let oracle = faces
        .iter()
        .map(|f| f.oracle_text.trim())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    doc_lines(&mut out, "Oracle: ", &oracle);
    out.push_str(&set_line(
        card.set.as_deref().unwrap_or("?"),
        card.collector_number.as_deref().unwrap_or("?"),
        card.set_name.as_deref().unwrap_or("?"),
        &card.id,
        &oracle_id,
    ));
    if faces.len() > 1 {
        for f in &faces {
            out.push_str(&format!(
                "//! Face: {} \u{2014} {} \u{2014} {}\n",
                f.name, f.mana_cost, f.type_line
            ));
        }
    }
    if let Some(body) = &land {
        let mut notes = body.notes.clone();
        notes.dedup();
        out.push_str(&format!(
            "{OWNED_MARKER}: {}.\n\n",
            if notes.is_empty() {
                "printed abilities".to_string()
            } else {
                notes.join(", ")
            }
        ));
    } else {
        out.push_str(&format!(
            "{STUB_MARKER} \u{2014} implement abilities + tests, see docs/card-dsl.md.\n\n"
        ));
    }
    if let Some(reason) = held {
        for line in reason.lines() {
            out.push_str(&format!("{HELD_MARKER}{line}\n"));
        }
        out.push('\n');
    }
    // One import, not six. The old list was the same for every card whether
    // or not the card used it, which is why every stub also had to carry
    // `#![allow(unused_imports)]` — an allow that then hid two dozen genuinely
    // dead imports in hand-finished files for as long as it existed.
    out.push_str("use baylee_cards_dsl::prelude::*;\n");

    let mut face_defs = String::new();
    for (i, f) in faces.iter().enumerate() {
        // Enter modifiers are printed on the front face, which is where both
        // readers mean them: a recognised land is single-faced by
        // construction, and a script's `K:etbCounter` is said about the card
        // rather than about one of its sides — a two-faced one is refused
        // earlier anyway, over the `AlternateMode:` line its reference script
        // opens with.
        let enters = match (&land, i) {
            (Some(body), 0) => body.enter_modifiers.as_slice(),
            _ => &[],
        };
        let flashback = match (&land, i) {
            (Some(body), 0) => body.flashback.as_deref(),
            _ => None,
        };
        let foretell = match (&land, i) {
            (Some(body), 0) => body.foretell.as_deref(),
            _ => None,
        };
        let cost_reduction = match (&land, i) {
            (Some(body), 0) => body.cost_reduction.as_deref(),
            _ => None,
        };
        let entwine = match (&land, i) {
            (Some(body), 0) => body.entwine.as_deref(),
            _ => None,
        };
        face_defs.push_str(&render_face(
            &card.name,
            f,
            cats,
            enters,
            flashback,
            foretell,
            cost_reduction,
            entwine,
            i > 0,
            card.layout.as_deref(),
        )?);
    }
    let literal = render_card_literal(
        card,
        &row.constant,
        &oracle_id,
        &faces,
        &face_defs,
        land,
        ledger,
        land.is_some() && tested(&path),
    )?;
    let statics = land.as_ref().map_or("", |b| b.statics.as_str());
    // Only when something names one: an unused import is a warning now that
    // the stub no longer carries a blanket `allow`. The card literal counts
    // too — `Modifier::AddSubtype` puts a subtype inside an ability, where
    // neither the faces nor the hoisted filters would see it.
    if face_defs.contains("subtypes::")
        || statics.contains("subtypes::")
        || literal.contains("subtypes::")
    {
        out.push_str("use baylee_core::generated::subtypes;\n");
    }
    // The token ledger, on the same terms and for the same reason: a card
    // that creates one names the constant it was filed under, and the module
    // is the one door to both halves of the table.
    if statics.contains("generated_tokens::") || literal.contains("generated_tokens::") {
        out.push_str("use crate::generated_tokens;\n");
    }
    out.push('\n');
    out.push_str(statics);
    out.push_str(&literal);
    // No test module. A card file is data, and a test that reads the literal
    // it sits under proves nothing — every one of the 194 hand-written cards
    // left the module empty, and the generated cards that filled it produced
    // assertions like `assert_eq!(*filter, BASIC_LAND_FILTER)`, which compares
    // the filter with itself. Behaviour is tested in baylee-engine, and a rule
    // that must hold for *every* card belongs in the cross-cutting tests at
    // the foot of baylee-cards/src/lib.rs.
    if land.is_none() {
        out.push_str("// TODO(card): implement abilities, see docs/card-dsl.md.\n");
    }

    Ok((
        StubInfo {
            path,
            slug,
            oracle_id,
            index: row.index,
        },
        out,
    ))
}
