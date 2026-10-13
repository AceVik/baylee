//! Stack sentence lookup and text measurement.

#[allow(clippy::wildcard_imports)] // the stack panel shares one drawing vocabulary
use super::*;

/// The line under the stack's title: whose answer the table is waiting for.
///
/// The local seat is **addressed**, not named. Its display name is the
/// language's own pronoun — offline it is literally `Phrase::You` — and this
/// sentence needs the accusative in German, so filling the slot with it reads
/// "wartet auf Du". [`Phrase::WaitingForYou`] is the sentence written out
/// instead, which is the fix that generalises: a slot that needs a different
/// word in different sentences is a slot that gets filled wrongly.
pub(in crate::hud) fn waiting_line(lang: Lang, name: &str, is_me: bool) -> String {
    if is_me {
        Phrase::WaitingForYou.text(lang).to_string()
    } else {
        Phrase::WaitingFor.fill(lang, &[name])
    }
}

/// The printed sentence a stack entry stands for, in the player's own
/// language, or `None` when there is nothing trustworthy to draw.
///
/// Captured token abilities use the verified token sentence table. Other
/// entries need the host's card and sentence; an unknown token sentence,
/// emblem or granted ability falls back to the
/// label it drew before. The source need not still be there: text is filed
/// under the card the entry's `rules` names, and that is a property of the
/// ability itself, which outlives its source (CR 113.7a). The words are
/// [`crate::cardtext::sentence`]'s, which falls to the card's English Oracle
/// when the player's language has none that pairs — offline, always.
///
/// The face is the host's answer and not the source's current one, for
/// the reason `baylee_view::StackText::face` gives.
///
/// The line is the host's too, counted in the host's build of the card
/// text, and `sentence` refuses it where this build counts the face
/// differently. The entry's [`AbilityRef`](baylee_core::ids::AbilityRef)
/// then places the ability in this build's own line table, as the ability
/// sheet always does — the right sentence and not a neighbour of it, which
/// is what a line counted in other text would point at. Only an entry with
/// no such handle is left to its source's name.
pub(in crate::hud) fn stack_sentence(
    item: &baylee_client_core::board::StackItem,
    faces: &FaceCtx<'_>,
) -> Option<Vec<TextBlock>> {
    let said = printed_sentence(item, faces);
    // The modes chosen for it (#23): every player is told them as it is
    // put on the stack (CR 601.2b, 603.3c), and the card alone cannot say
    // which they were. A modal spell has no sentence of its own here, and a
    // modal trigger's says only "choose one —".
    let Some(modes) = chosen_modes(item, faces) else {
        return said;
    };
    let mut blocks = said.unwrap_or_default();
    blocks.extend(modes);
    Some(blocks)
}

/// The chosen modes of a stack entry, one block each, in the order the card
/// prints them (CR 700.2a's order of resolution is that one, 608.2c).
fn chosen_modes(
    item: &baylee_client_core::board::StackItem,
    faces: &FaceCtx<'_>,
) -> Option<Vec<TextBlock>> {
    let (rules, set) = item.modes?;
    let lang = Lang::of(&faces.settings.lang);
    let blocks: Vec<TextBlock> = (0..8_usize)
        .filter(|i| set & (1 << i) != 0)
        .map(|i| {
            let words = crate::choices::mode_words(Some(faces.texts), rules.card, i)
                .unwrap_or_else(|| Phrase::CastModeNumber.fill(lang, &[&(i + 1).to_string()]));
            TextBlock::Rules(format!("\u{2022} {words}"))
        })
        .collect();
    (!blocks.is_empty()).then_some(blocks)
}

