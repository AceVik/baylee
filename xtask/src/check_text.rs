//! `validate`: what the printed text says against what the card does —
//! oracle, set line, targets, optional clauses, scope.

use crate::{Path, Pinned, PrintingTally, lines, pinned_printing, set_header_line, stubgen};

/// The `//! Oracle:` header against the text actually printed on the card.
///
/// The header is the pool's human-verification surface, and every other
/// check reads *around* it: the cost, the P/T, the colors and the keywords
/// are compared against the printing, and the abilities are compared
/// against nothing at all. So a card whose rules text had been paraphrased,
/// abbreviated or overtaken by errata read as correct to a person and to
/// every gate, and the code underneath it was written from the wrong
/// sentence.
///
/// That is not hypothetical: the first run of this check found 145
/// disagreements. Ninety-seven were one codegen bug — Scryfall carries a
/// double-faced card's text per face and `stubgen` wrote the absent
/// top-level field, so every two-faced header was blank — and the other
/// forty-eight were hand-written headers, refreshed by
/// [`refresh_oracle`]. Two of those forty-eight were a *card* written from
/// its own wrong header: Volrath's Stronghold's second ability had lost its
/// `{1}{B}`, and Mirrorhall Mimic's disturb was built at `{5}{U}` against a
/// printed `{3}{U}{U}`.
///
/// Whitespace is the only thing forgiven, because a line's indentation
/// inside a doc comment is not a rules fact. Wording is not: "you may draw a
/// card unless that player pays {1}" and "you may have that player pay {1};
/// if they don't, you draw a card" are the same card and *not* the same
/// question, and which of the two is printed decides who is asked.
pub(crate) fn check_oracle_matches_the_printing(
    slug: &str,
    content: &str,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    let header = content
        .lines()
        .filter_map(|l| l.strip_prefix("//! Oracle:"))
        .collect::<Vec<_>>()
        .join("\n");
    let printed = printed_text(payload);
    tally.oracle += 1;
    if squash(&header) == squash(&printed) {
        return;
    }
    println!("{slug}: the header's oracle text is not the printing's");
    for line in diff_lines(&header, &printed) {
        println!("    {line}");
    }
    *problems += 1;
}

/// The `//! Set:` line against the printing it claims to name.
///
/// `validate` checked that this line *existed* and never what it said, which
/// is the whole of its job: the Scryfall id in it is carried in three places
/// and agreed everywhere, so the set code, the collector number and the set
/// name beside that id were prose nobody read. Forty hand-owned cards named
/// the wrong printing — Sensei's Divining Top said "EMA #232 — Eternal
/// Masters" over an id that is Double Masters 2022 #314 — and a person
/// checking the card by eye would have looked up a different piece of
/// cardboard and found it agreed.
///
/// It compares the whole line rather than the fields, because the line has
/// one author ([`baylee_cards_codegen::stubgen::set_line`]) and `xtask
/// refresh-oracle` writes exactly what this reads.
pub(crate) fn check_set_line_matches_the_printing(
    root: &Path,
    slug: &str,
    content: &str,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    unpinned: &mut usize,
    problems: &mut usize,
) {
    // Held against the printing the header itself names, never against
    // today's default for the name. A printing id does not move, so this
    // comparison is the same one next year.
    let pinned = pinned_printing(root, content, payload);
    let printing = match &pinned {
        Pinned::Default(card) => *card,
        Pinned::Other(card) => card,
        // A skip with a number on it. `xtask scryfall-cache` is what fills
        // them, and the `printings` floor is what catches a run where the
        // skip has quietly become the rule.
        Pinned::Missing => {
            *unpinned += 1;
            return;
        }
    };
    let Some(want) = set_header_line(printing) else {
        return;
    };
    let want = want.trim_end();
    tally.printings += 1;
    let Some(have) = content.lines().find(|l| l.starts_with("//! Set:")) else {
        return; // the "set line" check above already reported the absence
    };
    if have == want {
        return;
    }
    println!("{slug}: the header names a printing its own Scryfall id does not");
    println!("    header  {have}");
    println!("    printing {want}");
    *problems += 1;
}

