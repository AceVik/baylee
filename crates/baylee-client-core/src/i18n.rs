//! The interface's own words, in every language the client speaks.
//!
//! Card text is translated by the gateway out of the catalog, field by field,
//! and has been for as long as `/pool?lang=` existed. The *interface* was
//! English and only English — every button, caption and status line a literal
//! at the point it was drawn. This is the other half.
//!
//! # Why an enum and a macro rather than a file of strings
//!
//! A translation table read at runtime — RON, JSON, Fluent — answers a missing
//! key with a fallback, and a fallback is a screen that is half German. Here a
//! [`Phrase`] is a variant and [`messages!`] writes one arm per language for
//! each: **a phrase with no German is a compilation error**, not a line of
//! English in the middle of a German sentence. The cost is that adding a third
//! language is a sweep through this file rather than a new file beside it —
//! which is the right way round, because that sweep is the work, and a build
//! that lets you ship half of it is what makes it never get finished.
//!
//! Nothing here touches a renderer, so the whole of it is testable without a
//! window, and the two rules worth having are tests: every phrase answers in
//! every language, and a phrase's placeholders are the same set in all of
//! them — `{0}` moving is what translation *is*, `{0}` vanishing is a bug.
//!
//! # What is not here
//!
//! The gateway's own refusals (`{"error":"…"}`) are shown in the words the
//! gateway sent, because it is the gateway that knows why it said no. Making
//! those translatable is a protocol change — a code beside the prose — and is
//! deliberately a separate piece of work.

/// A language the interface speaks.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum Lang {
    /// English.
    #[default]
    En,
    /// German.
    De,
}

impl Lang {
    /// Every language, in the order a picker offers them.
    pub const ALL: [Self; 2] = [Self::En, Self::De];

    /// The language a stored code names.
    ///
    /// Anything unrecognised is English rather than an error: the code comes
    /// from a settings file, a query string or an account, and a client that
    /// refused to start over one would be worse than one that speaks English.
    /// A regional code (`de-DE`, `en_GB`) is read by its first part, because
    /// the catalog's languages are plain two-letter codes and a player who
    /// wrote one out in full meant the language.
    #[must_use]
    pub fn of(code: &str) -> Self {
        let base = code
            .split(['-', '_'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        match base.as_str() {
            "de" => Self::De,
            _ => Self::En,
        }
    }

    /// The code this language is stored and requested under.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::De => "de",
        }
    }

    /// What the language calls itself. Never "German" in an English list: a
    /// player looking for their own language is looking for their own word
    /// for it.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::De => "Deutsch",
        }
    }

    /// The next language round the ring — one button rather than a menu,
    /// which is what two languages deserve.
    #[must_use]
    pub fn next(self) -> Self {
        let at = Self::ALL.iter().position(|l| *l == self).unwrap_or(0);
        Self::ALL[(at + 1) % Self::ALL.len()]
    }
}