/// The printed sentence of an ability on the stack: [`stack_sentence`]
/// before the modes.
fn printed_sentence(
    item: &baylee_client_core::board::StackItem,
    faces: &FaceCtx<'_>,
) -> Option<Vec<TextBlock>> {
    let baylee_client_core::board::StackKind::Ability {
        text,
        rules,
        ability,
        token,
        ..
    } = item.kind
    else {
        return None;
    };
    if let Some(token) = token {
        return crate::cardtext::token_sentence(token.token, token.index);
    }
    let rules = rules?;
    let card = rules.card;
    text.and_then(|at| crate::cardtext::sentence(Some(faces.texts), card, at))
        .or_else(|| {
            let ability = ability.filter(|ability| ability.card == card)?;
            let face = usize::from(rules.face);
            if let Some(own) = baylee_cards::lines::ability_line(card, face, ability.index) {
                return crate::cardtext::sentence(
                    Some(faces.texts),
                    card,
                    baylee_view::StackText {
                        face: rules.face,
                        line: own.line,
                        of: own.of,
                    },
                );
            }
            // Embedded grants (Vesuvan's upkeep ability) have no standalone
            // sentence. Preserve the complete source Oracle, never the face
            // the permanent currently copies or an invented short version.
            baylee_cards::by_index(card)?
                .abilities_for_face(face)
                .get(usize::try_from(ability.index).ok()?)?;
            Some(baylee_client_core::card_face::split_blocks(
                baylee_cards::oracle::face(card, face)?,
            ))
        })
}

/// One-line heading: preserve plain-name truncation and render symbols before budgeting.
pub(in crate::hud) fn queued_heading_spans(title: &str, room: f32, size: f32) -> Vec<Piece> {
    if manapip::inline(title)
        .iter()
        .any(|piece| matches!(piece, manapip::Inline::Mark(_)))
    {
        spans_of(
            &[TextBlock::Rules(title.to_string())],
            Some(budget(room, size)),
        )
    } else {
        vec![Piece {
            text: fit(title, room, size),
            mark: false,
            reminder: false,
        }]
    }
}

/// What a **queued** ability row is headed, which is not its source's name.
///
/// A queued row answers "what else is coming", and for an ability the
/// source's name answers it badly: #132 was reported off a stack of five
/// where two rows read `Sheoldred, the Apocalypse` and nothing told them
/// apart. This file already makes that argument for the full row —
/// *"a stack of three triggers that all read `Ability · Ondu Cleric` tells
/// them less"* — and the queue was simply not held to it.
///
/// An ability has no name of its own, so the printed sentence *is* its
/// identity, and the picture beside the row already says which permanent it
/// came from. That is also why this costs no height: it is the line the row
/// already had, carrying something that distinguishes it.
///
/// Reminder text is dropped. It is parenthetical by definition (CR 207.2)
/// and this is one line — the reminder would be the half a player does not
/// need, taking the room from the half they do. Symbol source stays intact
/// here; the heading renderer splits it into Manafont spans before budgeting.
///
/// [`None`] whenever the sentence is not *known*, and the caller then draws
/// the name as before: the host sends no line index for some abilities (a
/// token's, an emblem's, a granted one's). Where it does, the words are
/// there offline too — the English Oracle is compiled in — so a row never
/// comes out blank because a request did not get through.
pub(in crate::hud) fn queued_ability_line(
    item: &baylee_client_core::board::StackItem,
    faces: &FaceCtx<'_>,
) -> Option<String> {
    // The printed sentence alone: a queued spell keeps its name, and a
    // queued modal trigger its "choose one —" without the modes under it.
    ability_line(&printed_sentence(item, faces)?)
}

/// What a row is headed, given the `name` its object is drawn under.
///
/// The **full** row keeps the name whatever it is: it carries the printed
/// sentence on its own line underneath, so the name is the one thing there
/// that is not already said. A **queued ability** does not, and its name is
/// its *source's* — two triggers off one permanent were the same row twice.
///
/// The fallback is the name, not an empty line: see [`queued_ability_line`]
/// for the two ordinary ways the sentence is simply not known yet.
pub(in crate::hud) fn heading(
    item: &baylee_client_core::board::StackItem,
    full: bool,
    name: String,
    faces: &FaceCtx<'_>,
) -> String {
    if full {
        return name;
    }
    queued_ability_line(item, faces).unwrap_or(name)
}

/// The prose half of [`queued_ability_line`], with the two lookups taken
/// out — which is what makes the rule testable without a print table and a
/// catalog behind it.
///
/// Re-joining on whitespace rather than concatenating is doing work: a
/// reminder cut out of the middle of a sentence leaves the space that was in
/// front of it and the space that was behind it, and two spaces in the
/// middle of a one-line heading read as a missing word.
pub(in crate::hud) fn ability_line(blocks: &[TextBlock]) -> Option<String> {
    let joined = blocks
        .iter()
        .filter_map(|block| match block {
            TextBlock::Rules(text) => Some(text.as_str()),
            TextBlock::Reminder(_) => None,
        })
        .collect::<Vec<_>>()
        .join(" ");
    let line = joined.split_whitespace().collect::<Vec<_>>().join(" ");
    (!line.is_empty()).then_some(line)
}