/// A printed "up to N target" is a **count**, and the code has to be able to
/// say it.
///
/// A bare `TargetSpec` reads as *exactly one*, which is a different card: an
/// ability whose only legal answer is "none" cannot be activated at all, so
/// the sentence after it never happens. Karn, the Great Creator's `+1` and
/// Teferi, Time Raveler's `-3` were both written that way, and Teferi's is
/// the one that shows what it costs — "Return up to one target artifact,
/// creature, or enchantment to its owner's hand. **Draw a card.**" was a
/// draw a player could not reach with an empty board.
///
/// The check is textual on purpose. It reads the printing's own sentence and
/// then asks whether the card's source says a minimum of none *anywhere* —
/// `TargetReq::up_to_one`, `up_to`, `x_targets` or a written-out `min: 0`.
/// That is coarse: a card with two targeted abilities where only one prints
/// "up to" passes on the other's count. It is still worth having, because the
/// failure it is written for is a card that says "up to" in its header and
/// nowhere in its code. Every ability shape can say a count now. The last two
/// that could not were `Activated` and `ActivatedConditional`, whose target
/// was a bare spec until it became a `TargetReq`. Such a card could pass only
/// by marking itself `Partial`.
pub(crate) fn check_target_counts_match_the_printing(
    slug: &str,
    content: &str,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    // A stub claims nothing, so there is nothing to disagree with — and a
    // card marked `Partial` has already said, in writing, that it diverges
    // from its printing. That is the sanctioned answer for a sentence the
    // DSL cannot say (Sheoldred's chapter I destroys one permanent *per
    // opponent*, which is not a number `TargetReq` has), and a checker that
    // reported it anyway would be asking the pool to lie the other way.
    if content.contains(stubgen::STUB_MARKER) || content.contains("Coverage::Partial") {
        return;
    }
    let printed = printed_text(payload).to_lowercase();
    let Some(phrase) = up_to_target_phrase(&printed) else {
        return;
    };
    tally.targets += 1;
    // `ObjectOfEachOpponent` is "for each opponent, up to one target … that
    // player controls" and nothing else: the engine asks it as up to one per
    // opponent (`progress.rs`), so the spec is itself a minimum of none.
    if [
        "up_to_one(",
        "up_to(",
        "x_targets(",
        "min: 0",
        "ObjectOfEachOpponent(",
    ]
    .iter()
    .any(|way| content.contains(way))
    {
        return;
    }
    println!("{slug}: the printing says \"{phrase}\" and the code says exactly one");
    *problems += 1;
}

