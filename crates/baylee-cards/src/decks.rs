//! Deck loading: acceptance-suite text format → registry-resolved
//! presets. Used by self-play tests and the local play harness.

use crate::by_index;
use baylee_cards_dsl::{CommanderRule, PartnerKind};
use baylee_core::acceptance::{Zone, parse_decks};
use baylee_core::deckrow::PrintChoice;
use baylee_core::generated::subtypes;
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, FormatId, GamePreset, PrintInfo, SeatController, SeatSpec,
};
use baylee_core::preset::{Finish, HouseRules};

/// One copy of one card in a deck, with the printing its owner chose.
///
/// Rules identity and print identity travel together because a deck list is
/// where they are decided together and nowhere else: the engine takes the
/// first and never reads the second, and the client takes the second and
/// cannot derive it from the first. Two copies of the same card with
/// different finishes are two `DeckCard`s and one `CardIndex`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeckCard {
    /// Rules identity.
    pub index: CardIndex,
    /// Which physical printing this copy is.
    pub print: PrintInfo,
}

impl DeckCard {
    /// A copy printed as the registry's reference printing: English, non-foil.
    #[must_use]
    pub fn plain(index: CardIndex) -> Self {
        Self {
            index,
            print: reference_print(index),
        }
    }

    /// A copy printed as a deck row asked for, falling back field by field to
    /// the registry's reference printing.
    #[must_use]
    pub fn chosen(index: CardIndex, choice: &PrintChoice) -> Self {
        let mut print = reference_print(index);
        // Only the id names a *printing*; set and collector number narrow
        // towards one, and resolving them needs a catalog this crate does not
        // have. They are carried in the deck row and resolved by whoever
        // stores it, so by the time a deck is loaded either an id is set or
        // the reference printing is the answer.
        if let Some(id) = &choice.scryfall_id
            && let Ok(id) = uuid::Uuid::parse_str(id)
        {
            print.scryfall_id = id;
        }
        if let Some(lang) = &choice.lang {
            print.lang.clone_from(lang);
        }
        if let Some(finish) = choice.finish {
            print.finish = finish;
        }
        Self { index, print }
    }
}

/// The registry's own printing of a card: what a deck that named none gets.
#[must_use]
pub fn reference_print(card: CardIndex) -> PrintInfo {
    let id = by_index(card).map_or_else(uuid::Uuid::nil, |def| {
        uuid::Uuid::parse_str(def.scryfall_id).unwrap_or_default()
    });
    PrintInfo {
        scryfall_id: id,
        lang: "en".to_string(),
        finish: Finish::Normal,
    }
}

/// A loaded deck: main-deck entries plus commander(s).
#[derive(Clone, Debug)]
pub struct LoadedDeck {
    /// Deck name.
    pub name: String,
    /// Main-deck entries (one per copy).
    pub main: Vec<DeckCard>,
    /// Sideboard entries (one per copy). Reachable by wishes, never
    /// shuffled into the library — folding these into `main` quietly
    /// turned a 60-card deck with a 15-card sideboard into 75 cards.
    pub sideboard: Vec<DeckCard>,
    /// Commander card(s).
    pub commanders: Vec<DeckCard>,
}

/// Resolves a card name to its registry index, in constant time.
///
/// The pool's names are placed in a perfect hash when
/// [`generated_names`](crate::generated_names) is written, so this is one
/// hash, one array read and one string compare. It was a walk over all 1365
/// cards with a compare each — which the gateway ran once per row of every
/// deck it stored, and every self-play harness ran per card of every deck it
/// dealt.
///
/// **The compare is not a formality.** A perfect hash is perfect only over
/// the names it was built from: a string the pool does not have lands in
/// some slot as well, so without reading back the spelling that lives there
/// this would answer a *wrong card* rather than `None`.
///
/// What it answers about is the **pool** — the cards this engine can build —
/// and deliberately not the 33 694-row corpus in `baylee-cards-index`. A
/// deck row naming a card that exists but is unimplemented has to fail here,
/// because the `CardIndex` it would otherwise get back resolves to no
/// `CardDef`; and the corpus names are not compiled into this crate, which
/// the engine links. Telling a player *which* of the two a name is needs a
/// second lookup, not a wider table.
#[must_use]
pub fn by_name(name: &str) -> Option<CardIndex> {
    use crate::generated_names::{EMPTY, NAMES, SLOTS, TABLE};
    let at = SLOTS[TABLE.slot(name.as_bytes())];
    if at == EMPTY {
        return None;
    }
    let (spelling, index) = NAMES[at as usize];
    (spelling == name).then_some(index)
}

