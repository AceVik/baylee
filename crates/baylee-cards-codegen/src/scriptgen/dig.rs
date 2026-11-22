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
//! The reference's defaults are read as the reference reads them: one
//! card moved, to the hand, the rest on the bottom of the library in an
//! order the player chooses, the cards seen only by the player digging.

use super::{Params, Tx};

/// Where the cards the line moves first go (`DestinationZone$`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum First {
    Hand,
    Battlefield { tapped: bool },
    Graveyard,
    Exile,
    TopOfLibrary,
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
    Any,
    All,
}

/// Boolean modifiers read off a `Dig` line.
#[derive(Clone, Copy)]
struct DigFlags {
    reveal: bool,
    optional: bool,
    random: bool,
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
        let Some(count_raw) = p.take("DigNum") else {
            return self.deny("a `Dig` with no `DigNum$`".to_string());
        };
        let take = match p.take("ChangeNum").as_deref() {
            None => Take::Exactly(1),
            Some("All") => Take::All,
            Some("Any") => Take::Any,
            Some(n) => match self
                .plain_num(n)
                .and_then(|n| u8::try_from(n).ok())
                .filter(|&n| n > 0)
            {
                Some(n) => Take::Exactly(n),
                None => return self.deny(format!("a `Dig` moving `ChangeNum$ {n}` cards")),
            },
        };
        let optional = self.dig_flag(p, "Optional")?;
        let reveal = self.dig_flag(p, "Reveal")?;
        // Keeps the reference from revealing a card a filter picked; with
        // no filter, nothing picked is revealed anyway.
        let no_reveal = self.dig_flag(p, "NoReveal")?;
        let random = self.dig_flag(p, "RestRandomOrder")?;
        // The reference's way of making sure the activating player sees the
        // cards, which the player looking at their own library does here.
        let force_reveal = self.dig_flag(p, "ForceRevealToController")?;
        let tapped = self.dig_flag(p, "Tapped")?;
        let exile_face_down = self.dig_flag(p, "ExileFaceDown")?;
        let dest_zone = p.take("DestinationZone");
        let lib_pos = p.take("LibraryPosition");
        let rest_zone = p.take("DestinationZone2");
        let rest_position = p.take("LibraryPosition2");
        let rest_given = rest_zone.is_some() || rest_position.is_some();
        let rest = match (rest_zone.as_deref(), rest_position.as_deref()) {
            (None | Some("Library"), None | Some("-1")) => Rest::Bottom,
            (None | Some("Library"), Some("0")) => Rest::Top,
            (Some("Graveyard"), None) => Rest::Move("SearchDest::Graveyard"),
            (Some("Exile"), None) => Rest::Move("SearchDest::Exile"),
            (zone, position) => {
                return self.deny(format!(
                    "a `Dig` remainder put into `{}` at position `{}`",
                    zone.unwrap_or("Library"),
                    position.unwrap_or("-1")
                ));
            }
        };
        let first = match (dest_zone.as_deref(), tapped) {
            (None | Some("Hand"), false) => First::Hand,
            (Some("Battlefield"), tapped) => First::Battlefield { tapped },
            (Some("Graveyard"), false) if take == Take::Any && rest == Rest::Top => {
                First::Graveyard
            }
            (Some("Exile"), false) => First::Exile,
            (Some("Library"), false)
                if lib_pos.as_deref() == Some("0")
                    && (matches!(rest, Rest::Bottom | Rest::Top)
                        || (optional && rest == Rest::Move("SearchDest::Graveyard"))) =>
            {
                First::TopOfLibrary
            }
            (Some("Library"), true) => {
                return self.deny("`Tapped$` on a `Dig` to the library".to_string());
            }
            (None | Some("Hand" | "Exile"), true) => {
                return self.deny("`Tapped$` on a `Dig` to the hand or exile".to_string());
            }
            (Some(zone), _) => return self.deny(format!("a `Dig` destination `{zone}`")),
        };
        if first != First::TopOfLibrary {
            // In the reference, `LibraryPosition$` applies only when
            // `DestinationZone$` is `Library`; many scripts still write
            // `LibraryPosition$ -1` alongside `DestinationZone2$ Library` to
            // describe putting the rest on the bottom (the default).
            match lib_pos.as_deref() {
                None => {}
                Some("-1") if rest == Rest::Bottom => {}
                Some(pos) => return self.deny(format!("`LibraryPosition$ {pos}` on a `Dig`")),
            }
        }
        let valid = p.take("ChangeValid");

