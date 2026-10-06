//! A deck as it arrives: its body, its lines read against the pool, and
//! the rules a deck must meet before it is stored.

use crate::{Deserialize, ErrorBody, Json, Shared, StatusCode, db_down, err, store};

#[derive(Deserialize)]
pub(crate) struct DeckBody {
    pub(crate) name: String,
    pub(crate) cards: Vec<String>,
    /// Optional: a deck saved without one simply has no sideboard.
    #[serde(default)]
    pub(crate) sideboard: Vec<String>,
    /// The deck's commanders: none, one, or two under the partner rule.
    #[serde(default)]
    pub(crate) commanders: Vec<String>,
    /// The one commander an older client sends.
    ///
    /// A reader's tolerance, not a second field: a build made before the
    /// partner rule existed still saves decks, and refusing it would lose
    /// the commander rather than the feature. Nothing writes this back.
    #[serde(default)]
    pub(crate) commander: Option<String>,
    /// What the deck plays. A body that does not say keeps what the deck
    /// already said, and a new deck that does not say is read off its
    /// commander ([`baylee_cards::decks::format_of`]).
    #[serde(default)]
    pub(crate) format: Option<String>,
    /// What the deck is for, in its owner's words. Absent leaves it alone;
    /// an empty string clears it.
    #[serde(default)]
    pub(crate) description: Option<String>,
    /// What this change was called, for the deck's history.
    ///
    /// Only read when the cards actually change — a save that renames the
    /// deck writes no version, so there is nothing for a summary to be
    /// attached to.
    #[serde(default)]
    pub(crate) summary: Option<String>,
    /// Image id of the deck's sleeve, from `POST /images?kind=sleeve`.
    ///
    /// The caller must have uploaded it ([`check_pictures`]). Whether its
    /// file is still there is not asked: a sleeve that is not there is a deck
    /// that draws the generated back, which is what a deck with no sleeve
    /// does anyway.
    #[serde(default)]
    pub(crate) sleeve: Option<String>,
    /// Image id of the deck's playmat, from `POST /images?kind=playmat`,
    /// under the same rule as the sleeve.
    #[serde(default)]
    pub(crate) playmat: Option<String>,
}

impl DeckBody {
    /// The commanders this request names, however it named them.
    ///
    /// One list out of two fields: `commanders` is what a current client
    /// sends and `commander` is what an older one sends, and a body that
    /// somehow carries both is read as the list — the newer field is the one
    /// that can say everything the older one can.
    pub(crate) fn commander_names(&self) -> Vec<String> {
        if self.commanders.is_empty() {
            self.commander.clone().into_iter().collect()
        } else {
            self.commanders.clone()
        }
    }
}

/// Hard cap on the expanded card count of one deck. Comfortably above
/// every legal format size (100 for commander), far below anything that
/// could strain memory at game start.
pub(crate) const MAX_DECK_CARDS: u32 = 250;

/// Parsed lines as the flat card list the engine's preset takes, one entry
/// per copy, each carrying the printing its row named.
pub(crate) fn expand(lines: &[ParsedLine]) -> Vec<baylee_cards::decks::DeckCard> {
    let mut out = Vec::new();
    for line in lines {
        for _ in 0..line.count {
            out.push(baylee_cards::decks::DeckCard::chosen(
                line.index,
                &line.print,
            ));
        }
    }
    out
}

/// One parsed deck line: how many of which card, printed how.
pub(crate) struct ParsedLine {
    pub(crate) count: u32,
    pub(crate) index: baylee_core::ids::CardIndex,
    pub(crate) print: baylee_core::deckrow::PrintChoice,
}

/// Parses and validates deck lines. Shared by `validate_deck` and
/// `loaded_deck` so a deck can never pass one and explode the other:
/// counts are parsed here (1–4, unlimited for basic lands) and the
/// expanded total is capped at [`MAX_DECK_CARDS`].
///
/// The row grammar lives in `baylee_core::deckrow`, so a stored deck, an
/// exported file and an imported one are read by the same code. A row that
/// names only a card is the old form and still means what it meant.
///
/// The copy limit is counted **per card, not per row**. Once a printing became
/// part of a row's identity, `4 Lightning Bolt (LEA)` and `4 Lightning Bolt
/// (M10)` were two rows that each passed a per-row check — eight Bolts through
/// a route whose own error message says 1–4. The client's builder had it right
/// all along ("the copy limit is on the *card*", `deckbuilder/builder.rs`); it
/// was this side that counted the wrong thing, and being the enforcing side is
/// what made it a way to cheat rather than a display bug.
///
/// Per *list*, because this runs once for the deck and once for the sideboard
/// — which is the same split `DeckBuilder::add_print` applies.
/// Which of the two facts a name the pool does not know actually stands for.
///
/// `baylee_cards::decks::by_name` answers `None` for a typo and for Black
/// Lotus alike, and the second is the common one: this build compiles 2716 of
/// the ledger's 33 694 cards, so **92 %** of the real cards a player might
/// type are real and unavailable here. Telling that player `unknown card`
/// sends them hunting a spelling mistake they did not make — the one thing
/// the message rules out is the thing that is true.
///
/// The ledger is what can tell them apart, because it numbers every card
/// there is rather than this pool. It is asked only here, on a path that has
/// already missed in the pool's perfect hash, so a deck that imports cleanly
/// never reaches it at all.
///
/// It does not make such a card playable and does not hint that it might: the
/// deck is refused either way, and only the reason changes. `if_no_card` is
/// what to say when the name is nothing, which differs by where it was
/// written.
/// What a real card this build compiles nothing for is called, wherever it
/// is refused. One string, because a player who meets it twice through two
/// routes has met one problem.
pub(crate) const EXISTS_UNPLAYABLE: &str = "that card exists but this server cannot play it";

