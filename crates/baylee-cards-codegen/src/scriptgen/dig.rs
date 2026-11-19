//! Reading `Dig`: the top cards of a library looked at or revealed, some
//! of them moved, and the rest put where the line says.
//!
//! The reference's `Dig` is one effect with a dozen knobs; the DSL has one
//! effect per sentence the cards print. Every knob is read here, and a line
//! is written only as the effect whose sentence it is, each one an effect a
//! hand-written card already plays:
//!
//! | the line | the effect |
//! | --- | --- |
//! | look, take N into the hand, rest on the bottom | `LookAtTopPick` (Impulse) |
//! | look, take N into the hand, rest back on top | `LookAtTopPickThenReorder` (Diabolic Vision) |
//! | look, may take one matching, rest on the bottom | `LookAtTopFind` (Silundi Vision) |
//! | reveal, all matching, rest on the bottom in any order | `RevealTopAllMatching` (Goblin Ringleader) |
//! | reveal one, matching here, else there | `RevealTopAndSort` (Call of the Wild) |
//! | reveal, may take one matching, rest to a zone | `RevealTopFind` (Malevolent Rumble) |
//! | exile them all | `ExileTop` (Demonic Consultation) |
//!
//! The reference's defaults are read as its `DigEffect` reads them: one
//! card moved, to the hand, the rest on the bottom of the library in an
//! order the player chooses, the cards seen only by the player digging.

use super::{Params, Tx};

/// Where the cards the line moves first go (`DestinationZone$`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum First {
    Hand,
    Battlefield { tapped: bool },
    Exile,
}

/// Where the cards not moved go (`DestinationZone2$`, `LibraryPosition2$`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Rest {
    /// The bottom of the library, the reference's default.
    Bottom,
    /// Back on top (`LibraryPosition2$ 0`).
    Top,
    /// Into a zone, each card on its own: a `SearchDest` expression.
    Move(&'static str),
}

/// How many cards the line moves first (`ChangeNum$`, default 1).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Take {
    Exactly(u8),
    All,
}