        // "Look at the top card of [a player's] library": putting the single
        // looked-at card back on top leaves the library untouched.
        if first == First::TopOfLibrary
            && valid.is_none()
            && take == Take::Exactly(1)
            && count_raw.trim() == "1"
            && !reveal
            && !optional
            && !random
            && !force_reveal
            && !no_reveal
            && !exile_face_down
        {
            if targets_here && !targets_a_player {
                return self
                    .deny("a `Dig` inspection on a line that targets an object".to_string());
            }
            let who = self.player_of_line(defined.as_deref(), target, targets_a_player)?;
            if !Self::is_single_player_rel(who) {
                return self.deny("a `Dig` inspection of more than one library".to_string());
            }
            return Some(vec![format!(
                "Effect::LookAtTopOf {{ player: {who}, count: 1 }}"
            )]);
        }

        // "Exile the top N cards": every card moves, so nothing is left to
        // put anywhere, and nothing is revealed first (exile is public by
        // itself, CR 406.3). Face-down linked exile on a permanent
        // (`Bomat Courier`, `Expert-Level Safe`) uses `ExileTopFaceDownLinked`.
        if take == Take::All && first == First::Exile {
            if exile_face_down {
                if self.on_a_spell
                    || !your_library
                    || valid.is_some()
                    || reveal
                    || optional
                    || random
                    || force_reveal
                    || rest_given
                    || self.oracle.to_ascii_lowercase().contains("look at")
                {
                    return self.deny("a `Dig` with `ExileFaceDown$ True`".to_string());
                }
                let Some(n) = self
                    .plain_num(&count_raw)
                    .and_then(|n| usize::try_from(n).ok())
                    .filter(|&n| (1..=4).contains(&n))
                else {
                    return self.deny("a `Dig` with `ExileFaceDown$ True`".to_string());
                };
                return Some(vec!["Effect::ExileTopFaceDownLinked".to_string(); n]);
            }
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
            // A library is a player's: a line that targets an object names
            // no library, and reading its absent `Defined$` as "you" would
            // exile the wrong one.
            if targets_here && !targets_a_player {
                return self.deny("a `Dig` exile on a line that targets an object".to_string());
            }
            let who = self.player_of_line(defined.as_deref(), target, targets_a_player)?;
            let count = self.dig_count(&count_raw)?;
            return Some(vec![format!(
                "Effect::ExileTop {{ who: {who}, count: {count} }}"
            )]);
        }
        if exile_face_down {
            return self.deny("a `Dig` with `ExileFaceDown$ True`".to_string());
        }
        // "Look at the top N cards of [a player's] library. Put any number into
        // their graveyard / exile, then put the rest back on top in any order."
        if take == Take::Any
            && matches!(first, First::Graveyard | First::Exile)
            && rest == Rest::Top
            && valid.is_none()
            && !reveal
            && !optional
            && !random
            && !force_reveal
            && !no_reveal
        {
            if targets_here && !targets_a_player {
                return self.deny("a `Dig` on a line that targets an object".to_string());
            }
            let who = self.player_of_line(defined.as_deref(), target, targets_a_player)?;
            if !Self::is_single_player_rel(who) {
                return self.deny("a `Dig` of more than one library".to_string());
            }
            let count = self.dig_count(&count_raw)?;
            let picked = if first == First::Graveyard {
                "PeekPickDest::Graveyard"
            } else {
                "PeekPickDest::Exile"
            };
            return Some(vec![format!(
                "Effect::PeekAndReveal {{ library: {who}, count: {count}, reveal_all: false, \
                 reveal_chosen: false, filter: &Filter::Any, min_pick: 0, max_pick: u8::MAX, \
                 picked: {picked}, rest: PeekRestDest::TopAnyOrder, if_picked: &[] }}"
            )]);
        }
        if !your_library {
            return self.deny(match (targets_here, defined.as_deref()) {
                (true, _) => "a `Dig` of a targeted player's library".to_string(),
                (false, Some("Player")) => "a `Dig` of every player's library".to_string(),
                (false, other) => format!(
                    "a `Dig` of the library of `Defined$ {}`",
                    other.unwrap_or_default()
                ),
            });
        }
        if first == First::Graveyard {
            return self.deny("a `Dig` destination `Graveyard`".to_string());
        }
        if first == First::Exile {
            return self.deny("a `Dig` that exiles the cards it picks".to_string());
        }
        if force_reveal && reveal {
            return self.deny("a `Dig` revealed to its controller twice".to_string());
        }