pub(crate) fn no_such_card(name: &str, if_no_card: &'static str) -> &'static str {
    if baylee_cards_index::row_by_name(name).is_some() {
        EXISTS_UNPLAYABLE
    } else {
        if_no_card
    }
}

pub(crate) fn parse_deck_lines(
    lines: &[String],
) -> Result<Vec<ParsedLine>, (StatusCode, Json<ErrorBody>)> {
    let mut out = Vec::with_capacity(lines.len());
    let mut copies: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();
    let mut total: u32 = 0;
    for line in lines {
        let row = baylee_core::deckrow::parse(line).map_err(|e| match e {
            baylee_core::deckrow::RowError::Count => {
                err(StatusCode::BAD_REQUEST, "malformed card count")
            }
            baylee_core::deckrow::RowError::Finish => {
                err(StatusCode::BAD_REQUEST, "unknown finish")
            }
            baylee_core::deckrow::RowError::Lang => {
                err(StatusCode::BAD_REQUEST, "unknown language")
            }
            baylee_core::deckrow::RowError::Note => {
                err(StatusCode::BAD_REQUEST, "card note too long")
            }
            baylee_core::deckrow::RowError::Shape => {
                err(StatusCode::BAD_REQUEST, "malformed card line")
            }
        })?;
        let count = row.count;
        let Some(index) = baylee_cards::decks::by_name(&row.name) else {
            return Err(err(
                StatusCode::BAD_REQUEST,
                no_such_card(&row.name, "unknown card"),
            ));
        };

        let basic_land = baylee_cards::by_index(index).is_some_and(|def| {
            def.faces[0]
                .supertypes
                .contains(baylee_core::types::SupertypeSet::BASIC)
                && def.faces[0]
                    .types
                    .contains(baylee_core::types::TypeSet::LAND)
        });
        if count == 0 {
            return Err(err(
                StatusCode::BAD_REQUEST,
                "invalid card count (1-4, unlimited for basic lands)",
            ));
        }
        if !basic_land {
            let seen = copies.entry(index.get()).or_insert(0);
            *seen = seen.saturating_add(count);
            if *seen > 4 {
                return Err(err(
                    StatusCode::BAD_REQUEST,
                    "invalid card count (1-4, unlimited for basic lands)",
                ));
            }
        }
        total = total
            .checked_add(count)
            .ok_or_else(|| err(StatusCode::BAD_REQUEST, "deck too large"))?;
        if total > MAX_DECK_CARDS {
            return Err(err(StatusCode::BAD_REQUEST, "deck too large"));
        }
        out.push(ParsedLine {
            count,
            index,
            print: row.print,
        });
    }
    Ok(out)
}

pub(crate) fn validate_deck(body: &DeckBody) -> Result<(), (StatusCode, Json<ErrorBody>)> {
    if body.name.is_empty() || body.name.len() > 64 {
        return Err(err(StatusCode::BAD_REQUEST, "invalid deck name"));
    }
    if body.cards.is_empty() || body.cards.len() > 250 {
        return Err(err(StatusCode::BAD_REQUEST, "invalid card list"));
    }
    if body.sideboard.len() > 250 {
        return Err(err(StatusCode::BAD_REQUEST, "invalid sideboard"));
    }
    // The same parser, so a sideboard cannot hold what a deck could not.
    parse_deck_lines(&body.cards)?;
    parse_deck_lines(&body.sideboard)?;
    // Three questions, not one. Every name has to resolve, every card it
    // resolves to has to be allowed to lead a deck (CR 903.3) — the check
    // used to stop at the first, so any card in the pool could be named as a
    // commander and the engine would seat it without complaint — and a
    // second one has to be allowed to lead it *with the first*.
    let named = body.commander_names();
    if named.len() > 2 {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "a deck has at most two commanders",
        ));
    }
    let mut leaders = Vec::with_capacity(named.len());
    for name in &named {
        let Some(index) = baylee_cards::decks::by_name(name) else {
            return Err(err(
                StatusCode::BAD_REQUEST,
                no_such_card(name, "unknown commander"),
            ));
        };
        let Some(leader) = baylee_cards::decks::leader_of(index) else {
            return Err(err(StatusCode::BAD_REQUEST, "unknown commander"));
        };
        if !leader.eligible {
            return Err(err(
                StatusCode::BAD_REQUEST,
                "that card cannot be a commander",
            ));
        }
        leaders.push(leader);
    }
    if let [first, second] = leaders.as_slice()
        && !baylee_cards::decks::may_lead_together(first, second)
    {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "those two cards cannot lead one deck",
        ));
    }
    Ok(())
}

/// Refuses a sleeve or playmat the caller did not upload, 403 (#292).
///
/// Everybody at a table is told the deck's picture ids
/// (`GET /games/{id}/cosmetics`), and without this, having seen a picture
/// would be enough to wear it at one's own tables. Uploading the same
/// picture makes it one's own too, which asks for no more than having the
/// picture.
pub(crate) async fn check_pictures(
    state: &Shared,
    account_id: &str,
    body: &DeckBody,
) -> Result<(), (StatusCode, Json<ErrorBody>)> {
    for id in [&body.sleeve, &body.playmat].into_iter().flatten() {
        let claimed = store::claims_upload(&state.db, id, account_id)
            .await
            .map_err(|e| db_down(&e))?;
        if !claimed {
            return Err(err(StatusCode::FORBIDDEN, "that picture is not yours"));
        }
    }
    Ok(())
}
