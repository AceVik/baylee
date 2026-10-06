//! Rendering a card's abilities back into sentences, for the ambiguity
//! sweep and the oracle check.

use crate::{BTreeMap, Path, acceptance, cached_printing, face_texts, fs, lines};

/// One ability in `AMBIGUITY_ONE_IN` may fit two printed sentences equally
/// well before [`render_ability_lines`] refuses to write the table.
///
/// Measured on 19.09.2026 over the 2716-card pool: **67 of 2532** abilities
/// walked, which is one in 37.8. The bound is set at one in 25, and the
/// gap is deliberate — a batch of cards adds abilities this reader has
/// never seen, so a bound with no headroom would fire on growth rather
/// than on a regression. What it is there to catch is the reader getting
/// *worse*, which is a jump and not a drift.
///
/// It is a **ratio** rather than a count for the same reason: the count
/// grows with the pool, so a ceiling spelled as a count would have to be
/// raised after every batch, and a ceiling that is routinely raised is not
/// one. What this bounds is the reader — how often, per ability walked, it
/// could not tell two sentences apart — and that is a property the pool
/// growing does not change.
pub(crate) const AMBIGUITY_ONE_IN: usize = 25;

/// How few abilities the walk may reach before its ambiguity ratio means
/// nothing. A ratio over an empty population is satisfied by anything, and
/// the cache this walk reads is **not in the repository**, so "no printings
/// found" is a thing that happens rather than a thing that cannot. Set well
/// under the 2532 measured, because this is the difference between a
/// reading and no reading at all rather than a second ceiling.
pub(crate) const ABILITIES_WALKED_FLOOR: usize = 2000;

