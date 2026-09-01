//! What a player has told the client to do, and how they want to say it.
//!
//! Two things live here, and they belong together because they are the two
//! halves of the same promise: the **keymap** (how an intent is expressed) and
//! the **standing orders** (which intents are answered without asking). Both
//! follow the account rather than the machine — a player who has bound their
//! keys and set their phase rail should find both waiting on a friend's
//! laptop — so this whole struct is what the gateway stores per account.
//!
//! Deliberately *not* here: anything about this particular screen. The
//! preview's size, the interface language and the gateway address are
//! properties of a device, not of a player, and they stay in the client's own
//! local settings. Storing them per account would mean a phone and a desktop
//! fighting over one number.
//!
//! No renderer type appears in this module. A [`Chord`] names a key with a
//! string that the shell maps onto whatever its windowing library calls it,
//! which is what lets the whole thing be tested without a window.

use crate::automation::{PhaseOrders, RAIL_ROWS, RailRow, RailSide};
use std::collections::BTreeMap;

/// Everything a player can ask the client to do with one keystroke.
///
/// One flat list rather than per-screen sets: a player rebinding "confirm"
/// means confirm everywhere, and a binding UI that asked them to do it once
/// per context would be answering a question nobody has.
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    /// The one-key "do the obvious thing": the card under the cursor, then
    /// the selected rail button, then confirm.
    Primary,
    /// Confirm the answer being built, or pass priority.
    Confirm,
    /// Take back the answer being built, close a preview, drop a selection.
    Cancel,
    /// Act on the card under the cursor.
    ActivateCard,
    /// Move the card cursor.
    CursorLeft,
    /// Move the card cursor.
    CursorRight,
    /// Move the card cursor.
    CursorUp,
    /// Move the card cursor.
    CursorDown,
    /// Aim the next attack (or block) at the next defender (or attacker).
    CombatFocusNext,
    /// Aim it at the previous one.
    CombatFocusPrev,
    /// Declare no attackers / no blockers and move on.
    CombatNone,
    /// Fast-forward to the next phase.
    NextPhase,
    /// Fast-forward to the start of the next turn.
    NextTurn,
    /// Slide the own-board overlay out of the way.
    ToggleOverlay,
    /// Latch the constructed card face on.
    ToggleTextView,
    /// Keep the opening hand.
    MulliganKeep,
    /// Take another one.
    MulliganTake,
    /// Answer yes.
    AnswerYes,
    /// Answer no.
    AnswerNo,
    /// Step a number choice up.
    NumberUp,
    /// Step a number choice down.
    NumberDown,
    /// Move the phase-rail selection up.
    RailUp,
    /// Move the phase-rail selection down.
    RailDown,
    /// Look at the next opponent's board.
    FocusNextSeat,
    /// Look back at your own.
    FocusHome,
}

impl Action {
    /// Every action, in the order a settings screen should list them.
    pub const ALL: [Self; 25] = [
        Self::Primary,
        Self::Confirm,
        Self::Cancel,
        Self::ActivateCard,
        Self::CursorUp,
        Self::CursorDown,
        Self::CursorLeft,
        Self::CursorRight,
        Self::FocusNextSeat,
        Self::FocusHome,
        Self::CombatFocusNext,
        Self::CombatFocusPrev,
        Self::CombatNone,
        Self::NextPhase,
        Self::NextTurn,
        Self::RailUp,
        Self::RailDown,
        Self::MulliganKeep,
        Self::MulliganTake,
        Self::AnswerYes,
        Self::AnswerNo,
        Self::NumberUp,
        Self::NumberDown,
        Self::ToggleOverlay,
        Self::ToggleTextView,
    ];