/// Loads a named deck from the acceptance text.
///
/// # Errors
/// Returns the first unresolvable card name or the parse error.
pub fn load_acceptance(text: &str, deck_name: &str) -> Result<LoadedDeck, String> {
    let rows = parse_decks(text).map_err(|e| e.to_string())?;
    let mut main = Vec::new();
    let mut sideboard = Vec::new();
    let mut commanders = Vec::new();
    for row in rows.iter().filter(|r| r.deck == deck_name) {
        let index = by_name(&row.name).ok_or_else(|| format!("unknown card: {}", row.name))?;
        let target = match row.zone {
            Zone::Main => &mut main,
            Zone::Sideboard => &mut sideboard,
            Zone::Commander => &mut commanders,
        };
        for _ in 0..row.count {
            target.push(DeckCard::plain(index));
        }
    }
    if main.is_empty() {
        return Err(format!("deck not found: {deck_name}"));
    }
    Ok(LoadedDeck {
        name: deck_name.to_string(),
        main,
        sideboard,
        commanders,
    })
}

/// Loads a deck from the stored `"N Card Name"` rows a saved deck is kept as.
///
/// The grammar is `baylee_core::deckrow`'s, which is the same reader a stored
/// deck, an exported file and an imported one all go through, so a deck
/// resolves to the same cards wherever it came from. The commander is stored
/// by name and is **moved** out of the main list rather than copied: the
/// builder seats the leader among the rows on purpose, so copying it would
/// make one more card that sits in the library and the command zone at once.
///
/// What this deliberately does *not* do is enforce the copy limit or the deck
/// size. Those belong to the gateway, because they are checks on a deck
/// somebody else submitted, and a limit enforced in two places with two error
/// types is a limit that will one day differ. Offline there is no other
/// party — and the engine still refuses a preset it cannot play.
///
/// # Errors
/// Returns the first malformed row or unresolvable card name.
pub fn from_lines(
    name: &str,
    cards: &[String],
    sideboard: &[String],
    commanders: &[String],
) -> Result<LoadedDeck, String> {
    fn expand(lines: &[String]) -> Result<Vec<DeckCard>, String> {
        let mut out = Vec::new();
        for line in lines {
            let row = baylee_core::deckrow::parse(line)
                .map_err(|_| format!("malformed card line: {line}"))?;
            let index = by_name(&row.name).ok_or_else(|| format!("unknown card: {}", row.name))?;
            for _ in 0..row.count {
                out.push(DeckCard::chosen(index, &row.print));
            }
        }
        Ok(out)
    }

    let mut main = expand(cards)?;
    let sideboard = expand(sideboard)?;
    // Each leader is moved out of the main list in turn, so a deck that
    // named two of them loses both from the library and neither twice.
    let commanders = commanders
        .iter()
        .filter_map(|name| by_name(name))
        .map(|index| match main.iter().position(|c| c.index == index) {
            Some(at) => main.remove(at),
            None => DeckCard::plain(index),
        })
        .collect();
    Ok(LoadedDeck {
        name: name.to_string(),
        main,
        sideboard,
        commanders,
    })
}

/// Every deck the acceptance text names, in the order it names them.
///
/// The file is the only deck data every build carries — the browser build
/// embeds it — so this is what "which decks are there before anybody has
/// built one" is answered with.
#[must_use]
pub fn acceptance_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    for row in parse_decks(text).unwrap_or_default() {
        if !names.contains(&row.deck) {
            names.push(row.deck.clone());
        }
    }
    names
}

