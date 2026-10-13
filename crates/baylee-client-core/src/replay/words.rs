//! What a replay says, in every language (§7.6): its own list, so the
//! replay's words don't grow [`crate::i18n::Phrase`]'s, which every lane
//! edits. The keymap's rows stay `Phrase`s (`prefs::Action::label` is one).

use crate::i18n::{Lang, messages};

messages! {
    /// One thing a replay says.
    ReplayPhrase;
    /// The hub's tab, and the screen's heading.
    Replays { en: "Replays", de: "Wiederholungen" },
    /// The end sheet's way into the game's replay.
    WatchReplay { en: "Watch replay", de: "Wiederholung ansehen" },
    /// A row's button.
    Watch { en: "Watch", de: "Ansehen" },
    /// The end sheet of a replay: from the start again.
    WatchAgain { en: "Watch again", de: "Noch einmal ansehen" },
    /// The end sheet of a replay, and a row's menu: another perspective.
    WatchFrom { en: "Watch from\u{2026}", de: "Ansehen aus\u{2026}" },
    /// Back to the list.
    BackToReplays { en: "Back to replays", de: "Zur\u{fc}ck zu den Wiederholungen" },
    /// The game menu's way out of a replay (Concede's place).
    LeaveReplay { en: "Leave replay", de: "Wiederholung verlassen" },
    /// A perspective: the viewer's seat.
    YourSeat { en: "Your seat", de: "Dein Platz" },
    /// A perspective: another seat (a record opened by hand). `{0}` its name.
    SeatOf { en: "{0}\u{2019}s seat", de: "Platz von {0}" },
    /// A perspective: the spectators' table.
    TheTable { en: "The table, as a spectator", de: "Der Tisch, als Zuschauer" },
    /// The perspective pill, short.
    TheTableShort { en: "The table", de: "Der Tisch" },
    /// A perspective: every hand.
    AllHands { en: "All hands", de: "Alle H\u{e4}nde" },
    /// The top line. `{0}` where, `{1}` when, `{2}` the perspective.
    TopLine { en: "Replay \u{b7} {0} \u{b7} {1} \u{b7} {2}", de: "Wiederholung \u{b7} {0} \u{b7} {1} \u{b7} {2}" },
    /// Where the step stands. `{0}` the turn, `{1}` whose, `{2}` the step.
    WhereStep { en: "Turn {0} \u{b7} {1} \u{b7} {2}", de: "Zug {0} \u{b7} {1} \u{b7} {2}" },
    /// The veil, a local projection. `{0}` percent.
    RebuildingHere { en: "Rebuilding the game\u{2026} {0} %", de: "Das Spiel wird nachgebaut\u{2026} {0} %" },
    /// The veil, a hosted projection. `{0}` the gateway.
    RebuildingThere { en: "Rebuilding the game on {0}\u{2026}", de: "Das Spiel wird auf {0} nachgebaut\u{2026}" },
    /// The veil's way out.
    EscCancels { en: "Esc cancels", de: "Esc bricht ab" },
    /// Stopped early (`Verify::Stop`). `{0}` the turn, `{1}` the recording build.
    StoppedDiverged { en: "This version plays this game the same up to turn {0}; it was recorded by {1} and from there it would differ.", de: "Diese Version spielt dieses Spiel bis Zug {0} genauso; aufgezeichnet hat es {1}, und ab dort w\u{fc}rde es anders verlaufen." },
    /// Stopped: an answer refused. `{0}` the turn.
    StoppedRefused { en: "This version can\u{2019}t play this game past turn {0}.", de: "Diese Version kann dieses Spiel nicht \u{fc}ber Zug {0} hinaus spielen." },
    /// The same build, a different hash.
    StoppedMismatch { en: "This replay doesn\u{2019}t match the game it records. Please report it (F8).", de: "Diese Wiederholung passt nicht zu dem Spiel, das sie aufzeichnet. Bitte melde es (F8)." },
    /// The record ends before the game does.
    StoppedCut { en: "The record ends here: the game went on, but no more of it was kept.", de: "Die Aufzeichnung endet hier: Das Spiel ging weiter, aber mehr davon wurde nicht aufbewahrt." },
    /// The way past a divergence.
    PlayOnAnyway { en: "Play on anyway", de: "Trotzdem weiterspielen" },
    /// A list mark: checked on another build.
    Checked { en: "Checked", de: "Gepr\u{fc}ft" },
    /// Its hint. `{0}` the recording build.
    CheckedHint { en: "Recorded by {0}. This version plays it again and checks it against what the table showed.", de: "Aufgezeichnet von {0}. Diese Version spielt es erneut und pr\u{fc}ft es gegen das, was der Tisch zeigte." },
    /// A list mark: recorded before the checks.
    Unverified { en: "Unverified", de: "Ungepr\u{fc}ft" },
    /// Its hint.
    UnverifiedHint { en: "Recorded before replays were checked. This version may play it differently.", de: "Aufgezeichnet, bevor Wiederholungen gepr\u{fc}ft wurden. Diese Version spielt es vielleicht anders." },
    /// The timeline's hatched band.
    UnverifiedBand { en: "unverified", de: "ungepr\u{fc}ft" },
    /// A seat whose account is gone.
    FormerPlayer { en: "Former player", de: "Ehemalige Person" },
    /// A seat bridge.
    LanguageModel { en: "Language model", de: "Sprachmodell" },
    /// The house. `{0}` its level.
    HouseLevel { en: "House ({0})", de: "Haus ({0})" },
    /// The house, level unknown.
    House { en: "House", de: "Haus" },
    /// The viewer, in a row.
    You { en: "You", de: "Du" },
    /// The next page of online games.
    OlderOnline { en: "Older online games", de: "\u{c4}ltere Online-Spiele" },
    /// The source filter.
    FilterAll { en: "All", de: "Alle" },
    /// The source filter.
    FilterHouse { en: "Against the house", de: "Gegen das Haus" },
    /// The source filter.
    FilterOnline { en: "Online", de: "Online" },
    /// The result filter.
    AnyResult { en: "Any result", de: "Jedes Ergebnis" },
    /// A result.
    Won { en: "Won", de: "Gewonnen" },
    /// A result.
    Lost { en: "Lost", de: "Verloren" },
    /// A result.
    Draw { en: "Draw", de: "Unentschieden" },
    /// A result.
    Unfinished { en: "Unfinished", de: "Unbeendet" },
    /// A row's place: this computer.
    ThisComputer { en: "This computer", de: "Dieser Computer" },
    /// A row: against the house. `{0}` the level.
    VsHouse { en: "vs House ({0})", de: "gegen das Haus ({0})" },
    /// A row's turns. `{0}` the turn.
    TurnShort { en: "T{0}", de: "Z{0}" },
    /// A row's length. `{0}` minutes.
    Minutes { en: "{0} min", de: "{0} Min." },
    /// A day heading.
    Today { en: "Today", de: "Heute" },
    /// A day heading.
    Yesterday { en: "Yesterday", de: "Gestern" },
    /// The search's hint.
    SearchHint { en: "Players, commanders\u{2026}", de: "Spieler, Kommandeure\u{2026}" },
    /// A row's menu: delete a local record.
    Delete { en: "Delete\u{2026}", de: "L\u{f6}schen\u{2026}" },
    /// Asked before a local record is deleted.
    DeleteAsk { en: "Delete this game? It is kept nowhere else, and a report can no longer carry it.", de: "Dieses Spiel l\u{f6}schen? Es wird nirgends sonst aufbewahrt, und ein Bericht kann es dann nicht mehr mitnehmen." },
    /// The answer that deletes.
    DeleteYes { en: "Delete", de: "L\u{f6}schen" },
    /// The empty list.
    Empty { en: "No replays yet. Games against the house are kept here (the last 20), and the games you play online are on the server you played them on.", de: "Noch keine Wiederholungen. Spiele gegen das Haus werden hier aufbewahrt (die letzten 20), und deine Online-Spiele liegen auf dem Server, auf dem du sie gespielt hast." },
    /// The list's foot line.
    FootLine { en: "A replay shows what you were shown at the table, and nothing you weren\u{2019}t.", de: "Eine Wiederholung zeigt, was dir am Tisch gezeigt wurde, und nichts, was du nicht gesehen hast." },
    /// No match for the filters.
    NoMatch { en: "No replay matches these filters.", de: "Keine Wiederholung passt zu diesen Filtern." },
    /// The online list could not be read.
    OnlineUnread { en: "The online games could not be listed.", de: "Die Online-Spiele konnten nicht aufgelistet werden." },
    /// The transport bar: the start.
    Start { en: "Start", de: "Anfang" },
    /// The transport bar: the previous turn.
    PrevTurn { en: "Previous turn", de: "Vorheriger Zug" },
    /// The transport bar: one moment back.
    StepBack { en: "One moment back", de: "Einen Moment zur\u{fc}ck" },
    /// The transport bar: play.
    Play { en: "Play", de: "Abspielen" },
    /// The transport bar: pause.
    Pause { en: "Pause", de: "Pause" },
    /// The transport bar: one moment on.
    StepOn { en: "One moment on", de: "Einen Moment weiter" },
    /// The transport bar: the next turn.
    NextTurn { en: "Next turn", de: "N\u{e4}chster Zug" },
    /// The transport bar: the end.
    End { en: "End", de: "Ende" },
    /// The transport bar: the speed.
    Speed { en: "Speed", de: "Tempo" },
    /// The timeline's hint over a turn. `{0}` the turn, `{1}` whose.
    TurnHint { en: "Turn {0} \u{b7} {1}", de: "Zug {0} \u{b7} {1}" },
    /// The replay could not be made. `{0}` why.
    Failed { en: "This replay can\u{2019}t be shown: {0}", de: "Diese Wiederholung kann nicht gezeigt werden: {0}" },
}