/// A card whose printing names a **player** as a target has to be able to
/// name one.
///
/// The fault this closes is the one the owner reported from a live game and
/// it has a shape: `PlayerRel::Opponent` written where the card says "target
/// player", which is `EachOpponent` in `eval::players` — so Halimar
/// Excavator milled every opponent at once, and Primaris Eliminator's
/// Hyperfrag Round shrank every creature on the table including its own
/// side. Both read correctly in a duel, which is why both survived every
/// other check in this file and a whole test suite: with one opponent,
/// "each of them" and "the one you chose" are the same seat.
///
/// It is decidable without a second reader, because the card prints it. The
/// printed text says "target player" or "target opponent"; the code either
/// names a player-shaped [`TargetSpec`] or it does not. There is no
/// interpretation in between.
///
/// Three rules keep it from inventing findings:
///
/// - **Reminder text is stripped**, through the [`strip_reminders`] that
///   `check_scope_matches_the_text` already reads with. It is about a
///   keyword, not about this card — the same reading `claim_tests` had to
///   learn — and "target player" inside a parenthesis is somebody else's
///   sentence.
/// - **A stub claims nothing**, and a `Partial` card has already said in
///   writing that it diverges. Both are skipped, exactly as the target-count
///   check above skips them.
/// - It reads the **printing**, not the `//! Oracle:` header, so a card whose
///   header drifted cannot hide the gap by agreeing with its own code.
pub(crate) fn check_player_targets_match_the_printing(
    slug: &str,
    content: &str,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    if content.contains(stubgen::STUB_MARKER) || content.contains("Coverage::Partial") {
        return;
    }
    let printed = printed_text(payload).to_lowercase();
    let bare = strip_reminders(&printed);
    let Some(phrase) = ["target player", "target opponent"]
        .into_iter()
        .find(|p| bare.contains(p))
    else {
        return;
    };
    tally.player_targets += 1;
    // `OpponentOrObject` is "target opponent or [filter]" (Huntmaster of
    // the Fells' back face): one instance of the word that may name a player.
    if [
        "TargetSpec::AnyPlayer",
        "TargetSpec::AnyOpponent",
        "TargetSpec::AnyTarget",
        "TargetSpec::OpponentOrObject",
    ]
    .iter()
    .any(|spec| content.contains(spec))
    {
        return;
    }
    println!("{slug}: the printing says \"{phrase}\" and the code names no player to target");
    *problems += 1;
}