/// Renders `crates/baylee-cards/src/generated_lines.rs`: per card, per
/// face, which printed sentence each ability came from.
///
/// The answer has to be precomputed, and it has to be precomputed *here*.
/// It comes from reading the English oracle text against the compiled
/// ability list, and a running game holds neither — the engine carries no
/// card text at all, and a client fetches the printing and language the
/// player chose rather than Scryfall's English. `xtask` is the one place
/// that links both halves: `baylee-cards` for the compiled `CardDef`s and
/// `baylee-cards-codegen` for the reader, which is the same shape
/// `validate` already has.
///
/// # It is two-phase, and that is what `--check` is for
///
/// The table is built from the pool **compiled into this binary**, so a
/// card added by the run that is writing it is not in the walk: its row
/// lands on the *next* `codegen`, after a rebuild. That is not a race to
/// be fixed here — it is the same order `validate` reads the pool in — and
/// `codegen --check` in CI is what turns a forgotten second run into a
/// build failure rather than a card whose stack entry silently says
/// nothing.
pub(crate) fn render_ability_lines(root: &Path) -> anyhow::Result<String> {
    use baylee_cards::lines::face_modes;

    let decks_text = fs::read_to_string(root.join("data/acceptance-decks.txt"))?;
    let rows = acceptance::parse_decks(&decks_text)?;
    let pool_text = fs::read_to_string(root.join("data/card-pool.txt")).unwrap_or_default();
    // The pool's *own* spelling of a name, because that is what the
    // printing is cached under: a double-faced card is `A // B` there and
    // its front face alone in `CardDef::name`.
    let names = acceptance::all_names(&rows, &pool_text);
    let by_name: BTreeMap<&str, &'static baylee_cards::dsl::CardDef> =
        baylee_cards::all().map(|def| (def.name(), def)).collect();

    let mut table: Vec<String> =
        vec![String::from("&[],"); baylee_cards::generated::BY_INDEX.len()];
    // `Mapping::ambiguous` is the one number in this walk that says the
    // *reader* needs work rather than the pool — an ability that fitted two
    // sentences equally well, where whichever was taken is a coin toss
    // drawn to the player as the card's own words. It was computed on
    // every card and read by nobody, which is a green run with a defect in
    // it. Summed here and bounded below.
    let mut ambiguous = 0usize;
    let mut walked = 0usize;
    for name in &names {
        let Some(def) = by_name.get(name.split(" // ").next().unwrap_or(name)) else {
            continue;
        };
        let Some(payload) = cached_printing(root, name) else {
            continue;
        };
        let texts = face_texts(&payload);
        let mut faces: Vec<String> = Vec::new();
        let mut any = false;
        for face in 0..def.faces.len() {
            let abilities = def.abilities_for_face(face);
            let modes = face_modes(abilities);
            let alternatives = def.faces[face].alternative_costs;
            // An alternative cost is not an ability, so a face that prints
            // one and nothing else — none today — would be dropped by the
            // emptiness test below if it asked about abilities alone. The
            // modes need no such clause: they come out of `abilities`, so
            // a face that has any has an ability too.
            any |= !abilities.is_empty() || !alternatives.is_empty();
            let printed = texts.get(face).map_or("", String::as_str);
            let mapping = lines::map(abilities, printed);
            ambiguous += mapping.ambiguous;
            walked += abilities.len();
            let stackable = abilities
                .iter()
                .map(lines::ability_shape)
                .filter(|shape| shape.stackable())
                .count();
            let cell = |line: &Option<u8>| {
                line.map_or_else(|| "None".to_string(), |l| format!("Some({l})"))
            };
            let cells: Vec<String> = mapping.lines.iter().map(cell).collect();
            let mode_cells: Vec<String> =
                lines::map_modes(modes, printed).iter().map(cell).collect();
            let alt_cells: Vec<String> = lines::map_alternatives(alternatives, printed)
                .iter()
                .map(cell)
                .collect();
            faces.push(format!(
                "FaceLines {{ sentences: {}, stackable: {}, lines: &[{}], \
                 modes: &[{}], alternatives: &[{}] }}",
                baylee_core::oracle::sentence_count(printed),
                stackable,
                cells.join(", "),
                mode_cells.join(", "),
                alt_cells.join(", ")
            ));
        }
        // A card with no abilities on any face — a vanilla creature, a
        // stub — has nothing to point at, and an empty row keeps the file
        // to the cards the feature is about.
        if !any {
            continue;
        }
        let Some(slot) = table.get_mut(def.index.get() as usize) else {
            continue;
        };
        *slot = format!("// {}\n&[{}],", def.name(), faces.join(", "));
    }

    // A ratio and not a count, because the count grows with the pool and a
    // ceiling that has to be edited after every batch of cards is a ceiling
    // nobody reads. What it bounds is the *reader*: how often, per ability
    // walked, it could not tell two sentences apart.
    println!("note: {ambiguous} of {walked} abilities fit more than one sentence");
    anyhow::ensure!(
        walked >= ABILITIES_WALKED_FLOOR,
        "the walk reached only {walked} abilities, so the ambiguity ratio \
         below says nothing; is `data/scryfall-cache` filled?"
    );
    anyhow::ensure!(
        ambiguous * AMBIGUITY_ONE_IN <= walked,
        "{ambiguous} of {walked} abilities fit more than one printed sentence \
         equally well, which is worse than 1 in {AMBIGUITY_ONE_IN}; whichever \
         sentence each of them was given is a coin toss drawn to a player as \
         the card's own words"
    );

    // The two-phase gap, said out loud where it happens rather than found
    // in CI a day later: cards this run added to the registry are not in
    // the pool this binary compiled against, so they have no row yet.
    let added = names
        .iter()
        .filter(|name| !by_name.contains_key(name.split(" // ").next().unwrap_or(name)))
        .count();
    if added > 0 {
        println!(
            "note: {added} card(s) are newer than the compiled pool; \
             run `cargo xtask codegen` again for their ability lines"
        );
    }

    Ok(format!(
        "// GENERATED by `cargo xtask codegen` — do not edit by hand.\n\
         \n\
         #![allow(missing_docs, unused_imports, dead_code, clippy::all, clippy::pedantic)]\n\
         \n\
         use crate::lines::FaceLines;\n\
         \n\
         /// Per card by `CardIndex`, per face, in `abilities_for_face` order:\n\
         /// which printed sentence each ability came from. `&[]` is a card with\n\
         /// no abilities, a retired index, or one with no cached printing.\n\
         pub static ABILITY_LINES: &[&[FaceLines]] = &[\n{}\n];\n",
        table.join("\n")
    ))
}