    /// How the action is named to a player.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Primary => "Do the obvious thing",
            Self::Confirm => "Confirm / pass priority",
            Self::Cancel => "Cancel",
            Self::ActivateCard => "Play or choose the card",
            Self::CursorLeft => "Cursor left",
            Self::CursorRight => "Cursor right",
            Self::CursorUp => "Cursor up",
            Self::CursorDown => "Cursor down",
            Self::CombatFocusNext => "Aim at the next defender",
            Self::CombatFocusPrev => "Aim at the previous defender",
            Self::CombatNone => "Declare nothing",
            Self::NextPhase => "Skip to the next phase",
            Self::NextTurn => "Skip to the next turn",
            Self::ToggleOverlay => "Hide the board overlay",
            Self::ToggleTextView => "Read card text instead of art",
            Self::MulliganKeep => "Keep this hand",
            Self::MulliganTake => "Mulligan",
            Self::AnswerYes => "Yes",
            Self::AnswerNo => "No",
            Self::NumberUp => "Number up",
            Self::NumberDown => "Number down",
            Self::RailUp => "Rail selection up",
            Self::RailDown => "Rail selection down",
            Self::FocusNextSeat => "Look at the next opponent",
            Self::FocusHome => "Look at your own board",
        }
    }

    /// Which group a settings screen files it under.
    #[must_use]
    pub const fn group(self) -> &'static str {
        match self {
            Self::Primary | Self::Confirm | Self::Cancel | Self::ActivateCard => "Answering",
            Self::CursorLeft
            | Self::CursorRight
            | Self::CursorUp
            | Self::CursorDown
            | Self::FocusNextSeat
            | Self::FocusHome => "Moving around",
            Self::CombatFocusNext | Self::CombatFocusPrev | Self::CombatNone => "Combat",
            Self::NextPhase | Self::NextTurn | Self::RailUp | Self::RailDown => "Phases",
            Self::MulliganKeep
            | Self::MulliganTake
            | Self::AnswerYes
            | Self::AnswerNo
            | Self::NumberUp
            | Self::NumberDown => "Questions",
            Self::ToggleOverlay | Self::ToggleTextView => "Display",
        }
    }
}

/// One key, with the modifiers that must be held with it.
///
/// `key` is a stable name — `"KeyA"`, `"Space"`, `"Digit1"`, `"ArrowUp"` — and
/// not a character: a character depends on the layout, and a player who binds
/// a key on a German keyboard should not find it somewhere else on an English
/// one. The shell owns the table from these names to its own key codes.
#[derive(
    Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "a keyboard has exactly these four modifiers and they are independent; \
              a state machine over them would be a worse model of a keyboard"
)]
pub struct Chord {
    /// The physical key's stable name.
    pub key: String,
    /// Whether shift must be held.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub shift: bool,
    /// Whether control must be held.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ctrl: bool,
    /// Whether alt/option must be held.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub alt: bool,
    /// Whether command/super must be held.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub meta: bool,
}

impl Chord {
    /// A plain key with no modifiers.
    #[must_use]
    pub fn key(name: &str) -> Self {
        Self {
            key: name.to_string(),
            shift: false,
            ctrl: false,
            alt: false,
            meta: false,
        }
    }

    /// The same key with shift held.
    #[must_use]
    pub fn shift(name: &str) -> Self {
        Self {
            shift: true,
            ..Self::key(name)
        }
    }

    /// Whether any modifier at all is required.
    #[must_use]
    pub const fn plain(&self) -> bool {
        !self.shift && !self.ctrl && !self.alt && !self.meta
    }

    /// How the chord reads on a settings screen: `⇧W`, `Space`, `Esc`.
    #[must_use]
    pub fn display(&self) -> String {
        let mut out = String::new();
        if self.ctrl {
            out.push('⌃');
        }
        if self.alt {
            out.push('⌥');
        }
        if self.shift {
            out.push('⇧');
        }
        if self.meta {
            out.push('⌘');
        }
        out.push_str(&pretty_key(&self.key));
        out
    }
}

/// A key's name as a player would recognise it.
fn pretty_key(name: &str) -> String {
    match name {
        "Space" => "Space".to_string(),
        "Escape" => "Esc".to_string(),
        "Enter" => "Enter".to_string(),
        "Tab" => "Tab".to_string(),
        "ArrowUp" => "↑".to_string(),
        "ArrowDown" => "↓".to_string(),
        "ArrowLeft" => "←".to_string(),
        "ArrowRight" => "→".to_string(),
        other => other
            .strip_prefix("Key")
            .or_else(|| other.strip_prefix("Digit"))
            .unwrap_or(other)
            .to_string(),
    }
}