/// A card whose printing says "you may" has to hand the player the choice.
///
/// The fault this closes is the one the owner reported by name: Ondu Cleric
/// prints "you may gain life equal to the number of Allies you control" and
/// gained it every time, because the word was read as decoration. Four cards
/// were written that way, and each of them has a board where the automatic
/// answer is the wrong one — which is the whole reason the word is printed.
///
/// It is decidable without a second reader for [`check_player_targets_match_the_printing`]'s
/// reason: the card prints the word, and the code either carries a construct
/// that offers a choice or it does not. What it must **not** be is a list of
/// card names — this is a sweep over the pool, and a sweep that recognises
/// cards one at a time stops being one the moment a card is added.
///
/// So it reads the **compiled `CardDef`** and not the file, through the
/// `Debug` rendering `pool-dump` already writes. That is what makes it a
/// check about constructs: Restoration Angel writes its "may" as a literal
/// `TargetReq { min: 0, .. }` and Sun Titan writes the same thing as
/// `TargetReq::up_to_one`, and the compiled form of both says `min: 0`. A
/// grep over the source would have called one of them a bug.
///
/// The list is longer than `MayDo` because a printed "you may" is said
/// several different ways here, all of them already asking:
///
/// - "you may have **target** player lose life" — a target requirement whose
///   minimum is zero. Declining is choosing no target, which the target
///   prompt already offers (Hagra Diabolist, Sun Titan, Restoration Angel).
/// - "you may pay 2 life" as a land enters — `TappedOrPayLife`, and
///   `PayLifeOrEnterTapped` where the same choice is an effect.
/// - "you may pay `{N}`" during resolution — `PlayerMayPayOr`.
/// - "you may pay … rather than pay this spell's mana cost" — an
///   `AlternativeCost`; a kicker is an additional cost.
/// - "you may have this enter as a copy" — `CopyOnEnter`.
/// - "you may choose new targets for the copy" — the copy effects, which ask
///   on their own (`AwaitingOp::CopyNewTargets`; a copied ability through
///   `retarget::start_copy`, Vantress Visions).
/// - "you may choose a nonland card from it" — `BottomCardFromHand`, which
///   offers a choice of none (Vendilion Clique).
/// - "you may search your library" — an optional search.
/// - "you may put it on the bottom" — scry, a choice by construction.
/// - "you may play those cards … and you may spend mana as though" —
///   `SearchTakeover`. A permission is not a decision: playing a card is
///   optional already, and there is nothing to ask.
/// - "you may spend white mana as though it were red mana" — `SpendManaAs`.
///   The permission adds a payment option while preserving ordinary payment;
///   it neither produces mana nor asks for a separate resolution decision.
/// - "you may play lands from your graveyard" / "you may play an additional
///   land on each of your turns" — `PlayLandsFromGraveyard` and
///   `ExtraLandDrops`, the same argument one rule further on. These widen
///   what CR 305.1 and CR 305.2 allow; the land drop itself is already the
///   player's to make or not, and a permission that asked would be a prompt
///   nobody could answer "no" to usefully. Crucible of Worlds, Ramunap
///   Excavator and Exploration are the three, and they were the first cards
///   through a modifier family that had none when it landed — which is why
///   this bullet exists rather than the list quietly growing.
/// - "you may play it this turn" (Dauthi Voidwalker, Expressive Iteration)
///   — `ChooseExiledToPlay` and `LookAtTopKeepBottomPlay`, which leave a
///   permission to play one card, and the `SearchTakeover` argument again:
///   the player plays it or does not, and nothing is asked.
/// - "you may put it onto the battlefield tapped" (Risen Reef) —
///   `LookAtTopMayPut`, which asks about the looked-at card with `min: 0`
///   in the engine, not in the definition; and "you may discard up to two
///   cards" (Fable of the Mirror-Breaker) — `DiscardUpToThenDraw`, the
///   same `min: 0` question over the hand.
/// - "you may play a land and cast a permanent spell of each permanent type
///   from your graveyard" (Muldrotha) and "you may … cast permanent spells
///   from your graveyard" (Wrenn and Realmbreaker's emblem) —
///   `PermanentOfEachTypeFromGraveyard` and `CastPermanentSpellsFromGraveyard`,
///   the Crucible argument for casting: a permission, not a decision.
/// - "you may put a permanent card from among the milled cards into your
///   hand" (Wrenn's −2) — `MillMayTakeOne`, the `min: 0` question asked in
///   the engine.
/// - "for each card type, you may put a card of that type from among them
///   into your hand" — `RevealTopOnePerType` (Atraxa, Grand Unifier), which
///   asks once per card type with a minimum of none (`resolve::one_per_type`).
/// - "until end of turn, you may cast that card" (Ragavan, Nimble Pilferer)
///   — `ExileTopMayCast`, which leaves a cast-only permission: the
///   `ChooseExiledToPlay` argument, the card is cast or it is not.
///
/// A stub claims nothing and a `Partial` card has said in writing that it
/// diverges, so both are skipped — the same two exemptions the checks above
/// take.
///
/// What it counts is what it can decide, and the two are not the same
/// thing. It asks whether the card carries an asking construct *anywhere*,
/// so a printing with two "may"s and one construct passes. A clean sweep
/// therefore says no card in the pool is may-blind; it does not say every
/// printed "may" is asked, and reading it as the second claim is how a
/// check like this stops finding anything.
pub(crate) fn check_optional_clauses_are_offered(
    slug: &str,
    def: &baylee_cards::dsl::CardDef,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    // Every construct that hands a choice to the player, as the compiled
    // definition spells it. `min: 0` is a target requirement's and nothing
    // else's — it is the one `min` field in the whole DSL.
    const OFFERS_A_CHOICE: &[&str] = &[
        "MayDo",
        "min: 0",
        "PayLifeOrEnterTapped",
        "PlayerMayPayCostOr",
        "PlayerMayPayOr",
        "PlayerMayPayThen",
        "PlayerMayPayManaOr",
        "PlayerMayPayManaThen",
        "PreventNextFromChosenSource",
        "CopyOnEnter",
        "CopyTargetSpell",
        "CopyTargetAbility",
        "CopySpell",
        "BottomCardFromHand",
        "OptionalBasicLandSearchFor",
        "optional: true",
        "Scry",
        "PutFromHandOnTop",
        "ReorderTopLibrary",
        "SearchTakeover",
        "SpendManaAs",
        "GrantSpecialActionUntilEndOfTurn",
        "PlayLandsFromGraveyard",
        "PlayLandsFromLibraryTop",
        "ExtraLandDrops",
        "ChooseExiledToPlay",
        "LookAtTopKeepBottomPlay",
        "LookAtTopMayPut",
        "DiscardUpToThenDraw",
        "PermanentOfEachTypeFromGraveyard",
        "CastPermanentSpellsFromGraveyard",
        "MillMayTakeOne",
        "RevealTopOnePerType",
        "ExileTopMayCast",
    ];
    if !def.is_implemented() {
        return;
    }
    let printed = printed_text(payload).to_lowercase();
    if !strip_reminders(&printed).contains("you may") {
        return;
    }
    tally.optional_clauses += 1;
    // A cost a face offers rather than demands is read from the field, not
    // from the rendering: "you may pay … rather than" and a kicker are the
    // *absence* of a requirement, and an empty list renders as one word.
    if def.faces.iter().any(|f| {
        !f.alternative_costs.is_empty()
            || !f.additional_costs.is_empty()
            || f.miracle.is_some()
            || !f.enter_modifiers.is_empty()
    }) {
        return;
    }
    // Everything else is a construct that appears in the rendering.
    let compiled = format!("{def:#?}");
    if OFFERS_A_CHOICE.iter().any(|c| compiled.contains(c)) {
        return;
    }
    println!("{slug}: the printing says \"you may\" and the code never asks");
    *problems += 1;
}

