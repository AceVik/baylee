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
//! window, and the three rules worth having are tests: every phrase answers in
//! every language, a phrase's placeholders are the same set in all of
//! them — `{0}` moving is what translation *is*, `{0}` vanishing is a bug —
//! and **no phrase carries a bracketed plural**.
//!
//! That last one is [`Phrase::counted`]. A sentence whose subject is counted
//! is written **twice**, once for one thing and once for the rest, and the
//! number picks between them; `card(s)` and `Karte(n)` are not plurals, and
//! the sheet drew them in grey as bracketed asides beside the very number
//! they disagreed with.
//!
//! # What is not here
//!
//! A refusal that arrived as **prose from another process** is shown in the
//! words that process sent, because it is the one that knows why it said no.
//! [`Refusal`] is the pair — a phrase this client owns, or somebody else's
//! sentence — and the prompt bar renders either.
//!
//! This used to say only that *the gateway's* refusals (`{"error":"…"}`) are
//! shown in the gateway's words, and that translating them is "a protocol
//! change — a code beside the prose". That is still true, and it is about the
//! **lobby**: `ErrorBody` in the gateway is one `error` field and nothing
//! else, so a code there is a field that does not exist yet.
//!
//! What it was silent about is the **duel's** refusal slot, and #121 is that
//! measurement. Nothing the gateway says reaches it: the gateway forwards
//! `SeatFrame` bytes it never decodes. Two writers reach it, and they are
//! shaped differently, which is the whole argument for [`Refusal`] having two
//! arms:
//!
//! - **This client wrote eight** of the sentences there, a closed set that
//!   was English for no better reason than that `Duel::last_error` was a
//!   `String` — one stale-deed line at six call sites, one cast-mode line,
//!   and the six a mana run gives up with. They are phrases now.
//! - **The engine's half cannot be enumerated.** Three of its lines are
//!   fixed, and the other four call sites are `error(reason)`, forwarding
//!   whatever the rules kernel refused with. There is no list to translate,
//!   which is why `Verbatim` is the design and not the backlog.
//!
//! Should a *named* engine refusal ever want translating, that is still not a
//! protocol change: `v1::Error` has carried a `code` field the whole time,
//! hard-coded to `1` by both writers, so the wire is already there and what
//! is missing is a taxonomy.