        // `ChangeValid$ Card` on a private look imposes no type filter; treat
        // it like an unfiltered look when moving to hand or top of library with
        // the rest going to graveyard/exile.
        let valid = match valid {
            Some(ref v)
                if v.trim() == "Card"
                    && !reveal
                    && matches!(rest, Rest::Move(_))
                    && (first == First::Hand
                        || (first == First::TopOfLibrary && optional && !no_reveal)) =>
            {
                None
            }
            other => other,
        };

        let Some(valid) = valid else {
            // No filter: a pick into the hand (or top of library), nothing revealed.
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
                (Take::Exactly(pick), First::Hand, false, false, Rest::Move(dest)) if !random => {
                    let count = self.dig_count(&count_raw)?;
                    let rest_dest = Self::peek_rest_dest(Rest::Move(dest), false);
                    Some(vec![format!(
                        "Effect::PeekAndReveal {{ library: PlayerRel::You, count: {count}, \
                         reveal_all: false, reveal_chosen: false, filter: &Filter::Any, \
                         min_pick: {pick}, max_pick: {pick}, picked: PeekPickDest::Hand, \
                         rest: {rest_dest}, if_picked: &[] }}"
                    )])
                }
                (Take::Exactly(1), First::TopOfLibrary, false, true, Rest::Move(dest))
                    if !random && !no_reveal =>
                {
                    let count = self.dig_count(&count_raw)?;
                    let rest_dest = Self::peek_rest_dest(Rest::Move(dest), false);
                    Some(vec![format!(
                        "Effect::PeekAndReveal {{ library: PlayerRel::You, count: {count}, \
                         reveal_all: false, reveal_chosen: false, filter: &Filter::Any, \
                         min_pick: 0, max_pick: 1, picked: PeekPickDest::TopOfLibrary, \
                         rest: {rest_dest}, if_picked: &[] }}"
                    )])
                }
                (Take::Any, First::Hand, false, _, Rest::Bottom | Rest::Move(_))
                    if !(matches!(rest, Rest::Move(_)) && random) =>
                {
                    let count = self.dig_count(&count_raw)?;
                    let rest_dest = Self::peek_rest_dest(rest, random);
                    Some(vec![format!(
                        "Effect::PeekAndReveal {{ library: PlayerRel::You, count: {count}, \
                         reveal_all: false, reveal_chosen: false, filter: &Filter::Any, \
                         min_pick: 0, max_pick: u8::MAX, picked: PeekPickDest::Hand, \
                         rest: {rest_dest}, if_picked: &[] }}"
                    )])
                }
                (_, _, _, _, Rest::Top) if random => {
                    self.deny("a `Dig` whose rest goes back on top in a random order".to_string())
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
            First::TopOfLibrary => "Find::TOP_OF_LIBRARY",
            First::Hand | First::Graveyard | First::Exile => "Find::HAND",
        };
        match (take, reveal, optional, rest) {
            // "Reveal the top card of your library. If it's a creature card,
            // put it onto the battlefield. Otherwise, put it into your
            // graveyard."
            (Take::All, true, false, Rest::Move(otherwise))
                if fixed_count == Some(1)
                    && !random
                    && matches!(first, First::Hand | First::Battlefield { tapped: false }) =>
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
            (Take::All, true, false, Rest::Bottom) if !random && fixed_count.is_some() => {
                let count = fixed_count?;
                Some(vec![format!(
                    "Effect::RevealTopAllMatching {{ count: {count}, filter: &{name}, found: {find} }}"
                )])
            }
            // "Reveal the top four cards of your library. You may put a
            // permanent card from among them into your hand. Put the rest
            // into your graveyard."
            (Take::Exactly(1), true, true, Rest::Move(otherwise))
                if !random && first == First::Hand && fixed_count.is_some() =>
            {
                let count = fixed_count?;
                Some(vec![format!(
                    "Effect::RevealTopFind {{ found: {find}, count: {count}, filter: &{name}, otherwise: {otherwise} }}"
                )])
            }
            // "Look at the top four cards of your library. You may reveal a
            // creature card from among them and put it into your hand. Put
            // the rest on the bottom of your library in a random order."
            (Take::Exactly(1), false, true, Rest::Bottom) if fixed_count.is_some() => {
                let count = fixed_count?;
                Some(vec![format!(
                    "Effect::LookAtTopFind {{ found: {find}, count: {count}, filter: &{name}, random: {random} }}"
                )])
            }
            _ => {
                let flags = DigFlags {
                    reveal,
                    optional,
                    random,
                };
                if let Some(peek) =
                    self.filtered_dig_peek_and_reveal(&count_raw, &name, take, first, rest, flags)
                {
                    return Some(peek);
                }
                self.deny(format!(
                    "a `Dig` that {} and moves {} matching to {}, the rest {}{}",
                    if reveal { "reveals" } else { "looks" },
                    match (take, optional) {
                        (Take::All, _) => "every card".to_string(),
                        (Take::Any, _) => "any number".to_string(),
                        (Take::Exactly(n), true) => format!("up to {n}"),
                        (Take::Exactly(n), false) => n.to_string(),
                    },
                    match first {
                        First::Hand | First::Graveyard | First::Exile => "the hand",
                        First::Battlefield { .. } => "the battlefield",
                        First::TopOfLibrary => "the top of the library",
                    },
                    match rest {
                        Rest::Bottom => "on the bottom",
                        Rest::Top => "on top",
                        Rest::Move("SearchDest::Graveyard") => "into the graveyard",
                        Rest::Move(_) => "into exile",
                    },
                    if random { " in a random order" } else { "" },
                ))
            }
        }
    }