/// How many pool cards [`render_oracle`] must find a cached printing for.
///
/// 2716 cards on 24.09.2026, every one of them cached; the floor sits below
/// that for the reason [`ABILITIES_WALKED_FLOOR`] does — it separates a
/// reading from a cache that is mostly empty, not one card from the next.
pub(crate) const ORACLE_WRITTEN_FLOOR: usize = 2500;

/// Renders `crates/baylee-cards/src/generated_oracle.rs`: per card, per
/// printed face, the English Oracle text.
///
/// It is the text [`render_ability_lines`] counts sentences in and places
/// abilities against, read from the same cached payload by the same
/// [`face_texts`], so the index that table holds and the sentence this one
/// holds can only disagree if one of the two files is stale — which
/// `codegen --check` and `oracle_tests` are there to catch.
///
/// It exists because a client must never draw a blank row. The localized
/// text a player reads comes from a gateway or from Scryfall, and there are
/// three ways to have none of it: no network, a card nobody translated, or a
/// translation whose lines do not pair with the Oracle's. The owner's rule is
/// that each of them falls to the English Oracle sentence, and a running
/// client holds no English Oracle unless it is compiled in.
///
/// Not from the `//! Oracle:` headers, though they hold the same words: a
/// header is one line per *printed line* with no face boundary, so a
/// two-faced card's header cannot say where its back face starts.
///
/// Two-phase like stage 5, for its reason: the walk is over the pool compiled
/// into this binary.
pub(crate) fn render_oracle(root: &Path) -> anyhow::Result<String> {
    let decks_text = fs::read_to_string(root.join("data/acceptance-decks.txt"))?;
    let rows = acceptance::parse_decks(&decks_text)?;
    let pool_text = fs::read_to_string(root.join("data/card-pool.txt")).unwrap_or_default();
    let names = acceptance::all_names(&rows, &pool_text);
    let by_name: BTreeMap<&str, &'static baylee_cards::dsl::CardDef> =
        baylee_cards::all().map(|def| (def.name(), def)).collect();

    let mut table: Vec<String> =
        vec![String::from("&[],"); baylee_cards::generated::BY_INDEX.len()];
    let mut written = 0usize;
    for name in &names {
        let Some(def) = by_name.get(name.split(" // ").next().unwrap_or(name)) else {
            continue;
        };
        let Some(payload) = cached_printing(root, name) else {
            continue;
        };
        let Some(slot) = table.get_mut(def.index.get() as usize) else {
            continue;
        };
        let faces: Vec<String> = face_texts(&payload)
            .iter()
            .map(|text| format!("{text:?}"))
            .collect();
        *slot = format!("// {}\n&[{}],", def.name(), faces.join(", "));
        written += 1;
    }
    // The same floor as the walk it mirrors: a cache that is mostly empty
    // writes a table that is mostly `&[]`, and every row that points into
    // one of those holes falls to nothing.
    anyhow::ensure!(
        written >= ORACLE_WRITTEN_FLOOR,
        "only {written} cards had a cached printing to take their Oracle text \
         from; is `data/scryfall-cache` filled?"
    );

    Ok(format!(
        "// GENERATED by `cargo xtask codegen` — do not edit by hand.\n\
         //\n\
         // The English Oracle text of every card in the pool, per face, read from\n\
         // the same cached Scryfall payload `generated_lines.rs` was counted\n\
         // against. Card text is Wizards'; it stands here on the footing\n\
         // `docs/legal.md` gives the `//! Oracle:` headers that already carry it.\n\
         \n\
         #![allow(missing_docs, clippy::all, clippy::pedantic)]\n\
         \n\
         /// Per card by `CardIndex`, per printed face: the English Oracle text.\n\
         /// `&[]` is a retired index or a card with no cached printing.\n\
         pub static ORACLE: &[&[&str]] = &[\n{}\n];\n",
        table.join("\n")
    ))
}