impl Tx<'_> {
    /// One `Dig` line as the effects it stands for.
    ///
    /// `targets_here` is whether this line itself declared a target: a
    /// target an earlier line declared (Prophetic Bolt's "4 damage to any
    /// target") is not the library this line digs in.
    #[allow(clippy::too_many_lines)]
    pub(super) fn dig_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        let defined = p.take("Defined");
        let your_library = !targets_here && matches!(defined.as_deref(), None | Some("You"));
        let count_raw = p.take("DigNum")?;
        let take = match p.take("ChangeNum").as_deref() {
            None => Take::Exactly(1),
            Some("All") => Take::All,
            Some(n) => Take::Exactly(
                self.plain_num(n)
                    .and_then(|n| u8::try_from(n).ok())
                    .filter(|&n| n > 0)?,
            ),
        };
        let optional = flag(p.take("Optional"))?;
        let reveal = flag(p.take("Reveal"))?;
        // Keeps the reference from revealing a card a filter picked; with
        // no filter, nothing picked is revealed anyway.
        let no_reveal = flag(p.take("NoReveal"))?;
        let random = flag(p.take("RestRandomOrder"))?;
        // The reference's way of making sure the activating player sees the
        // cards, which the player looking at their own library does here.
        let force_reveal = flag(p.take("ForceRevealToController"))?;
        let tapped = flag(p.take("Tapped"))?;
        let first = match p.take("DestinationZone").as_deref() {
            None | Some("Hand") if !tapped => First::Hand,
            Some("Battlefield") => First::Battlefield { tapped },
            Some("Exile") if !tapped => First::Exile,
            _ => {
                return self
                    .deny("a `Dig` destination other than hand, battlefield or exile".to_string());
            }
        };
        let rest_zone = p.take("DestinationZone2");
        let rest_position = p.take("LibraryPosition2");
        let rest_given = rest_zone.is_some() || rest_position.is_some();
        let rest = match (rest_zone.as_deref(), rest_position.as_deref()) {
            (None | Some("Library"), None | Some("-1")) => Rest::Bottom,
            (None | Some("Library"), Some("0")) => Rest::Top,
            (Some("Graveyard"), None) => Rest::Move("SearchDest::Graveyard"),
            (Some("Exile"), None) => Rest::Move("SearchDest::Exile"),
            _ => return self.deny("a `Dig` remainder put elsewhere".to_string()),
        };
        let valid = p.take("ChangeValid");

        // "Exile the top N cards": every card moves, so nothing is left to
        // put anywhere, and nothing is revealed first (exile is public by
        // itself, CR 406.3).
        if take == Take::All && first == First::Exile {
            if valid.is_some()
                || reveal
                || optional
                || random
                || force_reveal
                || no_reveal
                || rest_given
            {
                return self.deny("a `Dig` that exiles some of the cards".to_string());
            }
            let who = self.player_of_line(defined.as_deref(), target, targets_a_player)?;
            let count = self.dig_count(&count_raw)?;
            return Some(vec![format!(
                "Effect::ExileTop {{ who: {who}, count: {count} }}"
            )]);
        }
        if !your_library {
            return self.deny("a `Dig` of another player's library".to_string());
        }
        if first == First::Exile {
            return self.deny("a `Dig` that exiles the cards it picks".to_string());
        }
        if force_reveal && reveal {
            return self.deny("a `Dig` revealed to its controller twice".to_string());
        }

        let Some(valid) = valid else {
            // No filter: a pick into the hand, nothing revealed.
            return match (take, first, reveal, optional, rest) {
                (Take::Exactly(pick), First::Hand, false, false, Rest::Bottom) => {
                    let count = self.dig_count(&count_raw)?;
                    Some(vec![format!(
                        "Effect::LookAtTopPick {{ count: {count}, pick: {pick}, random: {random} }}"
                    )])
                }
                (Take::Exactly(pick), First::Hand, false, false, Rest::Top) if !random => {
                    let count = self.dig_count(&count_raw)?;
                    Some(vec![format!(
                        "Effect::LookAtTopPickThenReorder {{ count: {count}, pick: {pick} }}"
                    )])
                }
                (_, _, _, _, Rest::Move(_)) => {
                    self.deny("a `Dig` pick whose rest goes to the graveyard or exile".to_string())
                }
                (_, _, true, _, _) => self.deny("a revealed `Dig` pick".to_string()),
                (_, _, _, true, _) => self.deny("an optional `Dig` pick".to_string()),
                _ => self.deny("a `Dig` pick of every card, or onto the battlefield".to_string()),
            };
        };
        // A filter names a kind of card the line moves, and the reference
        // reveals what it moved unless `NoReveal$` says otherwise, which is
        // what every effect below does.
        if no_reveal {
            return self.deny("a `Dig` that moves a matching card unrevealed".to_string());
        }
        let filter = self.filter_expr_in_zone(&valid, true)?;
        let name = self.body.filter_static("DIG", &filter);
        let fixed_count = count_raw.parse::<u8>().ok().filter(|&n| n > 0);
        let find = match first {
            First::Battlefield { tapped: false } => "Find::BATTLEFIELD",
            First::Battlefield { tapped: true } => "Find::BATTLEFIELD_TAPPED",
            First::Hand | First::Exile => "Find::HAND",
        };
        match (take, reveal, optional, rest) {
            // "Reveal the top card of your library. If it's a creature card,
            // put it onto the battlefield. Otherwise, put it into your
            // graveyard."
            (Take::All, true, false, Rest::Move(otherwise))
                if fixed_count == Some(1)
                    && !random
                    && first != (First::Battlefield { tapped: true }) =>
            {
                let matched = if first == First::Hand {
                    "SearchDest::Hand"
                } else {
                    "SearchDest::Battlefield"
                };
                Some(vec![format!(
                    "Effect::RevealTopAndSort {{ filter: &{name}, matched: {matched}, otherwise: {otherwise} }}"
                )])
            }
            // "Reveal the top four cards of your library. Put all Soldier
            // cards revealed this way into your hand and the rest on the
            // bottom of your library in any order."
            (Take::All, true, false, Rest::Bottom) if !random => {
                let count = fixed_count?;
                Some(vec![format!(
                    "Effect::RevealTopAllMatching {{ count: {count}, filter: &{name}, found: {find} }}"
                )])
            }
            // "Reveal the top four cards of your library. You may put a
            // permanent card from among them into your hand. Put the rest
            // into your graveyard."
            (Take::Exactly(1), true, true, Rest::Move(otherwise))
                if !random && first == First::Hand =>
            {
                let count = fixed_count?;
                Some(vec![format!(
                    "Effect::RevealTopFind {{ found: {find}, count: {count}, filter: &{name}, otherwise: {otherwise} }}"
                )])
            }
            // "Look at the top four cards of your library. You may reveal a
            // creature card from among them and put it into your hand. Put
            // the rest on the bottom of your library in a random order."
            (Take::Exactly(1), false, true, Rest::Bottom) => {
                let count = fixed_count?;
                Some(vec![format!(
                    "Effect::LookAtTopFind {{ found: {find}, count: {count}, filter: &{name}, random: {random} }}"
                )])
            }
            _ => self.deny(format!(
                "a `Dig` that {} and moves {} matching to {}, the rest {}",
                if reveal { "reveals" } else { "looks" },
                match (take, optional) {
                    (Take::All, _) => "every card".to_string(),
                    (Take::Exactly(n), true) => format!("up to {n}"),
                    (Take::Exactly(n), false) => n.to_string(),
                },
                if first == First::Hand {
                    "the hand"
                } else {
                    "the battlefield"
                },
                match rest {
                    Rest::Bottom => "on the bottom",
                    Rest::Top => "on top",
                    Rest::Move("SearchDest::Graveyard") => "into the graveyard",
                    Rest::Move(_) => "into exile",
                },
            )),
        }
    }

    /// How many cards a `Dig` line looks at, where its effect takes an
    /// `Amount`: a number, the X announced, or a count read as it resolves.
    /// Domain is read as the other readers read it, the basic land types
    /// among lands you control.
    fn dig_count(&mut self, raw: &str) -> Option<String> {
        if self.svars.get(raw.trim()).map(|d| d.trim()) == Some("Count$Domain") {
            return Some("Amount::BasicLandTypesAmong(&Filter::YOUR_LAND)".to_string());
        }
        let n = self.amount_or_count(raw)?;
        (n != "Amount::Fixed(0)").then_some(n)
    }
}

/// A `Key$ True` flag: absent is `false`, `True` is `true`, and any other
/// value is refused (the reference tests these keys by presence, so a
/// written `False` would mean `true` to it).
fn flag(value: Option<String>) -> Option<bool> {
    match value.as_deref() {
        None => Some(false),
        Some("True") => Some(true),
        Some(_) => None,
    }
}