/// Which keys do what.
///
/// Stored as action → chords rather than the reverse, because that is the
/// question the client asks sixty times a second ("was *confirm* pressed?")
/// and because an action may honestly have two bindings — `Enter` and the
/// numeric keypad's, say — while a chord doing two things is a bug.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Keymap {
    /// The bindings. A `BTreeMap` so the stored JSON is byte-stable and two
    /// saves of an unchanged keymap do not look like a change.
    bindings: BTreeMap<Action, Vec<Chord>>,
}

impl Default for Keymap {
    fn default() -> Self {
        Self::standard()
    }
}

impl Keymap {
    /// The bindings a player who has never opened the settings screen gets.
    ///
    /// `docs/keyboard-map.md` is the commitment these keep: every choice the
    /// game can ask is answerable without a pointer, and nothing needs a drag.
    #[must_use]
    pub fn standard() -> Self {
        let mut bindings = BTreeMap::new();
        let mut bind = |action, chords: Vec<Chord>| {
            bindings.insert(action, chords);
        };
        bind(Action::Primary, vec![Chord::key("Space")]);
        bind(Action::Confirm, vec![Chord::key("Enter")]);
        bind(Action::Cancel, vec![Chord::key("Escape")]);
        bind(Action::ActivateCard, vec![Chord::key("KeyE")]);
        bind(Action::CursorLeft, vec![Chord::key("KeyA")]);
        bind(Action::CursorRight, vec![Chord::key("KeyD")]);
        bind(Action::CursorUp, vec![Chord::key("KeyW")]);
        bind(Action::CursorDown, vec![Chord::key("KeyS")]);
        bind(Action::CombatFocusNext, vec![Chord::key("KeyC")]);
        bind(Action::CombatFocusPrev, vec![Chord::shift("KeyC")]);
        bind(Action::CombatNone, vec![Chord::key("KeyO")]);
        bind(Action::NextPhase, vec![Chord::key("Tab")]);
        bind(Action::NextTurn, vec![Chord::shift("Tab")]);
        bind(Action::ToggleOverlay, vec![Chord::key("KeyX")]);
        bind(Action::ToggleTextView, vec![Chord::key("KeyT")]);
        bind(Action::MulliganKeep, vec![Chord::key("KeyK")]);
        bind(Action::MulliganTake, vec![Chord::key("KeyB")]);
        bind(Action::AnswerYes, vec![Chord::key("KeyY")]);
        bind(Action::AnswerNo, vec![Chord::key("KeyN")]);
        bind(
            Action::NumberUp,
            vec![Chord::key("ArrowUp"), Chord::key("ArrowRight")],
        );
        bind(
            Action::NumberDown,
            vec![Chord::key("ArrowDown"), Chord::key("ArrowLeft")],
        );
        bind(Action::RailUp, vec![Chord::shift("KeyW")]);
        bind(Action::RailDown, vec![Chord::shift("KeyS")]);
        bind(Action::FocusNextSeat, vec![Chord::key("KeyF")]);
        bind(Action::FocusHome, vec![Chord::key("KeyH")]);
        Self { bindings }
    }

    /// The chords bound to an action. Empty means the player unbound it,
    /// which is allowed: a pointer can still reach everything.
    #[must_use]
    pub fn chords(&self, action: Action) -> &[Chord] {
        self.bindings.get(&action).map_or(&[], Vec::as_slice)
    }

    /// Replaces an action's bindings.
    ///
    /// Rebinding steals the chord from whatever else held it, rather than
    /// refusing: a player pressing a key expects that key, and being told
    /// "already taken" without being told by what is the worst of both.
    /// [`Keymap::holder_of`] is there so the screen can say what it took.
    pub fn bind(&mut self, action: Action, chords: Vec<Chord>) {
        for chord in &chords {
            for (other, existing) in &mut self.bindings {
                if *other != action {
                    existing.retain(|c| c != chord);
                }
            }
        }
        self.bindings.insert(action, chords);
    }