    fn filtered_dig_peek_and_reveal(
        &mut self,
        count_raw: &str,
        filter_name: &str,
        take: Take,
        first: First,
        rest: Rest,
        flags: DigFlags,
    ) -> Option<Vec<String>> {
        if flags.random && rest != Rest::Bottom {
            return None;
        }
        if first == First::TopOfLibrary && !matches!(take, Take::Exactly(1)) {
            return None;
        }
        let (min_pick, max_pick) = match (take, flags.reveal, flags.optional) {
            (Take::Any, _, _) => ("0".to_string(), "u8::MAX".to_string()),
            (Take::All, true, false) => ("u8::MAX".to_string(), "u8::MAX".to_string()),
            (Take::Exactly(pick), _, true) => ("0".to_string(), pick.to_string()),
            _ => return None,
        };
        let picked = match first {
            First::Hand => "PeekPickDest::Hand".to_string(),
            First::Battlefield { tapped } => {
                format!("PeekPickDest::Battlefield {{ tapped: {tapped} }}")
            }
            First::TopOfLibrary => "PeekPickDest::TopOfLibrary".to_string(),
            First::Graveyard | First::Exile => return None,
        };
        let count = self.dig_count(count_raw)?;
        let rest_dest = Self::peek_rest_dest(rest, flags.random);
        let reveal_all = flags.reveal;
        let reveal_chosen = !flags.reveal;
        Some(vec![format!(
            "Effect::PeekAndReveal {{ library: PlayerRel::You, count: {count}, \
             reveal_all: {reveal_all}, reveal_chosen: {reveal_chosen}, filter: &{filter_name}, \
             min_pick: {min_pick}, max_pick: {max_pick}, picked: {picked}, \
             rest: {rest_dest}, if_picked: &[] }}"
        )])
    }