/// A two-seat mirror game built to exercise one card.
///
/// A probe deck alone proves less than it looks: four copies in sixty cards
/// are often never drawn inside a short game, so a sweep over the pool can
/// pass without most of its cards ever having been in play. This puts the
/// card where it cannot be missed — one copy in the opening hand, and, when
/// it is a permanent, one already on the battlefield so its static,
/// triggered and activated abilities are live from turn one.
///
/// Returns `None` when the card is not registered or there are no basic
/// lands to pad with.
#[must_use]
pub fn probe_preset(seed: u64, card: CardIndex) -> Option<GamePreset> {
    use baylee_core::types::TypeSet;

    let deck = probe_deck(card, 4, 60)?;
    let def = by_index(card)?;
    let is_permanent = def.faces.first().is_some_and(|f| {
        f.types.contains(TypeSet::LAND)
            || f.types.contains(TypeSet::CREATURE)
            || f.types.contains(TypeSet::ARTIFACT)
            || f.types.contains(TypeSet::ENCHANTMENT)
            || f.types.contains(TypeSet::PLANESWALKER)
    });

    let mut prints: Vec<PrintInfo> = Vec::new();
    let mut entries = |cards: &[DeckCard]| -> Vec<DeckEntry> {
        cards
            .iter()
            .map(|c| DeckEntry {
                card: c.index,
                print: print_ref_for(&mut prints, &c.print),
            })
            .collect()
    };
    // The opening hand: the card plus enough lands to cast it.
    let mut hand_cards = vec![DeckCard::plain(card)];
    hand_cards.extend(deck.main.iter().rev().take(6).cloned());
    let seat = |deck: Vec<DeckEntry>, hand: Vec<DeckEntry>, field: Vec<DeckEntry>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck,
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: Some(hand),
        starting_battlefield: field,
        emblems: vec![],
        team: None,
    };
    let field_cards = if is_permanent {
        vec![DeckCard::plain(card)]
    } else {
        vec![]
    };
    let seats = (0..2)
        .map(|_| {
            let d = entries(&deck.main);
            let h = entries(&hand_cards);
            let f = entries(&field_cards);
            seat(d, h, f)
        })
        .collect();
    Some(GamePreset {
        format: FormatId::Freeform,
        seed,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints,
        seats,
    })
}

/// The print-table slot for one printing, deduplicated.
///
/// Deduplicated on the *whole* printing rather than on the id: a foil and a
/// non-foil copy of the same card share an image and must not share a slot,
/// because the slot is where the finish is written and one of the two would
/// come out wearing the other's.
fn print_ref_for(prints: &mut Vec<PrintInfo>, print: &PrintInfo) -> PrintRef {
    if let Some(pos) = prints.iter().position(|p| p == print) {
        return PrintRef::new(pos as u16);
    }
    prints.push(print.clone());
    PrintRef::new((prints.len() - 1) as u16)
}

/// What a card brings to the question "may these two lead one deck".
///
/// Five facts, of which the index is one, because `PartnerWith` names a card
/// and names it by [`CardIndex`]. That keeps the rule testable against pairs
/// the pool does not happen to contain, which is the property it was built
/// for: the ledger numbers every card there is, so `index::TYMNA_THE_WEAVER`
/// resolves whether or not this build compiles a `CardDef` for her, while a
/// rule reachable only through the registry could only ever be shown
/// refusing — the pool holds exactly one card with a partner ability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Leader {
    /// The card's rules identity, which is what `PartnerWith` names.
    pub index: CardIndex,
    /// Whether the card may lead a deck at all (CR 903.3).
    pub eligible: bool,
    /// Which partner family it is in (CR 702.124).
    pub partner: PartnerKind,
    /// Whether it is a Doctor, for Doctor's companion.
    pub doctor: bool,
    /// Whether it is a Background, for "Choose a Background".
    pub background: bool,
}

/// Whether two cards may lead one deck together (CR 702.124, CR 903.3b).
///
/// Five ways, and no sixth: both generic `Partner`; `Partner with` naming
/// **each other**, which is checked in both directions because a card that
/// names a partner is only paired by the card that names it back; both
/// `Friends forever`; a Doctor's companion beside a Doctor; and a
/// Background beside the card that chooses one. Anything else is two
/// legendary creatures in one command zone, which is not a deck.
#[must_use]
pub fn may_lead_together(a: &Leader, b: &Leader) -> bool {
    if !a.eligible || !b.eligible {
        return false;
    }
    match (a.partner, b.partner) {
        (PartnerKind::Partner, PartnerKind::Partner)
        | (PartnerKind::FriendsForever, PartnerKind::FriendsForever) => true,
        (PartnerKind::PartnerWith(mine), PartnerKind::PartnerWith(theirs)) => {
            mine == b.index && theirs == a.index
        }
        (PartnerKind::DoctorsCompanion, _) => b.doctor,
        (_, PartnerKind::DoctorsCompanion) => a.doctor,
        (PartnerKind::ChooseABackground, _) => b.background,
        (_, PartnerKind::ChooseABackground) => a.background,
        _ => false,
    }
}

