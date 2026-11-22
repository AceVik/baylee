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

use super::{Chain, Params, Tx, counter_kind};

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
            let who = self.wave11_player_of_line(defined.as_deref(), target, targets_a_player)?;
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
        if (take == Take::All || (take == Take::Exactly(1) && count_raw.trim() == "1"))
            && first == First::Exile
        {
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
            let who = self.wave11_player_of_line(defined.as_deref(), target, targets_a_player)?;
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
            let who = self.wave11_player_of_line(defined.as_deref(), target, targets_a_player)?;
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

    // -------------------------------------------------------------------------
    // Wave 23a: `PeekAndReveal` -> `TwoPiles`, `DigUntil` -> `RevealUntilThen`,
    // and `RepeatEach.RepeatCards` -> `RepeatEachObject`
    // -------------------------------------------------------------------------

    /// Canonicalizes modified-creature-you-control filter spellings to
    /// `Filter::YOUR_MODIFIED_CREATURE` (`CR 700.9`, where `Modified` already
    /// requires a creature).
    pub(super) fn simplify_your_modified_filter(expr: &str) -> String {
        match expr {
            "Filter::And(&[Filter::Modified, Filter::ControlledByYou])"
            | "Filter::And(&[Filter::ControlledByYou, Filter::Modified])"
            | "Filter::And(&[Filter::CREATURE, Filter::Modified, Filter::ControlledByYou])"
            | "Filter::And(&[Filter::CREATURE, Filter::ControlledByYou, Filter::Modified])" => {
                "Filter::YOUR_MODIFIED_CREATURE".to_string()
            }
            other => other.to_string(),
        }
    }

    /// `PeekAndReveal` -> `TwoPiles` (*Fact or Fiction* shape) ->
    /// `Effect::RevealAndSeparate { count }`.
    ///
    /// Strictly demands public top-N reveal of your own library, opponent
    /// separation, your choice of pile, chosen pile to hand, and unchosen pile
    /// to graveyard. Refuses inverted separator/chooser, face-down piles,
    /// bottom-of-library remainder, or dynamic reveal counts.
    pub(super) fn peek_and_reveal_two_piles_chain(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
        chain: &mut Chain,
    ) -> Option<()> {
        if targets_here || targets_a_player {
            return self.deny("`PeekAndReveal` `TwoPiles` with a target".to_string());
        }
        if !matches!(p.take("Defined").as_deref(), None | Some("You")) {
            return self.deny("`PeekAndReveal` `TwoPiles` of non-controller library".to_string());
        }
        if p.take("NoPeek").as_deref() != Some("True")
            || p.take("RememberRevealed").as_deref() != Some("True")
        {
            return self.deny("`PeekAndReveal` `TwoPiles` without public reveal".to_string());
        }
        for flag in ["NoReveal", "RevealOptional", "RememberPeeked"] {
            if !matches!(p.take(flag).as_deref(), None | Some("False")) {
                return self.deny(format!("`PeekAndReveal` `TwoPiles` with `{flag}$ True`"));
            }
        }
        if p.has("RevealValid") {
            return self.deny("`PeekAndReveal` `TwoPiles` with `RevealValid$`".to_string());
        }
        let Some(raw_amt) = p.take("PeekAmount") else {
            return self.deny("`PeekAndReveal` `TwoPiles` with no `PeekAmount$`".to_string());
        };
        let Some(count) = self
            .plain_num(&raw_amt)
            .and_then(|n| u32::try_from(n).ok())
            .filter(|&n| n > 0)
        else {
            return self.deny(format!(
                "`PeekAndReveal` `TwoPiles` with dynamic `PeekAmount$ {raw_amt}`"
            ));
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `PeekAndReveal.{key}`"));
        }
        let Some(two_name) = sub else {
            return self.deny("`PeekAndReveal` `TwoPiles` with no `SubAbility$`".to_string());
        };
        let Some(body) = self.svars.get(two_name.trim()).cloned() else {
            return self.deny(format!("`SubAbility$ {two_name}` names no SVar"));
        };
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if api != "TwoPiles" {
            return None;
        }
        if !matches!(sp.take("Defined").as_deref(), None | Some("You"))
            || sp.take("DefinedCards").as_deref() != Some("Remembered")
            || sp.take("Separator").as_deref() != Some("Opponent")
            || !matches!(sp.take("Chooser").as_deref(), None | Some("You"))
            || !matches!(sp.take("Zone").as_deref(), None | Some("Library"))
            || sp.has("FaceDown")
        {
            return self.deny("unsupported `TwoPiles` separation shape".to_string());
        }
        let Some(chosen_svar) = sp.take("ChosenPile") else {
            return self.deny("`TwoPiles` with no `ChosenPile$`".to_string());
        };
        let Some(unchosen_svar) = sp.take("UnchosenPile") else {
            return self.deny("`TwoPiles` with no `UnchosenPile$`".to_string());
        };
        let trailing = sp.take("SubAbility");
        if let Some(key) = sp.first_key() {
            return self.deny(format!("unclaimed parameter `TwoPiles.{key}`"));
        }
        if !self.is_library_remembered_move(&chosen_svar, "Hand")
            || !self.is_library_remembered_move(&unchosen_svar, "Graveyard")
        {
            return self.deny("`TwoPiles` pile destinations outside hand/graveyard".to_string());
        }
        let next_sub = match trailing {
            None => None,
            Some(ref name) => {
                if let Some(after_clean) = self
                    .dig_cleanup_next(name, false)
                    .map(CleanupTail::into_option)
                {
                    if after_clean
                        .as_deref()
                        .is_some_and(|n| self.chain_mentions_remembered(n))
                    {
                        return self.deny("`TwoPiles` follow-up reading remembered cards".into());
                    }
                    after_clean
                } else if self.chain_mentions_remembered(name) {
                    return self.deny("`TwoPiles` follow-up reading remembered cards".into());
                } else {
                    Some(name.clone())
                }
            }
        };
        chain
            .effects
            .push(format!("Effect::RevealAndSeparate {{ count: {count} }}"));
        self.chain_sub_ability(next_sub, chain)
    }

    fn is_library_remembered_move(&self, svar: &str, expected_dest: &str) -> bool {
        let Some(body) = self.svars.get(svar.trim()) else {
            return false;
        };
        let Some((api, mut mp)) = Params::parse(body) else {
            return false;
        };
        mp.drop_prose();
        match api.as_str() {
            "ChangeZone" => {
                mp.take("Defined").as_deref() == Some("Remembered")
                    && mp.take("Origin").as_deref() == Some("Library")
                    && mp.take("Destination").as_deref() == Some(expected_dest)
                    && mp.exhausted()
            }
            "ChangeZoneAll" => {
                mp.take("ChangeType").as_deref() == Some("Card.IsRemembered")
                    && mp.take("Origin").as_deref() == Some("Library")
                    && mp.take("Destination").as_deref() == Some(expected_dest)
                    && mp.exhausted()
            }
            _ => false,
        }
    }

    /// Wave 23a standalone `DigUntil` fallback (`Effect::RevealUntilThen` with
    /// empty `then` or optional battlefield entry before shuffle). Handles
    /// `Amount$` > 1 / dynamic, `MaxRevealed$`, `Tapped$`, `GainControl$`,
    /// `FoundLibraryPosition$`, `RestDestination$`, `BottomChosenOrder`,
    /// `RevealOptional$`/`Optional$`, `OptionalFoundMove$ True` with shuffle,
    /// and multi-seat `EachOpponent`/`EachPlayer`.
    pub(super) fn wave23a_dig_until_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        let parsed = self.parse_dig_until_core(p, target, targets_a_player, false)?;
        if !p.entries.iter().all(|(k, _)| k.starts_with("Condition")) {
            return None;
        }
        if parsed.optional_found_move {
            // Only lower when declining leaves the card to be shuffled back in
            // with the rest (`Hei Bai, Forest Guardian` shape), avoiding Auras
            // (CR 303.4f attachment choice) and bottom-random rest where a
            // declined hit would need to be randomized together with the rest.
            if !parsed.count_is_one
                || parsed.library != "PlayerRel::You"
                || parsed.multi_seat_domain.is_some()
                || parsed.optional_no_dest.is_some()
                || parsed.found_dest_raw.as_deref() != Some("Battlefield")
                || parsed.tapped
                || parsed.rest != "RevealUntilRest::ShuffleIntoLibrary"
                || parsed.valid_raw.contains("Aura")
            {
                return None;
            }
            let move_eff = "Effect::MayDo { effects: &[Effect::MoveAndModify { \
                 subject: MoveSubject::Target(TargetSpec::EventObject), \
                 expected_origin: Some(ZoneRef::Library), blink: false, owner_control: false, \
                 tapped: false, face: None, counters: None, modifiers: &[], \
                 duration: Duration::Indefinitely, attach_source: false, when_source_leaves: None }] }";
            let reveal = Self::format_reveal_until_then(
                &parsed,
                "RevealUntilFound::StayInLibrary",
                "RevealUntilRest::StayInLibrary",
                &[move_eff.to_string()],
            );
            let mut out = vec![
                reveal,
                "Effect::ShuffleLibrary { who: PlayerRel::You }".into(),
            ];
            if parsed.optional_whole {
                out = vec![format!(
                    "Effect::MayDo {{ effects: &[{}] }}",
                    out.join(", ")
                )];
            }
            return Some(out);
        }
        Some(Self::finish_reveal_until_effects(&parsed, &[]))
    }

    /// Wave 23a chained `DigUntil` with `RememberFound$ True` or
    /// `ImprintFound$ True` -> `Effect::RevealUntilThen { ..., then }`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn reveal_until_then_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_a_player: bool,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let mut parsed = self.parse_dig_until_core(p, target, targets_a_player, true)?;
        if parsed.multi_seat_domain.is_some()
            || parsed.optional_found_move
            || (!parsed.remember_found && !parsed.imprint_found)
        {
            return None;
        }
        if !p.entries.iter().all(|(k, _)| k.starts_with("Condition")) {
            return None;
        }
        let sub_name = sub.clone()?;

        // Case 0: Unconditional trailing chain that never reads remembered/imprinted cards
        // (e.g. clash after revealing a land). Runs even when nothing matched.
        if !parsed.remember_revealed
            && !parsed.imprint_revealed
            && !self.chain_mentions_remembered(&sub_name)
        {
            return Some(Self::finish_reveal_until_effects(&parsed, &[]));
        }

        // Case 1: Immediate reflexive trigger whose execute body reads the
        // revealed/found card's mana value or power.
        if parsed.count_is_one
            && parsed.remember_found
            && !parsed.remember_revealed
            && !parsed.imprint_revealed
            && self.sub_is_api(Some(&sub_name), "ImmediateTrigger")
            && let Some((reflexive, trailing)) =
                self.try_dig_until_immediate_trigger(&sub_name, &parsed.valid_raw)
        {
            *sub = trailing;
            return Some(Self::finish_reveal_until_effects(&parsed, &[reflexive]));
        }

        // Case 2: Exile-while-revealing followed by moving the found card out
        // of exile (+ optional moving revealed non-found cards to bottom).
        if parsed.exile_while_revealing
            && let Some((new_found, new_rest, trailing)) =
                self.try_dig_until_exile_move_followup(&sub_name, &parsed)
        {
            parsed.found = new_found;
            if let Some(r) = new_rest {
                parsed.rest = r;
            }
            *sub = trailing;
            return Some(Self::finish_reveal_until_effects(&parsed, &[]));
        }

        // Case 3: Single-hit `DigUntil` -> `DB$ Play` (optionally followed by
        // Exile->Hand fallback or post-cast `Shuffle`), when uncast revealed
        // cards stay in exile or are shuffled rather than randomized together
        // on the bottom with the uncast found card.
        if parsed.count_is_one
            && !parsed.remember_revealed
            && !parsed.imprint_revealed
            && self.sub_is_api(Some(&sub_name), "Play")
            && let Some((then_effs, add_shuffle, trailing)) =
                self.try_dig_until_play_followup(&sub_name, &parsed, target, targets_a_player)
        {
            if add_shuffle {
                parsed.trailing_shuffle = true;
            }
            *sub = trailing;
            return Some(Self::finish_reveal_until_effects(&parsed, &then_effs));
        }

        // Case 4: `FoundDestination$ Exile` -> `DB$ Effect` granting `MayPlay`
        // via `RevealUntilFound::ExileLinked` + `GrantLinkedExilePermission`.
        if parsed.count_is_one
            && parsed.found == "RevealUntilFound::Exile"
            && self.sub_is_api(Some(&sub_name), "Effect")
            && let Some((perm_eff, new_rest, trailing)) =
                self.try_dig_until_exile_permission_followup(&sub_name, &parsed)
        {
            parsed.found = "RevealUntilFound::ExileLinked".to_string();
            if let Some(r) = new_rest {
                parsed.rest = r;
            }
            *sub = trailing;
            return Some(Self::finish_reveal_until_effects(&parsed, &[perm_eff]));
        }

        // Case 5: General `RememberFound$` / `ImprintFound$` follow-up chain
        // (damage/pump by found card's MV/power, sacrifice-if-found, conditional
        // event-object damage, or `RepeatEach` over `only_found: true`).
        if !parsed.remember_revealed && !parsed.imprint_revealed {
            let use_imprinted = parsed.imprint_found && !parsed.remember_found;
            let (then_effs, trailing) = self.compile_dig_until_then_chain(
                &sub_name,
                parsed.count_is_one,
                use_imprinted,
                chain,
            )?;
            *sub = trailing;
            return Some(Self::finish_reveal_until_effects(&parsed, &then_effs));
        }
        None
    }

    /// Wave 23a `DigUntil | RememberRevealed$ True` (without `RememberFound$`)
    /// followed by a `DealDamage` / `LoseLife` / `GainLife` reading
    /// `Remembered$Amount` (*Audacious Reshapers* / *Madcap Experiment* shape):
    /// lowers to `[Effect::RevealUntilThen { ..., then: &[] }, <effect using
    /// Amount::ThisWay(ThisWay::Revealed(&Filter::Any))>]` so the count runs
    /// even when zero cards matched.
    pub(super) fn reveal_until_remembered_count_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_a_player: bool,
        chain: &Chain,
    ) -> Option<Vec<String>> {
        if !chain.effects.is_empty() {
            return None;
        }
        let parsed = self.parse_dig_until_core(p, target, targets_a_player, true)?;
        if !parsed.remember_revealed
            || parsed.remember_found
            || parsed.imprint_found
            || parsed.imprint_revealed
            || parsed.exile_while_revealing
            || parsed.multi_seat_domain.is_some()
            || parsed.optional_whole
            || parsed.optional_found_move
            || !matches!(
                parsed.rest.as_str(),
                "RevealUntilRest::BottomRandom"
                    | "RevealUntilRest::BottomChosenOrder"
                    | "RevealUntilRest::ShuffleIntoLibrary"
                    | "RevealUntilRest::TopOfLibrary"
            )
            || !p.entries.iter().all(|(k, _)| k.starts_with("Condition"))
        {
            return None;
        }
        let sub_name = sub.clone()?;
        let body = self.svars.get(sub_name.trim())?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if sp.has("ValidTgts") || sp.has("DamageSource") {
            return None;
        }
        let raw_amt = match api.as_str() {
            "DealDamage" => sp.take("NumDmg")?,
            "GainLife" | "LoseLife" => sp.take("LifeAmount")?,
            _ => return None,
        };
        if self
            .svars
            .get(raw_amt.trim())
            .map(String::as_str)
            .map(str::trim)
            != Some("Remembered$Amount")
        {
            return None;
        }
        sp.take("IsCurse");
        let who =
            self.wave11_player_of_line(sp.take("Defined").as_deref(), target, targets_a_player)?;
        if !Self::is_single_player_rel(who) {
            return None;
        }
        let next = sp.take("SubAbility");
        if !sp.exhausted() {
            return None;
        }
        let trailing = self.consume_dig_cleanup_tail(next, false)?.into_option();
        let count_amt = "Amount::ThisWay(ThisWay::Revealed(&Filter::Any))";
        let followup = match (api.as_str(), who) {
            ("DealDamage", _) => {
                format!(
                    "Effect::DealDamage {{ amount: {count_amt}, target: TargetSpec::Player({who}) }}"
                )
            }
            ("GainLife", "PlayerRel::You") => {
                format!("Effect::GainLife {{ amount: {count_amt} }}")
            }
            ("GainLife", _) => {
                format!("Effect::GainLifeFor {{ amount: {count_amt}, who: {who} }}")
            }
            ("LoseLife", _) => {
                format!("Effect::LoseLife {{ amount: {count_amt}, target: {who} }}")
            }
            _ => return None,
        };
        *sub = trailing;
        let mut out = Self::finish_reveal_until_effects(&parsed, &[]);
        out.push(followup);
        Some(out)
    }

    #[allow(clippy::too_many_lines)]
    fn parse_dig_until_core(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
        allow_remember: bool,
    ) -> Option<ParsedDigUntil> {
        let defined = p.take("Defined");
        let raw_player =
            self.wave11_player_of_line(defined.as_deref(), target, targets_a_player)?;
        let (library, multi_seat_domain) = if Self::is_single_player_rel(raw_player) {
            (raw_player, None)
        } else if matches!(
            raw_player,
            "PlayerRel::Opponent" | "PlayerRel::EachOpponent"
        ) {
            ("PlayerRel::EventPlayer", Some("RepeatDomain::EachOpponent"))
        } else if raw_player == "PlayerRel::EachPlayer" {
            ("PlayerRel::EventPlayer", Some("RepeatDomain::EachPlayer"))
        } else {
            return None;
        };

        let (count, count_is_one) = match p.take("Amount") {
            None => ("Amount::Fixed(1)".to_string(), true),
            Some(ref raw) if raw.trim() == "1" => ("Amount::Fixed(1)".to_string(), true),
            Some(raw) => {
                let amt = self.amount_or_count_ext(&raw)?;
                if amt == "Amount::Fixed(0)" {
                    return None;
                }
                let is_one = amt == "Amount::Fixed(1)";
                (amt, is_one)
            }
        };
        let max_revealed = match p.take("MaxRevealed") {
            None => None,
            Some(raw) => {
                let amt = self.amount_or_count_ext(&raw)?;
                if amt == "Amount::Fixed(0)" {
                    return None;
                }
                Some(amt)
            }
        };

        p.take("IsCurse");
        p.take("NoPutDesc");
        p.claim_label("ValidDescription", "Valid");
        let valid_raw = p.take("Valid")?;
        if matches!(valid_raw.trim(), "Card.NamedCard" | "NamedCard") {
            return None;
        }
        let raw_filter = self.filter_expr_in_zone(&valid_raw, true)?;
        let filter_expr = Self::simplify_your_modified_filter(&raw_filter);
        let filter_ref = format!("&{}", self.body.filter_static("REVEAL_UNTIL", &filter_expr));

        let take_bool = |params: &mut Params, key: &str| -> Option<bool> {
            match params.take(key).as_deref() {
                None | Some("False") => Some(false),
                Some("True") => Some(true),
                Some(_) => None,
            }
        };
        let tapped = take_bool(p, "Tapped")?;
        let gain_control = take_bool(p, "GainControl")?;
        let random_order = take_bool(p, "RevealRandomOrder")?;
        let shuffle = take_bool(p, "Shuffle")?;
        let no_move_revealed = take_bool(p, "NoMoveRevealed")?;
        let opt_a = take_bool(p, "Optional")?;
        let opt_b = take_bool(p, "RevealOptional")?;
        let optional_whole = opt_a || opt_b;
        if optional_whole && (library != "PlayerRel::You" || multi_seat_domain.is_some()) {
            return None;
        }
        let optional_found_move = take_bool(p, "OptionalFoundMove")?;
        let optional_no_dest = p.take("OptionalNoDestination");
        if optional_no_dest.is_some() && !optional_found_move {
            return None;
        }
        let (remember_found, imprint_found, remember_revealed, imprint_revealed) = if allow_remember
        {
            (
                take_bool(p, "RememberFound")?,
                take_bool(p, "ImprintFound")?,
                take_bool(p, "RememberRevealed")?,
                take_bool(p, "ImprintRevealed")?,
            )
        } else {
            (false, false, false, false)
        };

        let found_dest_raw = p.take("FoundDestination");
        let found_pos = p.take("FoundLibraryPosition");
        let revealed_raw = match (p.take("RevealedDestination"), p.take("RestDestination")) {
            (Some(r), None) | (None, Some(r)) => Some(r),
            (None, None) => None,
            (Some(_), Some(_)) => return None,
        };
        let rest_pos = p.take("RevealedLibraryPosition");

        let exile_while_revealing = revealed_raw.as_deref() == Some("Exile")
            && matches!(found_dest_raw.as_deref(), None | Some("Exile"))
            && !no_move_revealed;

        let mut trailing_shuffle = false;
        let rest = if no_move_revealed {
            if !matches!(revealed_raw.as_deref(), None | Some("Library")) || rest_pos.is_some() {
                return None;
            }
            if shuffle {
                "RevealUntilRest::ShuffleIntoLibrary".to_string()
            } else {
                "RevealUntilRest::StayInLibrary".to_string()
            }
        } else {
            match revealed_raw.as_deref() {
                Some("Library") => {
                    if shuffle && matches!(rest_pos.as_deref(), None | Some("-1" | "0")) {
                        "RevealUntilRest::ShuffleIntoLibrary".to_string()
                    } else if !shuffle && rest_pos.as_deref() == Some("-1") && random_order {
                        "RevealUntilRest::BottomRandom".to_string()
                    } else if !shuffle && rest_pos.as_deref() == Some("-1") && !random_order {
                        "RevealUntilRest::BottomChosenOrder".to_string()
                    } else if !shuffle
                        && matches!(rest_pos.as_deref(), None | Some("0"))
                        && !random_order
                    {
                        "RevealUntilRest::TopOfLibrary".to_string()
                    } else {
                        return None;
                    }
                }
                Some("Hand") if rest_pos.is_none() && !random_order => {
                    trailing_shuffle = shuffle;
                    "RevealUntilRest::Hand".to_string()
                }
                Some("Graveyard") if rest_pos.is_none() && !random_order => {
                    trailing_shuffle = shuffle;
                    "RevealUntilRest::Graveyard".to_string()
                }
                Some("Exile") if rest_pos.is_none() && !random_order => {
                    trailing_shuffle = shuffle;
                    "RevealUntilRest::Exile".to_string()
                }
                _ => return None,
            }
        };

        let found = match found_dest_raw.as_deref() {
            None => {
                if tapped || gain_control || found_pos.is_some() {
                    return None;
                }
                if no_move_revealed {
                    "RevealUntilFound::StayInLibrary".to_string()
                } else {
                    "RevealUntilFound::SameAsRest".to_string()
                }
            }
            Some("Hand") if !tapped && !gain_control && found_pos.is_none() => {
                "RevealUntilFound::Hand".to_string()
            }
            Some("Graveyard") if !tapped && !gain_control && found_pos.is_none() => {
                "RevealUntilFound::Graveyard".to_string()
            }
            Some("Exile") if !tapped && !gain_control && found_pos.is_none() => {
                "RevealUntilFound::Exile".to_string()
            }
            Some("Library") if !tapped && !gain_control => match found_pos.as_deref() {
                None | Some("0") => "RevealUntilFound::TopOfLibrary".to_string(),
                Some("-1")
                    if matches!(
                        rest.as_str(),
                        "RevealUntilRest::BottomRandom" | "RevealUntilRest::BottomChosenOrder"
                    ) =>
                {
                    "RevealUntilFound::SameAsRest".to_string()
                }
                _ => return None,
            },
            Some("Battlefield") if found_pos.is_none() => {
                let under_your_control =
                    (library == "PlayerRel::You" && multi_seat_domain.is_none()) || gain_control;
                format!(
                    "RevealUntilFound::Battlefield {{ tapped: {tapped}, \
                     under_your_control: {under_your_control} }}"
                )
            }
            _ => return None,
        };

        Some(ParsedDigUntil {
            library,
            multi_seat_domain,
            valid_raw,
            filter_ref,
            count,
            count_is_one,
            max_revealed,
            exile_while_revealing,
            found,
            found_dest_raw,
            tapped,
            rest,
            trailing_shuffle,
            optional_whole,
            optional_found_move,
            optional_no_dest,
            no_move_revealed,
            remember_found,
            imprint_found,
            remember_revealed,
            imprint_revealed,
        })
    }

    fn format_reveal_until_then(
        parsed: &ParsedDigUntil,
        found: &str,
        rest: &str,
        then_effs: &[String],
    ) -> String {
        let max_expr = match &parsed.max_revealed {
            None => "None".to_string(),
            Some(amt) => format!("Some({amt})"),
        };
        format!(
            "Effect::RevealUntilThen {{ library: {}, filter: {}, count: {}, \
             max_revealed: {max_expr}, exile_while_revealing: {}, found: {found}, \
             rest: {rest}, then: &[{}] }}",
            parsed.library,
            parsed.filter_ref,
            parsed.count,
            parsed.exile_while_revealing,
            then_effs.join(", ")
        )
    }

    fn finish_reveal_until_effects(parsed: &ParsedDigUntil, then_effs: &[String]) -> Vec<String> {
        let core = Self::format_reveal_until_then(parsed, &parsed.found, &parsed.rest, then_effs);
        let mut out = vec![core];
        if parsed.trailing_shuffle {
            out.push(format!(
                "Effect::ShuffleLibrary {{ who: {} }}",
                parsed.library
            ));
        }
        if let Some(domain) = parsed.multi_seat_domain {
            return vec![format!(
                "Effect::RepeatEach {{ domain: {domain}, effects: &[{}] }}",
                out.join(", ")
            )];
        }
        if parsed.optional_whole {
            return vec![format!(
                "Effect::MayDo {{ effects: &[{}] }}",
                out.join(", ")
            )];
        }
        out
    }

    fn dig_cleanup_next(&self, svar: &str, allow_imprinted: bool) -> Option<CleanupTail> {
        let body = self.svars.get(svar.trim())?;
        let (api, mut p) = Params::parse(body)?;
        p.drop_prose();
        if api != "Cleanup" {
            return None;
        }
        let next = p.take("SubAbility");
        let rem = p.take("ClearRemembered");
        let imp = if allow_imprinted {
            p.take("ClearImprinted")
        } else {
            None
        };
        if !p.exhausted() {
            return None;
        }
        let ok_rem = matches!(rem.as_deref(), None | Some("True"));
        let ok_imp = matches!(imp.as_deref(), None | Some("True"));
        if !(ok_rem && ok_imp && (rem.is_some() || imp.is_some())) {
            return None;
        }
        Some(CleanupTail::from_option(next))
    }

    fn chain_mentions_remembered(&self, start: &str) -> bool {
        let mut stack = vec![start.trim().to_string()];
        let mut visited = 0usize;
        while let Some(name) = stack.pop() {
            visited += 1;
            if visited > 24 {
                return true;
            }
            let Some(body) = self.svars.get(&name) else {
                continue;
            };
            if body.contains("Remembered")
                || body.contains("Imprinted")
                || body.contains("IsRemembered")
                || body.contains("IsImprinted")
            {
                return true;
            }
            let Some((_, p)) = Params::parse(body) else {
                continue;
            };
            for (k, v) in &p.entries {
                if matches!(
                    k.as_str(),
                    "SubAbility"
                        | "Execute"
                        | "WinSubAbility"
                        | "LoseSubAbility"
                        | "TrueSubAbility"
                        | "FalseSubAbility"
                        | "RepeatSubAbility"
                        | "StaticAbilities"
                ) {
                    for part in v.split(',') {
                        let trimmed = part.trim();
                        if !trimmed.is_empty() {
                            stack.push(trimmed.to_string());
                        }
                    }
                } else if let Some(svar_def) =
                    self.svars.get(v.trim().trim_start_matches(['+', '-']))
                    && (svar_def.contains("Remembered") || svar_def.contains("Imprinted"))
                {
                    return true;
                }
            }
        }
        false
    }

    fn strip_card_prefix(valid: &str) -> &str {
        let trimmed = valid.trim();
        trimmed.strip_prefix("Card.").unwrap_or(trimmed)
    }

    fn try_dig_until_immediate_trigger(
        &mut self,
        sub_name: &str,
        dig_valid: &str,
    ) -> Option<(String, Option<String>)> {
        let body = self.svars.get(sub_name.trim())?.clone();
        let (api, mut ip) = Params::parse(&body)?;
        ip.drop_prose();
        if api != "ImmediateTrigger"
            || ip.take("ConditionDefined").as_deref() != Some("Remembered")
            || !matches!(
                ip.take("ConditionCompare").as_deref(),
                None | Some("GE1" | "EQ1")
            )
        {
            return None;
        }
        let present = ip.take("ConditionPresent")?;
        if present != "Card"
            && Self::strip_card_prefix(&present) != Self::strip_card_prefix(dig_valid)
        {
            return None;
        }
        let optional = match ip.take("OptionalDecider").as_deref() {
            None => false,
            Some("You") => true,
            Some(_) => return None,
        };
        let exec_name = ip.take("Execute")?;
        let after = ip.take("SubAbility");
        if !ip.exhausted() {
            return None;
        }
        let trailing = match after {
            None => None,
            Some(ref cname) => {
                let next = self.dig_cleanup_next(cname, false)?.into_option();
                if next
                    .as_deref()
                    .is_some_and(|n| self.chain_mentions_remembered(n))
                {
                    return None;
                }
                next
            }
        };
        let mut inner = Chain::default();
        let (mut exec_effs, exec_trailing) =
            self.compile_dig_until_then_chain(&exec_name, true, false, &mut inner)?;
        if exec_trailing.is_some() || inner.second_target.is_some() {
            return None;
        }
        if optional {
            exec_effs = vec![format!(
                "Effect::MayDo {{ effects: &[{}] }}",
                exec_effs.join(", ")
            )];
        }
        let targets_expr = Self::target_req_expr(&inner)
            .map_or_else(|| "None".to_string(), |req| format!("Some({req})"));
        let reflexive = format!(
            "Effect::ReflexiveTrigger {{ when: ReflexiveWhen::Always, once: true, \
             condition: None, targets: {targets_expr}, remember: TriggerRemember::EventObject, \
             effects: &[{}] }}",
            exec_effs.join(", ")
        );
        Some((reflexive, trailing))
    }

    fn try_dig_until_exile_move_followup(
        &mut self,
        sub_name: &str,
        parsed: &ParsedDigUntil,
    ) -> Option<(String, Option<String>, Option<String>)> {
        let body = self.svars.get(sub_name.trim())?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if api != "ChangeZone" || sp.take("Origin").as_deref() != Some("Exile") {
            return None;
        }
        let want_def = if parsed.imprint_found && !parsed.remember_found {
            "Imprinted"
        } else {
            "Remembered"
        };
        if sp.take("Defined").as_deref() != Some(want_def) {
            return None;
        }
        let tapped = match sp.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let gain_control = match sp.take("GainControl").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let new_found = match sp.take("Destination").as_deref() {
            Some("Battlefield") => {
                let under_your_control = parsed.library == "PlayerRel::You" || gain_control;
                format!(
                    "RevealUntilFound::Battlefield {{ tapped: {tapped}, \
                     under_your_control: {under_your_control} }}"
                )
            }
            Some("Hand") if !tapped && !gain_control => "RevealUntilFound::Hand".to_string(),
            Some("Graveyard") if !tapped && !gain_control => {
                "RevealUntilFound::Graveyard".to_string()
            }
            _ => return None,
        };
        let next = sp.take("SubAbility");
        if !sp.exhausted() {
            return None;
        }
        if parsed.remember_revealed || parsed.imprint_revealed {
            let rest_name = next?;
            let (new_rest, after_rest) = self.parse_exile_rest_to_bottom(&rest_name, parsed)?;
            let trailing = self
                .consume_dig_cleanup_tail(after_rest, true)?
                .into_option();
            return Some((new_found, Some(new_rest), trailing));
        }
        let trailing = self
            .consume_dig_cleanup_tail(next, parsed.imprint_found)?
            .into_option();
        Some((new_found, None, trailing))
    }

    fn parse_exile_rest_to_bottom(
        &self,
        svar: &str,
        parsed: &ParsedDigUntil,
    ) -> Option<(String, Option<String>)> {
        let body = self.svars.get(svar.trim())?;
        let (api, mut rp) = Params::parse(body)?;
        rp.drop_prose();
        if api != "ChangeZoneAll"
            || rp.take("Origin").as_deref() != Some("Exile")
            || rp.take("Destination").as_deref() != Some("Library")
            || rp.take("LibraryPosition").as_deref() != Some("-1")
        {
            return None;
        }
        let change_type = rp.take("ChangeType")?;
        let valid_change = if parsed.remember_revealed && parsed.imprint_found {
            matches!(
                change_type.as_str(),
                "Card.IsRemembered" | "Card.IsRemembered+!IsImprinted"
            )
        } else if parsed.imprint_revealed && parsed.remember_found {
            matches!(
                change_type.as_str(),
                "Card.IsImprinted" | "Card.IsImprinted+!IsRemembered"
            )
        } else {
            false
        };
        if !valid_change {
            return None;
        }
        let random = match rp.take("RandomOrder").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let after = rp.take("SubAbility");
        if !rp.exhausted() {
            return None;
        }
        let rest = if random {
            "RevealUntilRest::BottomRandom".to_string()
        } else {
            "RevealUntilRest::BottomChosenOrder".to_string()
        };
        Some((rest, after))
    }

    fn consume_dig_cleanup_tail(
        &self,
        next: Option<String>,
        allow_imprinted: bool,
    ) -> Option<CleanupTail> {
        let Some(name) = next else {
            return Some(CleanupTail::End);
        };
        let after = self.dig_cleanup_next(&name, allow_imprinted)?.into_option();
        if after
            .as_deref()
            .is_some_and(|n| self.chain_mentions_remembered(n))
        {
            return None;
        }
        Some(CleanupTail::from_option(after))
    }

    #[allow(clippy::too_many_lines)]
    fn try_dig_until_play_followup(
        &mut self,
        sub_name: &str,
        parsed: &ParsedDigUntil,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<(Vec<String>, bool, Option<String>)> {
        let body = self.svars.get(sub_name.trim())?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if api != "Play" {
            return None;
        }
        let next = sp.take("SubAbility");
        let use_imprinted = parsed.imprint_found && !parsed.remember_found;
        let play_eff = self.compile_event_object_play(
            &mut sp,
            use_imprinted,
            parsed.found_dest_raw.as_deref(),
            target,
            targets_a_player,
        )?;
        if !sp.exhausted() {
            return None;
        }
        let Some(next_name) = next else {
            return Some((vec![play_eff], false, None));
        };
        if let Some(after) = self
            .dig_cleanup_next(&next_name, parsed.imprint_found)
            .map(CleanupTail::into_option)
        {
            if after
                .as_deref()
                .is_some_and(|n| self.chain_mentions_remembered(n))
            {
                return None;
            }
            return Some((vec![play_eff], false, after));
        }
        let next_body = self.svars.get(next_name.trim())?.clone();
        let (next_api, mut np) = Params::parse(&next_body)?;
        np.drop_prose();
        // Uncast exile -> hand fallback (`Breaching Dragonstorm` / `Solstice Revelations`).
        if next_api == "ChangeZone"
            && parsed.found == "RevealUntilFound::Exile"
            && np.take("Origin").as_deref() == Some("Exile")
            && np.take("Destination").as_deref() == Some("Hand")
            && np.take("Defined").as_deref()
                == Some(if use_imprinted {
                    "Imprinted"
                } else {
                    "Remembered"
                })
        {
            np.take("DefinedDesc");
            np.take("ConditionDescription");
            if !matches!(np.take("ForgetChanged").as_deref(), None | Some("True")) {
                return None;
            }
            let after_cz = np.take("SubAbility");
            if !np.exhausted() {
                return None;
            }
            let trailing = self
                .consume_dig_cleanup_tail(after_cz, parsed.imprint_found)?
                .into_option();
            return Some((
                vec![
                    play_eff,
                    "Effect::ReturnToHand { target: TargetSpec::EventObject }".to_string(),
                ],
                false,
                trailing,
            ));
        }
        // Reveal-in-library -> cast -> shuffle (`Spellshift` / `Underdark Beholder` shape).
        if next_api == "Shuffle"
            && parsed.no_move_revealed
            && parsed.found == "RevealUntilFound::StayInLibrary"
            && parsed.rest == "RevealUntilRest::StayInLibrary"
        {
            let shuf_who = self.wave11_player_of_line(
                np.take("Defined").as_deref(),
                target,
                targets_a_player,
            )?;
            if shuf_who != parsed.library {
                return None;
            }
            let after_shuf = np.take("SubAbility");
            if !np.exhausted() {
                return None;
            }
            let trailing = self
                .consume_dig_cleanup_tail(after_shuf, parsed.imprint_found)?
                .into_option();
            return Some((vec![play_eff], true, trailing));
        }
        None
    }

    fn compile_event_object_play(
        &mut self,
        sp: &mut Params,
        use_imprinted: bool,
        expected_zone: Option<&str>,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<String> {
        sp.take("DefinedDesc");
        if !matches!(sp.take("ForgetPlayed").as_deref(), None | Some("True")) {
            return None;
        }
        let expected_def = if use_imprinted {
            "Imprinted"
        } else {
            "Remembered"
        };
        let expected_attr = if use_imprinted {
            "IsImprinted"
        } else {
            "IsRemembered"
        };
        if let Some(vz) = sp.take("ValidZone")
            && expected_zone.is_some_and(|ez| ez != vz)
        {
            return None;
        }
        let mut filter_clauses: Vec<String> = Vec::new();
        let def = sp.take("Defined");
        let valid = sp.take("Valid");
        match (def.as_deref(), valid.as_deref()) {
            (Some(d), None) if d == expected_def => {}
            (Some(d), Some(v)) if d == expected_def => {
                let stripped = Self::strip_filter_attr(v, expected_attr)?;
                if stripped != "Card" {
                    filter_clauses.push(self.filter_expr_in_zone(&stripped, true)?);
                }
            }
            (None, Some(v)) => {
                let stripped = Self::strip_filter_attr(v, expected_attr)?;
                if stripped != "Card" {
                    filter_clauses.push(self.filter_expr_in_zone(&stripped, true)?);
                }
            }
            _ => return None,
        }
        let cost = match (
            sp.take("WithoutManaCost").as_deref(),
            sp.take("PlayCost").as_deref(),
        ) {
            (Some("True"), None) => "PlayCostMode::WithoutPayingManaCost".to_string(),
            (None | Some("False"), None) => "PlayCostMode::Normal".to_string(),
            _ => return None,
        };
        let copy = match sp.take("CopyCard").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let exile_instead_of_graveyard = match sp.take("ReplaceGraveyard").as_deref() {
            None => false,
            Some("Exile") => true,
            Some(_) => return None,
        };
        let optional = match sp.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let count = match sp.take("Amount").as_deref().map(str::trim) {
            None | Some("1") => "PlayCount::One".to_string(),
            Some("All") => "PlayCount::All".to_string(),
            Some(raw) => {
                let n: u8 = self.plain_num(raw)?.try_into().ok().filter(|&n| n >= 2)?;
                format!("PlayCount::UpTo({n})")
            }
        };
        let controller =
            self.wave11_player_of_line(sp.take("Controller").as_deref(), target, targets_a_player)?;
        if !Self::is_single_player_rel(controller) {
            return None;
        }
        let cast_only = match sp.take("ValidSA") {
            None => copy || exile_instead_of_graveyard,
            Some(raw_sa) => {
                if let DigPlaySaFilter::Filtered(sa_filter) =
                    self.parse_dig_play_sa_filter(&raw_sa)?
                {
                    filter_clauses.push(sa_filter);
                }
                true
            }
        };
        let filter_ref = match filter_clauses.len() {
            0 => "&Filter::Any".to_string(),
            1 => format!("&{}", self.body.filter_static("PLAY", &filter_clauses[0])),
            _ => {
                let combined = format!("Filter::And(&[{}])", filter_clauses.join(", "));
                format!("&{}", self.body.filter_static("PLAY", &combined))
            }
        };
        Some(format!(
            "Effect::Play {{ source: PlaySource::Subject(TargetSpec::EventObject), \
             filter: {filter_ref}, controller: {controller}, count: {count}, cost: {cost}, \
             cast_only: {cast_only}, copy: {copy}, \
             exile_instead_of_graveyard: {exile_instead_of_graveyard}, optional: {optional} }}"
        ))
    }

    fn parse_dig_play_sa_filter(&self, raw: &str) -> Option<DigPlaySaFilter> {
        let mut alts = Vec::new();
        for part in raw.split(',').map(str::trim) {
            if part == "Spell" {
                return Some(DigPlaySaFilter::AnySpell);
            }
            let suffix = part.strip_prefix("Spell.")?;
            let expr = self.filter_expr_in_zone(&format!("Card.{suffix}"), true)?;
            alts.push(expr);
        }
        match alts.len() {
            0 => None,
            1 => Some(DigPlaySaFilter::Filtered(alts.remove(0))),
            _ => Some(DigPlaySaFilter::Filtered(format!(
                "Filter::Or(&[{}])",
                alts.join(", ")
            ))),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn try_dig_until_exile_permission_followup(
        &mut self,
        sub_name: &str,
        parsed: &ParsedDigUntil,
    ) -> Option<(String, Option<String>, Option<String>)> {
        let body = self.svars.get(sub_name.trim())?.clone();
        let (api, mut ep) = Params::parse(&body)?;
        ep.drop_prose();
        if api != "Effect" {
            return None;
        }
        let use_imprinted = parsed.imprint_found && !parsed.remember_found;
        let rem_obj = ep.take("RememberObjects")?;
        let ok_rem = if use_imprinted {
            rem_obj == "Imprinted"
        } else {
            matches!(rem_obj.as_str(), "Remembered" | "RememberedCard")
        };
        if !ok_rem {
            return None;
        }
        if let Some(fom) = ep.take("ForgetOnMoved").or_else(|| ep.take("ExileOnMoved"))
            && fom != "Exile"
        {
            return None;
        }
        let window = match ep.take("Duration").as_deref() {
            None => {
                // `GrantLinkedExilePermission` applies to every card linked to
                // `res.source`; on a permanent that activates repeatedly across
                // turns, a `ThisTurn` window would erroneously refresh earlier
                // turns' linked cards, so allow `ThisTurn` only on spells.
                if !self.on_a_spell {
                    return None;
                }
                "PlayWindow::ThisTurn"
            }
            Some("Permanent" | "UntilHostLeavesPlay") if !self.on_a_spell => {
                "PlayWindow::WhileSourceOnBattlefield"
            }
            _ => return None,
        };
        let stat_name = ep.take("StaticAbilities")?;
        if stat_name.contains(',') {
            return None;
        }
        let next = ep.take("SubAbility");
        if !ep.exhausted() {
            return None;
        }
        let stat_body = self.svars.get(stat_name.trim())?.clone();
        let (s_mode, mut st) = Params::parse(&stat_body)?;
        st.drop_prose();
        if s_mode != "Continuous"
            || st.take("MayPlay").as_deref() != Some("True")
            || st.take("AffectedZone").as_deref() != Some("Exile")
        {
            return None;
        }
        let affected = st.take("Affected")?;
        // `DB$ Effect | RememberObjects$ ...` stores objects on the Effect
        // itself as `Remembered`, so its `StaticAbilities$` reads `IsRemembered`
        // (or `IsImprinted` when imprinted directly).
        let stripped = Self::strip_filter_attr(&affected, "IsRemembered").or_else(|| {
            if use_imprinted {
                Self::strip_filter_attr(&affected, "IsImprinted")
            } else {
                None
            }
        })?;
        let cast_only = match stripped.as_str() {
            "Card" => false,
            "Card.nonLand" => true,
            _ => return None,
        };
        let free = match st.take("MayPlayWithoutManaCost").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let any_type = match st.take("MayPlayAnyMana").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let any_color = match st.take("MayPlayAnyColor").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if any_type && any_color {
            return None;
        }
        if !st.exhausted() {
            return None;
        }
        let mut perm = if cast_only {
            format!("ExilePermission::cast({window})")
        } else {
            format!("ExilePermission::play({window})")
        };
        if free {
            perm.push_str(".without_paying_mana_cost()");
        }
        if any_type {
            perm.push_str(".spend_mana_as_any_type()");
        } else if any_color {
            perm.push_str(".spend_mana_as_any_color()");
        }
        let eff = format!("Effect::GrantLinkedExilePermission {{ permission: {perm} }}");
        if parsed.remember_revealed || parsed.imprint_revealed {
            let rest_name = next?;
            let (new_rest, after_rest) = self.parse_exile_rest_to_bottom(&rest_name, parsed)?;
            let trailing = self
                .consume_dig_cleanup_tail(after_rest, true)?
                .into_option();
            return Some((eff, Some(new_rest), trailing));
        }
        let trailing = self
            .consume_dig_cleanup_tail(next, parsed.imprint_found)?
            .into_option();
        Some((eff, None, trailing))
    }

    #[allow(clippy::too_many_lines)]
    fn compile_dig_until_then_chain(
        &mut self,
        start_svar: &str,
        count_is_one: bool,
        use_imprinted: bool,
        chain: &mut Chain,
    ) -> Option<(Vec<String>, Option<String>)> {
        let mut out: Vec<String> = Vec::new();
        let mut cursor = Some(start_svar.trim().to_string());
        let mut steps = 0usize;
        while let Some(name) = cursor.take() {
            steps += 1;
            if steps > 12 {
                return None;
            }
            if let Some(after) = self
                .dig_cleanup_next(&name, true)
                .map(CleanupTail::into_option)
            {
                if after
                    .as_deref()
                    .is_some_and(|n| self.chain_mentions_remembered(n))
                {
                    return None;
                }
                if out.is_empty()
                    || ((chain.target.is_some() || chain.second_target.is_some())
                        && out.iter().any(|e| e.contains("Filter::Reference")))
                {
                    return None;
                }
                return Some((out, after));
            }
            let body = self.svars.get(&name)?.clone();
            let (api, mut sp) = Params::parse(&body)?;
            sp.drop_prose();
            let next = sp.take("SubAbility");
            if api == "RepeatEach" && sp.has("RepeatCards") {
                let repeat_eff =
                    self.compile_repeat_each_object_core(sp, None, true, use_imprinted, chain)?;
                out.push(repeat_eff);
                cursor = next;
                continue;
            }
            if !count_is_one {
                return None;
            }
            let step_effs =
                self.compile_event_object_step(&api, &mut sp, use_imprinted, true, chain)?;
            out.extend(step_effs);
            cursor = next;
        }
        if out.is_empty()
            || ((chain.target.is_some() || chain.second_target.is_some())
                && out.iter().any(|e| e.contains("Filter::Reference")))
        {
            return None;
        }
        Some((out, None))
    }

    /// Compiles one step inside `RevealUntilThen.then` or `RepeatEachObject.then`
    /// where `Remembered` (or `Imprinted` when `use_imprinted`) names the bound
    /// `EventObject` (and its controller/owner is `PlayerRel::EventPlayer`).
    #[allow(clippy::too_many_lines)]
    fn compile_event_object_step(
        &mut self,
        api: &str,
        sp: &mut Params,
        use_imprinted: bool,
        allow_outer_targets: bool,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let obj_tag = if use_imprinted {
            "Imprinted"
        } else {
            "Remembered"
        };
        let ctrl_tag = if use_imprinted {
            "ImprintedController"
        } else {
            "RememberedController"
        };
        let owner_tag = if use_imprinted {
            "ImprintedOwner"
        } else {
            "RememberedOwner"
        };

        // Strip harmless Forge UI prompt hint naming the current iterated card.
        if sp.peek("ShowCurrentCard") == Some(obj_tag) {
            sp.take("ShowCurrentCard");
        }

        // Optional `ConditionDefined$ Remembered | ConditionPresent$ <F>` wrapper
        // around the step -> `Effect::IfEventObjectMatches`.
        let mut cond_filter: Option<String> = None;
        if sp.peek("ConditionDefined") == Some(obj_tag) && sp.has("ConditionPresent") {
            if !matches!(
                sp.take("ConditionCompare").as_deref(),
                None | Some("GE1" | "EQ1")
            ) {
                return None;
            }
            sp.take("ConditionDefined");
            sp.take("ConditionDescription");
            let present = sp.take("ConditionPresent")?;
            if present != "Card" {
                let expr = Self::simplify_your_modified_filter(&self.filter_expr(&present)?);
                cond_filter = Some(format!("&{}", self.body.filter_static("COND", &expr)));
            }
        }

        // Inside `RevealUntilThen.then`, `ConditionCheckSVar$ V | ConditionSVarCompare$ EQ1|GE1`
        // where `V` is `Remembered$Amount` (or `Imprinted$Amount`) holds automatically
        // because `then` runs only when at least one card was found.
        if allow_outer_targets
            && !sp.has("ConditionDefined")
            && let Some(var) = sp.peek("ConditionCheckSVar")
            && matches!(
                self.svars
                    .get(var.trim())
                    .map(String::as_str)
                    .map(str::trim),
                Some("Remembered$Amount" | "Imprinted$Amount")
            )
            && matches!(
                sp.peek("ConditionSVarCompare"),
                None | Some("EQ1" | "GE1" | "GT0")
            )
        {
            sp.take("ConditionCheckSVar");
            sp.take("ConditionSVarCompare");
            sp.take("ConditionDescription");
        }

        // Hoist `ValidTgts$` onto outer `chain.target` ONLY when inside
        // `RevealUntilThen.then` (single spell/ability target chosen on cast)
        // and no outer target has been declared yet.
        let mut local_target: Option<String> = None;
        if sp.has("ValidTgts") {
            if !allow_outer_targets || chain.target.is_some() || chain.second_target.is_some() {
                return None;
            }
            sp.claim_label("ValidTgtsDesc", "ValidTgts");
            let valid = sp.take("ValidTgts")?;
            self.chain_target(api, sp, &valid, chain, None)?;
            local_target = Self::effect_target(chain);
        }

        let effs = match api {
            "DealDamage" => {
                if let Some(eff) = self.try_event_object_deal_damage(
                    sp,
                    obj_tag,
                    ctrl_tag,
                    owner_tag,
                    local_target.as_deref(),
                    chain,
                ) {
                    vec![eff]
                } else {
                    self.delegate_event_object_step(api, sp, obj_tag, ctrl_tag, owner_tag)?
                }
            }
            "DamageAll" => {
                vec![self.compile_event_object_damage_all(sp, obj_tag)?]
            }
            "Pump" => {
                if let Some(eff) = self.try_event_object_pump(sp, obj_tag, local_target.as_deref())
                {
                    vec![eff]
                } else {
                    self.delegate_event_object_step(api, sp, obj_tag, ctrl_tag, owner_tag)?
                }
            }
            "PutCounter" => {
                if let Some(eff) =
                    self.try_event_object_put_counter(sp, obj_tag, local_target.as_deref(), chain)
                {
                    vec![eff]
                } else {
                    self.delegate_event_object_step(api, sp, obj_tag, ctrl_tag, owner_tag)?
                }
            }
            "GainLife" | "LoseLife" => {
                if let Some(eff) = self.try_event_object_life(api, sp, obj_tag, ctrl_tag, owner_tag)
                {
                    vec![eff]
                } else {
                    self.delegate_event_object_step(api, sp, obj_tag, ctrl_tag, owner_tag)?
                }
            }
            "CopyPermanent" => {
                vec![self.compile_event_object_copy(
                    sp,
                    obj_tag,
                    ctrl_tag,
                    owner_tag,
                    "CopyCleanup::None",
                )?]
            }
            "Play" => {
                let eff = self.compile_event_object_play(sp, use_imprinted, None, None, false)?;
                if !sp.exhausted() {
                    return None;
                }
                vec![eff]
            }
            "Sacrifice"
                if !self.on_a_spell
                    && !sp.has("Defined")
                    && !sp.has("SacValid")
                    && !sp.has("UnlessCost")
                    && sp.exhausted() =>
            {
                vec!["Effect::SacrificeSelf".to_string()]
            }
            _ => self.delegate_event_object_step(api, sp, obj_tag, ctrl_tag, owner_tag)?,
        };

        Ok::<Vec<String>, ()>(match cond_filter {
            None => effs,
            Some(filter) => vec![format!(
                "Effect::IfEventObjectMatches {{ filter: {filter}, then: &[{}] }}",
                effs.join(", ")
            )],
        })
        .ok()
    }

    fn remembered_event_amount(&self, raw: &str, obj_tag: &str) -> Option<(String, bool)> {
        let trimmed = raw.trim();
        let neg = trimmed.starts_with('-');
        let key = trimmed.trim_start_matches(['+', '-']);
        let def = match self.svars.get(key) {
            Some(d) => d.trim(),
            None if key.contains('$') => key,
            None => return None,
        };
        let (head, suffix) = def.split_once('/').map_or((def, ""), |(h, s)| (h, s));
        let (prefix, prop) = head.split_once('$')?;
        let matches_tag = prefix == obj_tag || prefix == format!("{obj_tag}LKI");
        if !matches_tag {
            return None;
        }
        let base = match prop {
            "CardManaCost" => "Amount::EventManaValue".to_string(),
            "CardPower" => "Amount::EventPower".to_string(),
            "CardToughness" => "Amount::EventLastToughness".to_string(),
            _ => return None,
        };
        let mag = if suffix.is_empty() {
            base
        } else if suffix == "Twice" {
            format!("Amount::Times {{ base: &{base}, factor: 2 }}")
        } else if suffix == "Thrice" {
            format!("Amount::Times {{ base: &{base}, factor: 3 }}")
        } else if let Some(n) = suffix
            .strip_prefix("Times.")
            .and_then(|s| s.parse::<u32>().ok())
            .filter(|&n| n >= 2)
        {
            format!("Amount::Times {{ base: &{base}, factor: {n} }}")
        } else if let Some(n) = suffix
            .strip_prefix("Plus.")
            .and_then(|s| s.parse::<u32>().ok())
        {
            format!("Amount::Plus {{ base: &{base}, offset: {n} }}")
        } else if let Some(n) = suffix
            .strip_prefix("Minus.")
            .and_then(|s| s.parse::<u32>().ok())
        {
            format!("Amount::SaturatingSub {{ base: &{base}, subtract: {n} }}")
        } else if suffix == "HalfDown" {
            format!("Amount::Half {{ base: &{base}, round_up: false }}")
        } else if suffix == "HalfUp" {
            format!("Amount::Half {{ base: &{base}, round_up: true }}")
        } else {
            return None;
        };
        Some((mag, neg))
    }

    fn try_event_object_deal_damage(
        &mut self,
        sp: &mut Params,
        obj_tag: &str,
        ctrl_tag: &str,
        owner_tag: &str,
        local_target: Option<&str>,
        chain: &Chain,
    ) -> Option<String> {
        let mut probe = sp.clone();
        let dmg_source = probe.take("DamageSource");
        let raw_num = probe.take("NumDmg")?;
        let ev_amt = self.remembered_event_amount(&raw_num, obj_tag);
        if dmg_source.is_none() && ev_amt.is_none() {
            return None;
        }
        let amount = match ev_amt {
            Some((amt, false)) => amt,
            Some((_, true)) => return None,
            None => self.amount_or_count_ext(&raw_num)?,
        };
        probe.take("IsCurse");
        let defined = probe.take("Defined");
        if !probe.exhausted() {
            return None;
        }
        let target_spec = match defined.as_deref() {
            None => local_target
                .map(str::to_string)
                .or_else(|| Self::effect_target(chain))?,
            Some(d) if d == obj_tag => "TargetSpec::EventObject".to_string(),
            Some(d) if d == ctrl_tag || d == owner_tag => {
                "TargetSpec::Player(PlayerRel::EventPlayer)".to_string()
            }
            Some(other) => {
                let who = self.wave11_player_rel(Some(other))?;
                format!("TargetSpec::Player({who})")
            }
        };
        let eff = match dmg_source.as_deref() {
            None => format!("Effect::DealDamage {{ amount: {amount}, target: {target_spec} }}"),
            Some(src) if src == obj_tag => {
                if amount == "Amount::EventPower" {
                    format!(
                        "Effect::EventObjectDealsDamageEqualToPower {{ target: {target_spec} }}"
                    )
                } else {
                    format!(
                        "Effect::EventObjectDealsDamage {{ amount: {amount}, target: {target_spec} }}"
                    )
                }
            }
            Some(_) => return None,
        };
        *sp = probe;
        Some(eff)
    }

    fn compile_event_object_damage_all(
        &mut self,
        sp: &mut Params,
        obj_tag: &str,
    ) -> Option<String> {
        let dmg_source = sp.take("DamageSource")?;
        if dmg_source != obj_tag || sp.has("ValidCards") {
            return None;
        }
        let valid_players = sp.take("ValidPlayers")?;
        let who = match valid_players.as_str() {
            "Opponent" | "Player.Opponent" => "PlayerRel::EachOpponent",
            "Player" => "PlayerRel::EachPlayer",
            "You" => "PlayerRel::You",
            _ => return None,
        };
        let raw_num = sp.take("NumDmg")?;
        let amount = match self.remembered_event_amount(&raw_num, obj_tag) {
            Some((amt, false)) => amt,
            Some((_, true)) => return None,
            None => self.amount_or_count_ext(&raw_num)?,
        };
        if !sp.exhausted() {
            return None;
        }
        if amount == "Amount::EventPower" {
            Some(format!(
                "Effect::EventObjectDealsDamageEqualToPower {{ target: TargetSpec::Player({who}) }}"
            ))
        } else {
            Some(format!(
                "Effect::EventObjectDealsDamage {{ amount: {amount}, target: TargetSpec::Player({who}) }}"
            ))
        }
    }

    fn try_event_object_pump(
        &mut self,
        sp: &mut Params,
        obj_tag: &str,
        local_target: Option<&str>,
    ) -> Option<String> {
        let att_raw = sp.peek("NumAtt").map(str::to_string);
        let def_raw = sp.peek("NumDef").map(str::to_string);
        let att_ev = att_raw
            .as_deref()
            .and_then(|r| self.remembered_event_amount(r, obj_tag));
        let def_ev = def_raw
            .as_deref()
            .and_then(|r| self.remembered_event_amount(r, obj_tag));
        if att_ev.is_none() && def_ev.is_none() {
            return None;
        }
        let mut probe = sp.clone();
        probe.take("NumAtt");
        probe.take("NumDef");
        probe.take("IsCurse");
        if probe.has("KW") || probe.has("KWChoice") {
            return None;
        }
        let duration = match probe.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some(_) => return None,
        };
        let defined = probe.take("Defined");
        if !probe.exhausted() {
            return None;
        }
        let resolve_side =
            |this: &mut Self, raw: Option<&str>, ev: Option<(String, bool)>| -> Option<String> {
                match (raw, ev) {
                    (_, Some((mag, false))) => Some(mag),
                    (_, Some((mag, true))) => Some(format!("Amount::Negated(&{mag})")),
                    (None, None) => Some("Amount::Fixed(0)".to_string()),
                    (Some(r), None) => super::pump_amount(r, this.svars, this.has_x),
                }
            };
        let power = resolve_side(self, att_raw.as_deref(), att_ev)?;
        let toughness = resolve_side(self, def_raw.as_deref(), def_ev)?;
        let eff = match defined.as_deref() {
            Some("Self") if !self.on_a_spell => format!(
                "Effect::PumpTargetSlot {{ slot: TargetSlot::This, power: {power}, \
                 toughness: {toughness}, keywords: KeywordSet::EMPTY, duration: {duration} }}"
            ),
            None if local_target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) => {
                format!(
                    "Effect::PumpTarget {{ power: {power}, toughness: {toughness}, \
                     keywords: KeywordSet::EMPTY, duration: {duration} }}"
                )
            }
            Some(d) if d == obj_tag => {
                format!(
                    "Effect::PumpEventObject {{ power: {power}, toughness: {toughness}, \
                     keywords: KeywordSet::EMPTY, duration: {duration} }}"
                )
            }
            _ => return None,
        };
        *sp = probe;
        Some(eff)
    }

    fn try_event_object_put_counter(
        &mut self,
        sp: &mut Params,
        obj_tag: &str,
        local_target: Option<&str>,
        chain: &Chain,
    ) -> Option<String> {
        let raw_num = sp.peek("CounterNum")?.to_string();
        let (amount, false) = self.remembered_event_amount(&raw_num, obj_tag)? else {
            return None;
        };
        let mut probe = sp.clone();
        probe.take("CounterNum");
        let kind = counter_kind(&probe.take("CounterType")?)?;
        probe.take("IsCurse");
        let defined = probe.take("Defined");
        if !probe.exhausted() {
            return None;
        }
        let eff = match defined.as_deref() {
            None if local_target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) => {
                format!("Effect::AddCounter {{ kind: {kind}, amount: {amount} }}")
            }
            Some("Self") if !self.on_a_spell => {
                format!("Effect::AddCounterSource {{ kind: {kind}, amount: {amount} }}")
            }
            None if !self.on_a_spell && local_target.is_none() && chain.target.is_none() => {
                format!("Effect::AddCounterSource {{ kind: {kind}, amount: {amount} }}")
            }
            Some(d) if d == obj_tag && chain.target.is_none() && chain.second_target.is_none() => {
                format!(
                    "Effect::AddCounterFilter {{ filter: &Filter::Reference, kind: {kind}, \
                     amount: {amount} }}"
                )
            }
            _ => return None,
        };
        *sp = probe;
        Some(eff)
    }

    fn try_event_object_life(
        &mut self,
        api: &str,
        sp: &mut Params,
        obj_tag: &str,
        ctrl_tag: &str,
        owner_tag: &str,
    ) -> Option<String> {
        let raw_num = sp.peek("LifeAmount")?.to_string();
        let (amount, false) = self.remembered_event_amount(&raw_num, obj_tag)? else {
            return None;
        };
        let mut probe = sp.clone();
        probe.take("LifeAmount");
        probe.take("IsCurse");
        let defined = probe.take("Defined");
        if !probe.exhausted() {
            return None;
        }
        let who = match defined.as_deref() {
            None | Some("You") => "PlayerRel::You",
            Some(d) if d == ctrl_tag || d == owner_tag => "PlayerRel::EventPlayer",
            Some(other) => self.wave11_player_rel(Some(other))?,
        };
        *sp = probe;
        Some(match (api, who) {
            ("GainLife", "PlayerRel::You") => format!("Effect::GainLife {{ amount: {amount} }}"),
            ("GainLife", _) => {
                format!("Effect::GainLifeFor {{ amount: {amount}, who: {who} }}")
            }
            _ => format!("Effect::LoseLife {{ amount: {amount}, target: {who} }}"),
        })
    }

    fn compile_event_object_copy(
        &mut self,
        sp: &mut Params,
        obj_tag: &str,
        ctrl_tag: &str,
        owner_tag: &str,
        cleanup: &str,
    ) -> Option<String> {
        if sp.take("Defined").as_deref() != Some(obj_tag) {
            return None;
        }
        let controller = match sp.take("Controller").as_deref() {
            None | Some("You") => "PlayerRel::You",
            Some(c) if c == ctrl_tag || c == owner_tag => "PlayerRel::EventPlayer",
            Some(other) => self.wave11_player_rel(Some(other))?,
        };
        if !Self::is_single_player_rel(controller) {
            return None;
        }
        let amount = match sp.take("NumCopies") {
            None => "Amount::Fixed(1)".to_string(),
            Some(raw) => self.amount_or_count_ext(&raw)?,
        };
        let take_bool = |params: &mut Params, key: &str| -> Option<bool> {
            match params.take(key).as_deref() {
                None | Some("False") => Some(false),
                Some("True") => Some(true),
                Some(_) => None,
            }
        };
        let tapped = take_bool(sp, "TokenTapped")?;
        let attacking = take_bool(sp, "TokenAttacking")?;
        let haste_until_eot = match sp.take("AddKeywords").as_deref() {
            None => false,
            Some("Haste") => true,
            Some(_) => return None,
        };
        let eff_cleanup = match sp.take("AtEOT").as_deref() {
            None => cleanup,
            Some("Sacrifice") if cleanup == "CopyCleanup::None" => {
                "CopyCleanup::SacrificeAtNextEndStep"
            }
            Some("Exile") if cleanup == "CopyCleanup::None" => "CopyCleanup::ExileAtNextEndStep",
            Some(_) => return None,
        };
        if !sp.exhausted() {
            return None;
        }
        Some(format!(
            "Effect::CreateTokenCopy {{ target: TargetSpec::EventObject, \
             controller: {controller}, amount: {amount}, mods: &[], tapped: {tapped}, \
             attacking: {attacking}, haste_until_eot: {haste_until_eot}, cleanup: {eff_cleanup} }}"
        ))
    }

    fn delegate_event_object_step(
        &mut self,
        api: &str,
        sp: &mut Params,
        obj_tag: &str,
        ctrl_tag: &str,
        owner_tag: &str,
    ) -> Option<Vec<String>> {
        if sp
            .entries
            .iter()
            .any(|(_, v)| v.contains("DelayTriggerRemembered") || v.contains("Triggered"))
        {
            return None;
        }
        let mut uses_event_obj = false;
        let mut uses_event_player = false;
        let lki_tag = format!("{obj_tag}LKI");
        for (k, v) in &mut sp.entries {
            if k == "Defined" {
                if v == obj_tag || v == &lki_tag {
                    *v = "DelayTriggerRemembered".to_string();
                    uses_event_obj = true;
                } else if v == ctrl_tag || v == owner_tag {
                    *v = "TriggeredPlayer".to_string();
                    uses_event_player = true;
                }
            } else if k == "UnlessPayer" && (v == ctrl_tag || v == owner_tag) {
                *v = "TriggeredPlayer".to_string();
                uses_event_player = true;
            }
        }
        let spec = Self::reconstruct_dig_spec(api, sp);
        let mut inner = Chain::default();
        let saved_delayed = std::mem::replace(&mut self.in_delayed, uses_event_obj);
        let saved_mode = if uses_event_player {
            self.trigger_mode.replace("LifeGained".into())
        } else {
            self.trigger_mode.clone()
        };
        let ok = self.chain(&spec, &mut inner);
        self.in_delayed = saved_delayed;
        self.trigger_mode = saved_mode;
        ok?;
        if inner.target.is_some() || inner.second_target.is_some() {
            return None;
        }
        Some(inner.effects)
    }

    fn reconstruct_dig_spec(api: &str, p: &Params) -> String {
        let mut spec = format!("DB$ {api}");
        for (k, v) in &p.entries {
            spec.push_str(" | ");
            spec.push_str(k);
            spec.push_str("$ ");
            spec.push_str(v);
        }
        spec
    }

    fn strip_filter_attr(raw: &str, attr: &str) -> Option<String> {
        let mut alts = Vec::new();
        for alt in raw.split(',') {
            let trimmed = alt.trim();
            let (noun, attrs) = trimmed.split_once('.')?;
            let mut kept = Vec::new();
            let mut found = false;
            for piece in attrs.split('+').map(str::trim) {
                if piece == attr {
                    found = true;
                } else if !piece.is_empty() {
                    kept.push(piece);
                }
            }
            if !found {
                return None;
            }
            if kept.is_empty() {
                alts.push(noun.trim().to_string());
            } else {
                alts.push(format!("{}.{}", noun.trim(), kept.join("+")));
            }
        }
        (!alts.is_empty()).then(|| alts.join(","))
    }

    /// Wave 23a fallback for `RepeatEach` -> `Effect::RepeatEachObject`:
    /// handles non-battlefield `Zone$` (`Exile`, `Graveyard`, `Hand`, `Library`),
    /// `UseImprinted$ True` (including `CopyPermanent` + end-of-combat/end-step
    /// delayed exile/sacrifice cleanup), `DamageMap$ True` per-object damage,
    /// `ShowCurrentCard$` UI prose, and nested `RepeatPlayers$ -> RepeatCards$`
    /// over `RememberedPlayerCtrl`.
    pub(super) fn wave23a_repeat_each_object_chain(
        &mut self,
        p: Params,
        sub: Option<&str>,
        chain: &mut Chain,
    ) -> Option<()> {
        if p.has("RepeatPlayers") && !p.has("RepeatCards") {
            return self.try_nested_player_card_repeat(p, sub, chain);
        }
        if !p.has("RepeatCards") {
            return None;
        }
        let mut trailing = sub.map(str::to_string);
        let eff =
            self.compile_repeat_each_object_core(p, Some(&mut trailing), false, false, chain)?;
        chain.effects.push(eff);
        self.chain_sub_ability(trailing, chain)
    }

    /// Collapses `RepeatEach | RepeatPlayers$ Player/Opponent` ->
    /// `RepeatEach | RepeatCards$ <Filter>.RememberedPlayerCtrl | Zone$ Battlefield`
    /// (*Fade Away* shape) into a single `Effect::RepeatEachObject` (which
    /// iterates in controller APNAP order with `PlayerRel::EventPlayer` bound
    /// to each permanent's controller).
    fn try_nested_player_card_repeat(
        &mut self,
        mut outer: Params,
        sub: Option<&str>,
        chain: &mut Chain,
    ) -> Option<()> {
        let rp = outer.take("RepeatPlayers")?;
        let opp_only = match rp.as_str() {
            "Player" => false,
            "Opponent" | "Player.Opponent" => true,
            _ => return None,
        };
        let mid_name = outer.take("RepeatSubAbility")?;
        if !outer.exhausted() {
            return None;
        }
        let mid_body = self.svars.get(mid_name.trim())?.clone();
        let (mid_api, mut mp) = Params::parse(&mid_body)?;
        mp.drop_prose();
        if mid_api != "RepeatEach" {
            return None;
        }
        if !matches!(mp.take("Zone").as_deref(), None | Some("Battlefield")) {
            return None;
        }
        let rc = mp.take("RepeatCards")?;
        let leaf_name = mp.take("RepeatSubAbility")?;
        if let Some(mid_sub) = mp.take("SubAbility")
            && self.dig_cleanup_next(&mid_sub, false).is_none()
        {
            return None;
        }
        if !mp.exhausted() {
            return None;
        }
        let stripped = Self::strip_filter_attr(&rc, "RememberedPlayerCtrl")?;
        let base_expr = Self::simplify_your_modified_filter(&self.filter_expr(&stripped)?);
        let full_expr = if opp_only {
            if base_expr == "Filter::Any" {
                "Filter::ControlledByOpponent".to_string()
            } else {
                format!("Filter::And(&[{base_expr}, Filter::ControlledByOpponent])")
            }
        } else {
            base_expr
        };
        let filter = self.body.filter_static("REPEAT", &full_expr);

        // Compile leaf sub-ability chain where `Player.IsRemembered` /
        // `RememberedPlayer` / `RememberedController` refer to `EventPlayer`.
        let mut body_effs = Vec::new();
        let mut cursor = Some(leaf_name.trim().to_string());
        let mut steps = 0usize;
        while let Some(name) = cursor.take() {
            steps += 1;
            if steps > 12 {
                return None;
            }
            if self.dig_cleanup_next(&name, false).is_some() {
                break;
            }
            let body = self.svars.get(&name)?.clone();
            let (api, mut sp) = Params::parse(&body)?;
            sp.drop_prose();
            if sp.has("ValidTgts")
                || sp
                    .entries
                    .iter()
                    .any(|(_, v)| v.contains("Triggered") || v.contains("DelayTriggerRemembered"))
            {
                return None;
            }
            let next = sp.take("SubAbility");
            for (k, v) in &mut sp.entries {
                if matches!(k.as_str(), "Defined" | "UnlessPayer") {
                    if matches!(
                        v.as_str(),
                        "Player.IsRemembered" | "RememberedPlayer" | "RememberedController"
                    ) {
                        *v = "TriggeredPlayer".to_string();
                    } else if v == "Remembered" {
                        return None;
                    }
                }
            }
            let spec = Self::reconstruct_dig_spec(&api, &sp);
            let mut inner = Chain::default();
            let saved_mode = self.trigger_mode.replace("LifeGained".into());
            let ok = self.chain(&spec, &mut inner);
            self.trigger_mode = saved_mode;
            ok?;
            if inner.target.is_some() || inner.second_target.is_some() {
                return None;
            }
            body_effs.extend(inner.effects);
            cursor = next;
        }
        if body_effs.is_empty() {
            return None;
        }
        chain.effects.push(format!(
            "Effect::RepeatEachObject {{ zone: ZoneSel::Battlefield, filter: &{filter}, \
             only_found: false, then: &[{}] }}",
            body_effs.join(", ")
        ));
        self.chain_sub_ability(sub.map(str::to_string), chain)
    }

    #[allow(clippy::too_many_lines)]
    fn compile_repeat_each_object_core(
        &mut self,
        mut p: Params,
        outer_sub: Option<&mut Option<String>>,
        allow_only_found: bool,
        outer_imprint_found: bool,
        chain: &Chain,
    ) -> Option<String> {
        let rc = p.take("RepeatCards")?;
        let repeat_sub = p.take("RepeatSubAbility")?;
        let zone_raw = p.take("Zone");
        let use_imprinted = match p.take("UseImprinted").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let damage_map = match p.take("DamageMap").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let change_zone_table = match p.take("ChangeZoneTable").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if p.take("ClearRemembered").is_some() && !allow_only_found {
            return None;
        }
        // Refuse `ChooseOrder$`, `Optional$`, `Condition*`, and any unknown param.
        if !p.exhausted() {
            return None;
        }

        let (zone_sel, filter_ref, only_found) = self.parse_repeat_zone_and_filter(
            zone_raw.as_deref(),
            &rc,
            allow_only_found,
            outer_imprint_found,
            chain,
        )?;

        let obj_tag = if use_imprinted {
            "Imprinted"
        } else {
            "Remembered"
        };
        let ctrl_tag = if use_imprinted {
            "ImprintedController"
        } else {
            "RememberedController"
        };
        let owner_tag = if use_imprinted {
            "ImprintedOwner"
        } else {
            "RememberedOwner"
        };

        // Special case: `DamageMap$ True` on `RepeatCards$` — strictly allowed
        // only for a single per-object `DealDamage | DamageSource$ <obj_tag>`.
        if damage_map {
            let body = self.svars.get(repeat_sub.trim())?.clone();
            let (api, mut sp) = Params::parse(&body)?;
            sp.drop_prose();
            if api != "DealDamage"
                || sp.has("ValidTgts")
                || sp.peek("DamageSource") != Some(obj_tag)
            {
                return None;
            }
            if let Some(clean) = sp.take("SubAbility")
                && self.dig_cleanup_next(&clean, true).is_none()
            {
                return None;
            }
            let dmg_eff = self
                .try_event_object_deal_damage(&mut sp, obj_tag, ctrl_tag, owner_tag, None, chain)?;
            if !sp.exhausted() {
                return None;
            }
            return Some(format!(
                "Effect::RepeatEachObject {{ zone: {zone_sel}, filter: {filter_ref}, \
                 only_found: {only_found}, then: &[{dmg_eff}] }}"
            ));
        }

        // Special case: `UseImprinted$ True` + `CopyPermanent | RememberTokens$ True`
        // paired with an outer `DelayedTrigger` exiling/sacrificing those tokens at
        // end of combat / next end step (`Mirror-Style Master` shape).
        if let Some(outer_slot) = outer_sub
            && let Some(ref del_name) = *outer_slot
            && self.sub_is_api(Some(del_name), "DelayedTrigger")
            && let Some((copy_eff, after_del)) = self.try_repeat_copy_with_delayed_cleanup(
                &repeat_sub,
                del_name,
                obj_tag,
                ctrl_tag,
                owner_tag,
            )
        {
            *outer_slot = after_del;
            return Some(format!(
                "Effect::RepeatEachObject {{ zone: {zone_sel}, filter: {filter_ref}, \
                 only_found: {only_found}, then: &[{copy_eff}] }}"
            ));
        }

        // General `RepeatSubAbility$` chain.
        let mut body_effs = Vec::new();
        let mut cursor = Some(repeat_sub.trim().to_string());
        let mut steps = 0usize;
        let mut dummy_chain = Chain::default();
        while let Some(name) = cursor.take() {
            steps += 1;
            if steps > 12 {
                return None;
            }
            if self.dig_cleanup_next(&name, true).is_some() {
                break;
            }
            let body = self.svars.get(&name)?.clone();
            let (api, mut sp) = Params::parse(&body)?;
            sp.drop_prose();
            if sp.has("ValidTgts") {
                return None;
            }
            let next = sp.take("SubAbility");
            let effs = self.compile_event_object_step(
                &api,
                &mut sp,
                use_imprinted,
                false,
                &mut dummy_chain,
            )?;
            body_effs.extend(effs);
            cursor = next;
        }
        if body_effs.is_empty() {
            return None;
        }
        if change_zone_table
            && !body_effs
                .iter()
                .all(|e| e.starts_with("Effect::CreateToken"))
        {
            return None;
        }
        if (chain.target.is_some() || chain.second_target.is_some())
            && body_effs.iter().any(|e| e.contains("Filter::Reference"))
        {
            return None;
        }
        Some(format!(
            "Effect::RepeatEachObject {{ zone: {zone_sel}, filter: {filter_ref}, \
             only_found: {only_found}, then: &[{}] }}",
            body_effs.join(", ")
        ))
    }

    #[allow(clippy::too_many_lines)]
    fn parse_repeat_zone_and_filter(
        &mut self,
        zone_raw: Option<&str>,
        rc: &str,
        allow_only_found: bool,
        outer_imprint_found: bool,
        chain: &Chain,
    ) -> Option<(String, String, bool)> {
        let mut only_found = false;
        let mut working = rc.trim().to_string();
        if allow_only_found {
            let found_attr = if outer_imprint_found {
                "IsImprinted"
            } else {
                "IsRemembered"
            };
            if let Some(stripped) = Self::strip_filter_attr(&working, found_attr) {
                working = stripped;
                only_found = true;
            }
        }
        if working.contains("IsRemembered")
            || working.contains("IsImprinted")
            || working.contains("Remembered")
            || working.contains("Imprinted")
            || working.contains("ChosenCard")
            || working.contains("NotedFor")
        {
            return None;
        }

        let targets_player = chain
            .target
            .as_deref()
            .is_some_and(|t| matches!(t, "TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent"));

        let (zone_sel, card_zone, cleaned) = match zone_raw {
            None | Some("Battlefield") => ("ZoneSel::Battlefield".to_string(), false, working),
            Some("Exile") => {
                if let Some(stripped) = Self::strip_filter_attr(&working, "ExiledWithSource") {
                    ("ZoneSel::LinkedExile".to_string(), true, stripped)
                } else {
                    ("ZoneSel::ExileAll".to_string(), true, working)
                }
            }
            Some("Graveyard") => {
                if let Some(stripped) = Self::strip_your_graveyard_owner(&working) {
                    ("ZoneSel::GraveyardYou".to_string(), true, stripped)
                } else if let Some(stripped) = Self::strip_opp_graveyard_owner(&working) {
                    ("ZoneSel::GraveyardOpponents".to_string(), true, stripped)
                } else if targets_player
                    && let Some(stripped) = Self::strip_filter_attr(&working, "TargetedPlayerOwn")
                        .or_else(|| Self::strip_filter_attr(&working, "TargetedPlayerCtrl"))
                {
                    (
                        "ZoneSel::GraveyardOf(PlayerRel::Chosen)".to_string(),
                        true,
                        stripped,
                    )
                } else if working.contains("Own") || working.contains("Ctrl") {
                    return None;
                } else {
                    ("ZoneSel::GraveyardAll".to_string(), true, working)
                }
            }
            Some("Hand") => {
                if let Some(stripped) = Self::strip_your_graveyard_owner(&working) {
                    ("ZoneSel::HandYou".to_string(), true, stripped)
                } else if let Some(stripped) = Self::strip_opp_graveyard_owner(&working) {
                    (
                        "ZoneSel::HandOf(PlayerRel::EachOpponent)".to_string(),
                        true,
                        stripped,
                    )
                } else if targets_player
                    && let Some(stripped) = Self::strip_filter_attr(&working, "TargetedPlayerOwn")
                        .or_else(|| Self::strip_filter_attr(&working, "TargetedPlayerCtrl"))
                {
                    ("ZoneSel::HandChosen".to_string(), true, stripped)
                } else if working.contains("Own") || working.contains("Ctrl") {
                    return None;
                } else {
                    (
                        "ZoneSel::HandOf(PlayerRel::EachPlayer)".to_string(),
                        true,
                        working,
                    )
                }
            }
            Some("Library") => {
                if let Some(stripped) = Self::strip_your_graveyard_owner(&working) {
                    ("ZoneSel::LibraryYou".to_string(), true, stripped)
                } else if let Some(stripped) = Self::strip_opp_graveyard_owner(&working) {
                    (
                        "ZoneSel::LibraryOf(PlayerRel::EachOpponent)".to_string(),
                        true,
                        stripped,
                    )
                } else if targets_player
                    && let Some(stripped) = Self::strip_filter_attr(&working, "TargetedPlayerOwn")
                        .or_else(|| Self::strip_filter_attr(&working, "TargetedPlayerCtrl"))
                {
                    (
                        "ZoneSel::LibraryOf(PlayerRel::Chosen)".to_string(),
                        true,
                        stripped,
                    )
                } else if working.contains("Own") || working.contains("Ctrl") {
                    return None;
                } else {
                    (
                        "ZoneSel::LibraryOf(PlayerRel::EachPlayer)".to_string(),
                        true,
                        working,
                    )
                }
            }
            Some(_) => return None,
        };

        let raw_expr = self.filter_expr_in_zone(&cleaned, card_zone)?;
        let filter_expr = Self::simplify_your_modified_filter(&raw_expr);
        let filter_ref = format!("&{}", self.body.filter_static("REPEAT", &filter_expr));
        Some((zone_sel, filter_ref, only_found))
    }

    fn try_repeat_copy_with_delayed_cleanup(
        &mut self,
        repeat_sub: &str,
        del_name: &str,
        obj_tag: &str,
        ctrl_tag: &str,
        owner_tag: &str,
    ) -> Option<(String, Option<String>)> {
        let copy_body = self.svars.get(repeat_sub.trim())?.clone();
        let (c_api, mut cp) = Params::parse(&copy_body)?;
        cp.drop_prose();
        if c_api != "CopyPermanent" || cp.take("RememberTokens").as_deref() != Some("True") {
            return None;
        }
        let del_body = self.svars.get(del_name.trim())?.clone();
        let (d_api, mut dp) = Params::parse(&del_body)?;
        dp.drop_prose();
        if d_api != "DelayedTrigger"
            || dp.take("Mode").as_deref() != Some("Phase")
            || dp.take("RememberObjects").as_deref() != Some("Remembered")
        {
            return None;
        }
        let phase = dp.take("Phase")?;
        let exec_name = dp.take("Execute")?;
        let after_del = dp.take("SubAbility");
        if !dp.exhausted() {
            return None;
        }
        let exec_body = self.svars.get(exec_name.trim())?.clone();
        let (e_api, mut ep) = Params::parse(&exec_body)?;
        ep.drop_prose();
        let is_exile = e_api == "ChangeZone"
            && ep.take("Origin").as_deref() == Some("Battlefield")
            && ep.take("Destination").as_deref() == Some("Exile")
            && matches!(
                ep.take("Defined").as_deref(),
                Some("DelayTriggerRemembered" | "DelayTriggerRememberedLKI")
            )
            && ep.exhausted();
        let is_sac = e_api == "Sacrifice"
            && matches!(
                ep.take("SacValid").as_deref(),
                Some("DelayTriggerRemembered" | "DelayTriggerRememberedLKI")
            )
            && ep.exhausted();
        let cleanup = match (phase.as_str(), is_exile, is_sac) {
            ("EndCombat", true, false) => "CopyCleanup::ExileAtEndOfCombat",
            ("EndCombat", false, true) => "CopyCleanup::SacrificeAtEndOfCombat",
            ("End of Turn", true, false) => "CopyCleanup::ExileAtNextEndStep",
            ("End of Turn", false, true) => "CopyCleanup::SacrificeAtNextEndStep",
            _ => return None,
        };
        let copy_eff =
            self.compile_event_object_copy(&mut cp, obj_tag, ctrl_tag, owner_tag, cleanup)?;
        let trailing = self
            .consume_dig_cleanup_tail(after_del, true)?
            .into_option();
        Some((copy_eff, trailing))
    }
}