/// One run of a sentence as it is drawn: prose, or a single mana mark.
///
/// A mark is its own piece because it is set in a different font at a
/// different size ([`STACK_MARK`]), and a reminder is flagged rather than
/// separated because the brackets it is drawn inside are pieces too and have
/// to carry the same slant and the same ink.
pub(in crate::hud) struct Piece {
    /// The characters.
    pub text: String,
    /// One glyph of `mana.ttf`, rather than prose.
    pub mark: bool,
    /// Reminder text (CR 207.2), including its own `" ("` and `")"`.
    pub reminder: bool,
}

/// Every run of `blocks`, in printed order, with a reminder's brackets in
/// place and the whole sentence held to `room` characters.
///
/// `room` is `None` for a surface that **wraps** instead of cutting, which is
/// the difference between the two places this is drawn. The stack panel's full
/// row is budgeted — its height is what the queue under it is measured
/// against, so a pathological card must not be able to push the queue off the
/// panel — while the slip under the hover preview is the place that answers
/// "what *exactly* is about to happen" and cuts nothing at all.
///
/// The order inside a block is split-then-budget, and that is the part worth
/// keeping: `{T}` is three characters of source and one mark on screen, so
/// cutting the line before the split drops text there was room for.
///
/// The three characters taken off a reminder's budget are its `" (…)"`, and
/// they come off before its first piece is measured — so what a block hands
/// on to the next one is what it did not spend.
pub(in crate::hud) fn spans_of(blocks: &[TextBlock], room: Option<usize>) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut left = room;
    for block in blocks {
        let reminder = matches!(block, TextBlock::Reminder(_));
        let mut room_for = left.map(|left| {
            if reminder {
                left.saturating_sub(3)
            } else {
                left
            }
        });
        if room_for == Some(0) {
            break;
        }
        let mut pieces: Vec<(String, bool)> = Vec::new();
        for piece in manapip::inline(block.text()) {
            if room_for == Some(0) {
                break;
            }
            match piece {
                manapip::Inline::Mark(mark) => {
                    room_for = room_for.map(|room| room - 1);
                    pieces.push((mark.to_string(), true));
                }
                manapip::Inline::Text(run) => {
                    // Prose, so the cut is [`cut_words`] — see its doc for
                    // why a name and a sentence are cut differently.
                    let run = match room_for {
                        Some(room) => cut_words(&run, room),
                        None => run,
                    };
                    room_for = room_for.map(|room| room - run.chars().count());
                    pieces.push((run, false));
                }
            }
        }
        if pieces.is_empty() {
            continue;
        }
        left = room_for;
        if reminder {
            pieces.insert(0, (" (".to_string(), false));
            pieces.push((")".to_string(), false));
        }
        out.extend(pieces.into_iter().map(|(text, mark)| Piece {
            text,
            mark,
            reminder,
        }));
    }
    out
}

/// The average advance of a lower-case letter, as a fraction of the size.
///
/// Measured, not guessed, and it survived the change of face by arithmetic
/// rather than by luck. Inter's lower case averaged 0.531 of its size and
/// 0.52 was that, rounded down to be generous. Alegreya Sans measures 0.445
/// of the size it is *rendered* at and Faustina 0.473, and they are rendered
/// at 1.32x and 1.21x the nominal size a caller passes — **0.587 and 0.572**
/// against the nominal. The budget uses their midpoint, keeping truncation
/// in step with the larger readability scale.
pub(in crate::hud) const CHAR_WIDTH: f32 = 0.58;