/// [`Leader`] read off a card in the registry.
///
/// `None` for an index this build does not have, which a caller turns into
/// "unknown card" rather than into "not a legal pair".
#[must_use]
pub fn leader_of(index: CardIndex) -> Option<Leader> {
    let def = crate::by_index(index)?;
    let face = &def.faces[0];
    Some(Leader {
        index: def.index,
        eligible: !matches!(def.commander, CommanderRule::NotEligible),
        partner: def.partner,
        doctor: face.subtypes.contains(&subtypes::creature::DOCTOR),
        background: face.subtypes.contains(&subtypes::enchantment::BACKGROUND),
    })
}

/// The format a table of these decks is playing.
///
/// A deck that named a commander is playing Commander; anything else is
/// `Freeform`, as every table was before this existed. The distinction buys
/// one thing today — 40 starting life (CR 903.7), the only `FormatId` the
/// engine reads — and it is deliberately *not* what puts commanders in the
/// command zone. `SeatSpec::commanders` does that on its own, so a test can
/// seat a commander without also inheriting a format's life total.
fn format_for<'a>(mut decks: impl Iterator<Item = &'a LoadedDeck>) -> FormatId {
    if decks.any(|d| !d.commanders.is_empty()) {
        FormatId::Commander
    } else {
        FormatId::Freeform
    }
}

/// Builds a two-player preset from two loaded decks.
#[must_use]
pub fn preset_for(seed: u64, a: &LoadedDeck, b: &LoadedDeck) -> GamePreset {
    let mut prints: Vec<PrintInfo> = Vec::new();
    // One closure for both lists: the print table is shared, so two closures
    // holding `prints` would each want it mutably.
    let mut entries = |cards: &[DeckCard]| -> Vec<DeckEntry> {
        cards
            .iter()
            .map(|card| DeckEntry {
                card: card.index,
                print: print_ref_for(&mut prints, &card.print),
            })
            .collect()
    };
    let seat = |entries: Vec<DeckEntry>, side: Vec<DeckEntry>, cmd: Vec<DeckEntry>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: entries,
        sideboard: side,
        commanders: cmd,
        starting_life: None,
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    let seats = vec![
        seat(
            entries(&a.main),
            entries(&a.sideboard),
            entries(&a.commanders),
        ),
        seat(
            entries(&b.main),
            entries(&b.sideboard),
            entries(&b.commanders),
        ),
    ];
    GamePreset {
        format: format_for([a, b].into_iter()),
        seed,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints,
        seats,
    }
}

/// A preset for a table of any size, one seat per deck.
///
/// [`preset_for`] is the two-deck case and the one most callers want; a room
/// where the host chose how many chairs there are needs this. The print table
/// is still shared and still deduplicated across every deck at the table —
/// which is exactly why this cannot be a fold over `preset_for`.
///
/// Every seat comes out as an AI, as in the two-deck case: who actually sits
/// where is the caller's business, and the gateway overwrites the controller
/// per seat before the engine ever sees it.
#[must_use]
pub fn preset_for_all(seed: u64, decks: &[&LoadedDeck]) -> GamePreset {
    let mut prints: Vec<PrintInfo> = Vec::new();
    let mut entries = |cards: &[DeckCard]| -> Vec<DeckEntry> {
        cards
            .iter()
            .map(|card| DeckEntry {
                card: card.index,
                print: print_ref_for(&mut prints, &card.print),
            })
            .collect()
    };
    let seats = decks
        .iter()
        .map(|deck| SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: baylee_core::preset::SeatCapabilities::default(),
            deck: entries(&deck.main),
            sideboard: entries(&deck.sideboard),
            commanders: entries(&deck.commanders),
            starting_life: None,
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        })
        .collect();
    GamePreset {
        format: format_for(decks.iter().copied()),
        seed,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints,
        seats,
    }
}

/// The five basic lands, by registry index, in colour order.
///
/// Resolved by name because that is the only handle a basic has that does
/// not move: the ledger hands out indices in the order cards were added, so
/// hard-coding them would break the first time somebody reorders the pool.
#[must_use]
pub fn basic_lands() -> [Option<CardIndex>; 5] {
    ["Plains", "Island", "Swamp", "Mountain", "Forest"].map(by_name)
}