/// Result of consuming a trailing `DB$ Cleanup` step.
enum CleanupTail {
    End,
    Next(String),
}

impl CleanupTail {
    fn from_option(next: Option<String>) -> Self {
        match next {
            None => Self::End,
            Some(s) => Self::Next(s),
        }
    }

    fn into_option(self) -> Option<String> {
        match self {
            Self::End => None,
            Self::Next(s) => Some(s),
        }
    }
}

/// Filter restriction decoded from `Play.ValidSA$` inside `RevealUntilThen` / `RepeatEachObject`.
enum DigPlaySaFilter {
    AnySpell,
    Filtered(String),
}

/// Parsed fields of a `DigUntil` line for `Effect::RevealUntilThen`.
#[allow(clippy::struct_excessive_bools)]
struct ParsedDigUntil {
    library: &'static str,
    multi_seat_domain: Option<&'static str>,
    valid_raw: String,
    filter_ref: String,
    count: String,
    count_is_one: bool,
    max_revealed: Option<String>,
    exile_while_revealing: bool,
    found: String,
    found_dest_raw: Option<String>,
    tapped: bool,
    rest: String,
    trailing_shuffle: bool,
    optional_whole: bool,
    optional_found_move: bool,
    optional_no_dest: Option<String>,
    no_move_revealed: bool,
    remember_found: bool,
    imprint_found: bool,
    remember_revealed: bool,
    imprint_revealed: bool,
}