    fn peek_rest_dest(rest: Rest, random: bool) -> String {
        match rest {
            Rest::Bottom => format!("PeekRestDest::BottomOfLibrary {{ random: {random} }}"),
            Rest::Top => "PeekRestDest::TopAnyOrder".to_string(),
            Rest::Move("SearchDest::Graveyard") => "PeekRestDest::Graveyard".to_string(),
            Rest::Move(_) => "PeekRestDest::Exile".to_string(),
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

    /// A `Key$ True` flag: absent is `false`, `True` is `true`, and any other
    /// value is refused by name (the reference tests these keys by presence,
    /// so a written `False` would mean `true` to it).
    fn dig_flag(&self, p: &mut Params, key: &str) -> Option<bool> {
        match p.take(key).as_deref() {
            None => Some(false),
            Some("True") => Some(true),
            Some(other) => self.deny(format!("`{key}$ {other}` on a `Dig`")),
        }
    }

    /// Wave-4 extensions for `DigUntil`: supports any `PlayerRel` (including
    /// targeted players like Balustrade Spy), omitted `FoundDestination$`
    /// defaulting to `RevealedDestination$` (`Graveyard`/`Exile`), `Amount$ 1`,
    /// `IsCurse$`, and `RevealUntil` (rest to bottom in random order).
    pub(super) fn wave4_dig_until_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        let player = self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        if !matches!(p.take("Amount").as_deref(), None | Some("1")) {
            return None;
        }
        p.take("IsCurse");
        p.claim_label("ValidDescription", "Valid");
        let valid = p.take("Valid")?;
        let found_raw = p.take("FoundDestination");
        let revealed_raw = p.take("RevealedDestination");
        let pos = p.take("RevealedLibraryPosition");
        let random_order = match p.take("RevealRandomOrder").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let shuffle = match p.take("Shuffle").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if !p.exhausted() {
            return None;
        }
        if matches!(valid.trim(), "Card.NamedCard" | "NamedCard") {
            if pos.is_some() || random_order || !Self::is_single_player_rel(player) {
                return None;
            }
            let otherwise = match revealed_raw.as_deref() {
                Some("Library") => "SearchDest::TopOfLibrary",
                Some("Graveyard") => "SearchDest::Graveyard",
                Some("Exile") => "SearchDest::Exile",
                _ => return None,
            };
            let matched = match found_raw.as_deref() {
                Some("Battlefield") => "SearchDest::Battlefield",
                Some("Hand") => "SearchDest::Hand",
                Some("Graveyard") => "SearchDest::Graveyard",
                Some("Exile") => "SearchDest::Exile",
                Some("Library") => "SearchDest::TopOfLibrary",
                None if matches!(otherwise, "SearchDest::Graveyard" | "SearchDest::Exile") => {
                    otherwise
                }
                _ => return None,
            };
            let mut out = vec![format!(
                "Effect::RevealUntilNamed {{ player: {player}, matched: {matched}, otherwise: {otherwise} }}"
            )];
            if shuffle {
                out.push(format!("Effect::ShuffleLibrary {{ who: {player} }}"));
            }
            return Some(out);
        }
        let expr = self.filter_expr_in_zone(&valid, true)?;
        let name = self.body.filter_static("REVEAL_UNTIL", &expr);
        // "Reveal cards from the top of your library until you reveal a [valid]
        // card. Put that card into [found] and the rest on the bottom of your
        // library in a random order."
        if player == "PlayerRel::You"
            && !shuffle
            && random_order
            && revealed_raw.as_deref() == Some("Library")
            && pos.as_deref() == Some("-1")
        {
            let found = match found_raw.as_deref() {
                Some("Hand") => "SearchDest::Hand",
                Some("Battlefield") => "SearchDest::Battlefield",
                Some("Exile") => "SearchDest::Exile",
                Some("Graveyard") => "SearchDest::Graveyard",
                _ => return None,
            };
            return Some(vec![format!(
                "Effect::RevealUntil {{ filter: &{name}, found: {found} }}"
            )]);
        }
        if pos.is_some() || random_order {
            return None;
        }
        let otherwise = match revealed_raw.as_deref() {
            Some("Library") => "SearchDest::TopOfLibrary",
            Some("Graveyard") => "SearchDest::Graveyard",
            Some("Exile") => "SearchDest::Exile",
            _ => return None,
        };
        let matched = match found_raw.as_deref() {
            Some("Battlefield") => "SearchDest::Battlefield",
            Some("Hand") => "SearchDest::Hand",
            Some("Graveyard") => "SearchDest::Graveyard",
            Some("Exile") => "SearchDest::Exile",
            Some("Library") => "SearchDest::TopOfLibrary",
            None if matches!(otherwise, "SearchDest::Graveyard" | "SearchDest::Exile") => otherwise,
            _ => return None,
        };
        let mut out = vec![format!(
            "Effect::RevealUntilMatching {{ player: {player}, filter: &{name}, matched: {matched}, otherwise: {otherwise} }}"
        )];
        if shuffle {
            out.push(format!("Effect::ShuffleLibrary {{ who: {player} }}"));
        }
        Some(out)
    }
}