/// A deck built to put one card into a real game: `copies` of it, padded to
/// `size` with basic lands in its own colours.
///
/// This is what lets the self-play harness reach a card that no acceptance
/// deck contains. The padding follows the card's colour identity so that a
/// spell in the deck is actually castable — a deck of Islands will never
/// cast a Forest card, and a card that is never cast proves nothing.
/// A colourless identity pads with all five, which keeps generic costs
/// payable without pretending to a colour.
///
/// Returns `None` when the registry has no basic lands to pad with.
#[must_use]
pub fn probe_deck(card: CardIndex, copies: usize, size: usize) -> Option<LoadedDeck> {
    let basics = basic_lands();
    let def = by_index(card)?;
    let identity = def.color_identity;
    let wanted: Vec<CardIndex> = if identity.is_empty() {
        basics.iter().flatten().copied().collect()
    } else {
        identity.iter().filter_map(|c| basics[c as usize]).collect()
    };
    // A colour whose basic is not registered falls back to whatever is.
    let pad: Vec<CardIndex> = if wanted.is_empty() {
        basics.iter().flatten().copied().collect()
    } else {
        wanted
    };
    if pad.is_empty() {
        return None;
    }
    let mut main: Vec<DeckCard> = (0..copies).map(|_| DeckCard::plain(card)).collect();
    for i in main.len()..size {
        main.push(DeckCard::plain(pad[i % pad.len()]));
    }
    Some(LoadedDeck {
        name: format!("probe: {}", def.name()),
        main,
        sideboard: vec![],
        commanders: vec![],
    })
}
/// Which zone a hand-dealt card goes into.
///
/// The two differ in more than a field name. A board is a *list* of
/// permanents and is appended to; a `starting_hand` is the whole opening hand
/// and **replaces** the deal for the seat it names, so an empty spec leaves
/// the deal alone and a non-empty one is the hand, exactly.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DevZone {
    /// [`SeatSpec::starting_battlefield`], appended to.
    Battlefield,
    /// [`SeatSpec::starting_hand`], replacing whatever would have been drawn.
    Hand,
}