/// Defines [`Phrase`] with one arm per language, so a missing translation is
/// a compilation error rather than a fallback.
macro_rules! messages {
    ($($(#[$doc:meta])* $key:ident { en: $en:literal, de: $de:literal $(,)? }),* $(,)?) => {
        /// One thing the interface says.
        #[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
        pub enum Phrase {
            $($(#[$doc])* $key,)*
        }

        impl Phrase {
            /// Every phrase, for the tests that hold this file honest.
            pub const ALL: &'static [Self] = &[$(Self::$key,)*];

            /// This phrase in one language.
            #[must_use]
            pub fn text(self, lang: Lang) -> &'static str {
                match (self, lang) {
                    $(
                        (Self::$key, Lang::En) => $en,
                        (Self::$key, Lang::De) => $de,
                    )*
                }
            }
        }
    };
}

messages! {
    // ---- the sign-in screen
    /// The product's name. Not translated, and here so that the one place it
    /// is written stays one place.
    AppName { en: "baylee", de: "baylee" },
    /// Caption over the address field.
    Email { en: "E-MAIL", de: "E-MAIL" },
    /// Caption over the name field, when registering.
    DisplayName { en: "DISPLAY NAME", de: "ANZEIGENAME" },
    /// Caption over the password field.
    Password { en: "PASSWORD", de: "PASSWORT" },
    /// The button that signs in.
    SignIn { en: "Sign in", de: "Anmelden" },
    /// The button that registers.
    CreateAccount { en: "Create account", de: "Konto erstellen" },
    /// Swaps the form to registering.
    WantAnAccount { en: "Create an account", de: "Konto erstellen" },
    /// Swaps the form back to signing in.
    HaveAnAccount { en: "I already have an account", de: "Ich habe schon ein Konto" },
    /// Plays the house AI with no account at all.
    PlayOffline { en: "Play the house AI offline", de: "Offline gegen die Haus-KI" },
    /// Opens the settings screen.
    Settings { en: "Settings", de: "Einstellungen" },
    /// The button that changes the interface language.
    Language { en: "Language", de: "Sprache" },

    // ---- the table screen
    /// Leaves the account.
    SignOut { en: "Sign out", de: "Abmelden" },
    /// Heading over the account's decks.
    YourDecks { en: "Your decks", de: "Deine Decks" },
    /// Opens the builder on a new deck.
    NewDeck { en: "New deck", de: "Neues Deck" },
    /// Saves the acceptance file's starter deck.
    AddStarterDeck { en: "Add the starter deck", de: "Starterdeck hinzufügen" },
    /// Shown in place of an empty deck list.
    NoDecksYet {
        en: "no decks yet — add the starter deck",
        de: "noch keine Decks — füge das Starterdeck hinzu",
    },
    /// Opens a saved deck in the builder.
    Edit { en: "Edit", de: "Bearbeiten" },
    /// Throws a saved deck away.
    Delete { en: "Delete", de: "Löschen" },
    /// Heading over the tables.
    Tables { en: "Tables", de: "Tische" },
    /// Caption over the table search box.
    Search { en: "SEARCH", de: "SUCHE" },
    /// Runs the search.
    DoSearch { en: "Search", de: "Suchen" },
    /// Re-reads decks and tables.
    Refresh { en: "Refresh", de: "Neu laden" },
    /// The one-tap game against the house AI.
    PlayTheHouse { en: "Play the house", de: "Gegen das Haus" },
    /// Caption over the room password box.
    RoomPassword { en: "ROOM PASSWORD", de: "RAUM-PASSWORT" },
    /// Before the row of table sizes.
    OpenATableFor { en: "Open a table for", de: "Tisch eröffnen für" },
    /// Shown in place of an empty table list.
    NoTablesOpen {
        en: "no tables are open — start one",
        de: "keine Tische offen — eröffne einen",
    },
    /// Shown when a search matched nothing. `{0}` is what was searched for.
    NoTableMatches {
        en: "no table matches “{0}”",
        de: "kein Tisch passt zu „{0}“",
    },
    /// Sits down at a table.
    Join { en: "Join", de: "Mitspielen" },
    /// Says this player is ready.
    Ready { en: "Ready", de: "Bereit" },
    /// Takes that back.
    NotReady { en: "Not ready", de: "Nicht bereit" },
    /// The host's go.
    Start { en: "Start", de: "Starten" },
    /// Gives up a chair.
    Leave { en: "Leave", de: "Verlassen" },
    /// Puts the selected deck in a chair.
    UseMyDeck { en: "use my deck", de: "mein Deck" },
    /// Hands the room to the player in a chair.
    MakeHost { en: "make host", de: "zum Gastgeber" },
    /// Takes a named chair.
    SitHere { en: "sit here", de: "hier sitzen" },
    /// Turns a chair over to the AI.
    SeatToAi { en: "→ AI", de: "→ KI" },
    /// Turns it back into a chair for a person.
    SeatToOpen { en: "→ open", de: "→ frei" },
    /// The gentlest house AI.
    AiNovice { en: "novice", de: "Anfänger" },
    /// The middle one.
    AiSteady { en: "steady", de: "Solide" },
    /// The one that plays to win.
    AiSharp { en: "sharp", de: "Scharf" },
    /// One page back through the table list.
    PageBack { en: "‹ Back", de: "‹ Zurück" },
    /// One page on.
    PageMore { en: "More ›", de: "Mehr ›" },
    /// Which rows are shown. `{0}`–`{1}` of `{2}`.
    PageOf { en: "{0}–{1} of {2}", de: "{0}–{1} von {2}" },
    /// A chair nobody is in. `{0}` is the seat number.
    SeatOpen { en: "seat {0} · open", de: "Platz {0} · frei" },
    /// A chair the AI plays. `{0}` is the seat, `{1}` the difficulty.
    SeatAi { en: "seat {0} · AI ({1})", de: "Platz {0} · KI ({1})" },
    /// Somebody else's chair. `{0}` is the seat, `{1}` their name.
    SeatTaken { en: "seat {0} · {1}", de: "Platz {0} · {1}" },
    /// This player's own chair. `{0}` is the seat, `{1}` their name.
    SeatYours { en: "seat {0} · {1} (you)", de: "Platz {0} · {1} (du)" },
    /// How full a table is. `{0}` seated of `{1}` chairs.
    Seated { en: "{0}/{1} seated", de: "{0}/{1} besetzt" },
    /// How big a table that is already playing is. `{0}` chairs.
    SeatCount { en: "{0} seats", de: "{0} Plätze" },
    /// What a waiting room is waiting for. `{0}` is how many are not ready.
    WaitingFor { en: "waiting for {0}", de: "wartet auf {0}" },
    /// A room whose chairs are all ready.
    AllReady { en: "all ready", de: "alle bereit" },
    /// A room that is locked.
    Locked { en: "locked", de: "abgeschlossen" },
    /// A table's state, as the listing gives it.
    StateWaiting { en: "waiting", de: "wartet" },
    /// The same, for one being played.
    StatePlaying { en: "playing", de: "läuft" },
    /// The same, for one that is over.
    StateOver { en: "over", de: "beendet" },
    /// The banner over a table of ours nobody has joined yet. `{0}` is its id.
    TableOpenWaiting {
        en: "your table {0} is open — waiting for an opponent",
        de: "dein Tisch {0} ist offen — warte auf Gegner",
    },
    /// The veil over the moment between a granted seat and the duel.
    TakingYourSeat { en: "taking your seat…", de: "nehme deinen Platz ein…" },
    /// Back to the lobby from a finished game.
    BackToLobby { en: "Back to the lobby", de: "Zurück zur Lobby" },

    // ---- the status line
    /// Signed in, nothing happening.
    SignedIn { en: "signed in", de: "angemeldet" },
    /// Signed out.
    SignedOut { en: "signed out", de: "abgemeldet" },
    /// The form was submitted with something missing.
    NeedEmailAndPassword {
        en: "an e-mail and a password, please",
        de: "bitte E-Mail und Passwort",
    },
    /// The same, registering.
    NeedDisplayName { en: "a display name, please", de: "bitte einen Anzeigenamen" },
    /// This gateway does not take sign-ups.
    NoSignUps {
        en: "this gateway is not taking new accounts",
        de: "dieses Gateway nimmt keine neuen Konten an",
    },
    /// Registering.
    CreatingAccount { en: "creating the account…", de: "erstelle das Konto…" },
    /// Signing in.
    SigningIn { en: "signing in…", de: "melde an…" },
    /// Registered, and signing in with the same credentials.
    AccountCreated {
        en: "account created — signing in…",
        de: "Konto erstellt — melde an…",
    },
    /// Fetching the card pool.
    LoadingPool { en: "loading the card pool…", de: "lade den Kartenpool…" },
    /// Saving a deck.
    SavingDeck { en: "saving the deck…", de: "speichere das Deck…" },
    /// Saved.
    DeckSaved { en: "deck saved", de: "Deck gespeichert" },
    /// Opening a deck in the builder.
    OpeningDeck { en: "opening the deck…", de: "öffne das Deck…" },
    /// Deleting a deck.
    DeletingDeck { en: "deleting the deck…", de: "lösche das Deck…" },
    /// Deleted.
    DeckDeleted { en: "deck deleted", de: "Deck gelöscht" },
    /// Opening a table.
    OpeningTable { en: "opening a table…", de: "eröffne einen Tisch…" },
    /// Sitting down at one.
    SittingDown { en: "sitting down…", de: "setze mich…" },
    /// Saying ready.
    SayingReady { en: "ready…", de: "bereit…" },
    /// Taking it back.
    SayingNotReady { en: "not ready…", de: "nicht bereit…" },
    /// The host's start.
    Starting { en: "starting…", de: "starte…" },
    /// Handing the room on.
    HandingOver { en: "handing the room over…", de: "übergebe den Raum…" },
    /// Arranging a chair.
    ArrangingTable { en: "arranging the table…", de: "richte den Tisch her…" },
    /// Standing up.
    LeavingTable { en: "leaving the table…", de: "verlasse den Tisch…" },
    /// A deck is needed first.
    PickADeckFirst { en: "pick a deck first", de: "wähle zuerst ein Deck" },
    /// Our own table is open and waiting.
    TableOpen {
        en: "table open — waiting for an opponent",
        de: "Tisch offen — warte auf Gegner",
    },
    /// Somebody joined it.
    OpponentSatDown { en: "an opponent sat down", de: "ein Gegner hat sich gesetzt" },
    /// The seat was granted and the duel is opening.
    TakingTheSeat { en: "taking the seat…", de: "nehme den Platz ein…" },
    /// The game we were in ended and the lobby is back.
    GameEnded { en: "the game ended", de: "das Spiel ist vorbei" },
    /// A body arrived that made no sense. `{0}` names what was being read.
    Unreadable {
        en: "could not read {0} the gateway sent",
        de: "konnte {0} vom Gateway nicht lesen",
    },
    /// What `{0}` is, in that message: the table list.
    TheGameList { en: "the game list", de: "die Tischliste" },
    /// …the deck list.
    TheDeckList { en: "the deck list", de: "die Deckliste" },
    /// …the sign-in.
    TheSignIn { en: "the sign-in", de: "die Anmeldung" },
    /// …a seat.
    TheSeat { en: "the seat", de: "den Platz" },
    /// …the card pool.
    ThePool { en: "the card pool", de: "den Kartenpool" },
    /// …a card's printings.
    ThePrintings { en: "the printings", de: "die Drucke" },
    /// …one deck.
    TheDeck { en: "the deck", de: "das Deck" },
    /// The gateway did not answer at all. `{0}` is the transport's word.
    GatewayNoAnswer {
        en: "the gateway did not answer: {0}",
        de: "das Gateway antwortete nicht: {0}",
    },
    /// It answered, but with a bare status. `{0}` is that status.
    GatewayAnswered {
        en: "the gateway answered {0}",
        de: "das Gateway antwortete mit {0}",
    },
    /// The offline duel could not be opened.
    NoOfflineDuel {
        en: "could not start the offline duel",
        de: "Offline-Partie konnte nicht starten",
    },
    /// Leaving the builder would drop an edit.
    UnsavedChanges {
        en: "unsaved changes — press again to leave",
        de: "ungespeicherte Änderungen — nochmal drücken zum Verlassen",
    },
    /// The deck already holds as many of that card as it may.
    NoRoomForCopy {
        en: "no room for another copy of that",
        de: "kein Platz für noch eine Kopie davon",
    },
    /// The table could not be reached. `{0}` is what went wrong.
    CouldNotReachTable {
        en: "could not reach the table: {0}",
        de: "Tisch nicht erreichbar: {0}",
    },
}

impl Phrase {
    /// This phrase with its placeholders filled in, left to right.
    ///
    /// `{0}` is replaced by the first argument, `{1}` by the second, and a
    /// placeholder with no argument is left standing rather than swallowed —
    /// a visible `{2}` is a bug report; a silently missing number is a
    /// sentence that means something else.
    #[must_use]
    pub fn fill(self, lang: Lang, args: &[&str]) -> String {
        let mut text = self.text(lang).to_string();
        for (index, arg) in args.iter().enumerate() {
            text = text.replace(&format!("{{{index}}}"), arg);
        }
        text
    }

    /// The placeholders this phrase carries, as their indices.
    ///
    /// Used by the test that keeps the languages in step: word order is the
    /// translator's business and `{0}` may move anywhere, but a `{0}` that is
    /// not there at all is a name, a count or a reason that never reaches the
    /// player.
    #[must_use]
    pub fn slots(self, lang: Lang) -> Vec<usize> {
        let text = self.text(lang);
        let mut found: Vec<usize> = (0..10)
            .filter(|i| text.contains(&format!("{{{i}}}")))
            .collect();
        found.sort_unstable();
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point of the macro: there is no such thing as a phrase with
    /// no German. This test cannot fail — it would not compile — and is here
    /// so that the guarantee is written down where a reader looks for it.
    #[test]
    fn every_phrase_answers_in_every_language() {
        for phrase in Phrase::ALL {
            for lang in Lang::ALL {
                assert!(
                    !phrase.text(lang).is_empty(),
                    "{phrase:?} says nothing in {lang:?}"
                );
            }
        }
    }

    /// Word order is the translator's; the values are not. A `{0}` that is
    /// dropped in one language is a name or a count the player never sees.
    #[test]
    fn a_translation_keeps_every_placeholder_it_was_given() {
        for phrase in Phrase::ALL {
            let english = phrase.slots(Lang::En);
            for lang in Lang::ALL {
                assert_eq!(
                    phrase.slots(lang),
                    english,
                    "{phrase:?} loses or invents a placeholder in {lang:?}"
                );
            }
        }
    }

    /// Two languages that say exactly the same thing everywhere would mean
    /// the second one was never written. A handful of phrases genuinely are
    /// the same word (`baylee`, `E-MAIL`), so this asks for most, not all.
    #[test]
    fn german_is_actually_german() {
        let same = Phrase::ALL
            .iter()
            .filter(|p| p.text(Lang::En) == p.text(Lang::De))
            .count();
        assert!(
            same * 5 < Phrase::ALL.len(),
            "{same} of {} phrases are untranslated",
            Phrase::ALL.len()
        );
    }

    #[test]
    fn a_stored_code_names_a_language() {
        assert_eq!(Lang::of("de"), Lang::De);
        assert_eq!(Lang::of("DE"), Lang::De);
        // A regional code is the language it is a region of…
        assert_eq!(Lang::of("de-AT"), Lang::De);
        assert_eq!(Lang::of("en_GB"), Lang::En);
        // …and anything else is English rather than a refusal to start.
        assert_eq!(Lang::of("kl"), Lang::En);
        assert_eq!(Lang::of(""), Lang::En);
        assert_eq!(Lang::of("de").code(), "de");
    }

    #[test]
    fn the_picker_walks_every_language_and_comes_back() {
        let mut lang = Lang::default();
        for _ in Lang::ALL {
            lang = lang.next();
        }
        assert_eq!(lang, Lang::default(), "the ring is not a ring");
    }

    #[test]
    fn filling_a_phrase_puts_the_arguments_where_the_language_wants_them() {
        assert_eq!(
            Phrase::PageOf.fill(Lang::En, &["1", "8", "12"]),
            "1–8 of 12"
        );
        assert_eq!(
            Phrase::PageOf.fill(Lang::De, &["1", "8", "12"]),
            "1–8 von 12"
        );
        // An argument that was not supplied leaves its placeholder showing.
        assert!(Phrase::PageOf.fill(Lang::En, &["1"]).contains("{1}"));
    }
}