impl ReplayPhrase {
    /// This phrase with its placeholders filled in, left to right; a
    /// placeholder with no value is left standing.
    #[must_use]
    pub fn fill(self, lang: Lang, args: &[&str]) -> String {
        let mut text = self.text(lang).to_string();
        for (index, arg) in args.iter().enumerate() {
            text = text.replace(&format!("{{{index}}}"), arg);
        }
        text
    }

    /// The placeholders this phrase carries, as their indices.
    #[must_use]
    pub fn slots(self, lang: Lang) -> Vec<usize> {
        let text = self.text(lang);
        (0..10)
            .filter(|i| text.contains(&format!("{{{i}}}")))
            .collect()
    }

    /// A result's word.
    #[must_use]
    pub const fn outcome(outcome: super::library::Outcome) -> Self {
        use super::library::Outcome;
        match outcome {
            Outcome::Won => Self::Won,
            Outcome::Lost => Self::Lost,
            Outcome::Draw => Self::Draw,
            Outcome::Unfinished => Self::Unfinished,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ReplayPhrase;
    use crate::i18n::Lang;

    /// A translation keeps every placeholder: a dropped `{0}` is a turn or
    /// a build the player never sees; and no phrase is empty.
    #[test]
    fn a_translation_keeps_every_placeholder_it_was_given() {
        for phrase in ReplayPhrase::ALL {
            let english = phrase.slots(Lang::En);
            for lang in Lang::ALL {
                assert_eq!(phrase.slots(lang), english, "{phrase:?} in {lang:?}");
                assert!(!phrase.text(lang).is_empty(), "{phrase:?} in {lang:?}");
            }
        }
    }
}