    /// Which action currently owns a chord, if any.
    #[must_use]
    pub fn holder_of(&self, chord: &Chord) -> Option<Action> {
        self.bindings
            .iter()
            .find(|(_, chords)| chords.contains(chord))
            .map(|(action, _)| *action)
    }

    /// Puts one action back to its default binding.
    pub fn reset(&mut self, action: Action) {
        let standard = Self::standard();
        self.bind(action, standard.chords(action).to_vec());
    }

    /// Any chord bound to more than one action.
    ///
    /// Should always be empty — [`Keymap::bind`] maintains that — but a
    /// keymap can also arrive over the wire from an older client, and a
    /// settings screen should be able to show the damage rather than
    /// silently picking a winner.
    #[must_use]
    pub fn conflicts(&self) -> Vec<(Chord, Vec<Action>)> {
        let mut seen: BTreeMap<&Chord, Vec<Action>> = BTreeMap::new();
        for (action, chords) in &self.bindings {
            for chord in chords {
                seen.entry(chord).or_default().push(*action);
            }
        }
        seen.into_iter()
            .filter(|(_, actions)| actions.len() > 1)
            .map(|(chord, actions)| (chord.clone(), actions))
            .collect()
    }
}

/// How much the client answers on the player's behalf.
///
/// Separate from the phase rail, which says *where* to stop; this says what
/// to do about the questions that are not really questions. Every one of them
/// defaults to off, for the reason the rail does: a client that answers
/// without being asked loses games its player never agreed to lose.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent checkboxes on a settings screen, and named fields keep \
              a stored blob readable and forward-compatible"
)]
pub struct AutoRules {
    /// Pass priority in a window that offers you nothing at all.
    ///
    /// "Nothing" is literal: no land, no spell, no ability, nothing to
    /// suspend. It is the common case of a player pressing pass forty times
    /// a turn for want of anything else to press.
    pub pass_when_nothing_to_do: bool,
    /// Pass priority through your opponents' turns.
    ///
    /// Priority only. It never declines a block: losing a creature you
    /// would have blocked with is not a decision a client may make for you.
    pub skip_opponent_turns: bool,
    /// Answer "no attackers" automatically when nothing you control can
    /// attack — the engine still asks, because it must.
    pub skip_empty_attacks: bool,
    /// Answer "no blockers" automatically when nothing you control can block.
    pub skip_empty_blocks: bool,
}

/// One switch on the automation panel.
///
/// Named separately from the fields so a settings screen can draw four rows
/// from one loop instead of four near-identical blocks — and so adding a
/// fifth rule is one arm here rather than a fifth copy of the same code.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AutoRule {
    /// [`AutoRules::pass_when_nothing_to_do`].
    PassWhenNothingToDo,
    /// [`AutoRules::skip_opponent_turns`].
    SkipOpponentTurns,
    /// [`AutoRules::skip_empty_attacks`].
    SkipEmptyAttacks,
    /// [`AutoRules::skip_empty_blocks`].
    SkipEmptyBlocks,
}

impl AutoRule {
    /// Every switch, in the order a settings screen should list them.
    pub const ALL: [Self; 4] = [
        Self::PassWhenNothingToDo,
        Self::SkipOpponentTurns,
        Self::SkipEmptyAttacks,
        Self::SkipEmptyBlocks,
    ];