/// A card name cut to one line `room` pixels wide, set at `size`.
///
/// A serif's lower case averages a little over half its point size and a card
/// name is mostly lower case, so [`CHAR_WIDTH`] is the budget ratio —
/// deliberately generous, because the cost of guessing narrow is one word
/// clipped by the body's own `overflow` and the cost of guessing wide is a
/// name cut short that would have fitted.
///
/// **It is a guess and the widening is what makes it a safe one.** Measured
/// against the shipped `Faustina.ttf`'s own advance widths, a real name runs
/// between 0.36 (`Teferi's Isle`) and 0.65 (`Damn`) of its length times its
/// size, not 0.52 — a ratio that is wrong in **both** directions at once, so
/// at the old 187-pixel column it both cut names that would have fitted and
/// passed names that then clipped. (In `Inter.ttf`, which this replaced, the
/// spread was 0.41 to 0.70 and the widest name set 227 px against Faustina's
/// 202.) The only caller of `fit` is the
/// **queued** row, whose column is now 267 px at 14: over all 1475 faces in
/// the pool the budget is 36 characters, nothing clips and nothing is cut
/// that would have fitted — the estimator is exercised and wrong about
/// nothing. The full row does not call it at all; it wraps. If a name column
/// is ever narrowed again, this is the thing to replace with a real
/// measurement rather than to re-tune.
///
/// **[`budget`] has a second consumer, and the sentence it serves is the one
/// this estimate is worst for.** That paragraph above said "the only caller
/// left" and meant `fit`; [`spawn_stack_sentence`] calls `budget` directly
/// for a *wrapped* run of prose, where a per-character average is not the
/// quantity wanted at all. Wrapping breaks at words, so each break gives up
/// part of a line — a German compound gives up most of one — and the
/// character count that fits in four lines of prose is well under four times
/// the character count that fits in one. #132 reports the consequence from
/// the drawing: a sentence cut mid-word with room still in the panel. The
/// mid-word half is fixed ([`cut_words`]); the budget itself is not, because
/// choosing it needs the measurement the report has and this file does not.
///
/// The ellipsis replaces characters rather than joining them, so the result
/// never grows past the budget, and the cut is on `char` boundaries because a
/// card name is not ASCII (Æther Vial, Márton Stromgald).
pub(in crate::hud) fn fit(name: &str, room: f32, size: f32) -> String {
    cut(name, budget(room, size))
}

/// How many characters of `size`-point text fit in `room` pixels.
///
/// [`CHAR_WIDTH`] is the ratio, and it is a nominal size that goes in: the
/// two faces are asked for a scaled one ([`super::UI_SCALE`],
/// [`super::SERIF_SCALE`]), and the scales were chosen so that the product
/// lands back where Inter was.
///
/// Split out of [`fit`] because a *sentence* spends one budget across several
/// spans — rules text and its reminder are two — and each span cutting itself
/// to the full budget would let the pair run twice as long as the room they
/// share.
pub(in crate::hud) fn budget(room: f32, size: f32) -> usize {
    (room / (size * CHAR_WIDTH)).max(4.0) as usize
}

/// `text`, cut to `budget` characters with an ellipsis if it is longer.
pub(in crate::hud) fn cut(text: &str, budget: usize) -> String {
    if text.chars().count() <= budget {
        return text.to_string();
    }
    let mut cut: String = text.chars().take(budget.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

/// The same, cut at the last **word** boundary that fits.
///
/// A card name is cut mid-word without anybody minding — a name is one thing
/// and half of it still points at the card. A *sentence* is not: a rules line
/// that stops at `…kannst du eine +1/+1-Marke auf den Klin…` reads as a
/// rendering fault rather than as text that goes on, and the fragment is
/// worth nothing. Which is why this is a second function and not a change to
/// [`cut`]: the two callers want different things, and a name cut to its last
/// whole word would lose most of `Asmoranomardicadaistinaculdacar`.
///
/// **A word longer than the whole budget falls back to the character cut.**
/// Backing off to the previous boundary would then be backing off to nothing
/// and returning a bare ellipsis, which says less than a cut word does — and
/// it is a real case, because the budget shrinks as spans are spent and the
/// last span can be handed four characters.
///
/// Trailing space goes before the ellipsis rather than after the cut, so the
/// mark sits against the word it follows.
pub(in crate::hud) fn cut_words(text: &str, budget: usize) -> String {
    if text.chars().count() <= budget {
        return text.to_string();
    }
    let head: String = text.chars().take(budget.saturating_sub(1)).collect();
    let Some(at) = head.rfind(char::is_whitespace) else {
        return cut(text, budget);
    };
    let kept = head[..at].trim_end();
    if kept.is_empty() {
        return cut(text, budget);
    }
    format!("{kept}…")
}
