//! Which printed sentence each mode of a modal ability came from.

use baylee_cards_dsl::{SpellMode, effect::Effect};

use super::{LineShape, cost_fits, line_shape, sentences, without_reminder};

/// The bullet a modal card lists its modes under.
const BULLET: char = '\u{2022}';

/// Whether a printed sentence is one mode of a modal card's list: a bullet,
/// or spree's plus sign in front of the mode's own cost — "+ {1} — …"
/// (CR 702.172b: the plus sign is a bullet with a reminder in it).
fn is_mode_line(line: &str) -> bool {
    let line = line.trim_start();
    line.starts_with(BULLET) || line.starts_with("+ {")
}

/// Which printed sentence each mode of a modal spell came from.
///
/// `CastModeKind::Mode(i)` is the only thing a client is told about a mode,
/// and a chooser that can only say "Mode 2" asks a player to pick between
/// two numbers. The sentence is what turns that into the card — but only
/// where it is *known*: a label pointing one sentence off is drawn as the
/// card's own words and is worse than the number it replaced, so anything
/// this cannot read whole answers `None` for every mode of the face.
///
/// Two printings, and the text says which. A card that prints `Choose one —`
/// — or `choose up to one —`, where one mode does nothing and is how a
/// player declines — lists its modes as bullets, one per mode that does
/// something, in order. A compiled subset can omit printed modes: each
/// retained mode must then have one unambiguous effect-word match, with
/// different modes owning different bullets. Unknown effects and ambiguous
/// matches refuse the whole subset. The other shape is **overload**, which
/// prints no bullet at all: the card's body, and a keyword line naming the other
/// cost. There, a mode with a `cost_override` finds the keyword line
/// printing that cost and a mode without one is the body, and both halves
/// have to come out to exactly the right count or the card is refused.
///
/// A modal **trigger** is read by the same two, and most of the pool's are
/// bulleted (Charming Prince, Aether Channeler, Ertai Resurrected,
/// Primaris Eliminator). Two are neither, and refuse: Derevi and Inspirit,
/// Flagship Vessel print their choice *inside* one sentence — "you may tap
/// or untap target permanent", "your choice of a +1/+1 counter or two
/// charge counters" — which is a mode with no sentence of its own, and a
/// number is the better label for it.
///
/// It reads modes and not abilities, so it is not part of [`super::map`]: an
/// `AbilityDef::ModalSpell` is [`LineShape::Other`] there, which is the
/// right answer for the ability — the whole card is its text — and no
/// answer at all for the modes inside it.
#[must_use]
pub fn map_modes(modes: &[SpellMode], oracle: &str) -> Vec<Option<u8>> {
    let lines: Vec<&str> = sentences(oracle).collect();
    let refuse = || vec![None; modes.len()];
    let bullets: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| is_mode_line(line))
        .map(|(at, _)| at)
        .collect();
    if !bullets.is_empty() {
        // A mode that does nothing is the one a player declines with, and
        // it is printed nowhere: "choose up to one —" states the
        // permission in the header and then lists only the things there
        // are to choose. Ertai Resurrected is the pool's card, its third
        // mode is `mode!(&[])`, and read as a bullet's owner it cost the
        // other two theirs — three `None`s for a card that prints two
        // perfectly good sentences.
        //
        // Both halves are asked for, because either alone is a guess. The
        // printing has to say "choose up to", and the modes have to hold
        // exactly one that does nothing; a bullet count one short on its
        // own is the ordinary disagreement below, where the honest answer
        // is still to refuse the card whole.
        let declining = |m: &SpellMode| {
            m.effects.is_empty()
                && m.targets.is_none()
                && m.cost_override.is_none()
                && m.additional_cost.is_none()
        };
        let spoken: Vec<usize> = modes
            .iter()
            .enumerate()
            .filter(|(_, m)| !declining(m))
            .map(|(at, _)| at)
            .collect();
        let declines = modes.len() - spoken.len();
        let optional = oracle.to_lowercase().contains("choose up to");
        // More compiled modes than printed bullets is never a subset.
        // A missing compiled mode must not shift the labels of those kept.
        if bullets.len() < spoken.len() || (declines > 0 && !(optional && declines == 1)) {
            return refuse();
        }
        if bullets.len() > spoken.len() {
            return map_subset(modes, &spoken, &bullets, &lines).unwrap_or_else(refuse);
        }
        let mut found = vec![None; modes.len()];
        for (at, line) in spoken.iter().zip(&bullets) {
            found[*at] = u8::try_from(*line).ok();
        }
        return found;
    }
    let mut found = vec![None; modes.len()];
    let mut spoken: Vec<usize> = Vec::new();
    for (at, mode) in modes.iter().enumerate() {
        let Some(mana) = mode.cost_override else {
            continue;
        };
        let cost = baylee_cards_dsl::Cost { mana, parts: &[] };
        let hits: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line_shape(line) == LineShape::Activated && cost_fits(&cost, line))
            .map(|(at, _)| at)
            .collect();
        // Two lines printing the same cost is the reader unable to tell
        // them apart, and whichever came first would be a coin toss drawn
        // to the player as the card's own sentence.
        let [only] = hits[..] else {
            return refuse();
        };
        found[at] = u8::try_from(only).ok();
        spoken.push(only);
    }
    // What is left is the card's body, and the counts have to meet exactly.
    // A modal card that also prints a keyword line — "Flying" over a body
    // over an overload cost — leaves two candidates for one mode and is
    // refused here rather than given the first of them.
    let body: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(at, line)| !spoken.contains(at) && line_shape(line) == LineShape::Other)
        .map(|(at, _)| at)
        .collect();
    let plain: Vec<usize> = (0..modes.len())
        .filter(|at| modes[*at].cost_override.is_none())
        .collect();
    if body.len() != plain.len() {
        return refuse();
    }
    for (at, line) in plain.iter().zip(body) {
        found[*at] = u8::try_from(line).ok();
    }
    found
}

/// Match a retained subset only when every mode has exactly one distinct bullet.
fn map_subset(
    modes: &[SpellMode],
    spoken: &[usize],
    bullets: &[usize],
    lines: &[&str],
) -> Option<Vec<Option<u8>>> {
    let mut found = vec![None; modes.len()];
    for &at in spoken {
        let mode = &modes[at];
        // Cost-bearing choices need a reader for both the cost and effect.
        if mode.cost_override.is_some() || mode.additional_cost.is_some() {
            return None;
        }
        let mut hits = bullets.iter().copied().filter(|&line| {
            let lower = without_reminder(lines[line]).to_lowercase();
            !mode.effects.is_empty()
                && mode
                    .effects
                    .iter()
                    .all(|effect| effect_fits(effect, &lower))
        });
        let line = u8::try_from(hits.next()?).ok()?;
        if hits.next().is_some() || found.contains(&Some(line)) {
            return None;
        }
        found[at] = Some(line);
    }
    Some(found)
}

/// Narrow effect-word discriminators; an unread effect cannot claim a bullet.
/// These identify provenance, not completeness or the correctness of a card.
fn effect_fits(effect: &Effect, lower: &str) -> bool {
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphabetic())
        .filter(|word| !word.is_empty())
        .collect();
    match effect {
        Effect::Mill { .. } => words.iter().any(|word| matches!(*word, "mill" | "mills")),
        Effect::PhaseOut { .. } => words
            .windows(2)
            .any(|pair| matches!(pair[0], "phase" | "phases") && pair[1] == "out"),
        _ => false,
    }
}