    /// The switch's name.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::PassWhenNothingToDo => "Pass when there is nothing to do",
            Self::SkipOpponentTurns => "Pass through opponents' turns",
            Self::SkipEmptyAttacks => "Skip an empty attack step",
            Self::SkipEmptyBlocks => "Skip an empty block step",
        }
    }

    /// The sentence under it, saying exactly what it will and will not do.
    #[must_use]
    pub const fn detail(self) -> &'static str {
        match self {
            Self::PassWhenNothingToDo => {
                "No land, no spell, no ability, nothing to suspend: pass without asking."
            }
            Self::SkipOpponentTurns => "Priority only. It never declines a block for you.",
            Self::SkipEmptyAttacks => "Only when nothing you control can attack.",
            Self::SkipEmptyBlocks => "Only when nothing you control can block.",
        }
    }

    /// Whether the switch is on.
    #[must_use]
    pub const fn get(self, rules: &AutoRules) -> bool {
        match self {
            Self::PassWhenNothingToDo => rules.pass_when_nothing_to_do,
            Self::SkipOpponentTurns => rules.skip_opponent_turns,
            Self::SkipEmptyAttacks => rules.skip_empty_attacks,
            Self::SkipEmptyBlocks => rules.skip_empty_blocks,
        }
    }

    /// Flips it.
    pub const fn toggle(self, rules: &mut AutoRules) {
        match self {
            Self::PassWhenNothingToDo => {
                rules.pass_when_nothing_to_do = !rules.pass_when_nothing_to_do;
            }
            Self::SkipOpponentTurns => rules.skip_opponent_turns = !rules.skip_opponent_turns,
            Self::SkipEmptyAttacks => rules.skip_empty_attacks = !rules.skip_empty_attacks,
            Self::SkipEmptyBlocks => rules.skip_empty_blocks = !rules.skip_empty_blocks,
        }
    }
}

/// A player's account-level preferences, as stored by the gateway.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Preferences {
    /// Which keys do what.
    pub keymap: Keymap,
    /// Which steps to stop at, per side of the table.
    pub orders: PhaseOrders,
    /// What the client answers on the player's behalf.
    pub auto: AutoRules,
}

impl Preferences {
    /// Reads preferences from stored JSON, falling back to the defaults for
    /// anything missing or unreadable.
    ///
    /// Never an error: preferences are a convenience, and a player whose
    /// stored blob is from a client three versions old should get a working
    /// keymap rather than a screen that will not open.
    #[must_use]
    pub fn from_json(text: &str) -> Self {
        serde_json::from_str(text).unwrap_or_default()
    }

