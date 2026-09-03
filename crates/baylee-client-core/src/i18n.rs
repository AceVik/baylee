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

    // ---- the settings screen
    /// Settings
    SettingsTitle { en: "Settings", de: "Einstellungen" },
    /// Back
    Back { en: "Back", de: "Zurück" },
    /// Saved to your account — these travel with you to any table.
    SettingsOnAccount {
        en: "Saved to your account — these travel with you to any table.",
        de: "In deinem Konto gespeichert — sie begleiten dich an jeden Tisch.",
    },
    /// Saved on this computer. Sign in and they follow your account.
    SettingsOnDevice {
        en: "Saved on this computer. Sign in and they follow your account.",
        de: "Auf diesem Rechner gespeichert. Melde dich an, und sie folgen deinem Konto.",
    },
    /// Keys
    Keys { en: "Keys", de: "Tasten" },
    /// Automation
    Automation { en: "Automation", de: "Automatik" },
    /// Where to stop
    WhereToStop { en: "Where to stop", de: "Wo angehalten wird" },
    /// press a key…
    PressAKey { en: "press a key…", de: "drücke eine Taste…" },
    /// unbound
    Unbound { en: "unbound", de: "nicht belegt" },
    /// Reset all
    ResetAll { en: "Reset all", de: "Alles zurücksetzen" },
    /// reset
    Reset { en: "reset", de: "zurücksetzen" },
    /// on
    SwitchOn { en: "on", de: "an" },
    /// off
    SwitchOff { en: "off", de: "aus" },
    /// A red step is one the client passes for you.
    RailExplain {
        en: "A red step is one the client passes for you. Nothing is red until you make it red.",
        de: "Ein roter Schritt ist einer, den der Client für dich abgibt. Nichts ist rot, bis du es rot machst.",
    },
    /// The half of the rail that is your own turns.
    YourTurns { en: "Your turns", de: "Deine Züge" },
    /// The other half.
    TheirTurns { en: "Opponents' turns", de: "Gegnerische Züge" },
    /// Hold the table still
    HoldTheTableStill { en: "Hold the table still", de: "Tisch ruhig halten" },
    /// Cards and the camera go straight there instead of moving.
    HoldTheTableStillWhy {
        en: "Cards and the camera go straight there instead of moving.",
        de: "Karten und Kamera springen hin, statt sich zu bewegen.",
    },
    /// Do the obvious thing
    ActPrimary { en: "Do the obvious thing", de: "Das Naheliegende tun" },
    /// Confirm / pass priority
    ActConfirm { en: "Confirm / pass priority", de: "Bestätigen / Priorität abgeben" },
    /// Cancel
    ActCancel { en: "Cancel", de: "Abbrechen" },
    /// Play or choose the card
    ActActivateCard { en: "Play or choose the card", de: "Karte spielen oder wählen" },
    /// Cursor left
    ActCursorLeft { en: "Cursor left", de: "Cursor nach links" },
    /// Cursor right
    ActCursorRight { en: "Cursor right", de: "Cursor nach rechts" },
    /// Cursor up
    ActCursorUp { en: "Cursor up", de: "Cursor nach oben" },
    /// Cursor down
    ActCursorDown { en: "Cursor down", de: "Cursor nach unten" },
    /// Aim at the next defender
    ActCombatFocusNext {
        en: "Aim at the next defender",
        de: "Auf den nächsten Verteidiger zielen",
    },
    /// Aim at the previous defender
    ActCombatFocusPrev {
        en: "Aim at the previous defender",
        de: "Auf den vorigen Verteidiger zielen",
    },
    /// Declare nothing
    ActCombatNone { en: "Declare nothing", de: "Nichts deklarieren" },
    /// Skip to the next phase
    ActNextPhase { en: "Skip to the next phase", de: "Zur nächsten Phase springen" },
    /// Skip to the next turn
    ActNextTurn { en: "Skip to the next turn", de: "Zum nächsten Zug springen" },
    /// Hide the board overlay
    ActToggleOverlay { en: "Hide the board overlay", de: "Brett-Overlay ausblenden" },
    /// Read card text instead of art
    ActToggleTextView { en: "Read card text instead of art", de: "Kartentext statt Bild lesen" },
    /// Keep this hand
    ActMulliganKeep { en: "Keep this hand", de: "Diese Hand behalten" },
    /// Mulligan
    ActMulliganTake { en: "Mulligan", de: "Mulligan" },
    /// Yes
    ActAnswerYes { en: "Yes", de: "Ja" },
    /// No
    ActAnswerNo { en: "No", de: "Nein" },
    /// Number up
    ActNumberUp { en: "Number up", de: "Zahl hoch" },
    /// Number down
    ActNumberDown { en: "Number down", de: "Zahl runter" },
    /// Rail selection up
    ActRailUp { en: "Rail selection up", de: "Phasenleiste hoch" },
    /// Rail selection down
    ActRailDown { en: "Rail selection down", de: "Phasenleiste runter" },
    /// Look at the next opponent
    ActFocusNextSeat { en: "Look at the next opponent", de: "Zum nächsten Gegner sehen" },
    /// Look at your own board
    ActFocusHome { en: "Look at your own board", de: "Auf das eigene Brett sehen" },
    /// Answering
    GroupAnswering { en: "Answering", de: "Antworten" },
    /// Moving around
    GroupMovingAround { en: "Moving around", de: "Bewegen" },
    /// Combat
    GroupCombat { en: "Combat", de: "Kampf" },
    /// Phases
    GroupPhases { en: "Phases", de: "Phasen" },
    /// Questions
    GroupQuestions { en: "Questions", de: "Fragen" },
    /// Display
    GroupDisplay { en: "Display", de: "Darstellung" },
    /// Pass when there is nothing to do
    AutoPassLabel {
        en: "Pass when there is nothing to do",
        de: "Abgeben, wenn nichts zu tun ist",
    },
    /// No land, no spell, no ability, nothing to suspend: pass without asking.
    AutoPassDetail {
        en: "No land, no spell, no ability, nothing to suspend: pass without asking.",
        de: "Kein Land, kein Zauber, keine Fähigkeit, nichts zu suspendieren: ohne Nachfrage abgeben.",
    },
    /// Pass through opponents' turns
    AutoSkipTurnsLabel {
        en: "Pass through opponents' turns",
        de: "Durch gegnerische Züge abgeben",
    },
    /// Priority only. It never declines a block for you.
    AutoSkipTurnsDetail {
        en: "Priority only. It never declines a block for you.",
        de: "Nur Priorität. Ein Blocken lehnt es nie für dich ab.",
    },
    /// Skip an empty attack step
    AutoSkipAttacksLabel {
        en: "Skip an empty attack step",
        de: "Leeren Angriffsschritt überspringen",
    },
    /// Only when nothing you control can attack.
    AutoSkipAttacksDetail {
        en: "Only when nothing you control can attack.",
        de: "Nur wenn nichts, das du kontrollierst, angreifen kann.",
    },
    /// Skip an empty block step
    AutoSkipBlocksLabel { en: "Skip an empty block step", de: "Leeren Blockschritt überspringen" },
    /// Only when nothing you control can block.
    AutoSkipBlocksDetail {
        en: "Only when nothing you control can block.",
        de: "Nur wenn nichts, das du kontrollierst, blocken kann.",
    },
    /// Untap
    RailUntap { en: "Untap", de: "Enttappen" },
    /// Upkeep
    RailUpkeep { en: "Upkeep", de: "Versorgung" },
    /// Draw
    RailDraw { en: "Draw", de: "Ziehen" },
    /// Main 1
    RailMain1 { en: "Main 1", de: "Haupt 1" },
    /// Begin Combat
    RailCombatBegin { en: "Begin Combat", de: "Kampfbeginn" },
    /// Attackers
    RailAttackers { en: "Attackers", de: "Angreifer" },
    /// Blockers
    RailBlockers { en: "Blockers", de: "Blocker" },
    /// Damage
    RailDamage { en: "Damage", de: "Schaden" },
    /// End of Combat
    RailCombatEnd { en: "End of Combat", de: "Kampfende" },
    /// Main 2
    RailMain2 { en: "Main 2", de: "Haupt 2" },
    /// End Step
    RailEndStep { en: "End Step", de: "Endschritt" },
    /// Cleanup
    RailCleanup { en: "Cleanup", de: "Aufräumen" },


    // ---- the deck builder
    /// White
    ColorWhite { en: "White", de: "Weiß" },
    /// Blue
    ColorBlue { en: "Blue", de: "Blau" },
    /// Black
    ColorBlack { en: "Black", de: "Schwarz" },
    /// Red
    ColorRed { en: "Red", de: "Rot" },
    /// Green
    ColorGreen { en: "Green", de: "Grün" },
    /// Colourless
    ColorColourless { en: "Colourless", de: "Farblos" },
    /// Creature
    KindCreature { en: "Creature", de: "Kreatur" },
    /// Instant
    KindInstant { en: "Instant", de: "Spontanzauber" },
    /// Sorcery
    KindSorcery { en: "Sorcery", de: "Hexerei" },
    /// Artifact
    KindArtifact { en: "Artifact", de: "Artefakt" },
    /// Enchantment
    KindEnchantment { en: "Enchantment", de: "Verzauberung" },
    /// Planeswalker
    KindPlaneswalker { en: "Planeswalker", de: "Planeswalker" },
    /// Battle
    KindBattle { en: "Battle", de: "Schlacht" },
    /// Land
    KindLand { en: "Land", de: "Land" },
    /// Other
    KindOther { en: "Other", de: "Sonstiges" },
    /// Creatures
    GroupCreatures { en: "Creatures", de: "Kreaturen" },
    /// Planeswalkers
    GroupPlaneswalkers { en: "Planeswalkers", de: "Planeswalker" },
    /// Instants
    GroupInstants { en: "Instants", de: "Spontanzauber" },
    /// Sorceries
    GroupSorceries { en: "Sorceries", de: "Hexereien" },
    /// Artifacts
    GroupArtifacts { en: "Artifacts", de: "Artefakte" },
    /// Enchantments
    GroupEnchantments { en: "Enchantments", de: "Verzauberungen" },
    /// Battles
    GroupBattles { en: "Battles", de: "Schlachten" },
    /// Lands
    GroupLands { en: "Lands", de: "Länder" },
    /// Other
    GroupOther { en: "Other", de: "Sonstiges" },
    /// A–Z
    SortName { en: "A–Z", de: "A–Z" },
    /// Cost
    SortCost { en: "Cost", de: "Kosten" },
    /// Type
    SortType { en: "Type", de: "Typ" },
    /// Sort: {0}
    SortBy { en: "Sort: {0}", de: "Sortierung: {0}" },
    /// Cards ({0})
    PaneCards { en: "Cards ({0})", de: "Karten ({0})" },
    /// Deck ({0} / {1})
    PaneDeck { en: "Deck ({0} / {1})", de: "Deck ({0} / {1})" },
    /// Leave without saving
    LeaveWithoutSaving { en: "Leave without saving", de: "Ohne Speichern verlassen" },
    /// ‹ Decks
    BackToDecks { en: "‹ Decks", de: "‹ Decks" },
    /// Editing a deck
    EditingADeck { en: "Editing a deck", de: "Deck bearbeiten" },
    /// A new deck
    ANewDeck { en: "A new deck", de: "Ein neues Deck" },
    /// Save deck
    SaveDeck { en: "Save deck", de: "Deck speichern" },
    /// Saved
    DeckIsSaved { en: "Saved", de: "Gespeichert" },
    /// Hide filters
    HideFilters { en: "Hide filters", de: "Filter ausblenden" },
    /// Filters
    ShowFilters { en: "Filters", de: "Filter" },
    /// Clear
    ClearFilters { en: "Clear", de: "Zurücksetzen" },
    /// Playable only
    PlayableOnly { en: "Playable only", de: "Nur spielbare" },
    /// {0} of {1} cards{2}
    PoolTally { en: "{0} of {1} cards{2}", de: "{0} von {1} Karten{2}" },
    ///  — showing {0}, keep typing to narrow it
    PoolNarrow {
        en: " — showing {0}, keep typing to narrow it",
        de: " — {0} gezeigt, tippe weiter zum Eingrenzen",
    },
    /// nothing matches — try fewer filters
    NothingMatches {
        en: "nothing matches — try fewer filters",
        de: "nichts gefunden — versuche weniger Filter",
    },
    /// no printings
    NoPrintings { en: "no printings", de: "keine Drucke" },
    /// looking for other printings…
    LookingForPrintings { en: "looking for other printings…", de: "suche weitere Drucke…" },
    /// {0} of {1}
    PrintingAt { en: "{0} of {1}", de: "{0} von {1}" },
    /// this gateway has no card catalog — only this build's printing
    NoCatalogOnlyThis {
        en: "this gateway has no card catalog — only this build's printing",
        de: "dieses Gateway hat keinen Kartenkatalog — nur den Druck dieses Builds",
    },
    /// All
    AllSets { en: "All", de: "Alle" },
    /// Plain
    FinishPlain { en: "Plain", de: "Normal" },
    /// Foil
    FinishFoil { en: "Foil", de: "Folie" },
    /// Etched
    FinishEtched { en: "Etched", de: "Geätzt" },
    /// Add
    AddPrinting { en: "Add", de: "Hinzufügen" },
    /// {0} in the {1}
    CountInZone { en: "{0} in the {1}", de: "{0} {1}" },
    /// deck
    ZoneDeck { en: "deck", de: "im Deck" },
    /// sideboard
    ZoneSideboard { en: "sideboard", de: "im Sideboard" },
    /// no art for this printing
    NoArtForPrinting { en: "no art for this printing", de: "kein Bild für diesen Druck" },
    /// no rules text — this gateway has no card catalog
    NoRulesText {
        en: "no rules text — this gateway has no card catalog",
        de: "kein Regeltext — dieses Gateway hat keinen Kartenkatalog",
    },
    /// {0} — this card will not play as printed
    NotAsPrinted {
        en: "{0} — this card will not play as printed",
        de: "{0} — diese Karte spielt nicht wie gedruckt",
    },
    /// + deck
    AddToDeck { en: "+ deck", de: "+ Deck" },
    /// + sideboard
    AddToSideboard { en: "+ sideboard", de: "+ Sideboard" },
    /// → sideboard
    MoveToSideboard { en: "→ sideboard", de: "→ Sideboard" },
    /// → deck
    MoveToDeck { en: "→ deck", de: "→ Deck" },
    /// remove
    RemoveCard { en: "remove", de: "entfernen" },
    /// commander ✓
    IsCommander { en: "commander ✓", de: "Kommandeur ✓" },
    /// set as commander
    SetCommander { en: "set as commander", de: "als Kommandeur" },
    /// {0} in the deck
    HeldInDeck { en: "{0} in the deck", de: "{0} im Deck" },
    /// {0} in the sideboard
    HeldInSideboard { en: "{0} in the sideboard", de: "{0} im Sideboard" },
    /// {0} in the deck, {1} in the sideboard
    HeldInBoth {
        en: "{0} in the deck, {1} in the sideboard",
        de: "{0} im Deck, {1} im Sideboard",
    },
    /// partial
    CoveragePartial { en: "partial", de: "teilweise" },
    /// stub
    CoverageStub { en: "stub", de: "Rumpf" },
    /// DECK NAME
    DeckNameLabel { en: "DECK NAME", de: "DECKNAME" },
    /// Main {0}
    TabMain { en: "Main {0}", de: "Deck {0}" },
    /// Sideboard {0}
    TabSide { en: "Sideboard {0}", de: "Sideboard {0}" },
    /// {0} lands · {1} creatures · {2} other spells
    DeckMakeup {
        en: "{0} lands · {1} creatures · {2} other spells",
        de: "{0} Länder · {1} Kreaturen · {2} andere Zauber",
    },
    /// empty — tap a card on the left to add it
    DeckEmptyHint {
        en: "empty — tap a card on the left to add it",
        de: "leer — tippe links auf eine Karte, um sie hinzuzufügen",
    },
    /// Empty the deck
    EmptyTheDeck { en: "Empty the deck", de: "Deck leeren" },
    /// dropped: {0}
    DroppedCards { en: "dropped: {0}", de: "entfallen: {0}" },
    /// The deck needs a name.
    DeckNeedsName { en: "The deck needs a name.", de: "Das Deck braucht einen Namen." },
    /// That name is too long (64 characters at most).
    DeckNameTooLong {
        en: "That name is too long (64 characters at most).",
        de: "Der Name ist zu lang (höchstens 64 Zeichen).",
    },
    /// The deck is empty.
    DeckIsEmpty { en: "The deck is empty.", de: "Das Deck ist leer." },
    /// At most {0} different cards per list.
    TooManyLines {
        en: "At most {0} different cards per list.",
        de: "Höchstens {0} verschiedene Karten je Liste.",
    },
    /// At most {0} cards in each list.
    TooManyCards { en: "At most {0} cards in each list.", de: "Höchstens {0} Karten je Liste." },
    /// {0} is no longer in the card pool.
    CardGoneFromPool {
        en: "{0} is no longer in the card pool.",
        de: "{0} ist nicht mehr im Kartenpool.",
    },
    /// {0} cards — a constructed deck wants at least {1}.
    DeckTooSmall {
        en: "{0} cards — a constructed deck wants at least {1}.",
        de: "{0} Karten — ein Constructed-Deck will mindestens {1}.",
    },
    /// A sideboard is usually at most {0} cards.
    SideboardTooBig {
        en: "A sideboard is usually at most {0} cards.",
        de: "Ein Sideboard hat üblicherweise höchstens {0} Karten.",
    },
    /// {0} lands in {1} cards is thin for this curve.
    ThinOnLands {
        en: "{0} lands in {1} cards is thin for this curve.",
        de: "{0} Länder auf {1} Karten sind dünn für diese Kurve.",
    },
    /// {0} card(s) are not fully implemented yet and will not play as printed.
    ShakyCards {
        en: "{0} card(s) are not fully implemented yet and will not play as printed.",
        de: "{0} Karte(n) sind noch nicht vollständig umgesetzt und spielen nicht wie gedruckt.",
    },


    // ---- the table
    /// Waiting for seat {0}
    WaitingForSeat { en: "Waiting for seat {0}", de: "Warte auf Platz {0}" },
    /// Waiting
    JustWaiting { en: "Waiting", de: "Warte" },
    /// Keep this hand? (the next mulligan is free)
    MulliganFree {
        en: "Keep this hand? (the next mulligan is free)",
        de: "Diese Hand behalten? (der nächste Mulligan ist frei)",
    },
    /// Keep this hand? ({0} taken)
    MulliganTaken { en: "Keep this hand? ({0} taken)", de: "Diese Hand behalten? ({0} genommen)" },
    /// Put {0} card(s) on the bottom
    PutOnBottom { en: "Put {0} card(s) on the bottom", de: "Lege {0} Karte(n) nach unten" },
    /// You have priority
    YouHavePriority { en: "You have priority", de: "Du hast Priorität" },
    /// Declare attackers
    DeclareAttackers { en: "Declare attackers", de: "Angreifer deklarieren" },
    /// Declare blockers
    DeclareBlockers { en: "Declare blockers", de: "Blocker deklarieren" },
    /// Discard {0} card(s)
    DiscardCards { en: "Discard {0} card(s)", de: "Wirf {0} Karte(n) ab" },
    /// Legend rule: keep one
    LegendRule { en: "Legend rule: keep one", de: "Legendenregel: behalte eine" },
    /// card(s)
    NounCards { en: "card(s)", de: "Karte(n)" },
    /// target(s)
    NounTargets { en: "target(s)", de: "Ziel(e)" },
    /// Choose up to {0} {1}
    ChooseUpTo { en: "Choose up to {0} {1}", de: "Wähle bis zu {0} {1}" },
    /// Choose {0} {1}
    ChooseExactly { en: "Choose {0} {1}", de: "Wähle {0} {1}" },
    /// Choose {0}–{1} {2}
    ChooseBetween { en: "Choose {0}–{1} {2}", de: "Wähle {0}–{1} {2}" },
    /// Choose a creature type
    ChooseCreatureType { en: "Choose a creature type", de: "Wähle einen Kreaturtyp" },
    /// Choose a colour
    ChooseColour { en: "Choose a colour", de: "Wähle eine Farbe" },
    /// Choose a number ({0}–{1})
    ChooseNumberIn { en: "Choose a number ({0}–{1})", de: "Wähle eine Zahl ({0}–{1})" },
    /// Choose a player
    ChoosePlayer { en: "Choose a player", de: "Wähle einen Spieler" },
    /// Choose how to cast
    ChooseHowToCast { en: "Choose how to cast", de: "Wähle, wie gewirkt wird" },
    /// Put these in order
    PutInOrder { en: "Put these in order", de: "Bringe diese in eine Reihenfolge" },
    /// The game is over
    TheGameIsOver { en: "The game is over", de: "Das Spiel ist vorbei" },
    /// Pay {0} life? Otherwise it enters tapped
    PayLifeOrTapped {
        en: "Pay {0} life? Otherwise it enters tapped",
        de: "{0} Leben zahlen? Sonst kommt es getappt ins Spiel",
    },
    /// Pay the additional cost?
    PayAdditionalCost { en: "Pay the additional cost?", de: "Die zusätzlichen Kosten zahlen?" },
    /// Pay {{0}}?
    PayTax { en: "Pay {{0}}?", de: "{{0}} zahlen?" },
    /// Cast it for its miracle cost?
    CastForMiracle { en: "Cast it for its miracle cost?", de: "Für die Wunderkosten wirken?" },
    /// A draw was offered. Accept?
    DrawWasOffered {
        en: "A draw was offered. Accept?",
        de: "Ein Remis wurde angeboten. Annehmen?",
    },
    /// Yes or no?
    YesOrNo { en: "Yes or no?", de: "Ja oder nein?" },
    /// Offer a draw
    OfferADraw { en: "Offer a draw", de: "Remis anbieten" },
    /// Concede
    Concede { en: "Concede", de: "Aufgeben" },
    /// Aim next
    AimNext { en: "Aim next", de: "Nächstes Ziel" },
    /// Attack
    Attack { en: "Attack", de: "Angreifen" },
    /// Block
    Block { en: "Block", de: "Blocken" },
    /// None
    DeclareNone { en: "None", de: "Keine" },
    /// Keep
    KeepHand { en: "Keep", de: "Behalten" },
    /// Mulligan
    TakeMulligan { en: "Mulligan", de: "Mulligan" },
    /// OK
    ConfirmOk { en: "OK", de: "OK" },
    /// Stack
    StackTitle { en: "Stack", de: "Stapel" },
    /// Spell
    StackSpell { en: "Spell", de: "Zauber" },
    /// Ability
    StackAbilityBare { en: "Ability", de: "Fähigkeit" },
    /// Ability · {0}
    StackAbility { en: "Ability · {0}", de: "Fähigkeit · {0}" },
    /// Seat {0}
    SeatNumbered { en: "Seat {0}", de: "Platz {0}" },
    /// You ({0})
    YouNamed { en: "You ({0})", de: "Du ({0})" },
    /// Aimed at {0} ({1} of {2})
    AimedAt { en: "Aimed at {0} ({1} of {2})", de: "Zielt auf {0} ({1} von {2})" },
    /// {0} declared
    DeclaredCount { en: "{0} declared", de: "{0} deklariert" },
    /// a seat
    ASeat { en: "a seat", de: "ein Platz" },
    /// a permanent
    APermanent { en: "a permanent", de: "eine bleibende Karte" },
    /// nothing
    AimingAtNothing { en: "nothing", de: "nichts" },
    /// YOU
    RailYou { en: "YOU", de: "DU" },
    /// OPPONENT
    RailOpponent { en: "OPPONENT", de: "GEGNER" },


    // ---- what an ability costs
    /// Tap for {0}
    TapFor { en: "Tap for {0}", de: "Tappen für {0}" },
    /// Ability {0}
    AbilityNumbered { en: "Ability {0}", de: "Fähigkeit {0}" },
    /// Sacrifice this
    CostSacrificeThis { en: "Sacrifice this", de: "Opfere dies" },
    /// Sacrifice
    CostSacrifice { en: "Sacrifice", de: "Opfern" },
    /// Pay {0} life
    CostPayLife { en: "Pay {0} life", de: "Zahle {0} Leben" },
    /// Pay X life
    CostPayXLife { en: "Pay X life", de: "Zahle X Leben" },
    /// Discard
    CostDiscard { en: "Discard", de: "Abwerfen" },
    /// Discard this
    CostDiscardThis { en: "Discard this", de: "Wirf dies ab" },
    /// Exile this
    CostExileThis { en: "Exile this", de: "Schicke dies ins Exil" },
    /// Return this
    CostReturnThis { en: "Return this", de: "Nimm dies zurück" },
    /// Exile a card
    CostExileACard { en: "Exile a card", de: "Schicke eine Karte ins Exil" },
    /// Rules text unavailable
    NoRulesTextHere { en: "Rules text unavailable", de: "Regeltext nicht verfügbar" },

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