/// Deals a hand-written list of cards into one zone of a preset.
///
/// The spec is a **semicolon**-separated list of card names, each optionally
/// prefixed with a seat and a colon: `Kazandu Blademaster; 1:Baleful Strix`
/// puts the first on seat 0's side and the second on seat 1's. The separator
/// is a semicolon because a comma is part of a card's name far too often —
/// "Sokka, Tenacious Tactician" — and a colon only counts as a seat prefix
/// when what precedes it is a number, for the same reason.
///
/// The cards arrive before turn one, through the same `SeatSpec` fields the
/// duel-flow tests use, so the engine treats them exactly as it treats a boss
/// board or a puzzle. It exists because the alternative is playing a duel
/// into position, and a singleton in a ninety-card deck is not something a
/// game reaches on request.
///
/// It takes the spec as an **argument** and reads no environment of its own.
/// Every caller is a development harness that has already decided it is one,
/// and a library that seated cards straight from the environment would put
/// that decision somewhere no binary gates it: the client's `dev-control` and
/// the gateway's `dev-table` are the two gates, and both call this.
///
/// # Errors
///
/// On a card name no printing in the registry answers to, a seat this game
/// does not have, or a card with no printing id to draw. Never silently,
/// because a typo that dealt nothing turns "this card does not work" into a
/// conclusion about the rules.
pub fn deal_named(preset: &mut GamePreset, spec: &str, zone: DevZone) -> Result<(), String> {
    for wanted in spec.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        let (seat, name) = match wanted.split_once(':') {
            Some((before, after)) if before.trim().parse::<usize>().is_ok() => (
                before.trim().parse::<usize>().unwrap_or_default(),
                after.trim(),
            ),
            _ => (0, wanted),
        };
        let card = crate::all()
            .find(|def| {
                def.faces
                    .first()
                    .is_some_and(|face| face.name.eq_ignore_ascii_case(name))
            })
            .ok_or_else(|| format!("no card named `{name}`"))?;
        // The card's **own** printing, and not `PrintRef::new(0)`.
        //
        // Print 0 is whatever the first card of the first decklist happened
        // to resolve to, so a dealt board wore a stranger's picture — and now
        // that a stack entry draws its printed sentence, it would have asked
        // the catalog for a stranger's text as well. That is the one failure
        // this must not have: a measurement of a board it never dealt. The
        // entry is appended to the game's own print table, which is what a
        // seat's entitlement is counted against, and deduplicated because a
        // board may deal two of a card.
        //
        // It asks [`reference_print`] rather than building the same three
        // fields a second time, because the dedup compares the **whole**
        // struct and the two constructions have to be identical for it to
        // fire at all. This line once wrote `"EN"` where a decklist writes
        // `"en"`, so every dealt card the deck already carried was appended a
        // second time under the same printing id — and `cardtext::absorb`,
        // which files an answer against the first index claiming that id,
        // left the dealt copy with no card text whatsoever. Six of ten cards
        // on a hand-dealt board came out blank, which reads exactly like a
        // hole in the catalog and is not one.
        let want = reference_print(card.index);
        if want.scryfall_id.is_nil() {
            return Err(format!("`{name}` has no printing id to draw"));
        }
        let print = print_ref_for(&mut preset.prints, &want);
        let chair = preset
            .seats
            .get_mut(seat)
            .ok_or_else(|| format!("this game has no seat {seat}"))?;
        let entry = DeckEntry {
            card: card.index,
            print,
        };
        match zone {
            DevZone::Battlefield => chair.starting_battlefield.push(entry),
            DevZone::Hand => chair.starting_hand.get_or_insert_default().push(entry),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn some_card() -> CardIndex {
        by_name("Forest").expect("the registry has Forest")
    }

    #[test]
    fn a_deck_that_names_no_printing_gets_the_registrys_own() {
        let card = some_card();
        let plain = DeckCard::plain(card);
        assert_eq!(plain.print, reference_print(card));
        assert_eq!(plain.print.finish, Finish::Normal);
        assert_eq!(plain.print.lang, "en");
        assert!(
            !plain.print.scryfall_id.is_nil(),
            "a nil id is one guaranteed 404 per card"
        );
    }

    #[test]
    fn a_row_that_names_a_printing_gets_that_one() {
        let card = some_card();
        let chosen = DeckCard::chosen(
            card,
            &PrintChoice {
                lang: Some("de".to_string()),
                finish: Some(Finish::Etched),
                scryfall_id: Some("11111111-2222-3333-4444-555555555555".to_string()),
                ..PrintChoice::default()
            },
        );
        assert_eq!(chosen.index, card, "the rules identity is untouched");
        assert_eq!(chosen.print.lang, "de");
        assert_eq!(chosen.print.finish, Finish::Etched);
        assert_eq!(
            chosen.print.scryfall_id.to_string(),
            "11111111-2222-3333-4444-555555555555"
        );
    }

    /// A choice narrows field by field: naming only the finish keeps the
    /// reference printing's id and language.
    #[test]
    fn an_unstated_field_falls_back_to_the_reference_printing() {
        let card = some_card();
        let foil = DeckCard::chosen(
            card,
            &PrintChoice {
                finish: Some(Finish::Foil),
                ..PrintChoice::default()
            },
        );
        assert_eq!(foil.print.scryfall_id, reference_print(card).scryfall_id);
        assert_eq!(foil.print.lang, "en");
        assert_eq!(foil.print.finish, Finish::Foil);
    }

    /// A garbled id is ignored rather than fatal: the deck still plays, with
    /// the reference art. Refusing to start a game over an image would be the
    /// wrong trade every time.
    #[test]
    fn an_unreadable_printing_id_falls_back_rather_than_failing() {
        let card = some_card();
        let broken = DeckCard::chosen(
            card,
            &PrintChoice {
                scryfall_id: Some("not-a-uuid".to_string()),
                ..PrintChoice::default()
            },
        );
        assert_eq!(broken.print.scryfall_id, reference_print(card).scryfall_id);
    }

    /// The print table is deduplicated, and the finish is part of what makes
    /// two printings different: a foil and a non-foil copy sharing a slot
    /// would put one of them in the other's finish.
    #[test]
    fn a_foil_and_a_plain_copy_do_not_share_a_print_slot() {
        let card = some_card();
        let plain = DeckCard::plain(card);
        let foil = DeckCard::chosen(
            card,
            &PrintChoice {
                finish: Some(Finish::Foil),
                ..PrintChoice::default()
            },
        );
        let deck = LoadedDeck {
            name: "mixed".to_string(),
            main: vec![plain.clone(), foil.clone(), plain.clone()],
            sideboard: vec![],
            commanders: vec![],
        };
        let preset = preset_for(1, &deck, &deck);
        assert_eq!(
            preset.prints.len(),
            2,
            "two printings, three copies: {:?}",
            preset.prints
        );
        let refs: Vec<_> = preset.seats[0].deck.iter().map(|e| e.print).collect();
        assert_eq!(refs[0], refs[2], "the two plain copies share a slot");
        assert_ne!(refs[0], refs[1], "the foil does not");
        assert_eq!(
            preset.prints[refs[1].get() as usize].finish,
            Finish::Foil,
            "and its slot is the one carrying the foil"
        );
    }

    /// The acceptance decks name no printings, so every card in them lands on
    /// the registry's own — which is what makes their art load at all.
    #[test]
    fn the_acceptance_decks_load_with_real_printings() {
        let text = include_str!("../../../data/acceptance-decks.txt");
        let deck = load_acceptance(text, "Allytifact").expect("Allytifact loads");
        assert!(!deck.main.is_empty());
        for card in &deck.main {
            assert!(
                !card.print.scryfall_id.is_nil(),
                "{:?} has no printing",
                card.index
            );
            assert_eq!(card.print.finish, Finish::Normal);
        }
    }
}

#[cfg(test)]
mod name_table_tests {
    use super::by_name;
    use crate::generated_names::NAMES;
    use std::collections::HashMap;

    #[test]
    fn every_card_in_the_pool_finds_itself_by_name() {
        for def in crate::all() {
            assert_eq!(
                by_name(def.name()),
                Some(def.index),
                "{} does not resolve to itself",
                def.name()
            );
        }
    }

    /// The table is regenerated from the compiled pool, so a card added
    /// without a second `codegen` run would leave it short. `codegen --check`
    /// says the same thing in CI; this says it to whoever is working.
    #[test]
    fn the_table_holds_the_whole_pool_and_nothing_else() {
        assert_eq!(NAMES.len(), crate::count());
        for (spelling, index) in NAMES {
            let def = crate::by_index(index).expect("a name resolves to a card in the pool");
            assert_eq!(def.name(), spelling);
        }
    }

    /// The owner's invariant, and the reason the generator can build the hash
    /// at all: two cards with one name have no answer `by_name` could give.
    #[test]
    fn no_two_cards_in_the_pool_share_a_name() {
        let mut seen: HashMap<&str, &str> = HashMap::new();
        for def in crate::all() {
            if let Some(other) = seen.insert(def.name(), def.name()) {
                panic!("two cards are named {}: {other}", def.name());
            }
        }
    }

    /// A perfect hash answers for *every* string, so the spelling stored
    /// beside the answer is what separates "no such card" from the wrong one.
    ///
    /// Deleting the comparison in `by_name` fails this about nine hundred
    /// times: two thirds of the table's slots are occupied, so that is
    /// roughly how often a name nobody prints lands on top of a real card.
    #[test]
    fn a_name_the_pool_does_not_have_never_answers_a_card() {
        let mut mutants: Vec<String> = Vec::new();
        for def in crate::all() {
            mutants.push(format!("{}x", def.name()));
            mutants.push(def.name().to_lowercase());
            let mut short = def.name().to_string();
            short.pop();
            mutants.push(short);
        }
        mutants.push(String::new());
        mutants.push("Not A Card At All".into());
        for m in &mutants {
            if let Some(index) = by_name(m) {
                let def = crate::by_index(index).expect("in the pool");
                assert_eq!(
                    def.name(),
                    m.as_str(),
                    "{m:?} resolved to a card that is not called that"
                );
            }
        }
    }
}

#[cfg(test)]
mod partner_tests {
    use super::*;

    use baylee_core::generated::index;

    fn leader(index: CardIndex, partner: PartnerKind) -> Leader {
        Leader {
            index,
            eligible: true,
            partner,
            doctor: false,
            background: false,
        }
    }

    /// Both halves of every arm, which is the point of a [`Leader`] being
    /// built by hand rather than read out of the registry: the pool holds
    /// exactly one card with a partner ability, so nothing built out of the
    /// compiled pool could ever show this function saying yes. The indices
    /// are real all the same — the ledger numbers every card there is, so a
    /// pair this build compiles neither half of is still named exactly.
    #[test]
    fn two_leaders_pair_only_the_five_ways_the_rules_allow() {
        let thrasios = leader(index::THRASIOS_TRITON_HERO, PartnerKind::Partner);
        let tymna = leader(index::TYMNA_THE_WEAVER, PartnerKind::Partner);
        assert!(may_lead_together(&thrasios, &tymna), "both have partner");

        let alone = leader(index::KATARA_THE_FEARLESS, PartnerKind::None);
        assert!(
            !may_lead_together(&thrasios, &alone),
            "one partner and one ordinary legend is not a pair"
        );
        assert!(
            !may_lead_together(&alone, &alone),
            "two ordinary legends are two decks"
        );

        // `Partner with` is a named pair, and naming is not mutual by
        // accident: a card is only paired by the card that names it back.
        let pir = leader(
            index::PIR_IMAGINATIVE_RASCAL,
            PartnerKind::PartnerWith(index::TOOTHY_IMAGINARY_FRIEND),
        );
        let toothy = leader(
            index::TOOTHY_IMAGINARY_FRIEND,
            PartnerKind::PartnerWith(index::PIR_IMAGINATIVE_RASCAL),
        );
        let stranger = leader(
            index::SAKASHIMA_OF_A_THOUSAND_FACES,
            PartnerKind::PartnerWith(index::TOOTHY_IMAGINARY_FRIEND),
        );
        assert!(may_lead_together(&pir, &toothy));
        assert!(may_lead_together(&toothy, &pir), "and the other way round");
        assert!(
            !may_lead_together(&stranger, &toothy),
            "naming a card does not make it name you back"
        );

        let a = leader(index::WILSON_REFINED_GRIZZLY, PartnerKind::FriendsForever);
        let b = leader(index::ZINNIA_VALLEY_S_VOICE, PartnerKind::FriendsForever);
        assert!(may_lead_together(&a, &b));
        assert!(
            !may_lead_together(&a, &thrasios),
            "friends forever does not pair with partner"
        );

        let companion = leader(index::ROSE_TYLER, PartnerKind::DoctorsCompanion);
        let mut doctor = leader(index::THE_TENTH_DOCTOR, PartnerKind::None);
        doctor.doctor = true;
        assert!(may_lead_together(&companion, &doctor));
        assert!(may_lead_together(&doctor, &companion), "either order");
        assert!(
            !may_lead_together(&companion, &alone),
            "a companion needs an actual Doctor"
        );

        let chooser = leader(
            index::WILSON_REFINED_GRIZZLY,
            PartnerKind::ChooseABackground,
        );
        let mut background = leader(index::CRIMINAL_PAST, PartnerKind::None);
        background.background = true;
        assert!(may_lead_together(&chooser, &background));
        assert!(
            !may_lead_together(&chooser, &alone),
            "a card that chooses a Background needs one"
        );
    }

    /// A card that may not lead a deck may not lead half of one either.
    #[test]
    fn a_card_that_cannot_be_a_commander_cannot_be_a_partner() {
        let mut ineligible = leader(index::LLANOWAR_ELVES, PartnerKind::Partner);
        ineligible.eligible = false;
        let partner = leader(index::THRASIOS_TRITON_HERO, PartnerKind::Partner);
        assert!(!may_lead_together(&ineligible, &partner));
        assert!(!may_lead_together(&partner, &ineligible));
        // The counter-half: the same pair with eligibility restored does.
        ineligible.eligible = true;
        assert!(may_lead_together(&ineligible, &partner));
    }

    /// The one card in the pool that has a partner ability reads as one.
    ///
    /// The bridge from the rule to the registry: `leader_of` is what a route
    /// calls, and without this the pure function above could be right about
    /// characteristics nothing ever supplies.
    #[test]
    fn the_registry_supplies_what_the_rule_asks_for() {
        let index = by_name("Sakashima of a Thousand Faces").expect("in the pool");
        let read = leader_of(index).expect("a card in the pool has a leader reading");
        assert_eq!(read.index, index);
        assert!(read.eligible, "a legendary creature may lead a deck");
        assert_eq!(read.partner, PartnerKind::Partner);

        let plain = by_name("Forest").expect("in the pool");
        let read = leader_of(plain).expect("a land has a reading too");
        assert!(!read.eligible, "a Forest may not lead a deck");
        assert_eq!(read.partner, PartnerKind::None);
    }
}