use baylee_core::ids::PlayerId;
use baylee_view::GameStatic;

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
    /// Restyle an existing row, preserving quantity.
    ApplyPrinting { en: "Apply to this row", de: "Auf diese Zeile anwenden" },
    /// Explicit catalog insertion destinations.
    AddMainShort { en: "+ Main", de: "+ Hauptdeck" },
    /// Explicit catalog insertion destinations.
    AddSideShort { en: "+ Side", de: "+ Sideboard" },
    /// Account-only save history.
    HistoryAccountHint { en: "History is available for account decks after signing in.", de: "Historie gibt es nach der Anmeldung für Account-Decks." },
    /// New deck without a persisted identity.
    HistorySaveHint { en: "Save this deck to start its history.", de: "Speichere das Deck, um seine Historie zu beginnen." },
    /// Destructive deck confirmation.
    DeleteDeckQuestion { en: "Delete “{0}”?", de: "„{0}“ löschen?" },
    /// Destructive deck confirmation.
    ClearDeckQuestion { en: "Empty “{0}”?", de: "„{0}“ leeren?" },
    /// Destructive deck confirmation.
    DestructiveHint { en: "Please confirm. Clearing removes the main deck and sideboard; deleting removes the saved deck.", de: "Bitte bestätigen. Leeren entfernt Hauptdeck und Sideboard; Löschen entfernt das gespeicherte Deck." },
    /// Commander controls in the deck overview.
    CommanderSection { en: "Commanders", de: "Commander" },
    /// Commander selection.
    ChooseCommander { en: "Choose / replace", de: "Auswählen / ersetzen" },
    /// Partner selection.
    ChoosePartner { en: "+ Partner", de: "+ Partner" },
    /// Return from selecting a leader to normal card browsing.
    DoneChoosing { en: "Back to all cards", de: "Zurück zu allen Karten" },
    /// Roles and deck cards are distinct.
    CommanderHint { en: "Choose a leader here. Removing the role keeps the card in your deck.", de: "Hier wählst du deinen Commander. Entfernen hebt die Rolle auf; die Karte bleibt im Deck." },
    /// A partner must match the selected commander's rules.
    PartnerHint { en: "Only compatible partners are shown. Some commanders cannot have a partner.", de: "Hier erscheinen nur passende Partner. Manche Commander können keinen Partner haben." },
    /// Show or hide detailed deck statistics.
    DeckStatistics { en: "Statistics", de: "Statistiken" },
    /// Hub navigation.
    HubPlay { en: "Play", de: "Spielen" },
    /// Hub navigation.
    HubDecks { en: "My collection", de: "Meine Sammlung" },
    /// Hub navigation.
    /// The deck selected for the next game
    SelectedDeck { en: "Your next game", de: "Dein nächstes Spiel" },
    /// Choose a deck
    ChooseDeck { en: "Choose a deck", de: "Deck auswählen" },
    /// Hub navigation.
    CreateTable { en: "Create table", de: "Tisch erstellen" },
    /// Hub navigation.
    PlayerCount { en: "{0} players", de: "{0} Spieler" },
    /// Hub navigation.
    NoOwnDecks { en: "Start with a house deck or build your own.", de: "Starte mit einem Hausdeck oder baue dein eigenes." },
    /// Hub navigation.
    CollectionHint { en: "Your decks, your ideas. Every saved change stays in your history.", de: "Deine Decks, deine Ideen. Jede gespeicherte Änderung bleibt in deiner Historie." },
    /// Gateway selection.
    GatewayStep { en: "01 / CONNECT", de: "01 / VERBINDEN" },
    /// Gateway selection.
    AccountStep { en: "02 / YOUR ACCOUNT", de: "02 / DEIN KONTO" },
    /// Gateway selection.
    ChooseGateway { en: "Choose your gateway", de: "Wähle deinen Gateway" },
    /// Gateway selection.
    GatewayHint { en: "Your gateway is the server where your account, decks and tables live. Saved addresses stay on this device.", de: "Dein Gateway ist der Server für dein Konto, deine Decks und Tische. Gespeicherte Adressen bleiben auf diesem Gerät." },
    /// Gateway selection.
    ChooseGatewayFirst { en: "Select a gateway to sign in or register.", de: "Wähle einen Gateway, um dich anzumelden oder zu registrieren." },
    /// Gateway selection.
    GatewayAddress { en: "GATEWAY ADDRESS", de: "GATEWAY-ADRESSE" },
    /// Gateway selection.
    SaveGateway { en: "Save gateway", de: "Gateway speichern" },
    /// Gateway selection.
    GatewayUrlInvalid { en: "Enter an http:// or https:// address without credentials, query parameters or a fragment.", de: "Gib eine http://- oder https://-Adresse ohne Zugangsdaten, Abfrageparameter oder Fragment ein." },
    /// Server account-name rules, displayed before a registration is submitted.
    AccountNameHint { en: "3–16 characters: A–Z, 0–9, _ or -. Start and end with a letter or number.", de: "3–16 Zeichen: A–Z, 0–9, _ oder -. Am Anfang und Ende ein Buchstabe oder eine Zahl." },
    /// Password registration guidance.
    AccountPasswordHint { en: "At least 8 characters. Avoid common passwords and your name or email.", de: "Mindestens 8 Zeichen. Kein häufiges Passwort und nicht dein Name oder deine E-Mail." },
    /// A rejected password during registration.
    AccountPasswordInvalid { en: "Choose a password with 8–256 characters, different from your name and email, and not a common password.", de: "Wähle ein Passwort mit 8–256 Zeichen, verschieden von Name und E-Mail und kein häufiges Passwort." },
    /// A library reply was not a valid response.
    LibraryReadFailed { en: "The library response could not be read. Please try again.", de: "Die Bibliotheksantwort konnte nicht gelesen werden. Bitte erneut versuchen." },
    /// Library and front-door interface.
    WelcomeTitle { en: "Your next game starts here.", de: "Dein nächstes Spiel beginnt hier." },
    /// Library and front-door interface.
    WelcomeNote { en: "Build a deck. Find your table. Make it yours.", de: "Baue dein Deck. Finde deinen Tisch. Spiele deinen Stil." },
    /// Library and front-door interface.
    AccountBenefit { en: "Your decks and their saved versions, together in one account.", de: "Deine Decks und ihre gespeicherten Versionen an einem Ort." },
    /// Library and front-door interface.
    OfflineBenefit { en: "Try a game without an account. Offline decks stay on this device.", de: "Spiele ohne Konto. Offline-Decks bleiben auf diesem Gerät." },
    /// Library and front-door interface.
    LobbyGuide { en: "Choose your deck, then join or open a table.", de: "Wähle dein Deck und tritt einem Tisch bei oder eröffne einen." },
    /// Library and front-door interface.
    HouseDecks { en: "House decks", de: "Hausdecks" },
    /// Library and front-door interface.
    HouseHint { en: "Ready to play. Copy a deck to make it your own.", de: "Bereit zum Spielen. Übernimm ein Deck als eigene Kopie." },
    /// Library and front-door interface.
    CopyToDecks { en: "Add to my decks", de: "Zu meinen Decks hinzufügen" },
    /// Library and front-door interface.
    InspectDeck { en: "View cards", de: "Karten ansehen" },
    /// Library and front-door interface.
    DeckHistory { en: "History", de: "Historie" },
    /// Library and front-door interface.
    HistoryHint { en: "Every save is kept. Restoring creates a new version; later saves remain available.", de: "Jeder Stand bleibt erhalten. Wiederherstellen erzeugt eine neue Version; spätere Stände bleiben verfügbar." },
    /// Library and front-door interface.
    WorkingDiff { en: "Changes from the latest saved version", de: "Änderungen gegenüber der zuletzt gespeicherten Version" },
    /// Library and front-door interface.
    RestoreVersion { en: "Restore this version", de: "Diesen Stand wiederherstellen" },
    /// Library and front-door interface.
    ConfirmRestore { en: "Confirm restore", de: "Wiederherstellen bestätigen" },
    /// Library and front-door interface.
    RestoreWarning { en: "This replaces your working deck, including unsaved edits. Saved versions remain available.", de: "Dies ersetzt dein Arbeitsdeck einschließlich ungespeicherter Änderungen. Gespeicherte Versionen bleiben erhalten." },
    /// Library and front-door interface.
    CurrentVersion { en: "Current", de: "Aktuell" },
    /// Compact printing-picker control.
    ChoosePrintShort { en: "Art", de: "Bild" },
    /// Timestamp of the current save.
    VersionSavedAt { en: "Saved", de: "Gespeichert" },
    /// Historical timestamps describe when the next save replaced a version.
    VersionSupersededAt { en: "Replaced by next save", de: "Durch nächsten Stand ersetzt" },
    /// Library and front-door interface.
    VersionLabel { en: "Version {0}", de: "Version {0}" },
    /// Library and front-door interface.
    LibraryEmpty { en: "No decks published yet. You can still build your own.", de: "Noch keine Decks veröffentlicht. Du kannst ein eigenes erstellen." },
    /// Library and front-door interface.
    NoPastVersions { en: "Your next save will appear here.", de: "Mit dem nächsten Speichern beginnt deine Historie." },
    /// Library and front-door interface.
    LibraryLoading { en: "Loading your library…", de: "Bibliothek wird geladen…" },
    /// Library and front-door interface.
    LibraryRetry { en: "Try again", de: "Erneut versuchen" },
    /// Library and front-door interface.
    LibraryBack { en: "Back", de: "Zurück" },
    /// Library and front-door interface.
    LibraryMain { en: "Main deck", de: "Hauptdeck" },
    /// Library and front-door interface.
    LibrarySide { en: "Sideboard", de: "Sideboard" },
    /// Library and front-door interface.
    LibraryCommanders { en: "Commanders", de: "Kommandeure" },
    /// Library and front-door interface.
    NoChanges { en: "No changes in this section.", de: "Keine Änderungen in diesem Bereich." },
    /// Library and front-door interface.
    DeckRows { en: "{0} main rows · {1} sideboard rows", de: "{0} Hauptdeck-Zeilen · {1} Sideboard-Zeilen" },
    /// Library and front-door interface.
    LibraryCounts { en: "{0} cards · {1} sideboard", de: "{0} Karten · {1} Sideboard" },
    /// Library and front-door interface.
    LobbyCounts { en: "{0} decks · {1} matching tables", de: "{0} Decks · {1} passende Tische" },
    /// Library and front-door interface.
    Composition { en: "Deck overview", de: "Deckübersicht" },
    /// Library and front-door interface.
    UniqueCards { en: "Unique cards", de: "Verschiedene Karten" },
    /// Library and front-door interface.
    AverageMana { en: "Avg. mana · nonlands", de: "Ø Mana · ohne Länder" },
    /// Library and front-door interface.
    LandShare { en: "Land share", de: "Länderanteil" },
    /// Library and front-door interface.
    OpeningLand { en: "Land in opening seven", de: "Land in den ersten sieben" },
    /// Library and front-door interface.
    ConfirmDeleteDeck { en: "Delete permanently?", de: "Endgültig löschen?" },
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
    /// Opens the lobby with no account and no gateway.
    PlayOffline { en: "Play offline", de: "Offline spielen" },
    /// The status line while offline play is on.
    PlayingOffline {
        en: "Offline: your decks and a table of house AI",
        de: "Offline: deine Decks und ein Tisch voll Haus-KI",
    },
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
    /// Inside the lobby's table search box while it is empty.
    SearchTables { en: "Table or host…", de: "Tisch oder Gastgeber …" },
    /// Inside the deck builder's search box while it is empty.
    ///
    /// Three words and not a syntax lesson: the box takes the whole query
    /// language, and the gear beside it is where that is taught. What a
    /// placeholder can honestly say is where a bare word looks — which here
    /// is one field wider than the zone browser's, because a pool row has
    /// the card's rules text and a projected object does not.
    SearchCards { en: "Name, type, text…", de: "Name, Typ, Text …" },
    /// Runs the search.
    DoSearch { en: "Search", de: "Suchen" },
    /// Re-reads decks and tables.
    Refresh { en: "Refresh", de: "Neu laden" },
    /// The one-tap game against the house AI.
    PlayTheHouse { en: "Play the house", de: "Gegen das Haus" },
    /// What a chair the house plays is called on its own bar.
    ///
    /// Written from the **flag** and never from the name the host sent, and
    /// that is safe for a plain reason rather than a clever one: neither
    /// producer has any other name for such a chair. `LocalHost` seats the
    /// literal `"House AI"` and the gateway writes the same string for every
    /// empty chair in a running game, so there is no host-chosen name for
    /// this to hide — only an English one for a German player to read.
    ///
    /// The day a host does name its AI chairs, this is the line that has to
    /// give way, and `a_house_chair_is_called_the_house_in_the_players_own_
    /// language`'s sibling in `host.rs` is what will say so: it pins the one
    /// string `LocalHost` writes.
    SeatHouse { en: "House AI", de: "Haus-KI" },
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
    /// The relaxed house AI.
    AiCasual { en: "casual", de: "Locker" },
    /// The middle one.
    AiSteady { en: "steady", de: "Solide" },
    /// The one that plays to win.
    AiSharp { en: "sharp", de: "Scharf" },
    /// The deepest house AI combat search.
    AiExpert { en: "expert", de: "Experte" },
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
    /// The same, when the seat being waited for is the one reading it.
    ///
    /// A phrase of its own rather than [`Phrase::WaitingFor`] filled with a
    /// pronoun, because a name and a pronoun do not decline alike: German
    /// wants the accusative after "auf", so this slot would have to be handed
    /// "dich" while every other line that names the local seat wants "Du". A
    /// slot that needs a different word in different sentences is a slot that
    /// gets filled wrongly, and the sentence is what to split.
    WaitingForYou { en: "waiting for you", de: "wartet auf dich" },
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
    /// The same banner offline, where the table carries no id worth reading
    /// and nobody is on their way to it.
    TableOpenHouseWaiting {
        en: "your table is open — arrange the chairs and start",
        de: "dein Tisch ist offen — richte die Plätze ein und starte",
    },
    /// The veil over the moment between a granted seat and the duel.
    TakingYourSeat { en: "taking your seat…", de: "nehme deinen Platz ein…" },
    /// Back to the lobby from a finished game.
    BackToLobby { en: "Back to the lobby", de: "Zurück zur Lobby" },
    /// The other button over a finished game, and the one on a rematch room
    /// in the list — the same press either way, from the two screens the two
    /// players are looking at.
    PlayAgain { en: "Play again", de: "Nochmal spielen" },

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
    /// Asking for a chair at the next table.
    PlayingAgain { en: "playing again…", de: "spiele nochmal…" },
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
    /// The same offline, where the rest of the chairs are the house.
    TableOpenHouse {
        en: "table open — arrange the chairs and start",
        de: "Tisch offen — richte die Plätze ein und starte",
    },
    /// Somebody joined it.
    OpponentSatDown { en: "an opponent sat down", de: "ein Gegner hat sich gesetzt" },
    /// A chair at somebody else's room, which does not start on sitting down.
    ///
    /// A room takes two statements by two people — this seat's own `ready`
    /// and the host's `start` — so the seat is held and the game is not
    /// running yet. Said in the second person because the next move is the
    /// player's.
    YouAreSeated {
        en: "you are seated — say you are ready, then the host starts",
        de: "du sitzt — geh auf Bereit, dann startet der Host",
    },
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
    /// The button that writes a preset into the rail.
    UsePreset { en: "use", de: "nutzen" },
    /// Stop everywhere
    RailPresetEveryStep { en: "Stop everywhere", de: "Überall anhalten" },
    /// Ask me in every step of both turns.
    RailPresetEveryStepDetail {
        en: "Ask me in every step of both turns.",
        de: "Frag mich in jedem Schritt beider Züge.",
    },
    /// Skip the quiet steps
    RailPresetQuietSteps { en: "Skip the quiet steps", de: "Stille Schritte überspringen" },
    /// Not untap, upkeep, draw, damage or cleanup — nothing is decided there.
    RailPresetQuietStepsDetail {
        en: "Not untap, upkeep, draw, damage or cleanup — nothing is decided there.",
        de: "Nicht Enttappen, Versorgung, Ziehen, Schaden oder Aufräumen — dort wird nichts entschieden.",
    },
    /// Competitive stops
    RailPresetCompetitive { en: "Competitive stops", de: "Turnier-Stopps" },
    /// Both your main phases, both combat declarations, and their end step.
    RailPresetCompetitiveDetail {
        en: "Both your main phases, both combat declarations, and their end step.",
        de: "Beide eigenen Hauptphasen, beide Kampfansagen und ihr Endsegment.",
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
    /// Heading over the day/night picker.
    Sky { en: "Sky", de: "Himmel" },
    /// What the sky is for, in one line.
    SkyWhy {
        en: "Weather behind the table. It changes nothing in the game.",
        de: "Wetter hinter dem Tisch. Am Spiel ändert es nichts.",
    },
    /// Follow the player's own clock.
    SkyAuto { en: "Auto", de: "Automatisch" },
    /// Always a clouded blue sky.
    SkyDay { en: "Day", de: "Tag" },
    /// Always stars and a crescent moon.
    SkyNight { en: "Night", de: "Nacht" },
    /// Heading over the weather picker.
    Atmosphere { en: "Atmosphere", de: "Atmosphäre" },
    /// What the weather is for, in one line.
    AtmosphereWhy {
        en: "Aurora and drifting stars in the night sky.",
        de: "Polarlichter und wandernde Sterne am Nachthimmel.",
    },
    /// No weather at all.
    AtmosphereOff { en: "Off", de: "Aus" },
    /// Half the budget: visible when looked for, invisible when played under.
    AtmosphereSoft { en: "Soft", de: "Zart" },
    /// The whole budget.
    AtmosphereFull { en: "Full", de: "Voll" },
    /// Heading over the loudness picker.
    Sound { en: "Sound", de: "Ton" },
    /// What the table says out loud, in one line.
    SoundWhy {
        en: "Short tones for life, your turn, and the end of a game.",
        de: "Kurze Töne für Leben, deinen Zug und das Spielende.",
    },
    /// Everything, at the levels the sink was balanced at.
    SoundFull { en: "On", de: "An" },
    /// The same balance, half the amplitude.
    SoundHalf { en: "Quieter", de: "Leiser" },
    /// Nothing at all.
    SoundOff { en: "Off", de: "Aus" },
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
    /// Read card text instead of art
    ActToggleTextView { en: "Read card text instead of art", de: "Kartentext statt Bild lesen" },
    /// Open the zone browser
    ActToggleBrowser { en: "Open the zone browser", de: "Zonenbrowser öffnen" },
    /// Let the stack resolve
    ActHoldForStack { en: "Let the stack resolve", de: "Stack auflösen lassen" },
    /// Nothing more this turn
    ActHoldForTurn { en: "Nothing more this turn", de: "Nichts mehr in diesem Zug" },
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
    /// Other printings could not be loaded. Showing the reference printing.
    NoCatalogOnlyThis {
        en: "Other printings could not be loaded. Showing the reference printing.",
        de: "Weitere Drucke konnten nicht geladen werden. Referenzdruck wird angezeigt.",
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
    /// {0} can no longer be a commander — this deck will save without one.
    CommanderNoLongerEligible {
        en: "{0} can no longer be a commander — this deck will save without one.",
        de: "{0} kann kein Kommandeur mehr sein — dieses Deck wird ohne einen gespeichert.",
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
    /// {0} card is not fully implemented yet and will not play as printed.
    ShakyCard {
        en: "{0} card is not fully implemented yet and will not play as printed.",
        de: "{0} Karte ist noch nicht vollständig umgesetzt und spielt nicht wie gedruckt.",
    },
    /// {0} cards are not fully implemented yet and will not play as printed.
    ShakyCards {
        en: "{0} cards are not fully implemented yet and will not play as printed.",
        de: "{0} Karten sind noch nicht vollständig umgesetzt und spielen nicht wie gedruckt.",
    },


    // ---- the table
    /// Caption over the small card beside a copy's preview.
    ///
    /// The preview draws what the permanent *is* — a copy is a Llanowar
    /// Elves, mark and all — and the little card beside it is the cardboard
    /// actually lying there. One word, because the picture says the rest and
    /// the caption is no wider than the card it labels.
    CardUnderneath { en: "UNDERNEATH", de: "DARUNTER" },
    /// Waiting for {0}
    ///
    /// `{0}` is a seat's *name* and falls back to [`Phrase::SeatNumbered`],
    /// so the old wording ("Waiting for seat 1") is what a table with no
    /// roster still says — and a table that has one says who.
    ///
    /// Not [`Phrase::WaitingFor`], which is the same fact reported *about*
    /// somebody ("wartet auf …") inside a stack entry or a lobby row. This
    /// is the prompt bar speaking in its own voice, and German conjugates
    /// the two differently.
    WaitingForPlayer { en: "Waiting for {0}", de: "Warte auf {0}" },
    /// Waiting for the house — {0} is away
    ///
    /// The same fact as [`Phrase::WaitingForPlayer`] about a chair the house
    /// is holding. Since #81 the client *draws* a held chair — a dashed rim
    /// on its mat — and had no word for one anywhere, so the one place a
    /// player asks the question ("why is nobody answering?") was answered
    /// with a name and nothing else.
    ///
    /// The indicative is right here and is not the tense
    /// [`Phrase::LinkStandIn`] uses, which is the distinction worth keeping:
    /// `SeatIdentity::away` is `SeatKind::StandIn`, so the roster is telling
    /// this client that the house **is** answering. The banner guesses about
    /// its own seat and may not say so; this reports another seat and may.
    WaitingForHeldSeat {
        en: "Waiting for the house — {0} is away",
        de: "Warte auf das Haus — {0} ist abwesend",
    },
    /// You owe mana. Tap lands to pay, or pass.
    ///
    /// The sentence a payment window had none of. A CR 605.3a window is an
    /// ordinary priority round offering mana abilities and nothing else, so
    /// without this it reads "Your move" over a board with nothing to play —
    /// which is the same thing the house agent saw before `PlayerView::owed`
    /// existed, and it passed and lost its spell.
    ///
    /// It says what the window *is* and not what is owed: the amount is drawn
    /// as pips beside the mana pool, where the mana that answers it is also
    /// drawn, and a number said twice in two registers is a number two things
    /// have to keep in step. It also stops short of what declining costs —
    /// countered, or a tax unpaid — because that is the engine's sentence and
    /// this client does not know which it is.
    PayOrPass {
        en: "You owe mana. Tap lands to pay, or pass.",
        de: "Du schuldest Mana. Tippe Länder zum Bezahlen, oder passe.",
    },
    /// Owed
    ///
    /// The word the mana pool's row opens its second half with, while a
    /// CR 605.3a window is open. One word and then the pips, beside the pips
    /// of what is floating, so "owe {2}{G}" and "have {G}" are one glance in
    /// one register — which is the argument for putting it here rather than
    /// in the shelf's middle column beside the sentence.
    Owed { en: "Owed", de: "Geschuldet" },
    /// Waiting
    JustWaiting { en: "Waiting", de: "Warte" },
    /// Keep this hand? (the next mulligan is free)
    MulliganFree {
        en: "Keep this hand? (the next mulligan is free)",
        de: "Diese Hand behalten? (der nächste Mulligan ist frei)",
    },
    /// Keep this hand? ({0} taken)
    MulliganTaken { en: "Keep this hand? ({0} taken)", de: "Diese Hand behalten? ({0} genommen)" },
    /// Put {0} card on the bottom
    PutCardOnBottom { en: "Put {0} card on the bottom", de: "Lege {0} Karte nach unten" },
    /// Put {0} cards on the bottom
    PutOnBottom { en: "Put {0} cards on the bottom", de: "Lege {0} Karten nach unten" },
    /// Declare attackers
    DeclareAttackers { en: "Declare attackers", de: "Angreifer deklarieren" },
    /// Declare blockers
    DeclareBlockers { en: "Declare blockers", de: "Blocker deklarieren" },
    /// Discard {0} card
    DiscardCard { en: "Discard {0} card", de: "Wirf {0} Karte ab" },
    /// Discard {0} cards
    DiscardCards { en: "Discard {0} cards", de: "Wirf {0} Karten ab" },
    /// Legend rule: keep one
    LegendRule { en: "Legend rule: keep one", de: "Legendenregel: behalte eine" },
    /// card
    NounCard { en: "card", de: "Karte" },
    /// cards
    NounCards { en: "cards", de: "Karten" },
    /// target
    NounTarget { en: "target", de: "Ziel" },
    /// targets
    NounTargets { en: "targets", de: "Ziele" },
    /// card from your library
    NounCardFromLibrary { en: "card from your library", de: "Karte aus deiner Bibliothek" },
    /// cards from your library
    NounCardsFromLibrary { en: "cards from your library", de: "Karten aus deiner Bibliothek" },
    /// card to put on the bottom
    NounCardToBottom { en: "card to put on the bottom", de: "Karte, die nach unten geht" },
    /// cards to put on the bottom
    NounCardsToBottom { en: "cards to put on the bottom", de: "Karten, die nach unten gehen" },
    /// card to put into your graveyard
    NounCardToGraveyard {
        en: "card to put into your graveyard",
        de: "Karte, die auf deinen Friedhof geht",
    },
    /// cards to put into your graveyard
    NounCardsToGraveyard {
        en: "cards to put into your graveyard",
        de: "Karten, die auf deinen Friedhof gehen",
    },
    /// card to put on top of your library
    NounCardToTop {
        en: "card to put on top of your library",
        de: "Karte, die oben auf deine Bibliothek kommt",
    },
    /// cards to put on top of your library
    NounCardsToTop {
        en: "cards to put on top of your library",
        de: "Karten, die oben auf deine Bibliothek kommen",
    },
    /// card from outside the game
    NounCardOutside { en: "card from outside the game", de: "Karte von außerhalb der Partie" },
    /// cards from outside the game
    NounCardsOutside { en: "cards from outside the game", de: "Karten von außerhalb der Partie" },
    /// permanent to sacrifice
    NounPermanentToSacrifice {
        en: "permanent to sacrifice",
        de: "bleibende Karte, die geopfert wird",
    },
    /// permanents to sacrifice
    NounPermanentsToSacrifice {
        en: "permanents to sacrifice",
        de: "bleibende Karten, die geopfert werden",
    },
    /// card to discard
    NounCardToDiscard { en: "card to discard", de: "Karte, die abgeworfen wird" },
    /// cards to discard
    NounCardsToDiscard { en: "cards to discard", de: "Karten, die abgeworfen werden" },
    /// untapped permanent to tap
    NounPermanentToTap {
        en: "untapped permanent to tap",
        de: "ungetappte bleibende Karte, die getappt wird",
    },
    /// untapped permanents to tap
    NounPermanentsToTap {
        en: "untapped permanents to tap",
        de: "ungetappte bleibende Karten, die getappt werden",
    },
    /// permanent to return to its owner's hand
    NounPermanentToReturn {
        en: "permanent to return to its owner's hand",
        de: "bleibende Karte, die auf die Hand ihres Besitzers zurückgenommen wird",
    },
    /// permanents to return to their owners' hands
    NounPermanentsToReturn {
        en: "permanents to return to their owners' hands",
        de: "bleibende Karten, die auf die Hand ihres Besitzers zurückgenommen werden",
    },
    /// permanent to leave tapped
    NounPermanentToLeaveTapped {
        en: "permanent to leave tapped",
        de: "bleibende Karte, die getappt bleibt",
    },
    /// permanents to leave tapped
    NounPermanentsToLeaveTapped {
        en: "permanents to leave tapped",
        de: "bleibende Karten, die getappt bleiben",
    },
    /// Convoke: tap creatures or artifacts to help pay
    ConvokeToHelpPay { en: "Tap creatures or artifacts to help pay — each pays for one", de: "Tippe Kreaturen oder Artefakte an, um mitzubezahlen — jedes zahlt eins" },
    /// Delve: exile cards from your graveyard to help pay
    DelveToHelpPay { en: "Exile cards from your graveyard to help pay — each pays for one", de: "Schicke Karten aus deinem Friedhof ins Exil, um mitzubezahlen — jede zahlt eine" },
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
    /// Printed cost
    CastNormal { en: "Printed cost", de: "Gedruckte Kosten" },
    /// Alternative cost
    CastAlternative { en: "Alternative cost", de: "Alternative Kosten" },
    /// Mode {0}
    CastModeNumber { en: "Mode {0}", de: "Modus {0}" },
    /// Back face
    CastBackFace { en: "Back face", de: "Rückseite" },
    /// Play as a land
    CastLandFace { en: "Play as a land", de: "Als Land spielen" },
    /// Miracle
    CastMiracle { en: "Miracle", de: "Wunder" },
    /// Click a card in your hand
    HintClickHand { en: "Click a card in your hand", de: "Klicke eine Karte auf deiner Hand an" },
    /// Click what you are choosing
    HintClickBoard { en: "Click what you are choosing", de: "Klicke an, was du wählst" },
    /// Type to narrow the list
    HintTypeToFilter { en: "Type to narrow the list", de: "Tippe, um die Liste einzugrenzen" },
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
    /// {0} offers a draw. Accept?
    ///
    /// Named rather than passive, and the name is the whole of the repair:
    /// at a duel "a draw was offered" is obvious, and at a table of four it
    /// is a question nobody can answer.
    DrawOfferedBy {
        en: "{0} offers a draw. Accept?",
        de: "{0} bietet ein Remis an. Annehmen?",
    },
    /// Put your commander into the command zone?
    CommanderToCommandZone {
        en: "Put your commander into the command zone?",
        de: "Deinen Kommandeur in die Kommandozone legen?",
    },
    /// Command zone instead of your hand? (CR 903.9b)
    CommanderInsteadOfHand {
        en: "Your commander would go to your hand. Command zone instead?",
        de: "Dein Kommandeur käme auf die Hand. Stattdessen in die Kommandozone?",
    },
    /// Command zone instead of your library? (CR 903.9b)
    CommanderInsteadOfLibrary {
        en: "Your commander would go into your library. Command zone instead?",
        de: "Dein Kommandeur käme in die Bibliothek. Stattdessen in die Kommandozone?",
    },
    /// Use the optional part of this ability? ("you may …")
    UseTheOptionalAbility {
        en: "This ability is optional. Use it?",
        de: "Diese Fähigkeit ist optional. Einsetzen?",
    },
    /// Yes or no?
    YesOrNo { en: "Yes or no?", de: "Ja oder nein?" },
    /// Offer a draw
    OfferADraw { en: "Offer a draw", de: "Remis anbieten" },
    /// Concede
    Concede { en: "Concede", de: "Aufgeben" },
    /// Concede, armed and waiting for the second press.
    ConcedeConfirm { en: "Concede? Press again", de: "Aufgeben? Nochmal drücken" },
    /// The indicator that says this seat is not being asked right now.
    HoldingPriority { en: "Not asking you", de: "Du wirst nicht gefragt" },
    /// The button that cancels a running hold.
    HoldRelease { en: "Ask me again", de: "Wieder fragen" },
    /// The armed button for a spell or a land: pressing it sends the card.
    ArmedPlay { en: "Play this card", de: "Diese Karte spielen" },
    /// The armed button for a spell whose mana still has to be tapped.
    ///
    /// `{0}` is the *price* — the spell's mana cost, tax and all — and it is
    /// one of the two placeholders here never filled with a string (the other
    /// is [`Self::ArmedSuspend`], which prices the same way): the row
    /// splits the phrase at it and draws the cost as mana pips, so a
    /// translation is free to put the price first (as the German does) and
    /// still gets the discs in the right place. It used to read "Tap 3",
    /// which counted the client's own lands instead of naming what the
    /// spell costs.
    ArmedPayAndCast { en: "Pay {0} and cast", de: "{0} zahlen und zaubern" },
    /// The armed button for suspending a card whose cost still has to be tapped.
    ///
    /// `{0}` is the suspend cost, drawn as pips like [`Self::ArmedPayAndCast`]
    /// — and it is the *suspend* cost, which is a different number from the
    /// card's own: Ancestral Vision prints no mana cost and suspends for
    /// `{U}`.
    ArmedSuspend { en: "Pay {0} and suspend", de: "{0} zahlen und aussetzen" },
    /// The armed button for suspending when the cost is already floating.
    ArmedSuspendNow { en: "Suspend this card", de: "Diese Karte aussetzen" },
    /// The button that puts an armed deed back, with nothing sent.
    ArmedCancel { en: "Not yet", de: "Doch nicht" },
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
    /// Pass
    PassPriority { en: "Pass", de: "Passen" },
    /// Skip the rest of the turn
    SkipTheTurn { en: "Skip turn", de: "Zug überspringen" },
    /// Hold priority until the stack is empty — the ledge's own button.
    ///
    /// [`Phrase::ActHoldForStack`] is the same hold *described* in the
    /// keyboard menu, where a line has room to say what a key does. This is
    /// the label on a 28-pixel button, so it says what pressing it does and
    /// nothing else.
    ResolveTheStack { en: "Resolve the stack", de: "Stack abarbeiten" },
    /// Day
    DesignationDay { en: "Day", de: "Tag" },
    /// Night
    DesignationNight { en: "Night", de: "Nacht" },
    /// In order
    SortByPlace { en: "In order", de: "Nach Lage" },
    /// By name
    SortByName { en: "By name", de: "Nach Name" },
    /// By cost
    SortByCost { en: "By cost", de: "Nach Kosten" },
    /// By type
    SortByType { en: "By type", de: "Nach Typ" },
    /// Token
    IsToken { en: "Token", de: "Spielstein" },
    /// Your move
    YourMove { en: "Your move", de: "Du bist dran" },
    /// You may respond
    ///
    /// The same priority window, on somebody else's turn. "Your move" there
    /// reads as "it is your turn", which it is not.
    YouMayRespond { en: "You may respond", de: "Du kannst reagieren" },
    /// Stack
    StackTitle { en: "Stack", de: "Stapel" },
    /// Spell
    StackSpell { en: "Spell", de: "Zauber" },
    /// Ability
    StackAbilityBare { en: "Ability", de: "Fähigkeit" },
    /// Ability · {0}
    StackAbility { en: "Ability · {0}", de: "Fähigkeit · {0}" },
    /// +{0} more
    StackMore { en: "+{0} more", de: "+{0} weitere" },
    /// Seat {0}
    SeatNumbered { en: "Seat {0}", de: "Platz {0}" },
    /// You ({0})
    YouNamed { en: "You ({0})", de: "Du ({0})" },
    /// You
    You { en: "You", de: "Du" },
    /// Aimed at {0} ({1} of {2})
    AimedAt { en: "Aimed at {0} ({1} of {2})", de: "Zielt auf {0} ({1} von {2})" },
    /// {0} declared
    DeclaredCount { en: "{0} declared", de: "{0} deklariert" },
    /// {0}: {1} attacking, {2} unblocked
    IncomingAt {
        en: "{0}: {1} attacking, {2} unblocked",
        de: "{0}: {1} greifen an, {2} ungeblockt",
    },
    /// you
    IncomingYou { en: "you", de: "dich" },
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

    // ---- the connection to the table
    /// Connection lost — reconnecting…
    ///
    /// Said for the first [`crate::reconnect::Retry::PATIENCE`] seconds of a
    /// drop — or for the whole of it, at a table that hands no chair over and
    /// at one this client was never told the window of.
    /// [`crate::reconnect::Window`] is which, and it is the only sentence
    /// that is true at all three.
    ///
    /// The wording is deliberately not "the game is over": the table is still
    /// there and the seat is resumed from where it left off. What it used to
    /// give as the reason was that "the engine's decision clock does not run
    /// for a seat with no socket" — true, and the wrong clock. The
    /// **reconnect** clock is running the whole time this is on screen, and
    /// when it expires the house takes the chair. That is why this sentence
    /// is the short one now and does not stand alone for two minutes.
    LinkLost {
        en: "Connection lost — reconnecting…",
        de: "Verbindung verloren — verbinde neu …",
    },
    /// Still reconnecting — the house will answer for your seat until you are back.
    ///
    /// The second form, once a drop has outlived
    /// [`crate::reconnect::Retry::PATIENCE`]. It says the consequence the
    /// first one leaves out: `Session::stand_in` gives the chair to the
    /// house after `HouseRules::reconnect_window_secs`, and `hand_back`
    /// returns it the moment the player is attached again.
    ///
    /// **The tense is still the finding, and half its reason has gone.** "The
    /// house *is* playing your seat" is the sentence this obviously wants and
    /// is the one thing that cannot be written: a client is disconnected for
    /// exactly the span it would have to count, so the moment of the handover
    /// is not observable from here however much it knows. That part has not
    /// changed. What has is that the window itself **is** known now —
    /// `GameStatic::reconnect_secs` since `VIEW_VERSION` 26 — so this is no
    /// longer shown at a table where the handover is not coming at all, and
    /// no longer shown late at one whose window is shorter than the cap.
    /// "Will answer" is true whenever it is on screen, which is what
    /// [`crate::reconnect::Retry::brief`] is for.
    ///
    /// The German says it in the present, which is not a drift: German
    /// present carries the near future, and "wird … antworten" reads as a
    /// prediction where the English "will" reads as a rule.
    LinkStandIn {
        en: "Still reconnecting — the house will answer for your seat until you are back.",
        de: "Verbinde weiter — das Haus übernimmt deinen Platz, bis du zurück bist.",
    },
    /// The table cannot be reached. Rejoin from the lobby.
    ///
    /// After the retry schedule runs out. "That game no longer exists" reaches
    /// a client as a refusal string rather than as a state it can match on, so
    /// this is what a client says when it cannot tell the two apart.
    LinkGaveUp {
        en: "The table cannot be reached. Rejoin from the lobby.",
        de: "Der Tisch ist nicht erreichbar. Tritt aus der Lobby erneut bei.",
    },


    /// A card has no candidate satisfying its required targets (#112).
    CardHasNoTarget {
        en: "Nothing left for this card to target",
        de: "Für diese Karte gibt es kein gültiges Ziel mehr",
    },
    /// Timing prevents playing a card (#112).
    CardWrongTime {
        en: "This card cannot be played in this window",
        de: "Diese Karte lässt sich in diesem Zeitfenster nicht spielen",
    },
    /// The current resources do not cover the card's costs (#112).
    CardCostsUnavailable {
        en: "The resources to pay for this card are not available",
        de: "Die Mittel zum Bezahlen dieser Karte fehlen",
    },
    // ---- refusals this client owns
    //
    // Everything here is a sentence the *client* decided, not one an engine
    // sent, and that is the whole reason it can be a phrase at all. They are
    // drawn in the one slot `Refusal` feeds, beside refusals that arrived as
    // prose — see [`Refusal`] for why that slot takes both.
    /// The engine no longer offers that.
    ///
    /// An armed deed re-checked against the current `LegalActions` and gone:
    /// the question moved on between the tap that armed it and the tap that
    /// would have sent it. Deliberately not an apology and not a diagnosis —
    /// the player's next action is to look at what is offered now.
    DeedWithdrawn {
        en: "The engine no longer offers that",
        de: "Die Engine bietet das nicht mehr an",
    },
    /// The engine no longer offers that way of casting it.
    ///
    /// The same, one level in: the card is still castable and the *mode*
    /// picked out of `ChooseCastMode` is not, which is a different sentence
    /// because the card is still worth looking at.
    CastModeWithdrawn {
        en: "The engine no longer offers that way of casting it",
        de: "Die Engine bietet diese Art, sie zu wirken, nicht mehr an",
    },
    /// A land the plan counted on can no longer be tapped.
    ///
    /// One of six a mana run gives up with. They are six sentences and not
    /// one, because each names a different thing to do next, which is the
    /// only justification a refusal has for existing: a land that went away
    /// means tap something else, a spell that is no longer castable means
    /// the mana is floating and the turn is not lost.
    PlanLandGone {
        en: "A land the plan counted on can no longer be tapped",
        de: "Ein Land, mit dem der Plan gerechnet hat, lässt sich nicht mehr tappen",
    },
    /// That source cannot make the colour the plan wanted.
    PlanColourGone {
        en: "That source cannot make the colour the plan wanted",
        de: "Diese Quelle kann die Farbe nicht erzeugen, die der Plan wollte",
    },
    /// The mana is up but the spell is not castable.
    PlanSpellRefused {
        en: "The mana is up but the spell is not castable",
        de: "Das Mana steht bereit, aber der Zauberspruch lässt sich nicht wirken",
    },
    /// The mana is up but the card cannot be suspended.
    PlanSuspendRefused {
        en: "The mana is up but the card cannot be suspended",
        de: "Das Mana steht bereit, aber die Karte lässt sich nicht aussetzen",
    },
    /// The run lost the card it was paying for.
    PlanCardGone {
        en: "The run lost the card it was paying for",
        de: "Der Ablauf hat die Karte verloren, für die er bezahlt hat",
    },
    /// The game asked something else.
    ///
    /// A mana ability does not use the stack, so priority never leaves the
    /// seat in the middle of a plan (CR 605.3a); a different question means
    /// the game moved on without this client and the plan is void.
    PlanQuestionChanged {
        en: "The game asked something else",
        de: "Das Spiel hat etwas anderes gefragt",
    },


    // ---- the mana pool
    /// Mana pool
    ManaPool { en: "Mana pool", de: "Manavorrat" },
    /// Tap for {0}, spendable only on some spells
    ///
    /// Cavern of Souls and its kin. The label has to say *both* halves: a
    /// button reading only "Tap for WUBRG" beside another reading "Tap for
    /// {C}" is two offers a player cannot tell apart, and the restricted one
    /// is usually the reason the land is in the deck.
    TapForRestricted { en: "Tap for {0} (restricted)", de: "Tappen für {0} (eingeschränkt)" },
    /// Tap for X mana ({0})
    ///
    /// Harabaz Druid: *"add X mana of any one color, where X is the number of
    /// Allies you control"*. The `X` is the card's own letter and is left
    /// standing, because what it comes to is the engine's arithmetic and no
    /// client counts it — what the row can say honestly is that the amount is
    /// a count, and which colours it is a count of. Without it the ability
    /// fell back to its cost and drew a row reading "{T}" beside a cost
    /// column reading "{T}".
    TapForVariable { en: "Tap for X mana ({0})", de: "Tappen für X Mana ({0})" },


    // ---- the ability sheet
    /// to confirm
    ///
    /// The footer while a row is armed, beside a keycap carrying that row's
    /// digit — the key that armed it is the key that sends it, and there is
    /// no second confirm key to learn. The digit used to be `{0}` inside this
    /// sentence; it is drawn as the key it is now, so the phrase is the word
    /// that goes with the cap and nothing else.
    ///
    /// It read "again", which is true of the gesture and silent about the
    /// consequence. This half of the footer is the only place the sheet says
    /// that the next press *sends* something, so it says that instead.
    SheetPressAgain { en: "to confirm", de: "zum Bestätigen" },
    /// A digit picks
    ///
    /// The same footer with nothing armed, and the one half that carries no
    /// cap: the sentence is about *any* digit, and a key drawn there would
    /// name the first row rather than the gesture. It names the gesture
    /// rather than a range, because how many rows there are is on the sheet
    /// already.
    SheetDigitPicks { en: "A digit picks", de: "Eine Ziffer wählen" },
    /// close
    ///
    /// The other end of the footer, beside the cap for the key itself. The
    /// key's own name is never translated and never spelled here: it comes
    /// from `prefs::Chord::display`, the same reader the settings screen
    /// draws every binding with.
    SheetCloses { en: "close", de: "schließen" },
    /// More — page {0} of {1}
    ///
    /// The tenth row, which exists only for a permanent with more than nine
    /// things to do: a land under a Chromatic Lantern is granted a mana
    /// ability on top of whatever it prints. It carries `0` rather than a
    /// digit, being the one key on that row of the keyboard that names no
    /// ability.
    SheetMorePage { en: "More — page {0} of {1}", de: "Weitere — Seite {0} von {1}" },

    // ---- what an ability costs
    /// Tap for {0}
    TapFor { en: "Tap for {0}", de: "Tappen für {0}" },
    /// any color
    ///
    /// What the *cards* say. Five discs in a row is not "any colour" — it is
    /// five claims a player has to add up — and a land that makes all five is
    /// making one offer, not five.
    AnyColor { en: "any color", de: "beliebige Farbe" },
    /// {0} or {1}
    ///
    /// The tail of a list of colours a source may choose between. Two or
    /// three symbols read at a glance where five do not, which is why this
    /// exists beside [`Phrase::AnyColor`] rather than instead of it.
    OrLast { en: "{0} or {1}", de: "{0} oder {1}" },
    /// Ability {0}
    AbilityNumbered { en: "Ability {0}", de: "Fähigkeit {0}" },
    /// Granted ability
    ///
    /// An ability another permanent handed this one. It has no position on
    /// the card to be numbered by, so "Ability 1" would be a lie about a
    /// printed ability that is also there.
    GrantedAbility { en: "Granted ability", de: "Verliehene Fähigkeit" },
    /// Cast the prepared spell
    ///
    /// Prepared (Emeritus of Woe) is offered under the other synthetic index,
    /// and it is a cast rather than an ability — so it is neither numbered
    /// nor ever a mana ability.
    PreparedCast { en: "Cast the prepared spell", de: "Vorbereiteten Zauber wirken" },
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
    /// Return another
    ///
    /// Not [`Self::CostReturnThis`], for [`Self::CostTapAnother`]'s reason
    /// one phrase down: Quirion Ranger returns a Forest and Recurring
    /// Nightmare returns itself, and a player told "Return this" over the
    /// Ranger would read its own death into a cost that only bounces a land.
    CostReturnAnother { en: "Return another", de: "Nimm eine andere zurück" },
    /// Exile a card
    CostExileACard { en: "Exile a card", de: "Schicke eine Karte ins Exil" },
    /// Tap another
    ///
    /// Not "{T}", which is the source tapping itself and is the symbol the
    /// card prints beside this one — Earthcraft's neighbours all read
    /// "{T}, Tap an untapped creature you control".
    CostTapAnother { en: "Tap another", de: "Tappe eine andere" },
    /// Remove a counter
    ///
    /// Which counter is deliberately not named. Nothing on this side of the
    /// wire knows a counter's *name* — the plate draws kinds as coloured
    /// chips and has no word for any of them — so spelling one here would
    /// mean inventing a second naming table for eleven kinds to serve the
    /// one land that needs it. The card's own printed line says "a charge
    /// counter" a few millimetres away, and no permanent in this pool pays
    /// with one kind while carrying another.
    CostRemoveCounter { en: "Remove a counter", de: "Entferne eine Marke" },
    /// Remove {0} counters
    CostRemoveCounters { en: "Remove {0} counters", de: "Entferne {0} Marken" },
    /// The storage lands' cost, whose number the player picks on activation
    /// — so the button says what is about to be asked rather than a count
    /// nobody has chosen yet. "Any number" is one of the two printed
    /// spellings and the friendlier one; the other is "Remove X".
    CostRemoveCountersX { en: "Remove any number of counters", de: "Entferne beliebig viele Marken" },
    /// Put a counter on this
    CostPutCounter { en: "Put a counter on this", de: "Lege eine Marke darauf" },
    /// Put {0} counters on this
    CostPutCounters { en: "Put {0} counters on this", de: "Lege {0} Marken darauf" },
    /// Rules text unavailable
    NoRulesTextHere { en: "Rules text unavailable", de: "Regeltext nicht verfügbar" },

    /// Taking your seat
    VeilTakingSeat { en: "Taking your seat", de: "Nehme deinen Platz ein" },
    /// Talking to the gateway
    VeilTalking {
        en: "Talking to the gateway",
        de: "Spreche mit dem Gateway",
    },
    /// The same veil offline, where there is nobody to talk to.
    VeilWorking { en: "One moment", de: "Einen Moment" },
    /// Check your e-mail — a confirmation link is on its way.
    ConfirmYourEmail {
        en: "account created — check your e-mail for the confirmation link",
        de: "Konto erstellt — prüfe deine E-Mail auf den Bestätigungslink",
    },
    /// A chair that plays for nobody but itself.
    SeatSideNone {
        en: "no team",
        de: "kein Team",
    },
    /// A chair's team, by number.
    SeatSide {
        en: "team {0}",
        de: "Team {0}",
    },
    /// The game is over and this seat won it.
    YouWon {
        en: "You won",
        de: "Du hast gewonnen",
    },
    /// The game is over and this seat did not.
    YouLost {
        en: "You lost",
        de: "Du hast verloren",
    },
    /// The game is over and this seat's team won it.
    YourTeamWon {
        en: "Team {0} wins — yours",
        de: "Team {0} gewinnt — deins",
    },
    /// The game is over and another team won it.
    TheirTeamWon {
        en: "Team {0} wins",
        de: "Team {0} gewinnt",
    },
    /// Nobody won.
    TheGameIsADraw {
        en: "The game is a draw",
        de: "Das Spiel endet unentschieden",
    },
    /// Why a game ended: one seat outlived every other
    /// (`EndReason::LastPlayerStanding`).
    EndedLastPlayer {
        en: "Only one player left in the game",
        de: "Nur noch ein Spieler im Spiel",
    },
    /// Why a game ended: one team outlived every other
    /// (`EndReason::LastTeamStanding`).
    EndedLastTeam {
        en: "Only one team left in the game",
        de: "Nur noch ein Team im Spiel",
    },
    /// Why a game ended: a card or an emblem declared a winner
    /// (`EndReason::EffectWin`).
    EndedByEffect {
        en: "An effect decided it",
        de: "Ein Effekt hat entschieden",
    },
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

    // ---- the zone browser
    /// Cards the engine is showing this seat — a search, a scry, a reveal.
    BrowseLooking { en: "Revealed", de: "Aufgedeckt" },
    /// A graveyard.
    BrowseGraveyard { en: "Graveyard", de: "Friedhof" },
    /// A public exile pile.
    BrowseExile { en: "Exile", de: "Exil" },
    /// A command zone.
    BrowseCommand { en: "Command", de: "Kommandozone" },
    /// The tab that shows every zone at once.
    BrowseAll { en: "All", de: "Alle" },
    /// A zone with nothing in it, or a filter that matched nothing.
    BrowseEmpty { en: "nothing here", de: "nichts hier" },
    /// The search box above the list.
    ///
    /// It said "by name" until the box learned the query language, which was
    /// a placeholder telling the player the box could do less than it can:
    /// a bare word reaches the type line too, and `t:creature pow>=4` works
    /// here. What it names is what a *bare* word looks at, because that is
    /// the reading a player gets without being taught anything.
    BrowseFilter { en: "Name, type…", de: "Name, Typ …" },
    /// How much of the answer is assembled, when the question takes a range.
    /// `{0}` is how many are chosen, `{1}` the most it will take.
    BrowseTallyUpTo { en: "{0} of up to {1} chosen", de: "{0} von bis zu {1} gewählt" },
    /// The same, for a question that names one number. `{0}` of `{1}`.
    BrowseTallyExact { en: "{0} of {1} chosen", de: "{0} von {1} gewählt" },
    /// Sends the answer the dialog has assembled.
    BrowseConfirm { en: "Confirm", de: "Bestätigen" },

    // ------------------------------------------- the filter string builder
    //
    // The words the gear opens. Keys are named in the *player's* language and
    // never with `Key::render`'s letter: `t` is the string's spelling and the
    // dialog exists so that nobody has to know it. What is deliberately not
    // here is a name for `Key::Unknown` or for a branch the controls cannot
    // draw — both show what was typed, and translating a player's own words
    // back at them is the one thing a dialog must not do.

    /// The gear inside a search box, and the heading over what it opens.
    FilterBuild { en: "Set up the filter", de: "Filter einstellen" },
    /// The row at the foot of the list that adds another condition.
    FilterAdd { en: "Add a condition…", de: "Bedingung hinzufügen …" },
    /// Closes the builder. The string is already in the box, so there is
    /// nothing for this to confirm.
    FilterDone { en: "Done", de: "Fertig" },
    /// Empties the builder, and with it the box.
    FilterClear { en: "Clear", de: "Leeren" },
    /// Beside a branch the controls cannot take apart.
    FilterAsTyped { en: "as typed", de: "wie getippt" },
    /// One line under the rows when any of them asks something this list
    /// cannot answer. One line and not one per row: it is the same fact.
    FilterUnanswerable {
        en: "A condition cannot be checked here — the list stays empty while it stands.",
        de: "Eine Bedingung kann hier nicht geprüft werden — die Liste bleibt leer, solange sie steht.",
    },
    /// Chips are narrowing this list as well: {0}
    FilterAlsoChips {
        en: "Chips are narrowing this list as well: {0}",
        de: "Zusätzlich grenzen Chips diese Liste ein: {0}",
    },
    /// Playable only
    FilterChipPlayable { en: "playable only", de: "nur spielbare" },
    /// mana value {0}
    FilterChipCmc { en: "mana value {0}", de: "Manawert {0}" },
    /// mana value {0} or more
    FilterChipCmcUp { en: "mana value {0} or more", de: "Manawert {0} oder mehr" },
    /// The five kinds of condition, which are the five kinds of control.
    FilterKindText { en: "Text", de: "Text" },
    /// A condition about colours.
    FilterKindColor { en: "Colour", de: "Farbe" },
    /// A condition about a number.
    FilterKindNumber { en: "Number", de: "Zahl" },
    /// A condition about a yes-or-no property.
    FilterKindFlag { en: "Property", de: "Eigenschaft" },
    /// A condition about a mana cost.
    FilterKindCost { en: "Cost", de: "Kosten" },
    /// A bare word, which looks at every line of the card.
    FilterKeyLoose { en: "Anywhere", de: "Überall" },
    /// The card's name.
    FilterKeyName { en: "Name", de: "Name" },
    /// The whole name and nothing else.
    FilterKeyExact { en: "Exact name", de: "Genauer Name" },
    /// What the card says.
    FilterKeyOracle { en: "Rules text", de: "Regeltext" },
    /// The type line.
    FilterKeyType { en: "Type", de: "Typ" },
    /// The card's own colours.
    FilterKeyColor { en: "Colour", de: "Farbe" },
    /// Its colour identity (CR 903.4).
    FilterKeyIdentity { en: "Colour identity", de: "Farbidentität" },
    /// The symbols of its mana cost.
    FilterKeyMana { en: "Mana cost", de: "Manakosten" },
    /// Its mana value.
    FilterKeyManaValue { en: "Mana value", de: "Manawert" },
    /// Printed power.
    FilterKeyPower { en: "Power", de: "Stärke" },
    /// Printed toughness.
    FilterKeyToughness { en: "Toughness", de: "Widerstandskraft" },
    /// Printed starting loyalty.
    FilterKeyLoyalty { en: "Loyalty", de: "Loyalität" },
    /// A colour reading: the card has these and may have others.
    FilterAtLeast { en: "at least", de: "mindestens" },
    /// A colour reading: the card has these and no others.
    FilterExactly { en: "exactly", de: "genau" },
    /// A colour reading: the card has no colour outside these.
    FilterAtMost { en: "at most", de: "höchstens" },
    /// What a colon means on a mana cost.
    FilterContains { en: "contains", de: "enthält" },
    /// Swaps the colour pips for a number of them.
    FilterCount { en: "Count", de: "Anzahl" },
    /// More than one colour, whatever they are.
    FilterMulticolor { en: "multicoloured", de: "mehrfarbig" },
    /// An even mana value.
    FilterEven { en: "even", de: "gerade" },
    /// An odd one.
    FilterOdd { en: "odd", de: "ungerade" },
    /// A card this build can actually play.
    FlagPlayable { en: "playable", de: "spielbar" },
    /// A card that may lead a commander deck.
    ///
    /// Not `IsCommander`, which is the deck builder saying that *this* card
    /// is the one — it carries a tick for that reason and a filter label
    /// must not.
    FlagCommander { en: "commander", de: "Kommandeur" },
    /// A basic land.
    FlagBasic { en: "basic land", de: "Standardland" },
    /// A card with two faces.
    FlagDfc { en: "double-faced", de: "doppelseitig" },
    /// Sends the *empty* answer, which is the only way out a question whose
    /// minimum is zero has — there is no cancel action on the wire.
    ///
    /// It said `Cancel` / `Abbrechen` until somebody read it out loud. The
    /// word was a small lie and the code already knew it: the handler clears
    /// the selection and then **confirms**, so a player who believed they had
    /// backed out of the question had in fact answered it, and the game had
    /// moved on without them. Naming the answer instead of the gesture is the
    /// fix — a button on a question says what it sends.
    BrowseNone { en: "None", de: "Keine" },
    /// How an ordering is answered.
    BrowseOrderHint {
        en: "click them in the order they should go",
        de: "in der gewünschten Reihenfolge anklicken",
    },
    /// A zone belonging to a seat. `{0}` is the zone, `{1}` the seat.
    BrowseZoneOf { en: "{0} · {1}", de: "{0} · {1}" },
    /// A zone tab with how many cards are in it. `{0}` is the zone's name,
    /// `{1}` the count.
    ///
    /// Bracketed on purpose: the sheet's own typography greys what a sentence
    /// says in brackets, and a count is exactly that kind of aside — a fact
    /// the pile itself already shows, riding along beside the name rather
    /// than being part of it. It is also the one thing the deleted pile chips
    /// said that nothing else on the sheet did.
    BrowseTabCount { en: "{0} ({1})", de: "{0} ({1})" },
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

    /// The form of a counted sentence that `n` things ask for.
    ///
    /// Both languages split in the same place — exactly one against anything
    /// else — so the number picks the sentence and the language never has to
    /// be asked. **Zero takes the plural**, which is both languages again:
    /// "Choose up to 0 cards", "Wähle bis zu 0 Karten".
    ///
    /// This exists because the file said `card(s)` and `Karte(n)` out loud in
    /// five places, and the sheet's own typography greys a bracketed aside
    /// ([`crate::prose::bracketed`]) — so a broken plural was drawn as an
    /// editorial remark, in grey, next to the number it disagreed with. The
    /// repair is not a suffix: German wants a relative clause here
    /// ("Karte, die nach unten geht" against "Karten, die nach unten gehen"),
    /// and the verb inside it agrees too. Only a whole second literal can say
    /// that, which is why a counted phrase is written twice rather than
    /// assembled.
    #[must_use]
    pub const fn counted(n: usize, one: Self, many: Self) -> Self {
        if n == 1 { one } else { many }
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

/// What the local seat's own tab is headed.
///
/// [`Phrase::YouNamed`] is "You ({0})" — the pronoun, and then whatever the
/// table calls this seat, because at a room of six "You" alone does not say
/// which chair is yours to anybody reading over your shoulder. The offline
/// `LocalHost` names seat 0 `"You"`, which is the pronoun itself, and the tab
/// then read **"You (You)"**.
///
/// So a name that already *is* this language's pronoun is drawn once. The
/// comparison is against [`Phrase::You`] rather than against a literal, or a
/// German table would go on saying "Du (Du)"; it ignores case, because the
/// name comes from a host and "you" is the same claim.
#[must_use]
pub fn own_seat_name(lang: Lang, name: &str) -> String {
    let pronoun = Phrase::You.text(lang);
    if name.trim().eq_ignore_ascii_case(pronoun) {
        pronoun.to_string()
    } else {
        Phrase::YouNamed.fill(lang, &[name])
    }
}

/// A refusal the prompt bar has to draw, in whichever form it arrived.
///
/// One slot, two kinds of thing, and the pair is the point. A refusal this
/// client decided is a [`Phrase`] and is translated like every other word on
/// the screen; a refusal another process sent is its sentence and is drawn as
/// it came, because the process that said no is the one that knows why. A
/// field typed `String` could only ever hold the second, which is how nine
/// client-owned sentences came to be English in a German interface (#121).
///
/// This is the shape `crate::Duel::link_note` already had for the same
/// reason — *"a phrase rather than a rendered string so the decision stays
/// where a test can read it, and the words stay in the overlay, which is the
/// only thing that knows the language"* — with one arm added for the case
/// `link_note` never has.
///
/// [`Self::Verbatim`] is **not** a deficiency to be driven to zero. It is
/// what keeps the pair forward-compatible: an engine that refuses for a
/// reason this client has never heard of renders its English sentence rather
/// than nothing, so the two sides need no lockstep deploy.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Refusal {
    /// A sentence this client owns.
    Said(Phrase),
    /// A sentence another process sent, in its own words.
    Verbatim(String),
}

impl Refusal {
    /// The sentence, in `lang` where this client has a say in it.
    #[must_use]
    pub fn text(&self, lang: Lang) -> String {
        match self {
            Self::Said(phrase) => phrase.text(lang).to_string(),
            Self::Verbatim(prose) => prose.clone(),
        }
    }
}

/// What a house AI's difficulty is called, from its wire spelling.
///
/// The wire value is an identifier — `"sharp"` is what a `GamePreset` holds
/// and what an HTTP route takes — and the root contract keeps it: *values
/// that are also identifiers keep their wire spelling and translate only the
/// label*. This is the label half, and it had only one caller, so the table
/// went on printing the identifier. A chair arranged in the lobby as
/// "Solide" sat down at the table called `steady 1`.
///
/// Unknown spellings are [`None`] rather than the middle difficulty, because
/// a caller that has a value the client does not know is in a different
/// situation from one that has none — and a wrong difficulty drawn
/// confidently is worse than no difficulty at all.
#[must_use]
pub fn ai_name(lang: Lang, wire: &str) -> Option<&'static str> {
    let phrase = match wire {
        "novice" => Phrase::AiNovice,
        "casual" => Phrase::AiCasual,
        "steady" => Phrase::AiSteady,
        "sharp" => Phrase::AiSharp,
        "expert" => Phrase::AiExpert,
        _ => return None,
    };
    Some(phrase.text(lang))
}

/// What to call a seat inside a sentence.
///
/// Every line that talks *about* another chair needs this, and four of them
/// were writing it out: a zone browser's tab, the player chooser's rows, the
/// bar's "waiting for" line and now a draw offer. Three spellings had grown
/// between them — `Phrase::SeatNumbered` in one, a developer's `#1` in
/// another, and a bare `PlayerId` in the third, which is how "Warte auf Platz
/// 1" was the best the prompt bar could say at a table where everyone has a
/// name.
///
/// The roster is [`Option`] because a seat is sent [`GameStatic`] once and a
/// client draws frames before it arrives; a seat it does not describe is
/// **numbered, not dropped**, because the sentence is about a chair that
/// exists either way.
///
/// This is for *another* seat. The viewing seat's own name is
/// [`own_seat_name`], which draws the pronoun instead — and no caller here
/// has to choose between them: a line that says "waiting for" or "offers a
/// draw" is never about the seat reading it.
#[must_use]
pub fn seat_name(lang: Lang, statics: Option<&GameStatic>, player: PlayerId) -> String {
    statics
        .and_then(|s| s.seats.iter().find(|seat| seat.player == player))
        .map_or_else(
            || Phrase::SeatNumbered.fill(lang, &[&player.get().to_string()]),
            |seat| seat.display_name.clone(),
        )
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

    /// The tab over the local seat says the pronoun once.
    #[test]
    fn a_seat_named_for_the_pronoun_is_not_named_twice() {
        assert_eq!(own_seat_name(Lang::En, "You"), "You");
        assert_eq!(own_seat_name(Lang::De, "Du"), "Du");
        // A real name still gets the pronoun in front of it: at a table of
        // six, "You" alone does not say which chair.
        assert_eq!(own_seat_name(Lang::En, "Viktor"), "You (Viktor)");
        assert_eq!(own_seat_name(Lang::De, "Viktor"), "Du (Viktor)");
        // The languages do not borrow each other's pronoun — a German table
        // whose host still names the seat in English is two claims, not one.
        assert_eq!(own_seat_name(Lang::De, "You"), "Du (You)");
        assert_eq!(own_seat_name(Lang::En, "you"), "You");
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

    /// `card(s)` is not a plural in any language, and the sheet greys what a
    /// sentence says in brackets — so the broken form was drawn as an
    /// editorial aside, in grey, right beside the number it disagreed with.
    /// [`Phrase::counted`] is the way to say this; a bracketed suffix is not.
    #[test]
    fn no_phrase_fakes_a_plural_with_a_bracket() {
        for phrase in Phrase::ALL {
            for lang in Lang::ALL {
                let text = phrase.text(lang);
                for fake in ["(s)", "(n)", "(e)", "(en)", "(er)"] {
                    assert!(
                        !text.contains(fake),
                        "{phrase:?} says {fake} in {lang:?} instead of being written twice"
                    );
                }
            }
        }
    }

    /// A counted phrase is two literals, and the pair has to stay one
    /// sentence: the same values in the same slots, or the singular quietly
    /// drops the number it is counting.
    #[test]
    fn both_forms_of_a_counted_phrase_carry_the_same_values() {
        let pairs = [
            (Phrase::ShakyCard, Phrase::ShakyCards),
            (Phrase::PutCardOnBottom, Phrase::PutOnBottom),
            (Phrase::DiscardCard, Phrase::DiscardCards),
            (Phrase::NounCard, Phrase::NounCards),
            (Phrase::NounTarget, Phrase::NounTargets),
            (Phrase::NounCardFromLibrary, Phrase::NounCardsFromLibrary),
            (Phrase::NounCardToBottom, Phrase::NounCardsToBottom),
            (Phrase::NounCardToGraveyard, Phrase::NounCardsToGraveyard),
            (Phrase::NounCardToTop, Phrase::NounCardsToTop),
            (Phrase::NounCardOutside, Phrase::NounCardsOutside),
            (
                Phrase::NounPermanentToSacrifice,
                Phrase::NounPermanentsToSacrifice,
            ),
            (Phrase::NounCardToDiscard, Phrase::NounCardsToDiscard),
            (Phrase::NounPermanentToTap, Phrase::NounPermanentsToTap),
            (
                Phrase::NounPermanentToReturn,
                Phrase::NounPermanentsToReturn,
            ),
            (
                Phrase::NounPermanentToLeaveTapped,
                Phrase::NounPermanentsToLeaveTapped,
            ),
        ];
        for (one, many) in pairs {
            for lang in Lang::ALL {
                assert_eq!(
                    one.slots(lang),
                    many.slots(lang),
                    "{one:?} and {many:?} disagree about their values in {lang:?}"
                );
            }
            assert_eq!(Phrase::counted(1, one, many), one);
            for n in [0, 2, 7] {
                assert_eq!(Phrase::counted(n, one, many), many);
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