    /// The stored form. Stable for an unchanged value, so saving twice
    /// writes the same bytes.
    ///
    /// # Panics
    /// Never in practice: every field is a plain serialisable value.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Whether these are still exactly the defaults, so a client can avoid
    /// writing a row for a player who has changed nothing.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

/// The rail rows, as a settings screen lists them: both sides, in turn order.
#[must_use]
pub fn rail_rows() -> Vec<(RailSide, RailRow)> {
    RailSide::BOTH
        .into_iter()
        .flat_map(|side| RAIL_ROWS.into_iter().map(move |row| (side, row)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_standard_keymap_binds_every_action_exactly_once() {
        let map = Keymap::standard();
        for action in Action::ALL {
            assert!(
                !map.chords(action).is_empty(),
                "{action:?} has no default binding, so a keyboard player cannot reach it"
            );
        }
        assert!(
            map.conflicts().is_empty(),
            "two actions share a key by default: {:?}",
            map.conflicts()
        );
    }

    #[test]
    fn every_action_is_listed_for_the_settings_screen() {
        // `ALL` is what a rebinding screen iterates; an action missing from
        // it is an action nobody can rebind.
        assert_eq!(
            Action::ALL.len(),
            Action::ALL
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            "ALL lists an action twice"
        );
        let map = Keymap::standard();
        assert_eq!(
            Action::ALL.len(),
            map.bindings.len(),
            "the default keymap and the settings list disagree about what exists"
        );
    }

    #[test]
    fn rebinding_takes_the_key_off_whatever_had_it() {
        let mut map = Keymap::standard();
        let space = Chord::key("Space");
        assert_eq!(map.holder_of(&space), Some(Action::Primary));
        map.bind(Action::AnswerYes, vec![space.clone()]);
        assert_eq!(map.holder_of(&space), Some(Action::AnswerYes));
        assert!(
            map.chords(Action::Primary).is_empty(),
            "the old owner must lose it, or the same key does two things"
        );
        assert!(map.conflicts().is_empty());
    }

    #[test]
    fn an_action_can_be_reset_on_its_own() {
        let mut map = Keymap::standard();
        map.bind(Action::Confirm, vec![Chord::key("KeyZ")]);
        map.reset(Action::Confirm);
        assert_eq!(
            map.chords(Action::Confirm),
            Keymap::standard().chords(Action::Confirm)
        );
    }

    #[test]
    fn unbinding_is_allowed_because_a_pointer_can_still_reach_everything() {
        let mut map = Keymap::standard();
        map.bind(Action::ToggleTextView, vec![]);
        assert!(map.chords(Action::ToggleTextView).is_empty());
        assert!(map.conflicts().is_empty());
    }

    #[test]
    fn a_chord_reads_the_way_a_player_would_write_it() {
        assert_eq!(Chord::key("KeyW").display(), "W");
        assert_eq!(Chord::shift("KeyW").display(), "⇧W");
        assert_eq!(Chord::key("Escape").display(), "Esc");
        assert_eq!(Chord::key("ArrowUp").display(), "↑");
        assert_eq!(Chord::key("Digit3").display(), "3");
        assert!(Chord::key("Space").plain());
        assert!(!Chord::shift("Space").plain());
    }

    #[test]
    fn preferences_survive_a_round_trip_through_the_gateway() {
        let mut prefs = Preferences::default();
        prefs
            .keymap
            .bind(Action::Confirm, vec![Chord::shift("KeyP")]);
        prefs.orders.toggle(RailSide::Theirs, RailRow::Upkeep);
        prefs.auto.pass_when_nothing_to_do = true;
        let stored = prefs.to_json();
        assert_eq!(Preferences::from_json(&stored), prefs);
        // And the same value serialises the same way twice, so a client can
        // tell "unchanged" from "changed" by comparing the bytes.
        assert_eq!(stored, Preferences::from_json(&stored).to_json());
    }

    #[test]
    fn nonsense_from_the_store_gives_a_working_client_rather_than_no_client() {
        let prefs = Preferences::from_json("{\"keymap\": 7, not json at all");
        assert!(prefs.is_default());
        assert_eq!(prefs.keymap.chords(Action::Confirm), &[Chord::key("Enter")]);
    }

    #[test]
    fn a_blob_from_an_older_client_keeps_what_it_knew_and_defaults_the_rest() {
        // Forward compatibility is the whole reason every field is
        // `#[serde(default)]`: a player mid-upgrade must not lose their keys.
        let prefs = Preferences::from_json("{\"auto\":{\"pass_when_nothing_to_do\":true}}");
        assert!(prefs.auto.pass_when_nothing_to_do);
        assert_eq!(prefs.keymap, Keymap::standard(), "the keymap fell back");
    }

    #[test]
    fn everything_the_client_can_answer_for_you_starts_switched_off() {
        let auto = AutoRules::default();
        assert!(!auto.pass_when_nothing_to_do);
        assert!(!auto.skip_opponent_turns);
        assert!(!auto.skip_empty_attacks);
        assert!(!auto.skip_empty_blocks);
        assert!(Preferences::default().is_default());
    }

    #[test]
    fn every_automation_switch_reads_and_writes_its_own_field() {
        // Four near-identical booleans is exactly the shape where a copied
        // arm reads one field and writes another, and nothing ever notices.
        for rule in AutoRule::ALL {
            let mut rules = AutoRules::default();
            assert!(!rule.get(&rules), "{rule:?} does not start off");
            rule.toggle(&mut rules);
            assert!(rule.get(&rules), "{rule:?} did not turn on");
            for other in AutoRule::ALL {
                if other != rule {
                    assert!(!other.get(&rules), "{rule:?} also flipped {other:?}");
                }
            }
            rule.toggle(&mut rules);
            assert_eq!(rules, AutoRules::default(), "{rule:?} did not flip back");
        }
    }

    #[test]
    fn the_settings_screen_can_list_both_rails_in_turn_order() {
        let rows = rail_rows();
        assert_eq!(rows.len(), RAIL_ROWS.len() * 2);
        // Opponents on top, you at the bottom — the order the rail is drawn in.
        assert_eq!(rows[0], (RailSide::Theirs, RailRow::Untap));
        assert_eq!(rows[RAIL_ROWS.len()], (RailSide::Mine, RailRow::Untap));
    }
}