/// The printed phrase that states a target count of "up to", if there is one.
///
/// Read by scanning rather than by matching a list of whole phrases, because
/// what sits between the number and the noun varies with the card — "up to
/// one target creature", "up to two target **other** creatures", "up to X
/// target lands" — and a list of exact spellings would quietly stop matching
/// the first time a card worded it a new way.
///
/// The window stops at a line break or a bullet, which is the difference
/// between a count of *targets* and a count of *modes*. Ertai Resurrected
/// prints "choose up to one —" and then two bulleted modes that each target
/// something; the "up to one" there is about how many modes are chosen, and
/// the card says it with an empty third mode rather than with a `TargetReq`.
pub(crate) fn up_to_target_phrase(printed: &str) -> Option<String> {
    let mut from = 0;
    while let Some(at) = printed[from..].find("up to ") {
        let start = from + at;
        let rest = &printed[start..printed.len().min(start + 44)];
        let window = rest.split(['\n', '\u{2022}']).next().unwrap_or_default();
        if let Some(i) = window.find("target") {
            return Some(window[..i + "target".len()].to_string());
        }
        from = start + "up to ".len();
    }
    None
}

/// Every non-blank line, trimmed — the one difference this check forgives.
pub(crate) fn squash(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

/// The two texts side by side, as `-` header and `+` printing lines.
///
/// A whole-line set difference rather than a real diff: the header is at
/// most a dozen lines, and what a reader needs is which sentence to look at,
/// not an edit script.
pub(crate) fn diff_lines(header: &str, printed: &str) -> Vec<String> {
    let (a, b) = (squash(header), squash(printed));
    let mut out = Vec::new();
    for line in &a {
        if !b.contains(line) {
            out.push(format!("- {line}"));
        }
    }
    for line in &b {
        if !a.contains(line) {
            out.push(format!("+ {line}"));
        }
    }
    out
}

/// Keyword bits that have a printed spelling to look for.
///
/// Not every bit does, and the missing ones are deliberate rather than
/// forgotten. `UNBLOCKABLE` and `UNCOUNTERABLE` are our names for printed
/// *sentences* — "can't be blocked", "this spell can't be countered" — that
/// every card words its own way, so there is no single word to find.
pub(crate) const KEYWORD_WORDS: &[(baylee_cards::dsl::KeywordSet, &str)] = {
    use baylee_cards::dsl::KeywordSet as K;
    &[
        (K::FLYING, "flying"),
        (K::FIRST_STRIKE, "first strike"),
        (K::DOUBLE_STRIKE, "double strike"),
        (K::DEATHTOUCH, "deathtouch"),
        (K::HASTE, "haste"),
        (K::HEXPROOF, "hexproof"),
        (K::INDESTRUCTIBLE, "indestructible"),
        (K::LIFELINK, "lifelink"),
        (K::MENACE, "menace"),
        (K::REACH, "reach"),
        (K::TRAMPLE, "trample"),
        (K::VIGILANCE, "vigilance"),
        (K::DEFENDER, "defender"),
        (K::FLASH, "flash"),
        (K::SHROUD, "shroud"),
        (K::FEAR, "fear"),
        (K::INTIMIDATE, "intimidate"),
        (K::SHADOW, "shadow"),
        (K::HORSEMANSHIP, "horsemanship"),
        (K::INFECT, "infect"),
        (K::WITHER, "wither"),
        (K::PERSIST, "persist"),
        (K::UNDYING, "undying"),
        (K::PROWESS, "prowess"),
        (K::SKULK, "skulk"),
        (K::FLANKING, "flanking"),
        (K::CHANGELING, "changeling"),
        (K::PARTNER, "partner"),
        (K::REBOUND, "rebound"),
        (K::PROTECTION_BLACK, "protection from black"),
        (K::DAYBOUND, "daybound"),
        (K::NIGHTBOUND, "nightbound"),
        (K::CANT_BLOCK, "can't block"),
        (K::SPLIT_SECOND, "split second"),
        (K::ASCEND, "ascend"),
        (K::CANT_ATTACK, "can't attack"),
        (K::UNDYING, "undying"),
        (K::PERSIST, "persist"),
    ]
};

/// Whether `text` says `word` as a word, so a Reach creature does not read as
/// having Flash because its own reminder text mentions flashback.
pub(crate) fn mentions_word(text: &str, word: &str) -> bool {
    let boundary = |c: char| !c.is_alphanumeric();
    text.match_indices(word).any(|(at, _)| {
        text[..at].chars().next_back().is_none_or(boundary)
            && text[at + word.len()..].chars().next().is_none_or(boundary)
    })
}

/// Every face's printed text, joined.
pub(crate) fn printed_text(payload: &serde_json::Value) -> String {
    let mut out = String::new();
    let mut push = |v: Option<&serde_json::Value>| {
        if let Some(s) = v.and_then(serde_json::Value::as_str) {
            out.push_str(s);
            out.push('\n');
        }
    };
    push(payload.get("oracle_text"));
    if let Some(faces) = payload
        .get("card_faces")
        .and_then(serde_json::Value::as_array)
    {
        for face in faces {
            push(face.get("oracle_text"));
        }
    }
    out
}

/// The printed text of each face, kept apart.
///
/// [`printed_text`] joins them, which is right for a search over "does this
/// card say X anywhere" and wrong for anything that wants an index into one
/// face's sentences: Sheoldred's front prints three lines and its back
/// three more, and a saga chapter numbered against the joined string points
/// at the front face a client is not drawing.
pub(crate) fn face_texts(payload: &serde_json::Value) -> Vec<String> {
    let text = |v: Option<&serde_json::Value>| {
        v.and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    if let Some(faces) = payload
        .get("card_faces")
        .and_then(serde_json::Value::as_array)
    {
        return faces.iter().map(|f| text(f.get("oracle_text"))).collect();
    }
    vec![text(payload.get("oracle_text"))]
}

/// Compares "onto the battlefield **tapped**" in the printed text against the
/// [`Find`](baylee_cards_dsl::effect::Find) the code searches with.
///
/// One word, and it is the difference between two families of land that look
/// identical in a card file: Evolving Wilds puts its basic in tapped and costs
/// nothing, a fetchland puts its dual in untapped and costs a life. Four of
/// the ten fetchlands in the pool shipped with `Find::BATTLEFIELD_TAPPED`
/// while their own header quoted "put it onto the battlefield" and their own
/// comment said `Find::BATTLEFIELD` — nothing compared the two, so the pool
/// disagreed with itself for as long as it had fetchlands in it, and the
/// engine test written from the code rather than from the card made it
/// permanent.
///
/// The reading is deliberately narrow. Only a *put* counts, so a land whose
/// own text says it enters the battlefield tapped is not mistaken for one
/// that taps what it finds, and the claim is presence rather than a count:
/// Cultivate names one destination per sentence and Sword of Hearth and Home
/// names two cards in one, and neither is a drift this check is for.
pub(crate) fn check_search_tapped_matches_text(slug: &str, content: &str, problems: &mut usize) {
    /// The printed phrase both halves of the comparison hang off.
    const ONTO: &str = "onto the battlefield";

    // The code only. A card's comments quote both spellings while explaining
    // which one it is, which is exactly the sentence that went stale.
    let code: String = content
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    let finds: String = code
        .match_indices("finds:")
        .filter_map(|(at, _)| {
            let rest = &code[at..];
            rest.find(']').map(|end| &rest[..end])
        })
        .collect::<Vec<_>>()
        .join(" ");
    if finds.is_empty() {
        return;
    }
    let code_tapped = finds.contains("Find::BATTLEFIELD_TAPPED");
    let code_plain = finds
        .match_indices("Find::BATTLEFIELD")
        .any(|(at, _)| !finds[at..].starts_with("Find::BATTLEFIELD_TAPPED"));

    let oracle: String = content
        .lines()
        .filter_map(|l| l.strip_prefix("//! Oracle:"))
        .collect::<Vec<_>>()
        .join(" ");
    let (mut text_tapped, mut text_plain) = (false, false);
    for (at, _) in oracle.match_indices(ONTO) {
        let before: String = oracle[..at]
            .chars()
            .rev()
            .take(60)
            .collect::<String>()
            .to_ascii_lowercase();
        if !before.contains("tup") {
            continue; // "put", read backwards — this is an entry, not a put.
        }
        if oracle[at + ONTO.len()..].starts_with(" tapped") {
            text_tapped = true;
        } else {
            text_plain = true;
        }
    }

    if code_tapped && !text_tapped {
        println!("{slug}: code puts a found card onto the battlefield tapped, the text does not");
        *problems += 1;
    }
    if code_plain && !text_plain {
        println!("{slug}: code puts a found card onto the battlefield untapped, the text does not");
        *problems += 1;
    }
}

/// Cards whose printed "you control" or "an opponent controls" is not a
/// filter on any object, with the reason.
///
/// Every entry here is a sentence the DSL says another way, and naming the
/// way is the point: an exception with no reason is a card nobody looked at.
pub(crate) const SCOPE_EXCEPTIONS: &[(&str, &str)] = &[
    ("Bleachbone Verge", "an Condition, not a filter"),
    ("Mox Opal", "metalcraft is an Condition"),
    ("Fierce Guardianship", "an AlternativeCost condition"),
    (
        "Deadly Rollick",
        "the same cycle, the same AlternativeCost condition",
    ),
    (
        "Deflecting Swat",
        "the third of that cycle, and the same condition again",
    ),
    (
        "Strength of the Harvest",
        "ModifyPTPerCount already counts the effect controller's side only",
    ),
    (
        "Reflecting Pool",
        "\"any type a land you control could produce\" is its own mana rule",
    ),
    ("Exotic Orchard", "the same rule, read across the table"),
    ("Fellwar Stone", "the same rule, read across the table"),
    (
        "Opposition Agent",
        "\"you control your opponents\" is the verb, not the zone",
    ),
    ("Urza's Saga", "the clause is printed on the token it makes"),
    (
        "Ashiok, Dream Render",
        "a player-wide static with no object to filter",
    ),
    (
        "Karn, the Great Creator",
        "the lock is a player rule; see karns_lock_spares_a_teammate",
    ),
    (
        "Leovold, Emissary of Trest",
        "the opponent controls the spell or ability, and `Trigger::TargetedByOpponent` asks that of its controller, not of a permanent",
    ),
];

/// What the card says about *whose* permanents it reaches, against what it
/// does.
///
/// The owner's question, in two directions: an effect that should only touch
/// your own side must say so, and one that reaches across the table must not
/// be able to come back. Both are invisible in a duel played once — a wrong
/// filter still points at *something* — and both are decidable by reading
/// the printed sentence beside the filters the card was built from.
///
/// Reminder text is stripped first. It is printed in parentheses, it is not
/// rules the card carries, and hexproof's own reminder ends "…spells or
/// abilities your opponents control" on every card that has it.
pub(crate) fn check_scope_matches_the_text(
    slug: &str,
    name: &str,
    def: &baylee_cards::dsl::CardDef,
    payload: &serde_json::Value,
    problems: &mut usize,
) {
    if !matches!(def.coverage, baylee_cards::dsl::Coverage::Implemented) {
        return;
    }
    if SCOPE_EXCEPTIONS.iter().any(|(card, _)| *card == name) {
        return;
    }
    let printed = strip_reminders(&printed_text(payload)).to_lowercase();

    // The filters the card was built from, read off the one rendering that
    // cannot go stale as the DSL grows a variant.
    let built = format!("{def:?}");
    let says_you = printed.contains("you control");
    let says_theirs =
        printed.contains("opponent controls") || printed.contains("opponents control");
    // `Condition::ControlCount` is the second spelling of "you control", and
    // not a filter at all: `eval::condition_holds` counts only permanents
    // whose `controller == you` and hands the filter each candidate's own id,
    // so the filter inside it says *what* to count and never whose. A card
    // whose only such clause is a count — "activate only if you control an
    // Island", "if you control three or more artifacts" — would otherwise be
    // asked for a `ControlledByYou` that would change nothing, and eighteen
    // generated lands landed in one commit with exactly that shape.
    //
    // Two more say it without a filter. `PtCount::YouControl` counts only
    // what the ability's controller controls ("the number of creatures you
    // control", `layers.rs`), and the rendering reaches into a token's
    // abilities, so Voice of Resurgence's Elemental says it there.
    // `Duration::WhileYouControlSource` is "for as long as you control this
    // creature" (CR 611.2b): a duration, not a set of objects. And
    // `Modifier::GainControl` is "you control enchanted creature" (Control
    // Magic, Steal Artifact, Treachery): the sentence is the change of
    // control itself, whose new controller is the effect's (CR 613.1b).
    let filters_you = built.contains("ControlledByYou")
        || built.contains("ControlCount(")
        || built.contains("YouControl(")
        || built.contains("WhileYouControlSource")
        || built.contains("GainControl");
    let filters_theirs = built.contains("ControlledByOpponent");

    if says_you && !filters_you {
        println!(
            "{slug}: the text says \"you control\" and no filter does; use `Filter::ControlledByYou`, or add an entry to SCOPE_EXCEPTIONS saying why not"
        );
        *problems += 1;
    }
    if says_theirs && !filters_theirs {
        println!(
            "{slug}: the text says an opponent controls it and no filter does; use `Filter::ControlledByOpponent`, or add an entry to SCOPE_EXCEPTIONS"
        );
        *problems += 1;
    }
    if filters_theirs && !printed.contains("opponent") {
        println!(
            "{slug}: reaches only an opponent's permanents and the text never says opponent; \"you don't control\" is `Filter::Not(&Filter::ControlledByYou)`, which a teammate's permanent matches too"
        );
        *problems += 1;
    }
}

/// Oracle text with its reminder text taken out.
///
/// One implementation, in `lines`, because the sentence reader there needs
/// exactly this and a second copy of it would be free to drift from the
/// one the generated table is built with.
pub(crate) fn strip_reminders(text: &str) -> String {
    lines::without_reminder(text)
}
