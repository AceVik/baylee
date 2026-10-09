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
//! Known engine and gateway refusals are localized by [`server_message`].
//! Unknown diagnostics keep the original detail so a newer server remains
//! diagnosable by an older client. Card and player names are never translated
//! by replacing words inside arbitrary messages.

mod server;
pub use server::server_message;

mod lang;
mod names;
mod phrase;
mod refusal;

pub use lang::Lang;
pub use names::{ai_name, own_seat_name, seat_name};
pub use refusal::Refusal;

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
    /// Hold Shift to turn the preview.
    PreviewTurn { en: "Turn over", de: "Drehen" },
    /// Hold Alt/Option for the alternative preview.
    PreviewAlternate { en: "Alternate view", de: "Andere Ansicht" },
    /// Compact preview footer: hold Shift.
    PreviewTurnCompact { en: "Turn", de: "Drehen" },
    /// Compact preview footer: hold Alt/Option.
    PreviewAlternateCompact { en: "View", de: "Ansicht" },
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
    /// The builder's and the deck list's button that opens the import dialog.
    ImportDeck { en: "Import", de: "Importieren" },
    /// The builder's button that opens the export dialog.
    ExportDeck { en: "Export", de: "Exportieren" },
    /// Heading of the import dialog.
    ImportTitle { en: "Import a deck", de: "Deck importieren" },
    /// Heading of the export dialog.
    ExportTitle { en: "Export this deck", de: "Dieses Deck exportieren" },
    /// Puts the clipboard's text into the import box.
    PasteFromClipboard { en: "Paste", de: "Einfügen" },
    /// Empties the import box.
    ImportClear { en: "Clear", de: "Leeren" },
    /// Takes the read deck into the builder.
    ImportTake { en: "Import deck", de: "Deck übernehmen" },
    /// Closes the import or export dialog.
    TransferClose { en: "Close", de: "Schließen" },
    /// Copies the export to the clipboard.
    CopyToClipboard { en: "Copy", de: "Kopieren" },
    /// Saves the export as a file.
    SaveToFile { en: "Save file", de: "Als Datei speichern" },
    /// What the import box shows while empty.
    ImportBoxEmpty { en: "Nothing pasted yet", de: "Noch nichts eingefügt" },
    /// Baylee's own text format, as the format chooser names it.
    FormatBaylee { en: "Baylee text", de: "Baylee-Text" },
    /// The name an imported deck gets when its file names none.
    ImportedDeckName { en: "Imported deck", de: "Importiertes Deck" },
    /// The import dialog's instructions before anything is pasted.
    ImportHowTo { en: "Copy a deck list, a Baylee, JSON or YAML export, or a Moxfield export (More → Export → Copy for Moxfield), then paste it here with Paste or Ctrl/Cmd+V. The format is recognised by itself.", de: "Kopiere eine Deckliste, einen Baylee-, JSON- oder YAML-Export oder einen Moxfield-Export (More → Export → Copy for Moxfield) und füge ihn hier mit Einfügen oder Strg/Cmd+V ein. Das Format wird selbst erkannt." },
    /// A paste read as a deck. `{0}` the format, `{1}` rows, `{2}` cards.
    ImportReadAs { en: "Read as {0}. Rows: {1} · Cards: {2}", de: "Gelesen als {0}. Zeilen: {1} · Karten: {2}" },
    /// Warns that importing replaces what is in the builder.
    ImportReplaces { en: "Importing replaces the deck in the builder with a new, unsaved deck. The saved deck stays as it is.", de: "Der Import ersetzt das Deck im Editor durch ein neues, ungespeichertes Deck. Das gespeicherte Deck bleibt, wie es ist." },
    /// A pasted Moxfield deck link. `{0}` is the site's name.
    ImportMoxfieldLink { en: "That is a link to a deck on {0}, which Baylee cannot read directly. Open the deck on {0} → More → Export → Copy for Moxfield, then paste it here.", de: "Das ist ein Link zu einem Deck auf {0}, das Baylee nicht direkt lesen kann. Öffne das Deck auf {0} → More → Export → Copy for Moxfield und füge es dann hier ein." },
    /// A link to a site with an API this build does not fetch yet. `{0}` the site.
    ImportNotFetched { en: "Decks linked from {0} cannot be fetched by this version yet. Export the deck there as text and paste it here.", de: "Decks von {0} kann diese Version noch nicht abrufen. Exportiere das Deck dort als Text und füge ihn hier ein." },
    /// A link to a site no source knows. `{0}` the host.
    ImportUnknownLink { en: "Baylee cannot read decks from {0}. Export the deck there as a text list and paste the list here.", de: "Baylee kann keine Decks von {0} lesen. Exportiere das Deck dort als Textliste und füge die Liste hier ein." },
    /// A deck taken. `{0}` rows, `{1}` cards, `{2}` the format.
    ImportTook { en: "Imported from {2}. Rows: {0} · Cards: {1}", de: "Aus {2} importiert. Zeilen: {0} · Karten: {1}" },
    /// The pool has not arrived, so names are not resolved yet.
    ImportWaitingForPool { en: "Waiting for the card pool to check the names…", de: "Warte auf den Kartenpool, um die Namen zu prüfen …" },
    /// Cards the pool does not have. `{0}` how many.
    ImportUnknownCards { en: "Not in this gateway's card pool: {0}. Remove them before saving:", de: "Nicht im Kartenpool dieses Gateways: {0}. Vor dem Speichern entfernen:" },
    /// Lines that were not rows. `{0}` how many.
    ImportSkipped { en: "Lines that are not deck rows: {0}", de: "Zeilen, die keine Deckzeilen sind: {0}" },
    /// One such line. `{0}` its number, `{1}` its text.
    ImportSkippedLine { en: "  line {0}: {1}", de: "  Zeile {0}: {1}" },
    /// Maybeboard rows, not kept. `{0}` how many.
    ImportMaybeNotKept { en: "Maybeboard rows not kept, as Baylee stores no maybeboard: {0}", de: "Vielleicht-Zeilen nicht übernommen, da Baylee keine speichert: {0}" },
    /// A commander the rules will not seat. `{0}` its name.
    ImportNotALeader { en: "{0} cannot lead a deck, so it is not marked as commander.", de: "{0} kann kein Deck anführen und ist deshalb nicht als Commander markiert." },
    /// The file named no deck. `{0}` the name given.
    ImportNamedForYou { en: "The file names no deck, so it is called “{0}”.", de: "Die Datei nennt keinen Decknamen, daher heißt es „{0}“." },
    /// Rows with no printing. `{0}` how many.
    ImportDefaultPrinting { en: "Rows without a printing play the default printing: {0}", de: "Zeilen ohne Druck spielen den Standarddruck: {0}" },
    /// Rows with no language. `{0}` how many.
    ImportDefaultLanguage { en: "Rows without a language are English: {0}", de: "Zeilen ohne Sprache sind englisch: {0}" },
    /// A hundred-card main deck without a commander.
    ImportNoCommander { en: "The file names no commander. For a Commander deck, set one in the deck list's card menu.", de: "Die Datei nennt keinen Commander. Für ein Commander-Deck lege ihn im Kartenmenü der Deckliste fest." },
    /// More entries than the list shows. `{0}` how many more.
    AndMore { en: "  and {0} more", de: "  und {0} weitere" },
    /// A paste over the document limit.
    ImportTooLarge { en: "That text is too large for a deck: at most 256 KiB.", de: "Der Text ist zu groß für ein Deck: höchstens 256 KiB." },
    /// A document with too many rows. `{0}` how many.
    ImportTooManyRows { en: "A deck file holds at most 1000 rows; this one holds {0}.", de: "Eine Deckdatei hat höchstens 1000 Zeilen; diese hat {0}." },
    /// Nothing but blank lines and comments.
    ImportNothing { en: "There is no deck in that text.", de: "In dem Text steht kein Deck." },
    /// Not one line was a row. `{0}` the first line's number, `{1}` its text.
    ImportUnreadable { en: "No deck rows found. Line {0} reads: {1}", de: "Keine Deckzeilen gefunden. Zeile {0} lautet: {1}" },
    /// JSON or YAML that did not parse. `{0}` the parser's words.
    ImportSyntax { en: "The file could not be read: {0}", de: "Die Datei ließ sich nicht lesen: {0}" },
    /// A document version this build does not read. `{0}` the version.
    ImportVersion { en: "This is a deck document of version {0}; this build reads version 1. Update Baylee to import it.", de: "Das ist ein Deckdokument der Version {0}; diese Version liest Version 1. Aktualisiere Baylee, um es zu importieren." },
    /// A deck name that is too long or holds a line break.
    ImportDeckName { en: "The deck's name is too long or holds a line break.", de: "Der Deckname ist zu lang oder enthält einen Zeilenumbruch." },
    /// One card refused. `{0}` its position, `{1}` why.
    ImportCardRefused { en: "Card {0} cannot be imported: {1}", de: "Karte {0} lässt sich nicht importieren: {1}" },
    /// Why: no name.
    ImportCardName { en: "it has no name.", de: "sie hat keinen Namen." },
    /// Why: a bad count.
    ImportCardCount { en: "its count must be from 1 to 1000000.", de: "ihre Anzahl muss zwischen 1 und 1000000 liegen." },
    /// Why: a long note.
    ImportCardNote { en: "its note is longer than 500 characters.", de: "ihre Notiz ist länger als 500 Zeichen." },
    /// Why: a field a stored row could not say back.
    ImportCardUnstorable { en: "its name or printing cannot be stored as written.", de: "ihr Name oder Druck lässt sich so nicht speichern." },
    /// The export says everything.
    ExportComplete { en: "This format keeps everything about the deck.", de: "Dieses Format behält alles am Deck." },
    /// The export leaves things out. `{0}` the format.
    ExportLeavesOut { en: "{0} cannot say everything; left out:", de: "{0} kann nicht alles ausdrücken; weggelassen:" },
    /// Loss: the deck's name.
    LossName { en: "the deck's name", de: "der Deckname" },
    /// Loss: languages. `{0}` rows.
    LossLang { en: "languages, rows: {0}", de: "Sprachen, Zeilen: {0}" },
    /// Loss: printing ids. `{0}` rows.
    LossScryfallId { en: "exact printing ids, rows: {0}", de: "genaue Druck-IDs, Zeilen: {0}" },
    /// Loss: notes. `{0}` rows.
    LossNote { en: "notes, rows: {0}", de: "Notizen, Zeilen: {0}" },
    /// Loss: a finish written as non-foil. `{0}` rows.
    LossFinish { en: "holographic, glitter and galaxy finishes, written as non-foil, rows: {0}", de: "Holo-, Glitzer- und Galaxy-Veredelungen, als nicht-foil geschrieben, Zeilen: {0}" },
    /// Loss: collector numbers without a set. `{0}` rows.
    LossCollectorNumber { en: "collector numbers without a set, rows: {0}", de: "Sammlernummern ohne Set, Zeilen: {0}" },
    /// Loss: the maybeboard. `{0}` rows.
    LossMaybeboard { en: "the maybeboard, rows: {0}", de: "das Vielleicht-Board, Zeilen: {0}" },
    /// The export is on the clipboard.
    ExportCopied { en: "Copied to the clipboard.", de: "In die Zwischenablage kopiert." },
    /// The clipboard refused.
    ExportCopyFailed { en: "The clipboard could not be written. Save it as a file instead.", de: "Die Zwischenablage ließ sich nicht beschreiben. Speichere es stattdessen als Datei." },
    /// Saved. `{0}` the path.
    ExportSavedTo { en: "Saved as {0}", de: "Gespeichert als {0}" },
    /// Handed to the browser. `{0}` the file name.
    ExportDownloaded { en: "Downloading {0}", de: "{0} wird heruntergeladen" },
    /// Not saved. `{0}` the system's words.
    ExportSaveFailed { en: "Could not save the file: {0}", de: "Die Datei ließ sich nicht speichern: {0}" },
    /// The import dialog's keys.
    ImportKeys { en: "Ctrl/Cmd+V pastes · Enter imports · Esc closes", de: "Strg/Cmd+V fügt ein · Enter importiert · Esc schließt" },
    /// The export dialog's keys.
    ExportKeys { en: "← → format · Ctrl/Cmd+C copies · Ctrl/Cmd+S saves · Esc closes", de: "← → Format · Strg/Cmd+C kopiert · Strg/Cmd+S speichert · Esc schließt" },    /// Destructive deck confirmation.
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
    /// Empty collection heading.
    FirstDeck { en: "Your first deck", de: "Dein erstes Deck" },
    /// Empty table listing heading.
    EmptyTablesTitle { en: "A place for your next game", de: "Platz für dein nächstes Spiel" },
    /// A new player needs a deck before hosting a game.
    ChooseDeckToBegin { en: "Open a table and choose your deck in the room. Friends can join through the table search.", de: "Erstelle einen Tisch und wähle dein Deck im Raum. Freunde können über die Tischsuche beitreten." },
    /// A table search returned no rows.
    NoMatches { en: "No matching tables", de: "Keine passenden Tische" },
    /// Clear a table search to see the unfiltered listing.
    ClearTableSearch { en: "Show all tables", de: "Alle Tische anzeigen" },
    /// Hub navigation.
    CollectionHint { en: "Your decks, your ideas. Every saved change stays in your history.", de: "Deine Decks, deine Ideen. Jede gespeicherte Änderung bleibt in deiner Historie." },
    /// Gateway selection.
    ChooseGateway { en: "Choose your gateway", de: "Wähle deinen Gateway" },
    /// Gateway selection.
    GatewayHint { en: "Where your account, decks and tables live.", de: "Wo dein Konto, deine Decks und Tische zu Hause sind." },
    /// Gateway selection.
    ChooseGatewayFirst { en: "Select a gateway to sign in or register.", de: "Wähle einen Gateway, um dich anzumelden oder zu registrieren." },
    /// Gateway selection.
    GatewayAddress { en: "GATEWAY ADDRESS", de: "GATEWAY-ADRESSE" },
    /// Gateway selection.
    SaveGateway { en: "Save gateway", de: "Gateway speichern" },
    /// Gateway selection.
    GatewayUrlInvalid { en: "Enter an http:// or https:// address without credentials, query parameters or a fragment.", de: "Gib eine http://- oder https://-Adresse ohne Zugangsdaten, Abfrageparameter oder Fragment ein." },
    /// The front door's gear, pointed at: what is behind it.
    LanguageAndSettings { en: "Language and settings", de: "Sprache und Einstellungen" },
    /// The front door's gear menu: the way to the whole settings screen.
    AllSettings { en: "All settings", de: "Alle Einstellungen" },
    /// A saved gateway's bin, pointed at.
    ForgetGateway { en: "Remove from this list", de: "Aus dieser Liste entfernen" },
    /// Asked before a saved gateway leaves the list.
    ForgetGatewayQuestion { en: "Remove “{0}” from this list?", de: "„{0}“ aus dieser Liste entfernen?" },
    /// Under that question: what removing it does, and what it does not.
    ForgetGatewayHint { en: "Only this device forgets it. Your account and decks stay on the gateway.", de: "Nur dieses Gerät vergisst ihn. Dein Konto und deine Decks bleiben auf dem Gateway." },
    /// A saved gateway's dot, pointed at, when it answered.
    GatewayAnswering { en: "answering", de: "antwortet" },
    /// Under the Fan Content notice: where the source is, the AGPL's §13
    /// offer (#270). `{0}` is the address, drawn as it came.
    SourceCode { en: "Source code (AGPL-3.0): {0}", de: "Quellcode (AGPL-3.0): {0}" },
    /// Scryfall's attribution, which its terms ask of every client that
    /// shows its data or images (`docs/legal.md` §3, #325). Under the Fan
    /// Content notice, and under the version in the game menu, which a
    /// client seated straight into a game shows instead of the front door.
    ScryfallCredit { en: "Card data and images provided by Scryfall.", de: "Kartendaten und -bilder bereitgestellt von Scryfall." },
    /// Gateway selection: an address is asked about itself before it is saved.
    GatewayChecking { en: "Checking {0}…", de: "Prüfe {0} …" },
    /// Gateway selection: the address answered and is saved.
    GatewaySaved { en: "Saved {0}.", de: "{0} gespeichert." },
    /// Gateway selection: nothing at the address answered as a gateway.
    GatewayNotFound { en: "No Baylee gateway answered at {0}. Check the address, or try again once it is up.", de: "Unter {0} hat kein Baylee-Gateway geantwortet. Prüfe die Adresse oder versuche es erneut, sobald er läuft." },
    /// Gateway row, beside its address, while it is being asked.
    GatewayCheckingShort { en: "checking…", de: "wird geprüft …" },
    /// Gateway row: a gateway from before version checks.
    GatewayVersionUnknown { en: "version unknown", de: "Version unbekannt" },
    /// Gateway row: a saved gateway that is down.
    GatewayNotAnswering { en: "not answering", de: "antwortet nicht" },
    /// Gateway warning. `{0}` is the gateway's protocol version, `{1}` this client's.
    GatewayProtocolMismatch { en: "This gateway speaks protocol {0} and this client speaks {1}, so its games will not open here.", de: "Dieser Gateway spricht Protokoll {0} und dieser Client {1}, deshalb öffnen sich seine Partien hier nicht." },
    /// Gateway warning. `{0}` is the gateway's view version, `{1}` this client's.
    GatewayViewMismatch { en: "This gateway's games send view version {0} and this client reads {1}, so they will not open here.", de: "Die Partien dieses Gateways senden Ansichtsversion {0} und dieser Client liest {1}, deshalb öffnen sie sich hier nicht." },
    /// Gateway warning: a gateway from before version checks.
    GatewayOlder { en: "This gateway is older than version checks. Whether its games open here shows only when one starts.", de: "Dieser Gateway ist älter als die Versionsprüfung. Ob sich seine Partien hier öffnen, zeigt erst eine Partie." },
    /// Gateway warning: a saved gateway that is down.
    GatewaySilent { en: "This gateway is not answering right now.", de: "Dieser Gateway antwortet gerade nicht." },
    /// Server display-name rules, displayed before a registration is
    /// submitted, and what the display name is for: the username is private.
    AccountNameHint { en: "Others see it as Name#tag. 3–16 characters: A–Z, 0–9, _ or -, a letter or number at each end.", de: "Andere sehen ihn als Name#Tag. 3–16 Zeichen: A–Z, 0–9, _ oder -, an beiden Enden ein Buchstabe oder eine Zahl." },
    /// Password registration guidance.
    AccountPasswordHint { en: "At least 8 characters, not a common password, and neither of your names.", de: "Mindestens 8 Zeichen, kein häufiges Passwort und keiner deiner beiden Namen." },
    /// A rejected password during registration.
    AccountPasswordInvalid { en: "Choose a password with 8–256 characters, different from your username and display name, and not a common password.", de: "Wähle ein Passwort mit 8–256 Zeichen, verschieden von Benutzer- und Anzeigename und kein häufiges Passwort." },
    /// A library reply was not a valid response.
    LibraryReadFailed { en: "The library response could not be read. Please try again.", de: "Die Bibliotheksantwort konnte nicht gelesen werden. Bitte erneut versuchen." },
    /// Library and front-door interface.
    WelcomeTitle { en: "Your next game starts here.", de: "Dein nächstes Spiel beginnt hier." },
    /// Library and front-door interface.
    WelcomeNote { en: "Build a deck. Find your table. Make it yours.", de: "Baue dein Deck. Finde deinen Tisch. Spiele deinen Stil." },
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
    /// Temporary player actions available until cleanup.
    GrantedActions { en: "Actions until end of turn", de: "Aktionen bis Zugende" },
    /// Explicit life payment for one temporary action.
    GrantedLifeCost { en: "Pay {0} life", de: "{0} Leben bezahlen" },
    /// Bound recipient of an already granted prevention action.
    GrantedRecipient { en: "Recipient: {0}", de: "Empfänger: {0}" },
    /// Confirms one explicitly selected temporary action.
    GrantedConfirm { en: "Perform action", de: "Aktion ausführen" },
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
    AppName { en: "Baylee", de: "Baylee" },
    /// Caption over the username field.
    Username { en: "USERNAME", de: "BENUTZERNAME" },
    /// Caption over the name field, when registering.
    DisplayName { en: "DISPLAY NAME", de: "ANZEIGENAME" },
    /// Caption over the password field.
    Password { en: "PASSWORD", de: "PASSWORT" },
    /// Caption of the second password box, when an account is created.
    PasswordAgain { en: "REPEAT PASSWORD", de: "PASSWORT WIEDERHOLEN" },
    /// The account form's submit, in both of its modes: the tab above it
    /// already says which.
    Continue { en: "Continue", de: "Weiter" },
    /// The button that signs in.
    SignIn { en: "Sign in", de: "Anmelden" },
    /// The button that registers.
    CreateAccount { en: "Create account", de: "Registrieren" },
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
    /// The music's switch while the music plays (#296); pressed, it stops.
    MusicPlaying { en: "Music", de: "Musik" },
    /// The music's switch while the music is silent; pressed, it plays.
    MusicSilent { en: "Music off", de: "Musik aus" },
    /// The button that changes the interface language.
    Language { en: "Language", de: "Sprache" },

    // ---- graphics and sound (this device's, `graphics.rs`, `audiomix.rs`)
    /// The heading over the graphics knobs.
    Graphics { en: "Graphics", de: "Grafik" },
    /// The preset row: one name for every knob below it.
    GraphicsPreset { en: "Quality", de: "Qualität" },
    /// A preset, and the lowest ambient-effects level.
    QualityLow { en: "Low", de: "Niedrig" },
    /// A preset, and the middle ambient-effects level.
    QualityMedium { en: "Medium", de: "Mittel" },
    /// A preset, and the highest ambient-effects level.
    QualityHigh { en: "High", de: "Hoch" },
    /// The highest preset.
    QualityUltra { en: "Ultra", de: "Ultra" },
    /// The preset once a knob was moved by hand.
    QualityCustom { en: "Custom", de: "Eigene" },
    /// Edge smoothing on the table.
    AntiAliasing { en: "Edge smoothing", de: "Kantenglättung" },
    /// Whether frames wait for the display.
    VSync { en: "VSync", de: "VSync" },
    /// Vertical sync that lets a late frame through.
    VSyncAdaptive { en: "adaptive", de: "adaptiv" },
    /// The most frames per second with the window in front.
    FrameLimit { en: "Frame limit", de: "Bildratenlimit" },
    /// No frame limit.
    Unlimited { en: "unlimited", de: "unbegrenzt" },
    /// The most frames per second behind other windows and on an idle
    /// front door.
    BackgroundFrames { en: "In the background", de: "Im Hintergrund" },
    /// Graphics: how many frames a table at rest draws.
    RowRestLimit { en: "Frame rate at rest", de: "Bildrate in Ruhe" },
    /// How much the ambient surfaces (the front door's world, the cloth,
    /// the sky) move and how finely.
    AmbientEffects { en: "Ambient effects", de: "Umgebungseffekte" },
    /// The heading over the volume knobs.
    Audio { en: "Volume", de: "Lautstärke" },
    /// Every sound and the music together.
    MasterVolume { en: "Overall", de: "Gesamt" },
    /// The game's sounds, not the music.
    EffectsVolume { en: "Game sounds", de: "Spielklänge" },
    /// Silence while another window has the focus.
    MuteInBackground { en: "Silent in the background", de: "Im Hintergrund stumm" },

    // ---- updating (#326)
    /// A newer release is downloaded, verified and waiting. `{0}` its version.
    UpdateReady {
        en: "Update {0} ready – installs when you quit",
        de: "Update {0} bereit – wird beim Beenden installiert",
    },
    /// A newer release exists and will not install itself. `{0}` its version.
    UpdateAvailable {
        en: "Update {0} available – download it here",
        de: "Update {0} verfügbar – hier herunterladen",
    },
    /// Shown once, at the first start after an update. `{0}` the version.
    UpdatedTo { en: "Updated to {0}", de: "Aktualisiert auf {0}" },
    /// Why an update is only a link: the player switched installing off.
    UpdateWhyOff {
        en: "Automatic updates are off in the settings.",
        de: "Automatische Updates sind in den Einstellungen aus.",
    },
    /// Why: a development build.
    UpdateWhyDev {
        en: "This is a development build: it checks, and never installs by itself.",
        de: "Das ist ein Entwicklungs-Build: Er sucht, installiert aber nie selbst.",
    },
    /// Why: the folder it runs from cannot be written by this user.
    UpdateWhyFolder {
        en: "Baylee cannot write to its own folder, so it cannot replace itself.",
        de: "Baylee darf seinen eigenen Ordner nicht beschreiben und sich darum nicht ersetzen.",
    },
    /// Why: macOS runs the app from a read-only copy (App Translocation)
    /// and would not say where the original is.
    UpdateWhyMoveApp {
        en: "macOS runs Baylee from a read-only copy. Move it to Applications and it can update itself.",
        de: "macOS startet Baylee aus einer schreibgeschützten Kopie. Verschiebe es nach „Programme“, dann kann es sich selbst aktualisieren.",
    },
    /// Why, exactly: the folder of the installation cannot be written by
    /// this user. `{0}` the folder, `{1}` what the system answered.
    UpdateWhyReadOnly {
        en: "Baylee installs no updates here: it may not write to {0} ({1}). Move it to your Applications folder and it can.",
        de: "Baylee installiert hier keine Updates: Es darf {0} nicht beschreiben ({1}). Verschiebe es in deinen Programme-Ordner, dann kann es das.",
    },
    /// Settings: macOS runs the app from a read-only copy, and updates
    /// install anyway.
    UpdateTranslocated {
        en: "macOS runs Baylee from a read-only copy, because it still lies where it was unpacked. Updates install anyway; moved to Applications, it starts without the detour.",
        de: "macOS startet Baylee aus einer schreibgeschützten Kopie, weil es noch dort liegt, wo es entpackt wurde. Updates installieren sich trotzdem; nach „Programme“ verschoben, startet es ohne diesen Umweg.",
    },
    /// The button: copy the app into `~/Applications` and start it there.
    UpdateMoveHome {
        en: "Move to my Applications folder",
        de: "In meinen Programme-Ordner verschieben",
    },
    /// The button: copy the app into `/Applications`, for every user.
    UpdateMoveSystem {
        en: "Move to Applications for all users",
        de: "Für alle nach „Programme“ verschieben",
    },
    /// Under the buttons: what moving does.
    UpdateMoveWhat {
        en: "Copies Baylee there, starts the copy and closes this one. The old copy is deleted only if you say so.",
        de: "Kopiert Baylee dorthin, startet die Kopie und schließt diese. Die alte Kopie wird nur gelöscht, wenn du es sagst.",
    },
    /// Moving failed. `{0}` why.
    UpdateMoveFailed {
        en: "Baylee could not be moved: {0}",
        de: "Baylee ließ sich nicht verschieben: {0}",
    },
    /// The first start after a move. `{0}` where it runs now.
    UpdateMoved {
        en: "Baylee now runs from {0}.",
        de: "Baylee läuft jetzt aus {0}.",
    },
    /// Under it. `{0}` the old copy.
    UpdateOldCopy {
        en: "The old copy is still at {0}.",
        de: "Die alte Kopie liegt noch unter {0}.",
    },
    /// Moves the old copy to the Trash.
    UpdateTrashOld { en: "Move old copy to Trash", de: "Alte Kopie in den Papierkorb" },
    /// Keeps the old copy, and stops asking.
    UpdateKeepOld { en: "Keep it", de: "Behalten" },
    /// The Trash refused. `{0}` why.
    UpdateTrashFailed {
        en: "It could not be moved to the Trash ({0}); drag it there yourself if you like.",
        de: "Sie ließ sich nicht in den Papierkorb legen ({0}); zieh sie selbst hinein, wenn du magst.",
    },
    /// Why: the download's signature or checksum did not verify.
    UpdateWhyNotVerified {
        en: "The download could not be verified as ours, so it was not installed.",
        de: "Der Download ließ sich nicht als unserer bestätigen und wurde nicht installiert.",
    },
    /// Why: anything else (no archive for this system, no signature, a
    /// failed download or install).
    UpdateWhyOther {
        en: "This update cannot be installed automatically.",
        de: "Dieses Update lässt sich nicht automatisch installieren.",
    },
    /// Opens the release page.
    ReleaseNotes { en: "Release notes", de: "Versionshinweise" },
    /// Puts the notice away for this session.
    UpdateHide { en: "Hide", de: "Ausblenden" },
    /// Settings: the per-device switch for asking GitHub.
    UpdateAutoCheck {
        en: "Check for updates automatically",
        de: "Automatisch nach Updates suchen",
    },
    /// Under it: what that costs in privacy (`docs/privacy.md`).
    UpdateAutoCheckWhy {
        en: "Asks GitHub at start and every six hours. GitHub sees your IP address and this version. Off: no request at all.",
        de: "Fragt GitHub beim Start und alle sechs Stunden. GitHub sieht dabei deine IP-Adresse und diese Version. Aus: gar keine Anfrage.",
    },
    /// Settings: the per-device switch for installing.
    UpdateAutoInstall { en: "Update automatically", de: "Automatisch aktualisieren" },
    /// Under it.
    UpdateAutoInstallWhy {
        en: "Downloads a signed update and installs it when you quit.",
        de: "Lädt ein signiertes Update herunter und installiert es beim Beenden.",
    },
    /// Settings: asks now.
    UpdateCheckNow { en: "Check for updates", de: "Nach Updates suchen" },
    /// While a check runs.
    UpdateChecking { en: "Checking…", de: "Suche läuft…" },
    /// A check found nothing newer.
    UpdateUpToDate { en: "Baylee is up to date.", de: "Baylee ist aktuell." },
    /// A check that could not ask (offline, rate limited).
    UpdateCheckFailed {
        en: "Could not check for updates.",
        de: "Die Suche nach Updates hat nicht geklappt.",
    },

    // ---- the table screen
    /// Leaves the account.
    SignOut { en: "Sign out", de: "Abmelden" },
    /// Opens the confirmation that deletes the account (#292), on the
    /// settings screen.
    DeleteAccount { en: "Delete account", de: "Konto löschen" },
    /// Asked before an account is deleted. `{0}` is its name.
    DeleteAccountQuestion { en: "Delete the account {0}?", de: "Das Konto {0} löschen?" },
    /// Under that question, for an account with a password.
    DeleteAccountHint {
        en: "The account, its decks and its settings leave this gateway for good, and every device signed in to it is signed out. Type your password to confirm.",
        de: "Das Konto, seine Decks und seine Einstellungen verschwinden endgültig von diesem Gateway, und jedes angemeldete Gerät wird abgemeldet. Gib zur Bestätigung dein Passwort ein.",
    },
    /// The same, for a guest, which has no password to type.
    DeleteGuestHint {
        en: "The guest, its decks and its settings leave this gateway for good.",
        de: "Der Gast, seine Decks und seine Einstellungen verschwinden endgültig von diesem Gateway.",
    },
    /// The confirmation's button that deletes.
    DeleteAccountConfirm { en: "Delete for good", de: "Endgültig löschen" },
    /// The confirmation was sent with no password typed.
    DeleteAccountNeedsPassword {
        en: "your password, please",
        de: "bitte dein Passwort",
    },
    /// The account is gone.
    AccountDeleted { en: "account deleted", de: "Konto gelöscht" },
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
    /// Localized name for an unnamed house-controlled chair. Named AI seats
    /// retain their roster name so the HUD, prompts and stack agree.
    SeatHouse { en: "House AI", de: "Haus-KI" },
    /// Caption over the room password box.
    RoomPassword { en: "ROOM PASSWORD", de: "RAUM-PASSWORT" },
    /// Before the row of table sizes.
    OpenATableFor { en: "Open a table for", de: "Tisch eröffnen für" },
    /// Shown in place of an empty table list.
    NoTablesOpen {
        en: "The tables are quiet for now. Open yours and invite someone to play.",
        de: "Noch sind alle Tische frei. Eröffne deinen und lade jemanden zum Spielen ein.",
    },
    /// Empty offline play area: only the local house AI is available.
    OfflineReadyToPlay {
        en: "Your deck is ready. Play the house AI now, or create a table to choose its seats and teams.",
        de: "Dein Deck ist bereit. Spiele direkt gegen die Haus-KI oder erstelle einen Tisch mit eigener Sitz- und Teamaufteilung.",
    },
    /// The gateway stopped answering: the feed closed or a request found
    /// nobody there. Stays until it answers again.
    GatewayUnreachable {
        en: "The gateway is not answering. Trying again\u{2026}",
        de: "Das Gateway antwortet nicht. Neuer Versuch l\u{e4}uft\u{2026}",
    },
    /// The gateway answered again after [`Phrase::GatewayUnreachable`].
    GatewayBack {
        en: "The gateway is back.",
        de: "Das Gateway ist wieder da.",
    },
    /// The listing said no agent is there: no room can open or start.
    NoNewGames {
        en: "No new games can start right now \u{2014} the server may be updating. Games already running go on.",
        de: "Gerade k\u{f6}nnen keine neuen Spiele beginnen \u{2014} vielleicht wird der Server aktualisiert. Laufende Spiele gehen weiter.",
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
    /// Seats a language model in an open chair (`docs/llm-seat.md`).
    SeatToLlm { en: "→ language model", de: "→ Sprachmodell" },
    /// What a language-model chair plays. `{0}` the model's label and id, `{1}` the effort.
    RoomLlmPlays { en: "Language model: {0}, effort {1}", de: "Sprachmodell: {0}, Aufwand {1}" },
    /// The effort a model picks itself.
    RoomLlmEffortOwn { en: "the model's own", de: "wie das Modell es vorgibt" },
    /// Over the profile chips of a language-model chair.
    RoomLlmProfile { en: "Profile (settings file)", de: "Profil (Einstellungsdatei)" },
    /// Under a model that takes no named effort.
    RoomLlmNoEfforts {
        en: "This model takes no effort this build can name.",
        de: "Dieses Modell nimmt keinen Aufwand, den dieser Build benennen kann.",
    },
    /// Over the deck chips of a language-model chair.
    RoomLlmDeck { en: "Deck it brings", de: "Deck, das es mitbringt" },
    /// Keeps a chair's model and effort as its profile's.
    RoomLlmSave { en: "Keep as the profile's", de: "Ins Profil übernehmen" },
    /// Takes the language model out of the chair.
    RoomLlmRemove { en: "Remove the model", de: "Modell entfernen" },
    /// Before the chair's bridge has said anything.
    RoomLlmStarting {
        en: "Its bridge takes the chair once the room lists it open.",
        de: "Seine Brücke nimmt den Stuhl, sobald der Raum ihn als frei führt.",
    },
    /// The in-game panel's title (a debug build only).
    GameLlmTitle {
        en: "Language-model chairs (debug: from the next decision)",
        de: "Sprachmodell-Stühle (Debug: ab der nächsten Entscheidung)",
    },
    /// Hands a language-model chair to the house during the game.
    GameLlmHouse { en: "House AI", de: "Haus-KI" },
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
    NeedUsernameAndPassword {
        en: "a username and a password, please",
        de: "bitte Benutzername und Passwort",
    },
    /// Registering, and the username has an invisible or direction-turning
    /// character in it (`names::UsernameFault::Invisible`).
    UsernameInvisible {
        en: "that username holds a character that cannot be seen",
        de: "dieser Benutzername enthält ein unsichtbares Zeichen",
    },
    /// Registering, and the username has a character the rule does not allow.
    UsernameCharacters {
        en: "a username is letters A–Z, digits and _ - . only",
        de: "ein Benutzername besteht nur aus A–Z, Ziffern und _ - .",
    },
    /// Registering, and the username is too short or too long.
    UsernameLength {
        en: "a username has 3 to 24 characters",
        de: "ein Benutzername hat 3 bis 24 Zeichen",
    },
    /// Registering, and the username begins or ends with a separator.
    UsernameEdge {
        en: "a username starts and ends with a letter or a digit",
        de: "ein Benutzername beginnt und endet mit einem Buchstaben oder einer Ziffer",
    },
    /// Registering, and the username has two separators in a row.
    UsernameDoubled {
        en: "a username has no two of _ - . in a row",
        de: "in einem Benutzernamen stehen _ - . nie zweimal hintereinander",
    },
    /// Signed in with an address: the name to sign in with from now on.
    /// `{0}` is the username.
    YourUsernameIs {
        en: "signed in — your username is {0}; the address works until the end of 2026",
        de: "angemeldet — dein Benutzername ist {0}; die Adresse geht noch bis Ende 2026",
    },
    /// The button that plays as a new guest (#269).
    PlayAsGuest { en: "Play as guest", de: "Als Gast spielen" },
    /// The same button, for the guest this device already holds here. `{0}`
    /// is its handle.
    ContinueAsGuest { en: "Continue as {0}", de: "Weiter als {0}" },
    /// Caption of the box for a guest's name.
    GuestName { en: "GUEST NAME", de: "GASTNAME" },
    /// What that box says while empty: the name the gateway gives a guest
    /// that chose none. The gateway's word, in every language, because it is
    /// the name the other players will read.
    GuestDefaultName { en: "Guest", de: "Guest" },
    /// While the gateway makes the guest.
    JoiningAsGuest { en: "joining as a guest…", de: "trete als Gast bei …" },
    /// Just in, as a guest.
    PlayingAsGuest { en: "playing as a guest", de: "du spielst als Gast" },
    /// On the tables screen for the whole of a guest's visit: what a guest is.
    GuestNotice {
        en: "You are playing as a guest. This account and its decks are deleted about 30 days after your last visit.",
        de: "Du spielst als Gast. Dieses Konto und seine Decks werden etwa 30 Tage nach deinem letzten Besuch gelöscht.",
    },
    /// The guest this device held has ended on the gateway.
    GuestEnded {
        en: "that guest has ended — play as a new one, or sign in",
        de: "dieses Gastkonto ist abgelaufen — spiel als neuer Gast oder melde dich an",
    },
    /// The gateway takes no guests (its `/auth/config` said so).
    NoGuests { en: "this gateway takes no guests", de: "dieses Gateway nimmt keine Gäste auf" },
    /// Caption of the box for a closed-beta key (#317).
    InviteKey { en: "CLOSED BETA KEY", de: "BETA-SCHLÜSSEL" },
    /// What that box says while empty: the shape of a key.
    InviteKeyShape { en: "BAYLEE-XXXX-XXXX-XXXX-XXXX", de: "BAYLEE-XXXX-XXXX-XXXX-XXXX" },
    /// Under the box: what the key is for, and that pasting it is fine.
    InviteKeyHint {
        en: "This gateway is a closed beta: a new account or a new guest needs a key. Paste it as you got it.",
        de: "Dieses Gateway ist eine geschlossene Beta: Ein neues Konto oder ein neuer Gast braucht einen Schlüssel. Füg ihn einfach so ein, wie du ihn bekommen hast.",
    },
    /// A new account or guest was asked for with the box empty.
    NeedInviteKey { en: "your closed beta key, please", de: "bitte deinen Beta-Schlüssel" },
    /// The gateway refused the key, for whichever reason: mistyped, used,
    /// expired or revoked. It does not say which, and neither does this.
    InviteKeyInvalid {
        en: "this closed beta key is not valid — check it, or ask for a new one",
        de: "dieser Beta-Schlüssel ist nicht gültig — prüf ihn oder frag nach einem neuen",
    },
    /// The gateway wanted a key and none reached it.
    InviteKeyNeeded {
        en: "this gateway is a closed beta: a new account or guest needs a closed beta key",
        de: "dieses Gateway ist eine geschlossene Beta: ein neues Konto oder ein neuer Gast braucht einen Beta-Schlüssel",
    },
    /// Asked before a guest signs out. `{0}` is its handle.
    GuestSignOutQuestion { en: "Sign out {0}?", de: "{0} abmelden?" },
    /// Under that question: what signing out does to a guest.
    GuestSignOutHint {
        en: "A guest cannot sign in again. Signing out deletes this guest and its decks now.",
        de: "Ein Gast kann sich nicht wieder anmelden. Abmelden löscht diesen Gast und seine Decks sofort.",
    },
    /// The same, registering.
    NeedDisplayName { en: "a display name, please", de: "bitte einen Anzeigenamen" },
    /// Registering, and the password was typed differently the second time.
    PasswordsDiffer {
        en: "the two passwords differ",
        de: "die beiden Passwörter sind verschieden",
    },
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
    /// Explicit shortcut for the current repeated targeting series.
    ActConfirmTargetBatch { en: "Confirm targets for this series", de: "Ziele für diese Serie bestätigen" },
    /// Ordinary confirmation or priority pass.
    ActConfirm { en: "Confirm / pass priority", de: "Bestätigen / Priorität abgeben" },
    /// Cancel
    ActCancel { en: "Cancel", de: "Abbrechen" },
    /// Play or choose the card
    ActActivateCard { en: "Play or choose the card", de: "Karte spielen oder wählen" },
    /// Choose every card in the group
    ActActivateGroup { en: "Choose every card in the group", de: "Alle Karten der Gruppe wählen" },
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
    /// Open the zone browser. The clause is the settings row's help: a tap on
    /// a pile whose top card is lit arms that card's cast rather than opening
    /// the pile (#242), and this key is the way in that is left.
    ActToggleBrowser {
        en: "Open the zone browser, also when a graveyard's top card is lit",
        de: "Zonenbrowser öffnen, auch wenn die oberste Karte eines Friedhofs leuchtet",
    },
    /// Open the game log (#262).
    ActToggleLog { en: "Open the game log", de: "Spielprotokoll öffnen" },
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
    /// Finite batch of already stacked identical may decisions.
    AnswerYesBatch { en: "Yes to all {0} identical abilities", de: "Ja für alle {0} gleichen Fähigkeiten" },
    /// Public continuing effect on a player's hand limit.
    NoMaxHandSize { en: "No maximum hand size", de: "Kein Handkartenlimit" },
    /// Finish only the current creature's remaining damage division.
    AutoCombatDamage { en: "Assign the rest automatically", de: "Rest automatisch verteilen" },
    /// No
    ActAnswerNo { en: "No", de: "Nein" },
    /// Number up
    ActNumberUp { en: "Number up", de: "Zahl hoch" },
    /// Number down
    ActNumberDown { en: "Number down", de: "Zahl runter" },
    /// Scroll a stack entry's long text back a page
    ActTextPageUp { en: "Scroll a stack entry's text up", de: "Text eines Stapeleintrags hochblättern" },
    /// Scroll it on a page
    ActTextPageDown { en: "Scroll a stack entry's text down", de: "Text eines Stapeleintrags runterblättern" },
    /// Rail selection up
    ActRailUp { en: "Rail selection up", de: "Phasenleiste hoch" },
    /// Rail selection down
    ActRailDown { en: "Rail selection down", de: "Phasenleiste runter" },
    /// The table-angle row (DESIGN-v7 D20)
    RowTableLean { en: "Table angle", de: "Tischwinkel" },
    /// What the table angle does
    HelpTableLean {
        en: "How steeply the camera looks at a table of three seats or more. Steep draws your own board about a third larger and the far seats a little smaller; flat draws every seat's board the same size.",
        de: "Wie steil die Kamera auf einen Tisch mit drei oder mehr Plätzen blickt. Steil zeichnet dein eigenes Brett etwa ein Drittel größer und die fernen Plätze etwas kleiner; flach zeichnet alle Bretter gleich groß.",
    },
    /// The steep lean
    LeanSteep { en: "Steep", de: "Steil" },
    /// The gentle lean
    LeanGentle { en: "Flat", de: "Flach" },
    /// The visit-camera row (DESIGN-v7 D21)
    RowVisitCamera { en: "Looking at a seat", de: "Sitz ansehen" },
    /// What the visit camera does
    HelpVisitCamera {
        en: "Where the camera stands when you look at another seat (its button, F). Automatic: behind a teammate, as if sitting beside them, and across from an opponent, whose board you then read as in a duel. Or always behind it with your own lane or the dial in view, behind it with the dial always in view, or across from it.",
        de: "Wo die Kamera steht, wenn du einen anderen Platz ansiehst (sein Knopf, F). Automatisch: hinter einem Mitspieler, als säßest du neben ihm, und gegenüber einem Gegner, dessen Brett du dann wie im Duell liest. Oder immer dahinter mit deiner Reihe oder dem Zifferblatt im Bild, dahinter immer mit dem Zifferblatt, oder gegenüber.",
    },
    /// The automatic visit: behind a teammate, across from an opponent
    VisitAuto { en: "Automatic", de: "Automatisch" },
    /// Behind, with the rule
    VisitBehind { en: "Behind", de: "Dahinter" },
    /// Behind, the dial always
    VisitBehindDial { en: "Behind, dial", de: "Dahinter, Zifferblatt" },
    /// Across
    VisitAcross { en: "Across", de: "Gegenüber" },
    /// The game menu's priority-sound switch, on
    PriorityCueOn { en: "Priority sound: on", de: "Prioritätston: an" },
    /// …and off
    PriorityCueOff { en: "Priority sound: off", de: "Prioritätston: aus" },
    /// The priority-sound row (DESIGN-v7 D22)
    RowPriorityCue { en: "Priority sound", de: "Prioritätston" },
    /// What the priority sound does
    HelpPriorityCue {
        en: "A soft strike when the table starts waiting for you — only on your own device, never for the others. It has its own switch; the music's switch never silences it.",
        de: "Ein weicher Schlag, wenn der Tisch auf dich zu warten beginnt — nur auf deinem eigenen Gerät, nie für die anderen. Er hat einen eigenen Schalter; der Musik-Schalter schaltet ihn nie stumm.",
    },
    /// Visit the next seat (TABLE-KEYBOARD §9)
    ActFocusNextSeat { en: "Visit the next seat", de: "Sitz ansehen" },
    /// Visit the previous seat
    ActFocusPrevSeat { en: "Visit the previous seat", de: "Vorigen Sitz ansehen" },
    /// Back to my own seat
    ActFocusHome { en: "Back to my seat", de: "Zurück zu meinem Platz" },
    /// Open the arrangement menu (DESIGN-v8 §2.3)
    ActArrangementMenu { en: "Choose the arrangement", de: "Anordnung wählen" },
    /// Cycle to the next offered arrangement
    ActNextArrangement { en: "Next arrangement", de: "Nächste Anordnung" },
    /// Open or shut the hand's drawer on a phone (DESIGN-v8 WA11)
    ActHandDrawer { en: "Open or shut the hand (phone)", de: "Hand auf- oder zuklappen (Telefon)" },
    /// The creature-type chooser's quick list: the types the deck plays most.
    TypesInDeck { en: "Most in your deck", de: "Am häufigsten in deinem Deck" },
    /// The creature-type chooser's full list.
    TypesAll { en: "All types", de: "Alle Typen" },
    /// A cast made before its mana, asking its own question first: the
    /// spell, the question, and that the payment comes after (CR 601.2g).
    CastFirstQuestion { en: "Cast {0}: {1} — paid after", de: "{0} wirken: {1} — bezahlt wird danach" },
    /// The answer that takes back a spell being cast (CR 732), in its
    /// questions and its payment window.
    CancelCast { en: "Cancel cast", de: "Abbrechen" },
    /// Fold the question's sheet to its pill, or open it again.
    ActFoldDecision { en: "Fold or open the question's sheet", de: "Entscheidung ein- oder ausklappen" },
    /// The ring (DESIGN-v8 arrangement 1)
    ArrRing { en: "Ring", de: "Ring" },
    /// What the ring does
    ArrRingBlurb {
        en: "Every seat on the ring; the camera visits.",
        de: "Alle Sitze am Ring; die Kamera besucht.",
    },
    /// The upright ring (arrangement 2)
    ArrUprightRing { en: "Upright ring", de: "Aufrechter Ring" },
    /// What it does
    ArrUprightRingBlurb {
        en: "The ring with every board upright to you; a visit zooms in.",
        de: "Der Ring, jedes Brett aufrecht zu dir; ein Besuch zoomt heran.",
    },
    /// The turntable (arrangement 3)
    ArrTurntable { en: "Turntable", de: "Drehteller" },
    /// What it does
    ArrTurntableBlurb {
        en: "You and one opponent as in a duel, the others small at the sides; choosing a seat turns the table.",
        de: "Du und ein Gegner wie im Duell, die anderen klein an den Seiten; einen Sitz wählen dreht den Tisch.",
    },
    /// The arc rail (arrangement 4)
    ArrArcRail { en: "Arc rail", de: "Bogenschiene" },
    /// What it does
    ArrArcRailBlurb {
        en: "The opponents on an arc above you; the camera slides along the rail.",
        de: "Die Gegner auf einem Bogen über dir; die Kamera fährt die Schiene entlang.",
    },
    /// The pods (arrangement 5)
    ArrPods { en: "Pods", de: "Raster" },
    /// What they do
    ArrPodsBlurb {
        en: "Every opponent upright in a grid, seen from above; a visit zooms in.",
        de: "Jeder Gegner aufrecht in einem Raster, von oben gesehen; ein Besuch zoomt heran.",
    },
    /// The spotlight (arrangement 6)
    ArrSpotlight { en: "Spotlight", de: "Rampenlicht" },
    /// What it does
    ArrSpotlightBlurb {
        en: "A duel against one seat, the others as chips in the strip; turn on “Table follows the turn” to follow play.",
        de: "Ein Duell gegen einen Sitz, die anderen als Chips in der Leiste; mit „Tisch folgt dem Zug“ folgt es dem Spiel.",
    },
    /// The turntable with rows (arrangement 7)
    ArrTurntableRows { en: "Turntable+", de: "Drehteller+" },
    /// What it does, and what it is here
    ArrTurntableRowsBlurb {
        en: "Turntable up to four seats, Spotlight from five or on a phone.",
        de: "Drehteller bis vier Sitze, Rampenlicht ab fünf oder auf dem Handy.",
    },
    /// What the turntable with rows resolved to at this table
    ArrResolvedTo { en: "Here: {0}", de: "Hier: {0}" },
    /// The focus ring (arrangement 8)
    ArrFocusRing { en: "Focus ring", de: "Fokusring" },
    /// What it does
    ArrFocusRingBlurb {
        en: "A duel in the middle, the other seats as peeks at the edges.",
        de: "Ein Duell in der Mitte, die anderen Sitze als Spalten an den Rändern.",
    },
    /// A greyed row: a duel has one arrangement
    ArrNotInADuel {
        en: "A duel has only one arrangement",
        de: "Im Duell gibt es nur eine Anordnung",
    },
    /// A greyed row: not built yet
    ArrComing { en: "Coming ({0})", de: "Kommt ({0})" },
    /// A greyed row: not at this many seats on this window
    ArrNotHereSeats {
        en: "Not at this many seats on a window this size",
        de: "Nicht bei so vielen Sitzen in einem Fenster dieser Größe",
    },
    /// A greyed row: not on a window this small
    ArrNotHereWindow {
        en: "Not on a window this small",
        de: "Nicht in einem so kleinen Fenster",
    },
    /// A greyed row: not on a phone
    ArrNotOnAPhone { en: "Not on a phone", de: "Nicht auf dem Handy" },
    /// The tag on a row whose measurement is outstanding
    ArrExperimental { en: "experimental", de: "experimentell" },
    /// The menu's last row: keep this choice for the seat count
    ArrRememberForSeats {
        en: "Remember for {0} seats",
        de: "Für {0} Sitze merken",
    },
    /// The placeholder note under the menu
    ArrLettersNote {
        en: "The letters stand in for pictograms still to be drawn.",
        de: "Die Buchstaben stehen für Bildzeichen, die noch gezeichnet werden.",
    },
    /// The game menu's row
    ArrGameMenuRow { en: "Arrangement › {0}", de: "Anordnung › {0}" },
    /// The arrangement settings row
    RowArrangement { en: "Table arrangement", de: "Tischanordnung" },
    /// What it does
    HelpArrangement {
        en: "How the seats of a table of three or more are placed. P chooses one at the table, Shift+P the next; this is the default.",
        de: "Wie die Sitze an einem Tisch mit drei oder mehr Plätzen stehen. P wählt am Tisch eine, Umschalt+P die nächste; dies ist der Standard.",
    },
    /// The follow switch's row
    RowFollowTurn { en: "Table follows the turn", de: "Tisch folgt dem Zug" },
    /// What it does
    HelpFollowTurn {
        en: "As another player's turn begins the table shows that player's seat — a camera arrangement visits it, the others bring it across — and on your turn it comes back to yours. It waits while you are answering a question; a seat you look at by hand stays until the next turn begins.",
        de: "Beginnt der Zug eines anderen Spielers, zeigt der Tisch dessen Sitz — eine Kamera-Anordnung besucht ihn, die anderen holen ihn herüber — und in deinem Zug kehrt er zu deinem zurück. Er wartet, während du eine Frage beantwortest; ein Sitz, den du selbst ansiehst, bleibt bis zum nächsten Zugbeginn.",
    },
    /// The per-seat-count row
    RowArrangementBySeats { en: "Per seat count", de: "Je Sitzzahl" },
    /// What it does
    HelpArrangementBySeats {
        en: "An arrangement remembered for one seat count wins over the default at a table of that size.",
        de: "Eine für eine Sitzzahl gemerkte Anordnung gilt an einem Tisch dieser Größe statt des Standards.",
    },
    /// A seat count's label in the per-count rows
    ArrSeats { en: "{0} seats", de: "{0} Sitze" },
    /// The per-count choice that takes the default
    ArrAsDefault { en: "Default", de: "Standard" },
    /// The `?` overlay's line for the two keys
    ArrKeysHint {
        en: "P chooses the arrangement · Shift+P the next",
        de: "P wählt die Anordnung · Umschalt+P die nächste",
    },
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
    /// Unapplied host edits.
    RoomDraft { en: "Unsaved settings · apply your changes before starting.", de: "Ungespeicherte Einstellungen · übernimm deine Änderungen vor dem Start." },
    /// Room configuration title.
    RoomTitle { en: "Your table", de: "Dein Spielraum" },
    /// Public name input.
    RoomName { en: "Room name", de: "Raumname" },
    /// Shared configuration heading.
    RoomRules { en: "Rules & starting position", de: "Regeln & Startaufstellung" },
    /// Host authority explanation.
    RoomHostHelp { en: "You arrange the table. Players choose their own decks and confirm they are ready.", de: "Du konfigurierst den Tisch. Jeder Spieler wählt sein eigenes Deck und bestätigt seine Bereitschaft." },
    /// Offline room guidance: the local player controls every seat.
    RoomOfflineHelp { en: "Choose decks and AI strength. Adjust the starting rules if you like, then start the game.", de: "Wähle Decks und KI-Stärke. Passe bei Bedarf die Startregeln an und starte das Spiel." },
    /// Visitor authority explanation.
    RoomGuestHelp { en: "The host arranges the table. Choose your deck below, then mark yourself ready.", de: "Der Host konfiguriert den Tisch. Wähle unten dein Deck und bestätige anschließend mit Bereit." },
    /// Seats heading.
    RoomPlayers { en: "Players & seats", de: "Spieler & Sitze" },
    /// Global starting life.
    RoomLife { en: "Starting life", de: "Start-Lebenspunkte" },
    /// Free mulligan count.
    RoomMulligans { en: "Free mulligans", de: "Freie Mulligans" },
    /// Permanent input label.
    RoomBoard { en: "Find starting permanents", de: "Start-Permanents suchen" },
    /// Expandable starting-position editor; {0} counts configured permanents.
    RoomStartingCards { en: "Starting cards · {0}", de: "Startkarten · {0}" },
    /// Draft submit.
    RoomApply { en: "Apply settings", de: "Einstellungen übernehmen" },
    /// Optional password help.
    RoomLockHelp { en: "Leave blank to keep the current password. Changes take effect with Apply settings.", de: "Leer lassen, um das bisherige Kennwort beizubehalten. Änderungen gelten nach dem Übernehmen." },
    /// Remove lock.
    RoomUnlock { en: "Remove password", de: "Kennwort entfernen" },
    /// Preset picker heading.
    RoomTemplate { en: "Starting template", de: "Startvorlage" },
    /// Starting-position card editor.
    RoomCardAppearance { en: "Edition & finish", de: "Edition & Foil" },
    /// Starting-position card editor.
    RoomCardCounters { en: "Starting counters", de: "Startcounter" },
    /// Starting-position card editor.
    RoomCounterKind { en: "Counter type", de: "Counter-Typ" },
    /// Starting-position card editor.
    RoomCounterHint { en: "+1/+1, -1/-1, loyalty, lore, time, charge, poison, energy, rad, lifelink, level; any +X/+Y or -X/-Y pair; custom:ID", de: "+1/+1, -1/-1, loyalty, lore, time, charge, poison, energy, rad, lifelink, level; beliebige +X/+Y- oder -X/-Y-Paare; custom:ID" },
    /// Starting-position card editor.
    RoomAddCounter { en: "Add counter", de: "Counter hinzufügen" },
    /// Starting-position card editor.
    RoomRemoveCard { en: "Remove card", de: "Karte entfernen" },
    /// Starting-position card editor.
    RoomNoCardMatches { en: "No matching permanent.", de: "Kein passendes Permanent gefunden." },
    /// Accelerated original house-rule scenario.
    RoomFast { en: "Five-land start", de: "Start mit fünf Ländern" },
    /// Planechase honesty.
    RoomPlanechase { en: "Planechase · unavailable: planar cards and planar die are not implemented yet.", de: "Planechase · noch nicht verfügbar: Weltenkarten und Weltenwürfel sind noch nicht implementiert." },
    /// Host handover guarantee.
    RoomSuccession { en: "If the host leaves, the longest-standing player takes over.", de: "Verlässt der Host den Raum, übernimmt der am längsten anwesende Spieler." },
    /// Deck selector heading.
    RoomPickDeck { en: "Choose a deck", de: "Deck auswählen" },
    /// Shared default for per-seat life.
    RoomDefault { en: "Use default", de: "Standard verwenden" },
    /// Empty deck notice in room.
    RoomNoDeck { en: "No deck selected", de: "Noch kein Deck gewählt" },
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
    /// Caption over the small cards beside a permanent's preview that are
    /// attached to it (CR 701.3; #305): on the table they lie under it and
    /// may be folded out of sight, which its attachment mark says.
    CardAttached { en: "ATTACHED", de: "ANGELEGT" },
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
    /// Waiting for {0} players
    ///
    /// The bar's line to a seat that has kept while several others still
    /// decide their opening hands (#257); one other is named, with
    /// [`Phrase::WaitingForPlayer`]. `{0}` is always two or more, so there
    /// is no singular to write (German "Spieler" is both anyway).
    WaitingForPlayers { en: "Waiting for {0} players", de: "Warte auf {0} Spieler" },
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
    /// You owe mana. Activate mana abilities to pay, or pass.
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
        en: "You owe mana. Activate mana abilities to pay, or pass.",
        de: "Du schuldest Mana. Nutze Manafähigkeiten zum Bezahlen, oder passe.",
    },
    /// Pay what a payment window still owes with the lands the client picked,
    /// then settle it: the prompt's confirm button while mana is owed and
    /// the pool does not cover it yet.
    PayRemainder { en: "Pay (tap {0})", de: "Zahlen ({0} tappen)" },
    /// Pass a payment window without paying it.
    DeclinePayment { en: "Don't pay", de: "Nicht zahlen" },
    /// Settle a payment window the pool already covers.
    PayNow { en: "Pay", de: "Zahlen" },
    /// The AI log's heading line for one entry: whose mind it is.
    AiSaidHead { en: "{0} thinks", de: "{0} denkt" },
    /// What the model thought before it answered.
    AiSaidThinking { en: "Reasoning: {0}", de: "Überlegung: {0}" },
    /// The answer the model chose.
    AiSaidChose { en: "Chose: {0}", de: "Wahl: {0}" },
    /// What the answer cost.
    AiSaidCost { en: "{0} tokens, {1} s", de: "{0} Tokens, {1} s" },
    /// Private optional creature cast from a resolving Mask ability.
    MaskChoose { en: "Choose a creature to cast face down (X = {0}), or decline", de: "Wähle eine Kreatur zum verdeckten Wirken (X = {0}), oder lehne ab" },
    /// Actual receipt with fixed costs shown separately; no invented partition.
    MaskReceipt { en: "Actual mana payment: {0}; fixed activation costs: {1}", de: "Tatsächliche Manazahlung: {0}; feste Aktivierungskosten: {1}" },
    /// The controlled player's mandatory playable-card choice.
    CommandChoose { en: "Choose the card this player must play if able", de: "Wähle die Karte, die dieser Spieler nach Möglichkeit spielen muss" },
    /// An optional face-down cast's explicit confirmation.
    ConfirmFaceDown { en: "Cast face down", de: "Verdeckt wirken" },
    /// Explicitly decline an optional resolving cast.
    DeclineCast { en: "Decline casting", de: "Nicht wirken" },
    /// A text change asks two separate words.
    ChooseTextReplacement { en: "Choose the old word and its replacement", de: "Wähle das bisherige und das neue Wort" },
    /// Current explicit text-change draft, without rewriting Oracle.
    TextReplacementSummary { en: "Word change: {0} → {1}", de: "Wortänderung: {0} → {1}" },
    /// First vocabulary column.
    TextOldWord { en: "Old word", de: "Bisheriges Wort" },
    /// Second vocabulary column.
    TextNewWord { en: "New word", de: "Neues Wort" },
    /// Mandatory mana activation during a resolving instruction.
    ChooseRequiredMana { en: "Choose a land mana ability", de: "Wähle eine Manafähigkeit eines Landes" },
    /// Mandatory instruction progress, separate from the compact headline.
    RequiredManaProgress { en: "Required — {0} activations completed", de: "Verpflichtend — {0} Aktivierungen ausgeführt" },
    /// Decision controller differs from the resource owner.
    DecidingFor { en: "You decide for {0}", de: "Du entscheidest für {0}" },
    /// A replacement effect turns the affected face-down creature face up.
    DamageTurnFaceUp { en: "Turn face up before damage", de: "Vor dem Schaden aufdecken" },
    /// Compact fixed-payment guidance when temporary actions share the answer row.
    GrantedPaymentHint { en: "Produce mana or pass.", de: "Mana erzeugen oder passen." },
    /// Mana may be generated before choosing an optional amount to prevent damage.
    PrepareManaPayment {
        en: "You may generate mana to prevent up to {0} damage. Pass to choose the amount.",
        de: "Erzeuge bei Bedarf Mana gegen bis zu {0} Schaden. Passe zur Wahl des Betrags.",
    },
    /// A variable payment can exceed the damage it could prevent.
    ChooseManaPayment {
        en: "How much mana will you pay? ({0}–{1}; prevent up to {2} damage)",
        de: "Wie viel Mana zahlen? ({0}–{1}; bis zu {2} Schaden verhindern)",
    },
    /// One non-targeted, exact source selection.
    ChooseDamageSource { en: "Choose a damage source", de: "Wähle eine Schadensquelle" },
    /// Movement from the stack in the event log.
    LogFromStack { en: "from the stack", de: "vom Stapel" },
    /// Movement onto the stack in the event log.
    LogIntoStack { en: "onto the stack", de: "auf den Stapel" },
    /// The hand zone in a source description.
    SourceHand { en: "hand", de: "Hand" },
    /// The library zone in a source description.
    SourceLibrary { en: "library", de: "Bibliothek" },
    /// Incarnation still present in the game.
    SourceCurrent { en: "current", de: "aktuell" },
    /// Remembered incarnation that has left its zone.
    SourceHistorical { en: "earlier incarnation", de: "früheres Objekt" },
    /// Secondary exact-incarnation label for otherwise similar rows.
    SourceVersion { en: "incarnation {0}", de: "Objektversion {0}" },
    /// Publicly visible spell/ability reference count.
    SourceReferences { en: "{0} on the stack", de: "{0} auf dem Stapel" },
    /// Generic public label for a face-down source.
    SourceFaceDown { en: "Face-down source", de: "Verdeckte Quelle" },
    /// No entitled identity is available for the offered source.
    SourceUnknown { en: "Unknown source", de: "Unbekannte Quelle" },
    /// Damage decision: `DamageCombat`.
    DamageCombat { en: "combat damage", de: "Kampfschaden" },
    /// Damage decision: `DamageNoncombat`.
    DamageNoncombat { en: "noncombat damage", de: "Nichtkampfschaden" },
    /// Damage decision: `DamagePart`.
    DamagePart { en: "{0} → {1}: {2} ({3})", de: "{0} → {1}: {2} ({3})" },
    /// Damage decision: `DamageShield`.
    DamageShield { en: "Prevent the next damage; shield remaining: {0}", de: "Verhindere die nächsten Schadenspunkte; Schild übrig: {0}" },
    /// Damage decision: `DamageEventShield`.
    DamageEventShield { en: "Prevent this event’s damage; remaining: {0}; excess expires", de: "Verhindere Schaden dieses Ereignisses; übrig: {0}; Rest verfällt" },
    /// Damage decision: `DamagePreventCombat`.
    DamagePreventCombat { en: "Prevent all indicated combat damage", de: "Verhindere den gesamten angegebenen Kampfschaden" },
    /// Damage decision: `DamageProtection`.
    DamageProtection { en: "Protection prevents the indicated damage", de: "Schutz verhindert den angegebenen Schaden" },
    /// Damage decision: `DamageRedirect`.
    DamageRedirect { en: "Redirect the damage to {0}", de: "Leite den Schaden auf {0} um" },
    /// Finite redirection names its recipient and remaining capacity.
    DamageRedirectNext { en: "Redirect damage to {0}; remaining: {1}", de: "Schaden umleiten → {0}; noch {1}" },
    /// Damage decision: `DamageCounter`.
    DamageCounter { en: "Remove one {0} counter and try to prevent 1 damage; counters remaining: {1}", de: "Entferne eine {0}-Marke und versuche, 1 Schaden zu verhindern; Marken übrig: {1}" },
    /// Damage decision: `DamageSourceShield`.
    DamageSourceShield { en: "Use the shield against {0}; leave {1} damage unprevented", de: "Nutze das Schild gegen {0}; {1} Schaden bleibt unverhindert" },
    /// Damage decision: `DamageShieldLife`.
    DamageShieldLife { en: "; {0} gains life equal to the damage prevented", de: "; {0} erhält Lebenspunkte in Höhe des verhinderten Schadens" },
    /// Prevention assigned to one damage part.
    PreventionShare { en: "{0} — prevent {1} of at most {2}", de: "{0} — verhindere {1} von höchstens {2}" },
    /// Damage assigned to redirection rather than prevention.
    RedirectionShare { en: "{0} — redirect {1} of at most {2}", de: "{0} — leite {1} von höchstens {2} um" },
    /// Damage decision: `DamageRule`.
    DamageRule { en: "Rule effect", de: "Regeleffekt" },
    /// Explicitly unpreventable damage in a replacement decision.
    DamageUnpreventable { en: "; damage cannot be prevented", de: "; Schaden kann nicht verhindert werden" },
    /// Counter removal allocation can still consume counters without preventing damage.
    AllocateDamageCounters { en: "Distribute {0} counter removals — {1} remaining. Select a source and enter its share.", de: "Verteile {0} Markenentfernungen — {1} übrig. Wähle eine Quelle und gib ihren Anteil ein." },
    /// A counter-removal share, distinct from guaranteed prevention.
    DamageCounterShare { en: "{0} — apply {1} counter removals (maximum {2})", de: "{0} — {1} Markenentfernungen anwenden (höchstens {2})" },
    /// Choosing between damage replacement and prevention effects.
    ChooseDamageEffect { en: "Choose which effect modifies the damage next", de: "Wähle, welcher Effekt den Schaden als Nächstes verändert" },
    /// Exact prevention budget and its still unassigned portion.
    AllocatePrevention { en: "Distribute {0} prevention — {1} remaining. Select a source and enter its share.", de: "Verteile {0} Schadensverhinderung — {1} übrig. Wähle eine Quelle und gib ihren Anteil ein." },
    /// Exact redirection budget and its still unassigned portion.
    AllocateRedirection { en: "Redirect {0} damage — {1} remaining. Select a source and enter its share.", de: "{0} Schaden umleiten — {1} übrig. Wähle eine Quelle und gib ihren Anteil ein." },
    /// A payment window with no fixed amount owed.
    OptionalPayment { en: "Optional payment", de: "Freiwillige Zahlung" },
    /// {0} is the player who must sacrifice the chosen permanents.
    SacrificeForPlayer {
        en: "{0} sacrifices the chosen permanents. {1}",
        de: "{0} opfert die gewählten bleibenden Karten. {1}",
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
    /// Current attack aim and the number already assigned there.
    AttackAim { en: "Aim: {0} · {1} assigned", de: "Angriffsziel: {0} · {1} zugewiesen" },
    /// Adds undeclared creatures only; other assignments are preserved.
    AttackRemaining { en: "Attack with all remaining → {0}", de: "Mit allen übrigen angreifen → {0}" },
    /// Clears the draft without submitting it.
    AttackWithdrawAll { en: "Withdraw all attackers", de: "Alle Angreifer zurückziehen" },
    /// A single unassigned creature.
    AttackSend { en: "Attack: {0} → {1}", de: "Angreifen: {0} → {1}" },
    /// A single assigned creature, naming its existing defender.
    AttackWithdraw { en: "Withdraw: {0} → {1}", de: "Zurückziehen: {0} → {1}" },
    /// Instructions for the reversible draft.
    AttackDraftHint {
        en: "Choose a defender, then creatures. Withdraw an attacker here to reassign it. Confirm when ready.",
        de: "Angriffsziel wählen, dann Kreaturen. Zum Umverteilen hier zurückziehen. Zum Schluss bestätigen.",
    },
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
    /// card to put into your hand
    NounCardToHand { en: "card to put into your hand", de: "Karte, die auf deine Hand kommt" },
    /// cards to put into your hand
    NounCardsToHand { en: "cards to put into your hand", de: "Karten, die auf deine Hand kommen" },
    /// card to put on the bottom of your library
    NounCardToBottom {
        en: "card to put on the bottom of your library",
        de: "Karte, die unter deine Bibliothek kommt",
    },
    /// cards to put on the bottom of your library
    NounCardsToBottom {
        en: "cards to put on the bottom of your library",
        de: "Karten, die unter deine Bibliothek kommen",
    },
    /// card you may play this turn
    NounCardToPlay { en: "card you may play this turn", de: "Karte, die du in diesem Zug spielen darfst" },
    /// cards you may play this turn
    NounCardsToPlay {
        en: "cards you may play this turn",
        de: "Karten, die du in diesem Zug spielen darfst",
    },
    /// card to put onto the battlefield
    NounCardToBattlefield {
        en: "card to put onto the battlefield",
        de: "Karte, die aufs Spielfeld kommt",
    },
    /// cards to put onto the battlefield
    NounCardsToBattlefield {
        en: "cards to put onto the battlefield",
        de: "Karten, die aufs Spielfeld kommen",
    },
    /// card to put into its owner's graveyard
    NounCardToGraveyard {
        en: "card to put into its owner's graveyard",
        de: "Karte, die in den Friedhof ihres Besitzers kommt",
    },
    /// cards to put into their owner's graveyard
    NounCardsToGraveyard {
        en: "cards to put into their owner's graveyard",
        de: "Karten, die in den Friedhof ihres Besitzers kommen",
    },
    /// A private hand inspection is acknowledged without choosing a card.
    InspectHand {
        en: "Look at the hand, then confirm when finished",
        de: "Sieh dir die Hand an und bestätige, wenn du fertig bist",
    },
    /// card for the first pile — the rest are the second
    NounCardForFirstPile {
        en: "card for the first pile (the rest are the second)",
        de: "Karte für den ersten Stapel (der Rest ist der zweite)",
    },
    /// cards for the first pile — the rest are the second
    NounCardsForFirstPile {
        en: "cards for the first pile (the rest are the second)",
        de: "Karten für den ersten Stapel (der Rest ist der zweite)",
    },
    /// card from your graveyard — none searches the library instead
    NounCardFromGraveyard {
        en: "card from your graveyard, or none to search your library",
        de: "Karte aus deinem Friedhof, oder keine, um deine Bibliothek zu durchsuchen",
    },
    /// cards from your graveyard — none searches the library instead
    NounCardsFromGraveyard {
        en: "cards from your graveyard, or none to search your library",
        de: "Karten aus deinem Friedhof, oder keine, um deine Bibliothek zu durchsuchen",
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
    /// land from which to remove counters
    NounLandToRemoveCounters { en: "land to remove counters from", de: "Land, von dem die Marken entfernt werden" },
    /// lands from which to remove counters
    NounLandsToRemoveCounters { en: "lands to remove counters from", de: "Länder, von denen die Marken entfernt werden" },
    /// land to keep
    NounLandToKeep { en: "land to keep", de: "Land, das du behältst" },
    /// lands to keep
    NounLandsToKeep { en: "lands to keep", de: "Länder, die du behältst" },
    /// creature to keep
    NounCreatureToKeep { en: "creature to keep", de: "Kreatur, die du behältst" },
    /// creatures to keep
    NounCreaturesToKeep { en: "creatures to keep", de: "Kreaturen, die du behältst" },
    /// permanent to keep
    NounPermanentToKeep { en: "permanent to keep", de: "bleibende Karte, die du behältst" },
    /// permanents to keep
    NounPermanentsToKeep { en: "permanents to keep", de: "bleibende Karten, die du behältst" },
    /// hand card to keep
    NounCardToKeep { en: "hand card to keep", de: "Handkarte, die du behältst" },
    /// cards to keep
    NounCardsToKeep { en: "hand cards to keep", de: "Handkarten, die du behältst" },
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
    /// card to exile from your graveyard
    NounCardToExile {
        en: "card to exile from your graveyard",
        de: "Karte aus deinem Friedhof, die ins Exil geschickt wird",
    },
    /// cards to exile from your graveyard
    NounCardsToExile {
        en: "cards to exile from your graveyard",
        de: "Karten aus deinem Friedhof, die ins Exil geschickt werden",
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
    /// permanent to untap
    NounPermanentToUntap {
        en: "permanent to untap",
        de: "bleibende Karte, die enttappt wird",
    },
    /// permanents to untap
    NounPermanentsToUntap {
        en: "permanents to untap",
        de: "bleibende Karten, die enttappt werden",
    },
    /// card to reveal
    NounCardToReveal { en: "card to reveal", de: "Karte, die aufgedeckt wird" },
    /// cards to reveal
    NounCardsToReveal { en: "cards to reveal", de: "Karten, die aufgedeckt werden" },
    /// attacker to join the band (banding, CR 702.22c)
    NounAttackerToBand {
        en: "attacker to join the band",
        de: "Angreifer, der sich der Gruppe anschließt",
    },
    /// attackers to join the band
    NounAttackersToBand {
        en: "attackers to join the band",
        de: "Angreifer, die sich der Gruppe anschließen",
    },
    /// creature for a pile (Camouflage, the pile unnamed)
    NounCreatureForPile {
        en: "creature for this pile",
        de: "Kreatur für diesen Stapel",
    },
    /// creatures for a pile
    NounCreaturesForPile {
        en: "creatures for this pile",
        de: "Kreaturen für diesen Stapel",
    },
    /// creature for pile {0} of {1} (Camouflage: piles go to attackers at random)
    NounCreatureForPileOf {
        en: "creature for pile {0} of {1} (each pile blocks an attacker chosen at random)",
        de: "Kreatur für Stapel {0} von {1} (jeder Stapel blockt einen zufälligen Angreifer)",
    },
    /// creatures for pile {0} of {1}
    NounCreaturesForPileOf {
        en: "creatures for pile {0} of {1} (each pile blocks an attacker chosen at random)",
        de: "Kreaturen für Stapel {0} von {1} (jeder Stapel blockt einen zufälligen Angreifer)",
    },
    /// creature for the "left" pile (Raging River; the rest go right)
    NounCreatureForLeft {
        en: "creature for the \"left\" pile (the rest go right)",
        de: "Kreatur für den \"linken\" Stapel (der Rest geht nach rechts)",
    },
    /// creatures for the "left" pile
    NounCreaturesForLeft {
        en: "creatures for the \"left\" pile (the rest go right)",
        de: "Kreaturen für den \"linken\" Stapel (der Rest geht nach rechts)",
    },
    /// Raging River: a label for an attacker, the pile that may block it
    ChooseRiverLabel {
        en: "Choose \"left\" (first) or \"right\": only that pile and fliers may block it",
        de: "Wähle \"links\" (zuerst) oder \"rechts\": nur dieser Stapel und Flieger dürfen blocken",
    },
    /// attacker to block (False Orders' re-block, its blocker unnamed)
    NounAttackerToBlock {
        en: "attacker to block",
        de: "Angreifer, der geblockt wird",
    },
    /// attackers to block
    NounAttackersToBlock {
        en: "attackers to block",
        de: "Angreifer, die geblockt werden",
    },
    /// attacker for {0} to block
    NounAttackerForBlocker {
        en: "attacker for {0} to block",
        de: "Angreifer, den {0} blockt",
    },
    /// attackers for {0} to block
    NounAttackersForBlocker {
        en: "attackers for {0} to block",
        de: "Angreifer, die {0} blockt",
    },
    /// attacker to band with {0} (banding, CR 702.22c): the leader named
    NounAttackerToBandWith {
        en: "attacker to band with {0}",
        de: "Angreifer, der mit {0} eine Gruppe bildet",
    },
    /// attackers to band with {0}
    NounAttackersToBandWith {
        en: "attackers to band with {0}",
        de: "Angreifer, die mit {0} eine Gruppe bilden",
    },
    /// Convoke or waterbend: tap permanents to help pay.
    ///
    /// Neutral, because the question does not say which keyword asked it:
    /// both arrive as `TargetPrompt::Convoke`, and they tap different things
    /// (CR 702.51a creatures, CR 701.67a artifacts and creatures). The
    /// permanents it may tap are the lit ones, and naming a type here was
    /// wrong for one of the two.
    TapToHelpPay { en: "Tap permanents to help pay — each pays for one", de: "Tappe bleibende Karten, um mitzubezahlen — jede zahlt eins" },
    /// Delve: exile cards from your graveyard to help pay
    DelveToHelpPay { en: "Exile cards from your graveyard to help pay — each pays for one", de: "Schicke Karten aus deinem Friedhof ins Exil, um mitzubezahlen — jede zahlt eine" },
    /// Crew N: tap creatures with total power {0} or more
    CrewWithPower { en: "Crew {0}: tap any number of your other untapped creatures with total power {0} or more", de: "Besatzung {0}: Tappe beliebig viele deiner anderen ungetappten Kreaturen mit einer Gesamtstärke von {0} oder mehr" },
    /// Choose up to {0} {1}
    ChooseUpTo { en: "Choose up to {0} {1}", de: "Wähle bis zu {0} {1}" },
    /// A resolving effect changes one target at a time.
    ChooseNewTarget { en: "Choose a new target", de: "Wähle ein neues Ziel" },
    /// Selecting nothing preserves the current target of the spell or copy.
    ChooseNewTargetOrKeep { en: "Choose a new target, or keep this target without selecting one", de: "Neues Ziel wählen oder ohne Auswahl dieses Ziel behalten" },
    /// Identifies the current target before asking whether to change it.
    RetargetContext { en: "Target {0} of {1}: {2}. {3}", de: "Ziel {0} von {1}: {2}. {3}" },
    /// An exact former target, without exposing its internal version number.
    TargetBeforeZoneChange { en: "before it changed zones", de: "vor dem Zonenwechsel" },
    /// A returned card must be chosen anew to replace its earlier identity.
    TargetReturnedHint { en: "To target a returned card, select it again below.", de: "Soll die zurückgekehrte Karte das Ziel sein, wähle sie unten erneut." },
    /// Identifies the original target without repeating selection instructions.
    RetargetOriginal { en: "Previous target {0} of {1}: {2}", de: "Bisheriges Ziel {0} von {1}: {2}" },
    /// A former object target may no longer be visible in the current view.
    PreviousTarget { en: "previous target", de: "bisheriges Ziel" },
    /// Confirm an optional retarget with no selected replacement.
    KeepCurrentTarget { en: "Keep current target", de: "Bisheriges Ziel behalten" },
    /// Confirm a selected replacement target.
    ChangeTarget { en: "Change target", de: "Ziel ändern" },
    /// Choose {0} {1}
    ChooseExactly { en: "Choose {0} {1}", de: "Wähle {0} {1}" },
    /// Choose {0}–{1} {2}
    ChooseBetween { en: "Choose {0}–{1} {2}", de: "Wähle {0}–{1} {2}" },
    /// Choose a creature type
    ChooseCreatureType { en: "Choose a creature type", de: "Wähle einen Kreaturtyp" },
    /// Choose a basic land type (Phantasmal Terrain)
    ChooseBasicLandType { en: "Choose a basic land type", de: "Wähle einen Standardlandtyp" },
    /// Choose a card name (Pithing Needle)
    ChooseCardName { en: "Choose a card name", de: "Wähle einen Kartennamen" },
    /// Choose a colour
    ChooseColour { en: "Choose a colour", de: "Wähle eine Farbe" },
    /// Counter amount chosen while an effect resolves.
    ChooseCounters { en: "How many {0}? ({1}–{2})", de: "Wie viele {0}? ({1}–{2})" },
    /// Counter amount with the recipient named.
    ChooseCountersNamed { en: "How many {0} on {1}? ({2}–{3})", de: "Wie viele {0} auf {1}? ({2}–{3})" },
    /// Choose a number ({0}–{1})
    ChooseNumberIn { en: "Choose a number ({0}–{1})", de: "Wähle eine Zahl ({0}–{1})" },
    /// One target's share of damage divided as the player chooses (Fury).
    DamageShare {
        en: "Damage to target {0} of {1}, {2} left to divide ({3}–{4})",
        de: "Schaden an Ziel {0} von {1}, noch {2} zu verteilen ({3}–{4})",
    },
    /// One creature's share of combat damage this player divides
    /// (CR 510.1c–d, 702.22j–k), when the creatures cannot be named.
    CombatDamageShare {
        en: "Combat damage to creature {0} of {1}, {2} left to divide ({3}–{4})",
        de: "Kampfschaden an Kreatur {0} von {1}, noch {2} zu verteilen ({3}–{4})",
    },
    /// The same share, naming the creature dealing the damage and the one
    /// the share goes to.
    CombatDamageShareNamed {
        en: "Combat damage from {0} to {1} ({2} of {3}), {4} left to divide ({5}–{6})",
        de: "Kampfschaden von {0} an {1} ({2} von {3}), noch {4} zu verteilen ({5}–{6})",
    },
    /// Replicate {0}: pay it how many times? ({1}–{2})
    ReplicateHowOften {
        en: "Replicate {0}: pay it how many times? ({1}–{2})",
        de: "Replikation {0}: wie oft zahlen? ({1}–{2})",
    },
    /// Choose a player
    ChoosePlayer { en: "Choose a player", de: "Wähle einen Spieler" },
    /// Choose how to cast
    ChooseHowToCast { en: "Choose how to cast", de: "Wähle, wie gewirkt wird" },
    /// Choose a pile to put into your hand (Fact or Fiction)
    ChoosePileForHand {
        en: "Choose a pile for your hand; the other goes to your graveyard",
        de: "Wähle einen Stapel für deine Hand; der andere kommt in deinen Friedhof",
    },
    /// Pile {0}: {1} — one row of a pile choice
    PileRow { en: "Pile {0}: {1}", de: "Stapel {0}: {1}" },
    /// no cards — an empty pile's row
    EmptyPile { en: "no cards", de: "keine Karten" },
    /// Mode {0}
    ///
    /// Only where a mode has no printed sentence: a modal trigger that states
    /// its choice inside one sentence, or a mode that declines.
    CastModeNumber { en: "Mode {0}", de: "Modus {0}" },
    /// Back face
    CastBackFace { en: "Back face", de: "Rückseite" },
    /// Play as a land
    CastLandFace { en: "Play as a land", de: "Als Land spielen" },
    /// Miracle
    CastMiracle { en: "Miracle", de: "Wunder" },
    /// Flashback
    CastFlashback { en: "Flashback", de: "Rückblende" },
    /// Dash (CR 702.109a), the keyword as the German printing names it
    CastDash { en: "Dash", de: "Sturmangriff" },
    /// Escape (CR 702.138a), the keyword as the German printing names it
    CastEscape { en: "Escape", de: "Befreiung" },
    /// Click a card in your hand
    HintClickHand { en: "Click a card in your hand", de: "Klicke eine Karte auf deiner Hand an" },
    /// Click what you are choosing
    HintClickBoard { en: "Click what you are choosing", de: "Klicke an, was du wählst" },
    /// Type to narrow the list
    HintTypeToFilter { en: "Type to narrow the list", de: "Tippe, um die Liste einzugrenzen" },
    /// Put these in order
    PutInOrder { en: "Put these in order", de: "Bringe diese in eine Reihenfolge" },
    /// The first card in a graveyard ordering will be on top of the others.
    OrderGraveyard {
        en: "Order graveyard cards: first card on top",
        de: "Friedhof ordnen: erste Karte oben",
    },
    /// Put these back on top: the first is the new top card
    OrderOnTop {
        en: "Put these back on top: the first is the new top card",
        de: "Lege diese oben zurück: die erste ist die neue oberste Karte",
    },
    /// Put these on the bottom: the last is the bottom card
    OrderOnBottom {
        en: "Put these on the bottom: the last is the bottom card",
        de: "Lege diese unter die Bibliothek: die letzte ist die unterste Karte",
    },
    /// A scry (CR 701.22a): the looked-at cards go back on top or under the
    /// library, each pile in an order.
    ScryPrompt {
        en: "Scry: keep cards on top or put them on the bottom",
        de: "Hellsicht: Karten oben lassen oder unter die Bibliothek legen",
    },
    /// A surveil (CR 701.25a): the looked-at cards go back on top or into
    /// the graveyard.
    SurveilPrompt {
        en: "Surveil: keep cards on top or put them into your graveyard",
        de: "Überwachen: Karten oben lassen oder auf deinen Friedhof legen",
    },
    /// Library of Leng: cards an effect makes you discard go into the
    /// graveyard, or on top of your library in an order.
    DiscardToLibraryPrompt {
        en: "Discard: into your graveyard, or on top of your library instead",
        de: "Abwerfen: auf deinen Friedhof oder stattdessen oben auf deine Bibliothek",
    },
    /// The game is over
    TheGameIsOver { en: "The game is over", de: "Das Spiel ist vorbei" },
    /// Pay {0} life? Otherwise it enters tapped
    PayLifeOrTapped {
        en: "Pay {0} life? Otherwise it enters tapped",
        de: "{0} Leben zahlen? Sonst kommt es getappt ins Spiel",
    },
    /// Pay the additional cost?
    PayAdditionalCost { en: "Pay the additional cost?", de: "Die zusätzlichen Kosten zahlen?" },
    /// Optional life payment for a resolving ability.
    PayLife { en: "Pay {0} life?", de: "{0} Lebenspunkte bezahlen?" },
    /// Pay {{0}}?
    PayTax { en: "Pay {{0}}?", de: "{{0}} zahlen?" },
    /// Pay a price with colour in it; the cost renders its own braces.
    PayMana { en: "Pay {0}?", de: "{0} zahlen?" },
    /// The player may activate mana abilities before paying a pact.
    PayPact {
        en: "Pact: pay {0}. Make mana now? If you decline or finish without enough mana, you lose the game.",
        de: "Pakt: {0} bezahlen. Jetzt Mana erzeugen? Wenn du ablehnst oder ohne genug Mana abschließt, verlierst du das Spiel.",
    },
    /// Cast it for its miracle cost?
    CastForMiracle { en: "Cast it for its miracle cost?", de: "Für die Wunderkosten wirken?" },
    /// Cast it without paying its mana cost? (cascade)
    CastWithoutPaying {
        en: "Cast it without paying its mana cost?",
        de: "Ohne Zahlung seiner Manakosten wirken?",
    },
    /// Cast it now, paying its mana cost? (Conduit of Worlds)
    CastPaying {
        en: "Cast it now, paying its mana cost? You make the mana first.",
        de: "Jetzt wirken und seine Manakosten bezahlen? Du erzeugst zuerst das Mana.",
    },
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
    /// Your card goes into your library: the top, or the bottom? (Subtlety)
    TopOfLibraryOrBottom {
        en: "Your card goes into your library. On top? (No puts it on the bottom.)",
        de: "Deine Karte kommt in deine Bibliothek. Oben drauf? (Nein legt sie unter.)",
    },
    /// One card of a named card type into the hand, or none (Atraxa).
    TakeOneOfType {
        en: "Put up to one {0} card into your hand",
        de: "Nimm bis zu eine Karte vom Typ {0} auf deine Hand",
    },
    /// Cast the discovered card for free, or take it into the hand?
    CastDiscovered {
        en: "You discovered this card. Cast it without paying its mana cost? (No puts it into your hand.)",
        de: "Du hast diese Karte entdeckt. Ohne ihre Manakosten zu bezahlen wirken? (Nein nimmt sie auf deine Hand.)",
    },
    /// A turn replacement with an explicit turn cost, not a free optional effect.
    SkipTurnToUntap {
        en: "Skip this turn to untap this permanent?",
        de: "Diesen Zug überspringen, um diese bleibende Karte zu enttappen?",
    },
    /// Yes or no?
    YesOrNo { en: "Yes or no?", de: "Ja oder nein?" },
    /// Offer a draw
    OfferADraw { en: "Offer a draw", de: "Remis anbieten" },
    /// Concede
    Concede { en: "Concede", de: "Aufgeben" },
    /// Concede, armed and waiting for the second press.
    ConcedeConfirm { en: "Concede? Press again", de: "Aufgeben? Nochmal drücken" },
    /// The game log panel's title (#262). Not a `Log…` phrase: those are the
    /// book's own sentences, which `gamelog` writes; these are the panel's.
    GameLogTitle { en: "Game log", de: "Spielprotokoll" },
    /// The game log panel before its first line has arrived.
    GameLogEmpty { en: "Nothing has happened yet", de: "Noch ist nichts passiert" },
    /// The game log panel has been scrolled up, and lines arrived under it.
    GameLogNewBelow { en: "New lines below", de: "Neue Zeilen unten" },
    /// The head of the sheet that holds up cards another seat revealed.
    /// `{0}` is the seat's name. Present tense, unlike the log's line: the
    /// sheet stands while the cards are being shown (CR 701.20a).
    RevealedBy { en: "{0} reveals", de: "{0} zeigt offen vor" },
    /// Under that head: how many more reveals wait behind this one.
    RevealedWaiting { en: "{0} more to come", de: "{0} weitere folgen" },
    /// The indicator that says this seat is not being asked right now.
    HoldingPriority { en: "Not asking you", de: "Du wirst nicht gefragt" },
    /// The button that cancels a running hold.
    HoldRelease { en: "Ask me again", de: "Wieder fragen" },
    /// The armed button for a spell or a land: pressing it sends the card.
    ArmedPlay { en: "Play this card", de: "Diese Karte spielen" },
    /// Explicit confirmation of the ability shown in the retained dialog.
    ArmedActivate { en: "Activate ability", de: "Fähigkeit aktivieren" },
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
    /// Confirm an ability and the automatic taps that pay its mana.
    ArmedPayAndActivate { en: "Pay {0} and activate", de: "{0} zahlen und aktivieren" },
    /// The armed button for suspending a card whose cost still has to be tapped.
    ///
    /// `{0}` is the suspend cost, drawn as pips like [`Self::ArmedPayAndCast`]
    /// — and it is the *suspend* cost, which is a different number from the
    /// card's own: Ancestral Vision prints no mana cost and suspends for
    /// `{U}`.
    ArmedSuspend { en: "Pay {0} and suspend", de: "{0} zahlen und aussetzen" },
    /// Heading of the public suspend queue.
    SuspendedCards { en: "Suspended cards", de: "Ausgesetzte Karten" },
    /// Card name, owner and remaining time counters.
    SuspendedCard { en: "{0} · {1}\n{2} time counters · inspect", de: "{0} · {1}\n{2} Zeitmarken · ansehen" },
    /// Confirm a suspend action with no outstanding mana payment.
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
    /// Target selection explanation or explicit shortcut.
    TargetingFor { en: "Targets for {0}", de: "Ziele für {0}" },
    /// Target selection explanation or explicit shortcut.
    TargetingSecond { en: "Second target clause", de: "Zweites Ziel des Effekts" },
    /// Target selection explanation or explicit shortcut.
    /// Exact target count, available choices and current selection.
    TargetingExactly { en: "Selected: {3}/{0} • {2} available targets", de: "Ausgewählt: {3}/{0} • {2} mögliche Ziele" },
    /// Permitted target range and current selection.
    TargetingChoices { en: "Choose {0}–{1} • {2} legal choices • {3} selected", de: "Wähle {0}–{1} • {2} gültige Ziele • {3} ausgewählt" },
    /// Target selection explanation or explicit shortcut.
    TargetingBatch { en: "Confirm for all {0}", de: "Für alle {0} bestätigen" },
    /// Target selection explanation or explicit shortcut.
    TargetingBatchHint { en: "{0} identical triggers waiting. Confirm once, or use this selection for all {0}.", de: "{0} gleiche Auslöser warten. Einzeln bestätigen oder diese Auswahl für alle {0} übernehmen." },
    /// Target selection explanation or explicit shortcut.
    TargetingSelected { en: "Selected: {0}", de: "Ausgewählt: {0}" },
    /// Clear the target list's controller filter.
    TargetingAllSeats { en: "All players", de: "Alle Spieler" },
    /// Current page and total legal choices in the filtered list.
    TargetingPage { en: "Choices {0}–{1} of {2}. You can also select on the table.", de: "Ziele {0}–{1} von {2}. Auswahl auch direkt am Tisch möglich." },
    /// Cast with its optional additional kicker cost paid.
    CastKicked { en: "With kicker", de: "Mit Bonuskosten" },
    /// Cast face down using disguise.
    CastDisguise { en: "Disguise — 2/2, ward {2}", de: "Verkleidung — 2/2, Abwehr {2}" },
    /// Face-up special action.
    TurnFaceUp { en: "Turn face up", de: "Aufdecken" },
    /// Unlock {0}: a Room's special action, naming the door it opens.
    UnlockDoor { en: "Unlock {0}", de: "{0} aufschließen" },
    /// Cast using prototype characteristics.
    CastPrototype { en: "Prototype", de: "Prototyp" },
    /// Confirm the current selection.
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
    /// Resolve the stack while retaining manual choices.
    StackRun { en: "Resolve stack", de: "Stack abarbeiten" },
    /// Stack automation control.
    StackRunTo { en: "Resolve up to selection", de: "Bis zur Markierung abarbeiten" },
    /// Stack automation control.
    StackStopHint { en: "Stop before the marked ability. Click again to clear.", de: "Stopp vor der markierten Fähigkeit. Erneut klicken zum Aufheben." },
    /// Stack automation control.
    StackSelectHint { en: "Select an entry to stop before it.", de: "Eintrag markieren, um davor anzuhalten." },
    /// Stack automation control.
    StackAbilityPolicy { en: "For this card ability", de: "Für diese Kartenfähigkeit" },
    /// Stack automation control.
    StackAlwaysPass { en: "Always pass", de: "Immer passen" },
    /// Stack automation control.
    StackAsk { en: "Always ask", de: "Immer fragen" },
    /// Stack automation control.
    StackAlwaysYes { en: "Always yes", de: "Immer Ja" },
    /// Stack automation control.
    StackAlwaysNo { en: "Always no", de: "Immer Nein" },
    /// Stack automation control.
    StackResetRules { en: "Reset ability automation", de: "Fähigkeitsautomatik zurücksetzen" },
    /// Stack automation control.
    StackNoPolicy { en: "This entry has no reusable card ability.", de: "Für diesen Eintrag ist keine dauerhafte Kartenfähigkeit verfügbar." },
    /// Stack automation control.
    StackPolicyHint { en: "Targets and payments still require your decision.", de: "Ziele und Zahlungen entscheidest du weiterhin selbst." },
    /// Stack panel title.
    StackRuleName { en: "{0} · Ability {1}", de: "{0} · Fähigkeit {1}" },
    /// Holographic card finish.
    FinishHolographic { en: "Holographic", de: "Holografisch" },
    /// Glitter card finish.
    FinishGlitter { en: "Glitter", de: "Glitzer" },
    /// Galaxy card finish.
    FinishGalaxy { en: "Galaxy", de: "Galaxie" },
    /// Stack panel title.
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
    /// The table refused this client's protocol, and speaks a newer one
    /// (#271). No numbers: the log has both, and the player can act only on
    /// which side is behind.
    ClientOutdated {
        en: "Your client is out of date: update it to join.",
        de: "Dein Client ist veraltet: Aktualisiere ihn, um beizutreten.",
    },
    /// The table refused this client's protocol, and speaks an older one
    /// (#271).
    TableOlder {
        en: "This table runs an older version than your client.",
        de: "Dieser Tisch läuft mit einer älteren Version als dein Client.",
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
    /// A seat's plate and chip, said in words (tooltip, accessible name).
    PlateLife { en: "{0} life", de: "{0} Leben" },
    /// Cards in a seat's hand.
    PlateHand { en: "{0} in hand", de: "{0} auf der Hand" },
    /// Cards in a seat's library.
    PlateLibrary { en: "{0} in library", de: "{0} in der Bibliothek" },
    /// Cards in a seat's graveyard.
    PlateGraveyard { en: "{0} in graveyard", de: "{0} im Friedhof" },
    /// Cards in a seat's public exile.
    PlateExile { en: "{0} in exile", de: "{0} im Exil" },
    /// Poison counters on a seat.
    PlatePoison { en: "{0} poison", de: "{0} Gift" },
    /// Energy counters on a seat.
    PlateEnergy { en: "{0} energy", de: "{0} Energie" },
    /// The most combat damage one commander has dealt a seat.
    PlateCommander { en: "{0} commander damage", de: "{0} Kommandeurschaden" },
    /// The monarch designation (CR 724), beside the crown.
    PlateMonarch { en: "Monarch", de: "Monarch" },
    /// A seat that is out of the game.
    PlateLost { en: "Out of the game", de: "Ausgeschieden" },
    /// The seat whose turn it is (the ☀ tag's words).
    PlateTurn { en: "Their turn", de: "Am Zug" },
    /// The seat the table waits for (the ⌛ tag's words).
    PlateWaiting { en: "The table waits for them", de: "Der Tisch wartet" },
    /// Badge on the player whose turn is in progress (not merely priority).
    ActiveTurn { en: "Turn", de: "Am Zug" },
    /// The creature type publicly named for this permanent.
    ChosenType { en: "Chosen: {0}", de: "Gewählt: {0}" },
    /// The card name publicly chosen for this permanent (Pithing Needle).
    ChosenName { en: "Named: {0}", de: "Genannt: {0}" },
    /// A Room's doors still locked, by name.
    LockedDoors { en: "Locked: {0}", de: "Verschlossen: {0}" },
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
    /// Granted ability
    ///
    /// An ability another permanent handed this one. It has no position on
    /// the card to be numbered by, so "Ability 1" would be a lie about a
    /// printed ability that is also there.
    GrantedAbility { en: "Granted ability", de: "Verliehene Fähigkeit" },
    /// Rules text unavailable
    NoRulesTextHere { en: "Rules text unavailable", de: "Regeltext nicht verfügbar" },

    /// Startup and game preparation use the same original portal scene.
    ArrivalLogin { en: "At the threshold", de: "An der Schwelle" },
    /// The table is being prepared beneath the cover.
    ArrivalTable { en: "One table. A new story.", de: "Ein Tisch. Eine neue Geschichte." },
    /// Preparation failed; the player can leave the table.
    ArrivalHeld { en: "The journey is on hold", de: "Die Reise wartet" },
    /// The prepared destination is being revealed.
    ArrivalPortal { en: "The portal opens", de: "Das Portal öffnet sich" },
    /// Counts human seats that have acknowledged render readiness.
    ArrivalPlayers { en: "Waiting for the table  ·  {0} / {1} ready", de: "Warte auf die Runde  ·  {0} / {1} bereit" },
    /// Required fonts and local assets.
    ArrivalWorld { en: "Awakening the world", de: "Die Welt erwacht" },
    /// All initial gateway probes, including failed replies.
    ArrivalGateways { en: "Checking gateways", de: "Gateways werden geprüft" },
    /// Opening snapshot and visible card textures.
    ArrivalCards { en: "Preparing cards and table", de: "Karten und Tisch werden vorbereitet" },
    /// The render thread is preparing the covered scene.
    ArrivalGraphics { en: "Setting the scene", de: "Der letzte Feinschliff" },
    /// Local preparation has completed.
    ArrivalReady { en: "Everything is ready", de: "Alles bereit" },
    /// The three chapters of the game entrance.
    ArrivalTableSteps { en: "PREPARE    ·    GATHER    ·    ENTER", de: "VORBEREITEN    ·    VERSAMMELN    ·    EINTRETEN" },
    /// The three chapters of startup.
    ArrivalLoginSteps { en: "WORLD    ·    CONNECTION    ·    ARRIVAL", de: "WELT    ·    VERBINDUNG    ·    ANKUNFT" },
    /// Cancel preparation, including a failed connection.
    ArrivalLeave { en: "Return to lobby  ·  Esc", de: "Zur Lobby  ·  Esc" },
    /// Direct seat launches have no lobby to return to.
    ArrivalLeaveTable { en: "Leave table  ·  Esc", de: "Tisch verlassen  ·  Esc" },
    /// A recoverable preparation timeout.
    ArrivalTimeout { en: "Loading could not finish. Check the connection and try again.", de: "Das Laden konnte nicht abgeschlossen werden. Prüfe die Verbindung und versuche es erneut." },
    /// Required local files are missing; the default font keeps this readable.
    ArrivalAssetsFailed { en: "Required interface files could not load. Please restart or reinstall the client.", de: "Benötigte Dateien für die Oberfläche konnten nicht geladen werden. Bitte starte den Client neu oder installiere ihn erneut." },
    /// A failed render pipeline cannot safely reveal the scene.
    ArrivalGraphicsFailed { en: "The graphics could not be prepared. Please restart the client.", de: "Die Grafik konnte nicht vorbereitet werden. Bitte starte den Client neu." },

    /// Taking your seat
    VeilTakingSeat { en: "Taking your seat", de: "Nehme deinen Platz ein" },
    /// Talking to the gateway
    VeilTalking {
        en: "Talking to the gateway",
        de: "Spreche mit dem Gateway",
    },
    /// The same veil offline, where there is nobody to talk to.
    VeilWorking { en: "One moment", de: "Einen Moment" },
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
    /// Why the reading seat lost: its life total (`LossCause::Life`,
    /// CR 104.3b).
    ///
    /// Every loss line is in the past tense and says only what
    /// `SeatView::loss` records: the rule that took the seat out, never how
    /// it got there.
    LostLifeYou {
        en: "Your life fell to 0 or less",
        de: "Deine Lebenspunkte fielen auf 0 oder weniger",
    },
    /// Why another seat lost: its life total. `{0}` is the seat's name.
    LostLifeOther {
        en: "{0}'s life fell to 0 or less",
        de: "Die Lebenspunkte von {0} fielen auf 0 oder weniger",
    },
    /// Why the reading seat lost: a draw from an empty library
    /// (`LossCause::EmptyDraw`, CR 104.3c).
    LostEmptyDrawYou {
        en: "You tried to draw from an empty library",
        de: "Du wolltest aus einer leeren Bibliothek ziehen",
    },
    /// Why another seat lost: a draw from an empty library.
    LostEmptyDrawOther {
        en: "{0} tried to draw from an empty library",
        de: "{0} wollte aus einer leeren Bibliothek ziehen",
    },
    /// Why the reading seat lost: poison (`LossCause::Poison`, CR 104.3d).
    LostPoisonYou {
        en: "You had ten or more poison counters",
        de: "Du hattest zehn oder mehr Giftmarken",
    },
    /// Why another seat lost: poison.
    LostPoisonOther {
        en: "{0} had ten or more poison counters",
        de: "{0} hatte zehn oder mehr Giftmarken",
    },
    /// Why the reading seat lost: one commander's combat damage
    /// (`LossCause::CommanderDamage`, CR 903.10a).
    LostCommanderDamageYou {
        en: "You took 21 or more combat damage from one commander",
        de: "Du hast 21 oder mehr Kampfschaden von einem Kommandeur erhalten",
    },
    /// Why another seat lost: one commander's combat damage.
    LostCommanderDamageOther {
        en: "{0} took 21 or more combat damage from one commander",
        de: "{0} hat 21 oder mehr Kampfschaden von einem Kommandeur erhalten",
    },
    /// Why the reading seat lost: it conceded (`LossCause::Conceded`,
    /// CR 104.3a).
    LostConcededYou {
        en: "You conceded",
        de: "Du hast aufgegeben",
    },
    /// Why another seat lost: it conceded.
    LostConcededOther {
        en: "{0} conceded",
        de: "{0} hat aufgegeben",
    },
    /// Why the reading seat lost: an effect said so (`LossCause::Effect`,
    /// CR 104.3e), such as a pact left unpaid.
    LostEffectYou {
        en: "An effect made you lose",
        de: "Ein Effekt ließ dich verlieren",
    },
    /// Why another seat lost: an effect said so.
    LostEffectOther {
        en: "An effect made {0} lose",
        de: "Ein Effekt ließ {0} verlieren",
    },
    /// The reading seat's clock answered its last decision
    /// (`HouseAnswer::Clock`): the socket was there and the time ran out.
    /// Beside a loss, this is a game lost to the clock.
    HouseClockYou {
        en: "Your time ran out, and the house answered your last decision",
        de: "Deine Zeit lief ab, und das Haus traf deine letzte Entscheidung",
    },
    /// Another seat's clock answered its last decision.
    HouseClockOther {
        en: "{0}'s time ran out, and the house answered their last decision",
        de: "Die Zeit von {0} lief ab, und das Haus traf die letzte Entscheidung",
    },
    /// The house answered the reading seat's last decision while it had no
    /// socket (`HouseAnswer::StandIn`).
    HouseStandInYou {
        en: "You were not connected, and the house answered your last decision",
        de: "Du warst nicht verbunden, und das Haus traf deine letzte Entscheidung",
    },
    /// The house answered another seat's last decision while it had no
    /// socket.
    HouseStandInOther {
        en: "{0} was not connected, and the house answered their last decision",
        de: "{0} war nicht verbunden, und das Haus traf die letzte Entscheidung",
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
    BrowseLooking { en: "Shown", de: "Gezeigt" },
    /// Offered battlefield cards inside a mixed-zone question.
    BrowseBattlefield { en: "Battlefield", de: "Spielfeld" },
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
        en: "tap a card, then the card it goes in front of or a pile's end",
        de: "Karte antippen, dann die, vor die sie soll, oder ein Stapelende",
    },
    /// An arrangement's pile that goes back on top of the library, its
    /// first card the new top card.
    ArrangeLibraryTop { en: "On top of the library", de: "Oben auf die Bibliothek" },
    /// An arrangement's pile that goes under the library, its last card the
    /// new bottom card.
    ArrangeLibraryBottom { en: "Under the library", de: "Unter die Bibliothek" },
    /// An arrangement's pile that goes into the graveyard.
    ArrangeGraveyard { en: "Into the graveyard", de: "Auf den Friedhof" },
    /// The cards of an arrangement not yet put in any pile.
    ArrangeUnplaced { en: "Not placed yet", de: "Noch nicht gelegt" },
    /// The end of a pile, while a card is held: a tap puts it there.
    ArrangePutHere { en: "Put it here", de: "Hierher legen" },
    /// A pile nothing has been put in yet, while no card is held.
    ArrangeEmptyPile { en: "Empty", de: "Leer" },
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

    // ---- the game log (#262) ---------------------------------------------
    //
    // One line per `baylee_view::LogEvent`, written by `crate::gamelog`. A
    // line about a player is written twice, once for the reading seat in the
    // second person and once for any other seat by name, because the verb
    // agrees with its subject in both languages. No line ends in a full stop:
    // the log reuses the loss sentences (`LostLifeYou` …), which have none.
    // `{0}` is always the player a line is about, even where the reading
    // seat's form does not say it.

    /// A card a seat may not see.
    LogACard { en: "a card", de: "eine Karte" },
    /// A face-down card a seat may not look at (CR 708.5).
    LogAFaceDownCard { en: "a face-down card", de: "eine verdeckte Karte" },
    /// A planeswalker attacked that the reading seat's view no longer shows.
    LogAPlaneswalker { en: "a planeswalker", de: "einen Planeswalker" },
    /// The last two names of a list, joined.
    LogAnd { en: "{0} and {1}", de: "{0} und {1}" },
    /// The reading seat inside a list of winners, never first.
    LogYouInList { en: "you", de: "du" },
    /// A line that happened more than once in a row. `{0}` is the line, `{1}`
    /// how many times.
    LogRepeated { en: "{0} (×{1})", de: "{0} (×{1})" },
    /// The heading of the reading seat's own turn. `{1}` is the turn number.
    LogTurnYou { en: "Turn {1} · your turn", de: "Zug {1} · dein Zug" },
    /// The heading of another seat's turn.
    LogTurn { en: "Turn {1} · {0}", de: "Zug {1} · {0}" },
    /// The reading seat took a mulligan.
    LogMulliganYou { en: "{7} took a mulligan", de: "{7} hast einen Mulligan genommen" },
    /// Another seat took a mulligan.
    LogMulligan { en: "{0} took a mulligan", de: "{0} hat einen Mulligan genommen" },
    /// The reading seat kept one card. `{1}` is the count.
    LogKeptCardYou { en: "{7} kept {1} card", de: "{7} hast {1} Karte behalten" },
    /// The reading seat kept its hand. `{1}` is the count.
    LogKeptCardsYou { en: "{7} kept {1} cards", de: "{7} hast {1} Karten behalten" },
    /// Another seat kept one card.
    LogKeptCard { en: "{0} kept {1} card", de: "{0} hat {1} Karte behalten" },
    /// Another seat kept its hand.
    LogKeptCards { en: "{0} kept {1} cards", de: "{0} hat {1} Karten behalten" },
    /// The decision clock passed priority for the reading seat.
    LogTimedPassedYou {
        en: "{7} ran out of time and passed",
        de: "{7} hattest keine Zeit mehr und hast gepasst",
    },
    /// The decision clock passed priority for another seat.
    LogTimedPassed {
        en: "{0} ran out of time and passed",
        de: "{0} hatte keine Zeit mehr und hat gepasst",
    },
    /// The decision clock kept the reading seat's opening hand.
    LogTimedKeptYou {
        en: "{7} ran out of time and kept your hand",
        de: "{7} hattest keine Zeit mehr und hast deine Hand behalten",
    },
    /// The decision clock kept another seat's opening hand.
    LogTimedKept {
        en: "{0} ran out of time and kept their hand",
        de: "{0} hatte keine Zeit mehr und hat die Hand behalten",
    },
    /// The decision clock declared no attackers for the reading seat.
    LogTimedNoAttackersYou {
        en: "{7} ran out of time and did not attack",
        de: "{7} hattest keine Zeit mehr und hast nicht angegriffen",
    },
    /// The decision clock declared no attackers for another seat.
    LogTimedNoAttackers {
        en: "{0} ran out of time and did not attack",
        de: "{0} hatte keine Zeit mehr und hat nicht angegriffen",
    },
    /// The decision clock declared no blockers for the reading seat.
    LogTimedNoBlockersYou {
        en: "{7} ran out of time and did not block",
        de: "{7} hattest keine Zeit mehr und hast nicht geblockt",
    },
    /// The decision clock declared no blockers for another seat.
    LogTimedNoBlockers {
        en: "{0} ran out of time and did not block",
        de: "{0} hatte keine Zeit mehr und hat nicht geblockt",
    },
    /// The decision clock said no for the reading seat.
    LogTimedDeclinedYou {
        en: "{7} ran out of time and declined",
        de: "{7} hattest keine Zeit mehr und hast abgelehnt",
    },
    /// The decision clock said no for another seat.
    LogTimedDeclined {
        en: "{0} ran out of time and declined",
        de: "{0} hatte keine Zeit mehr und hat abgelehnt",
    },
    /// The house chose for the reading seat when its clock ran out.
    LogTimedChosenYou {
        en: "{7} ran out of time and the house chose for {8}",
        de: "{7} hattest keine Zeit mehr, und das Haus hat für {8} gewählt",
    },
    /// The house chose for another seat when its clock ran out.
    LogTimedChosen {
        en: "{0} ran out of time and the house chose for them",
        de: "{0} hatte keine Zeit mehr, und das Haus hat gewählt",
    },
    /// The house sat down at the reading seat's chair.
    LogStandInYou {
        en: "{7} went away and the house took over your seat",
        de: "{7} bist weggegangen, und das Haus hat deinen Platz übernommen",
    },
    /// The house sat down at another seat's chair.
    LogStandIn {
        en: "{0} went away and the house took over their seat",
        de: "{0} ist weggegangen, und das Haus hat den Platz übernommen",
    },
    /// The reading seat is back at its chair.
    LogReturnedYou { en: "{7} came back", de: "{7} bist zurückgekommen" },
    /// Another seat is back at its chair.
    LogReturned { en: "{0} came back", de: "{0} ist zurückgekommen" },
    /// The reading seat played a land. `{1}` is the land.
    LogLandPlayedYou { en: "{7} played {1}{2}", de: "{7} hast {1}{2} gespielt" },
    /// Another seat played a land.
    LogLandPlayed { en: "{0} played {1}{2}", de: "{0} hat {1}{2} gespielt" },
    /// The reading seat cast a spell. `{1}` is the spell.
    LogCastYou { en: "{7} cast {1}{2}", de: "{7} hast {1}{2} gewirkt" },
    /// Another seat cast a spell.
    LogCast { en: "{0} cast {1}{2}", de: "{0} hat {1}{2} gewirkt" },
    /// An ability the reading seat controls went on the stack. `{1}` is its
    /// source. Activated or triggered, which the line cannot tell: its
    /// controller puts either on the stack (CR 602.2a, CR 603.3).
    LogAbilityYou {
        en: "{7} put an ability of {1} on the stack",
        de: "{7} hast eine Fähigkeit auf den Stapel gelegt: {1}",
    },
    /// An ability another seat controls went on the stack.
    LogAbility {
        en: "{0} put an ability of {1} on the stack",
        de: "{0} hat eine Fähigkeit auf den Stapel gelegt: {1}",
    },
    /// A spell was countered. `{1}` is the spell.
    LogCountered { en: "{1} was countered", de: "{1} wurde neutralisiert" },
    /// A spell or ability left the stack without resolving. `{1}` is it.
    LogDidNotResolve { en: "{1} did not resolve", de: "{1} wurde nicht verrechnet" },
    /// The reading seat drew cards it may name. `{1}` is the list.
    LogDrewYou { en: "{7} drew {1}", de: "{7} hast {1} gezogen" },
    /// Another seat drew cards the reading seat may name.
    LogDrew { en: "{0} drew {1}", de: "{0} hat {1} gezogen" },
    /// The reading seat drew one card it may not name. `{1}` is the count.
    LogDrewCardYou { en: "{7} drew {1} card", de: "{7} hast {1} Karte gezogen" },
    /// The reading seat drew cards it may not name.
    LogDrewCardsYou { en: "{7} drew {1} cards", de: "{7} hast {1} Karten gezogen" },
    /// Another seat drew one card.
    LogDrewCard { en: "{0} drew {1} card", de: "{0} hat {1} Karte gezogen" },
    /// Another seat drew cards.
    LogDrewCards { en: "{0} drew {1} cards", de: "{0} hat {1} Karten gezogen" },
    /// The reading seat discarded a card. `{1}` is the card.
    LogDiscardedYou { en: "{7} discarded {1}", de: "{7} hast {1} abgeworfen" },
    /// Another seat discarded a card.
    LogDiscarded { en: "{0} discarded {1}", de: "{0} hat {1} abgeworfen" },
    /// A card went into a library, and the library was shuffled in the
    /// same action (#300). `{1}` is the card, `{2}` where from, `{3}` which
    /// library ([`Phrase::LogIntoLibrary`]).
    LogMovedShuffled { en: "{1} was shuffled {2} {3}", de: "{1} wurde {2} {3} gemischt" },
    /// A card went on top of the reading seat's library.
    LogOntoLibraryTopYou { en: "on top of your library", de: "oben auf deine Bibliothek" },
    /// A card went on top of another seat's library. `{0}` is the seat.
    LogOntoLibraryTop { en: "on top of {0}'s library", de: "oben auf die Bibliothek von {0}" },
    /// A card went on the bottom of the reading seat's library.
    LogOntoLibraryBottomYou { en: "on the bottom of your library", de: "unter deine Bibliothek" },
    /// A card went on the bottom of another seat's library.
    LogOntoLibraryBottom { en: "on the bottom of {0}'s library", de: "unter die Bibliothek von {0}" },
    /// A card went into the reading seat's library at a place counted from
    /// the top. `{1}` is the ordinal: "3rd" in English, "3" in German, whose
    /// sentence writes the full stop.
    LogIntoLibraryAtYou { en: "{1} from the top of your library", de: "als {1}. Karte von oben in deine Bibliothek" },
    /// A card went into another seat's library at a place counted from the
    /// top.
    LogIntoLibraryAt { en: "{1} from the top of {0}'s library", de: "als {1}. Karte von oben in die Bibliothek von {0}" },
    /// Where another seat played a land or cast a spell from, when the zone
    /// is its own (#300). The reading seat's own is [`Phrase::LogFromHandYou`].
    LogFromOwnHand { en: "from their hand", de: "aus der eigenen Hand" },
    /// The same, from its own library.
    LogFromOwnLibrary { en: "from their library", de: "aus der eigenen Bibliothek" },
    /// The same, from its own graveyard.
    LogFromOwnGraveyard { en: "from their graveyard", de: "aus dem eigenen Friedhof" },
    /// "You" as the subject of a log sentence (#300): `{7}` in every one, so
    /// a panel can set the reading seat in bold like any other seat.
    LogYouSubject { en: "you", de: "du" },
    /// "You" as a direct object: `{8}`.
    LogYouObject { en: "you", de: "dich" },
    /// "You" as an indirect object: `{9}`.
    LogYouIndirect { en: "you", de: "dir" },
    /// The reading seat lost by drawing from an empty library, as the log
    /// says it; [`Phrase::LostEmptyDrawYou`] is the end screen's.
    LogLostEmptyDrawYou { en: "{7} tried to draw from an empty library", de: "{7} wolltest aus einer leeren Bibliothek ziehen" },
    /// The reading seat lost to poison.
    LogLostPoisonYou { en: "{7} had ten or more poison counters", de: "{7} hattest zehn oder mehr Giftmarken" },
    /// The reading seat lost to a commander's damage.
    LogLostCommanderDamageYou { en: "{7} took 21 or more combat damage from one commander", de: "{7} hast 21 oder mehr Kampfschaden von einem Kommandeur erhalten" },
    /// The reading seat conceded.
    LogLostConcededYou { en: "{7} conceded", de: "{7} hast aufgegeben" },
    /// An effect made the reading seat lose.
    LogLostEffectYou { en: "An effect made {8} lose", de: "Ein Effekt ließ {8} verlieren" },
    /// An object changed zones. `{1}` is the object, `{2}` where it came
    /// from and `{3}` where it went, each one of the `LogFrom…`/`LogInto…`
    /// phrases.
    LogMoved { en: "{1} moved {2} {3}", de: "{1} ist {2} {3} gelangt" },
    /// Out of the reading seat's library.
    LogFromLibraryYou { en: "from your library", de: "aus deiner Bibliothek" },
    /// Out of another seat's library. `{0}` is the seat.
    LogFromLibrary { en: "from {0}'s library", de: "aus der Bibliothek von {0}" },
    /// Out of the reading seat's hand.
    LogFromHandYou { en: "from your hand", de: "aus deiner Hand" },
    /// Out of another seat's hand.
    LogFromHand { en: "from {0}'s hand", de: "aus der Hand von {0}" },
    /// Out of the reading seat's graveyard.
    LogFromGraveyardYou { en: "from your graveyard", de: "aus deinem Friedhof" },
    /// Out of another seat's graveyard.
    LogFromGraveyard { en: "from {0}'s graveyard", de: "aus dem Friedhof von {0}" },
    /// Off the battlefield.
    LogFromBattlefield { en: "from the battlefield", de: "vom Spielfeld" },
    /// Out of exile.
    LogFromExile { en: "from exile", de: "aus dem Exil" },
    /// Out of the command zone.
    LogFromCommand { en: "from the command zone", de: "aus der Kommandozone" },
    /// Into the reading seat's library.
    LogIntoLibraryYou { en: "into your library", de: "in deine Bibliothek" },
    /// Into another seat's library.
    LogIntoLibrary { en: "into {0}'s library", de: "in die Bibliothek von {0}" },
    /// Into the reading seat's hand.
    LogIntoHandYou { en: "into your hand", de: "auf deine Hand" },
    /// Into another seat's hand.
    LogIntoHand { en: "into {0}'s hand", de: "auf die Hand von {0}" },
    /// Into the reading seat's graveyard.
    LogIntoGraveyardYou { en: "into your graveyard", de: "in deinen Friedhof" },
    /// Into another seat's graveyard.
    LogIntoGraveyard { en: "into {0}'s graveyard", de: "in den Friedhof von {0}" },
    /// Onto the battlefield.
    LogIntoBattlefield { en: "onto the battlefield", de: "aufs Spielfeld" },
    /// Into exile.
    LogIntoExile { en: "into exile", de: "ins Exil" },
    /// Into the command zone.
    LogIntoCommand { en: "into the command zone", de: "in die Kommandozone" },
    /// The reading seat created a token. `{1}` is the token.
    LogCreatedYou { en: "{7} created {1}", de: "{7} hast {1} erschaffen" },
    /// Another seat created a token.
    LogCreated { en: "{0} created {1}", de: "{0} hat {1} erschaffen" },
    /// Combat damage to the reading seat. `{1}` is the amount, `{2}` the
    /// source.
    LogCombatDamageYou {
        en: "{2} dealt {1} combat damage to {9}",
        de: "{2} hat {9} {1} Kampfschaden zugefügt",
    },
    /// Damage to the reading seat.
    LogDamageYou { en: "{2} dealt {1} damage to {9}", de: "{2} hat {9} {1} Schaden zugefügt" },
    /// Combat damage to another seat or a permanent, `{0}`.
    LogCombatDamage {
        en: "{2} dealt {1} combat damage to {0}",
        de: "{0} hat {1} Kampfschaden durch {2} erlitten",
    },
    /// Damage to another seat or a permanent, `{0}`.
    LogDamage { en: "{2} dealt {1} damage to {0}", de: "{0} hat {1} Schaden durch {2} erlitten" },
    /// Damage to the reading seat from a source the line cannot name.
    LogDamageUnsourcedYou { en: "{7} were dealt {1} damage", de: "{7} hast {1} Schaden erlitten" },
    /// Damage to another seat or a permanent from a source the line cannot
    /// name.
    LogDamageUnsourced { en: "{0} was dealt {1} damage", de: "{0} hat {1} Schaden erlitten" },
    /// The reading seat's life total is 1. `{2}` is it, `{1}` what it was.
    LogLifePointYou {
        en: "{7} went to {2} life (from {1})",
        de: "{7} hattest danach {2} Lebenspunkt (vorher {1})",
    },
    /// The reading seat's life total changed.
    LogLifePointsYou {
        en: "{7} went to {2} life (from {1})",
        de: "{7} hattest danach {2} Lebenspunkte (vorher {1})",
    },
    /// Another seat's life total is 1.
    LogLifePoint {
        en: "{0} went to {2} life (from {1})",
        de: "{0} hatte danach {2} Lebenspunkt (vorher {1})",
    },
    /// Another seat's life total changed.
    LogLifePoints {
        en: "{0} went to {2} life (from {1})",
        de: "{0} hatte danach {2} Lebenspunkte (vorher {1})",
    },
    /// Counters on a permanent changed. `{1}` is the permanent, `{2}` how
    /// many there are now, `{3}` the counter noun for that many, `{4}` how
    /// many there were.
    LogCounters { en: "{1} went to {2} {3} (from {4})", de: "{1} hatte danach {2} {3} (vorher {4})" },
    /// One +X/+Y counter. `{0}` is X and `{1}` is Y.
    LogCounterPlus { en: "+{0}/+{1} counter", de: "+{0}/+{1}-Marke" },
    /// +X/+Y counters.
    LogCountersPlus { en: "+{0}/+{1} counters", de: "+{0}/+{1}-Marken" },
    /// One -X/-Y counter.
    LogCounterMinus { en: "-{0}/-{1} counter", de: "-{0}/-{1}-Marke" },
    /// -X/-Y counters.
    LogCountersMinus { en: "-{0}/-{1} counters", de: "-{0}/-{1}-Marken" },
    /// One loyalty counter.
    LogCounterLoyalty { en: "loyalty counter", de: "Loyalitätsmarke" },
    /// Loyalty counters.
    LogCountersLoyalty { en: "loyalty counters", de: "Loyalitätsmarken" },
    /// One lore counter (CR 714.3).
    LogCounterLore { en: "lore counter", de: "Kapitelmarke" },
    /// Lore counters.
    LogCountersLore { en: "lore counters", de: "Kapitelmarken" },
    /// One time counter.
    LogCounterTime { en: "time counter", de: "Zeitmarke" },
    /// Time counters.
    LogCountersTime { en: "time counters", de: "Zeitmarken" },
    /// One charge counter.
    LogCounterCharge { en: "charge counter", de: "Ladungsmarke" },
    /// Charge counters.
    LogCountersCharge { en: "charge counters", de: "Ladungsmarken" },
    /// One poison counter.
    LogCounterPoison { en: "poison counter", de: "Giftmarke" },
    /// Poison counters.
    LogCountersPoison { en: "poison counters", de: "Giftmarken" },
    /// One energy counter.
    LogCounterEnergy { en: "energy counter", de: "Energiemarke" },
    /// Energy counters.
    LogCountersEnergy { en: "energy counters", de: "Energiemarken" },
    /// One rad counter.
    LogCounterRad { en: "rad counter", de: "Strahlungsmarke" },
    /// Rad counters.
    LogCountersRad { en: "rad counters", de: "Strahlungsmarken" },
    /// One lifelink counter.
    LogCounterLifelink { en: "lifelink counter", de: "Lebensverknüpfungsmarke" },
    /// Lifelink counters.
    LogCountersLifelink { en: "lifelink counters", de: "Lebensverknüpfungsmarken" },
    /// One level counter.
    LogCounterLevel { en: "level counter", de: "Stufenmarke" },
    /// Level counters.
    LogCountersLevel { en: "level counters", de: "Stufenmarken" },
    /// One counter of a kind this client has no name for.
    LogCounterOther { en: "counter", de: "Marke" },
    /// Counters of a kind this client has no name for.
    LogCountersOther { en: "counters", de: "Marken" },
    /// A creature attacks the reading seat. `{1}` is the attacker.
    LogAttackedYou { en: "{1} attacked {8}", de: "{1} hat {8} angegriffen" },
    /// A creature attacks another seat or a planeswalker, `{0}`.
    LogAttacked { en: "{1} attacked {0}", de: "{1} hat {0} angegriffen" },
    /// A creature blocks. `{1}` is the blocker, `{2}` the attacker.
    LogBlocked { en: "{1} blocked {2}", de: "{1} hat {2} geblockt" },
    /// An attacker joined a band (banding). `{1}` joined the band of `{2}`.
    LogBanded {
        en: "{1} attacks in a band with {2}",
        de: "{1} greift in einer Gruppe mit {2} an",
    },
    /// The reading seat gained control of a permanent. `{1}` is it.
    LogControlYou { en: "{7} gained control of {1}", de: "{7} hast die Kontrolle über {1} übernommen" },
    /// Another seat gained control of a permanent.
    LogControl { en: "{0} gained control of {1}", de: "{0} hat die Kontrolle über {1} übernommen" },
    /// A permanent transformed. `{1}` is it.
    /// A face-down permanent becomes public.
    LogTurnedFaceUp { en: "{1} turned face up", de: "{1} wurde aufgedeckt" },
    /// A permanent transforms.
    LogTransformed { en: "{1} transformed", de: "{1} hat sich verwandelt" },
    /// The reading seat chose which permanents to keep. `{1}` is the list.
    LogCardsKeptYou { en: "{7} chose to keep {1}", de: "{7} behältst {1}" },
    /// Another seat chose which permanents to keep.
    LogCardsKept { en: "{0} chose to keep {1}", de: "{0} behält {1}" },
    /// The reading seat revealed cards. `{1}` is the list.
    LogRevealedYou { en: "{7} revealed {1}", de: "{7} hast {1} offen vorgezeigt" },
    /// Another seat revealed cards.
    LogRevealed { en: "{0} revealed {1}", de: "{0} hat {1} offen vorgezeigt" },
    /// The reading seat shuffled its library.
    LogShuffledYou { en: "{7} shuffled your library", de: "{7} hast deine Bibliothek gemischt" },
    /// Another seat shuffled its library.
    LogShuffled { en: "{0} shuffled their library", de: "{0} hat die eigene Bibliothek gemischt" },
    /// The reading seat rolled a die. `{1}` is its sides, `{2}` the result.
    LogRolledYou { en: "{7} rolled a d{1} and got {2}", de: "{7} hast mit einem W{1} eine {2} gewürfelt" },
    /// Another seat rolled a die.
    LogRolled { en: "{0} rolled a d{1} and got {2}", de: "{0} hat mit einem W{1} eine {2} gewürfelt" },
    /// The reading seat won.
    LogWonYou { en: "{7} won the game", de: "{7} hast das Spiel gewonnen" },
    /// One other seat won. `{0}` is it.
    LogWonOne { en: "{0} won the game", de: "{0} hat das Spiel gewonnen" },
    /// A team won. `{0}` is the list of its seats.
    LogWonMany { en: "{0} won the game", de: "{0} haben das Spiel gewonnen" },
    /// Nobody won.
    LogDrawn { en: "The game was a draw", de: "Das Spiel endete unentschieden" },
    /// The host found a loop and broke it.
    LogLoopBroken {
        en: "A loop was found and broken",
        de: "Eine Endlosschleife wurde erkannt und beendet",
    },
    /// The host found a loop.
    LogLoop { en: "A loop was found", de: "Eine Endlosschleife wurde erkannt" },
    /// It became day (CR 730).
    LogDay { en: "It became day", de: "Es wurde Tag" },
    /// It became night (CR 730).
    LogNight { en: "It became night", de: "Es wurde Nacht" },
    /// The reading seat became the monarch (CR 724.3).
    LogMonarchYou { en: "{7} became the monarch", de: "{7} bist der Monarch geworden" },
    /// Another seat became the monarch.
    LogMonarch { en: "{0} became the monarch", de: "{0} ist der Monarch geworden" },

    // ---- reports (#309, #310) ------------------------------------------
    /// The button, the form's title and the keymap row that opens it.
    ReportButton { en: "Report a problem", de: "Problem melden" },
    /// A kind of report: something did the wrong thing.
    ReportKindBug { en: "Bug", de: "Fehler" },
    /// A kind of report: something could be better.
    ReportKindImprovement { en: "Suggestion", de: "Vorschlag" },
    /// A kind of report: anything said about the game.
    ReportKindFeedback { en: "Feedback", de: "Rückmeldung" },
    /// A kind of report the client sends by itself.
    ReportKindCrash { en: "Crash", de: "Absturz" },
    /// A kind of report: none of the others.
    ReportKindOther { en: "Other", de: "Sonstiges" },
    /// The empty text box's prompt.
    ReportTextHint {
        en: "What happened, and what did you expect?",
        de: "Was ist passiert, und was hast du erwartet?",
    },
    /// Under the text box. `{0}` characters written of `{1}`.
    ReportChars { en: "{0} / {1} characters", de: "{0} / {1} Zeichen" },
    /// The text is too long to send. `{0}` is the limit.
    ReportTextTooLong {
        en: "The text is longer than {0} characters.",
        de: "Der Text ist länger als {0} Zeichen.",
    },
    /// Above the boxes.
    ReportIncludeHeading {
        en: "Also send (remembered on this device):",
        de: "Außerdem senden (auf diesem Gerät gemerkt):",
    },
    /// What is always sent, whatever is ticked.
    ReportAlways {
        en: "Always sent: your text, this client's version and the game's id. The gateway adds its own record of the game, which names no one.",
        de: "Immer gesendet: dein Text, die Version dieses Clients und die Kennung der Partie. Das Gateway fügt seine eigene Aufzeichnung der Partie hinzu, die niemanden namentlich nennt.",
    },
    /// The same, for a report written at no networked table: no game id,
    /// and no record but one the player ticks below.
    ReportAlwaysLocal {
        en: "Always sent: your text and this client's version.",
        de: "Immer gesendet: dein Text und die Version dieses Clients.",
    },
    /// A box: system and hardware.
    ReportCatSystem { en: "System and hardware", de: "System und Hardware" },
    /// Under it.
    ReportCatSystemHint {
        en: "Platform, processor count, graphics adapter, window size, language.",
        de: "Plattform, Prozessoranzahl, Grafikadapter, Fenstergröße, Sprache.",
    },
    /// A box: the table.
    ReportCatGame { en: "The table as you see it", de: "Der Tisch, wie du ihn siehst" },
    /// Under it.
    ReportCatGameHint {
        en: "Your view of the board, the open question and what you had selected. Nothing that is hidden from you.",
        de: "Deine Sicht aufs Spielfeld, die offene Frage und was du ausgewählt hattest. Nichts, was vor dir verborgen ist.",
    },
    /// A box: the seat's log.
    ReportCatLog { en: "Your game log", de: "Dein Spielprotokoll" },
    /// Under it.
    ReportCatLogHint {
        en: "Other players' names are replaced by Player A, Player B, …",
        de: "Die Namen anderer Spieler werden durch Player A, Player B, … ersetzt.",
    },
    /// A box: settings.
    ReportCatSettings { en: "Settings", de: "Einstellungen" },
    /// Under it.
    ReportCatSettingsHint {
        en: "Language, display settings, key bindings and standing answers. No names, addresses or tokens.",
        de: "Sprache, Anzeige, Tastenbelegung und Daueranweisungen. Keine Namen, Adressen oder Tokens.",
    },
    /// A box: the picture.
    ReportCatScreenshot { en: "Screenshot", de: "Bildschirmfoto" },
    /// Under it.
    ReportCatScreenshotHint {
        en: "The window as it was when you opened this form. It can show other players' names.",
        de: "Das Fenster, wie es beim Öffnen dieses Formulars aussah. Es kann Namen anderer Spieler zeigen.",
    },
    /// A box with nothing behind it.
    ReportCatNothing { en: "(nothing to send here)", de: "(hier gibt es nichts zu senden)" },
    /// Beside the screenshot box in a browser, which takes no picture.
    ReportCatNoShotOnWeb {
        en: "(the browser version takes no picture)",
        de: "(die Browser-Version macht kein Bild)",
    },
    /// Under the screenshot box in a browser.
    ReportCatScreenshotHintWeb {
        en: "The browser version of the client cannot take a picture of its window, so a report from here never carries one. Ticked, the box applies when you report from the desktop client.",
        de: "Die Browser-Version des Clients kann kein Bild ihres Fensters machen, ein Bericht von hier enthält also nie eines. Angekreuzt gilt das Kästchen, wenn du aus dem Desktop-Client meldest.",
    },
    /// The picture's size. `{0}` × `{1}` pixels, `{2}` kilobytes.
    ReportShotSize { en: "{0} × {1}, {2} KB", de: "{0} × {1}, {2} KB" },
    /// The box for crash reports, which the client sends by itself.
    ReportCrashesBox {
        en: "Send crash reports automatically",
        de: "Absturzberichte automatisch senden",
    },
    /// Opens the preview.
    ReportPreviewShow { en: "Show what is sent", de: "Zeigen, was gesendet wird" },
    /// Closes it.
    ReportPreviewHide { en: "Hide what is sent", de: "Vorschau schließen" },
    /// Sends.
    ReportSend { en: "Send", de: "Senden" },
    /// Closes the form.
    ReportClose { en: "Close", de: "Schließen" },
    /// While it goes.
    ReportSending { en: "Sending …", de: "Wird gesendet …" },
    /// Received. `{0}` is the report's id.
    ReportSent {
        en: "Thank you. Report {0} was received.",
        de: "Danke. Bericht {0} ist angekommen.",
    },
    /// 503.
    ReportsUnavailable {
        en: "This gateway does not take reports right now.",
        de: "Dieses Gateway nimmt gerade keine Berichte an.",
    },
    /// 401.
    ReportSignInAgain {
        en: "Your session has ended. Sign in again, then send the report.",
        de: "Deine Sitzung ist abgelaufen. Melde dich erneut an und sende den Bericht dann.",
    },
    /// 413, or too large even without the picture.
    ReportTooLarge {
        en: "The report is too large. Leave out the screenshot or the log and try again.",
        de: "Der Bericht ist zu groß. Lass das Bildschirmfoto oder das Protokoll weg und versuch es erneut.",
    },
    /// 429.
    ReportTooMany {
        en: "Too many reports in a short time. Wait a little, then send again.",
        de: "Zu viele Berichte in kurzer Zeit. Warte etwas und sende dann erneut.",
    },
    /// No answer at all.
    ReportUnreachable {
        en: "The gateway could not be reached. Your report is still here.",
        de: "Das Gateway war nicht erreichbar. Dein Bericht ist noch da.",
    },
    /// 502: the gateway took it and its feedback service did not.
    ReportNotPassedOn {
        en: "The gateway could not pass the report on. Your report is still here; try again later.",
        de: "Das Gateway konnte den Bericht nicht weitergeben. Dein Bericht ist noch da; versuch es später erneut.",
    },
    /// Any other answer.
    ReportFailed {
        en: "The report could not be sent. Try again later.",
        de: "Der Bericht konnte nicht gesendet werden. Versuch es später erneut.",
    },
    /// No session to send it with, and no feedback service to send it to.
    ReportNeedsSession {
        en: "Sign in to a gateway to send a report. This client knows no feedback service to send it to otherwise.",
        de: "Melde dich bei einem Gateway an, um einen Bericht zu senden. Einen anderen Feedback-Dienst, an den er gehen könnte, kennt dieser Client nicht.",
    },
    /// Signed in nowhere, the report goes straight to the service. `{0}` is
    /// its address.
    ReportGoesDirect {
        en: "You are not signed in, so this report goes straight to the Baylee feedback service at {0}.",
        de: "Du bist nicht angemeldet, darum geht dieser Bericht direkt an den Baylee-Feedback-Dienst unter {0}.",
    },
    /// 503 from the service itself.
    ReportsDirectUnavailable {
        en: "The feedback service takes no reports straight from a client right now. Sign in to a gateway to send it.",
        de: "Der Feedback-Dienst nimmt gerade keine Berichte direkt von einem Client an. Melde dich bei einem Gateway an, um ihn zu senden.",
    },
    /// No answer from the service.
    ReportDirectUnreachable {
        en: "The feedback service could not be reached. Your report is still here.",
        de: "Der Feedback-Dienst war nicht erreichbar. Dein Bericht ist noch da.",
    },
    /// The box for a local game's record, ticked per report.
    ReportRecordBox {
        en: "Attach the whole record of this game",
        de: "Die vollständige Aufzeichnung dieses Spiels anhängen",
    },
    /// Under it. `{0}` is its size in kilobytes.
    ReportRecordHint {
        en: "Every move of the game this device hosted, from the shuffle on ({0} KB). It shows every seat's cards, the hidden ones too: hands, libraries, face-down cards. It names nobody. Asked again for each report.",
        de: "Jeder Zug des Spiels, das dieses Gerät ausgerichtet hat, ab dem Mischen ({0} KB). Sie zeigt die Karten aller Plätze, auch die verdeckten: Hände, Bibliotheken, verdeckte Karten. Sie nennt niemanden. Wird bei jedem Bericht neu gefragt.",
    },
    /// The standing "never" for records.
    ReportRecordNever {
        en: "Never offer to attach a game's record",
        de: "Nie anbieten, die Aufzeichnung eines Spiels anzuhängen",
    },
    /// The confirmation's heading.
    ReportConfirmTitle { en: "Send this report?", de: "Diesen Bericht senden?" },
    /// Where it goes. `{0}` is the address.
    ReportConfirmTo { en: "To: {0}", de: "An: {0}" },
    /// The words. `{0}` is how many characters.
    ReportConfirmText {
        en: "Your text ({0} characters) and this build's version",
        de: "Dein Text ({0} Zeichen) und die Version dieses Builds",
    },
    /// A ticked box, before its label.
    ReportConfirmPart { en: "and: {0}", de: "und: {0}" },
    /// The record. `{0}` is its size in kilobytes.
    ReportConfirmRecord {
        en: "and the whole record of the game this device hosted ({0} KB): every seat's cards, the hidden ones too",
        de: "und die vollständige Aufzeichnung des Spiels, das dieses Gerät ausgerichtet hat ({0} KB): die Karten aller Plätze, auch die verdeckten",
    },
    /// The device id, on a direct report.
    ReportConfirmDevice {
        en: "and a random id this device made for reports, so that reports from one device can be told apart; never your name, account or address",
        de: "und eine zufällige Kennung, die dieses Gerät für Berichte erzeugt hat, damit sich Berichte eines Geräts zuordnen lassen; nie dein Name, Konto oder deine Adresse",
    },
    /// Sends after the confirmation.
    ReportConfirmSend { en: "Send now", de: "Jetzt senden" },
    /// Back to the form.
    ReportConfirmBack { en: "Back", de: "Zurück" },
    /// The record was too large and stayed home.
    ReportRecordLeftOut {
        en: "The game's record was too large to attach and was left out.",
        de: "Die Aufzeichnung des Spiels war zu groß zum Anhängen und wurde weggelassen.",
    },
    /// `seal` found a secret. `{0}` names which, never its value.
    ReportLeaked {
        en: "Not sent: the report contains your {0}. Remove it and try again.",
        de: "Nicht gesendet: Der Bericht enthält „{0}“. Entferne es und versuch es erneut.",
    },
    /// The report was cut to fit.
    ReportTrimmed {
        en: "To fit the size limit, the screenshot or older log lines were left out.",
        de: "Um die Größengrenze einzuhalten, wurden das Bildschirmfoto oder ältere Protokollzeilen weggelassen.",
    },
    /// Asked once, at the start after a crash.
    CrashAskTitle {
        en: "Baylee closed unexpectedly last time.",
        de: "Baylee wurde beim letzten Mal unerwartet beendet.",
    },
    /// Under it.
    CrashAskBody {
        en: "May the client send crash reports to the gateway you play on? A crash report holds the error, where in the program it happened (the backtrace) and this client's version, and system details if you allow them in the report form. Nothing about your games or your account. You can change this in the report form at any time.",
        de: "Darf der Client Absturzberichte an das Gateway senden, auf dem du spielst? Ein Absturzbericht enthält den Fehler, die Stelle im Programm, an der er auftrat (den Backtrace), und die Version dieses Clients, und Systemangaben, wenn du sie im Meldeformular erlaubst. Nichts über deine Partien oder dein Konto. Du kannst das jederzeit im Meldeformular ändern.",
    },
    /// The yes.
    CrashAskSend { en: "Send crash reports", de: "Absturzberichte senden" },
    /// The no.
    CrashAskNever { en: "Don't send", de: "Nicht senden" },
    /// The settings screen's panel for language-model seats (`docs/llm-seat.md`).
    SeatPanelTitle { en: "Language-model seats", de: "Sprachmodell-Sitze" },
    /// Under its title.
    SeatPanelAbout {
        en: "Which model the seat bridge (baylee-seat) plays at a table, and what it may spend. No key is ever in this file: each profile names the environment variable its key is read from, or keeps it in this machine's credential store.",
        de: "Welches Modell die Sitz-Brücke (baylee-seat) am Tisch spielt und was sie ausgeben darf. Kein Schlüssel steht je in dieser Datei: Jedes Profil nennt die Umgebungsvariable, aus der sein Schlüssel gelesen wird, oder legt ihn im Schlüsselbund dieses Rechners ab.",
    },
    /// Where the file is. `{0}` its path.
    SeatFileAt { en: "File: {0}", de: "Datei: {0}" },
    /// A browser or a phone runs no bridge.
    SeatDesktopOnly {
        en: "Language-model seats are set up on a desktop, where the seat bridge runs.",
        de: "Sprachmodell-Sitze werden am Desktop eingerichtet, wo die Sitz-Brücke läuft.",
    },
    /// No directory for the file.
    SeatNoConfigDir {
        en: "This system names no configuration directory, so there is no settings file for the seat bridge.",
        de: "Dieses System nennt kein Konfigurationsverzeichnis, daher gibt es keine Einstellungsdatei für die Sitz-Brücke.",
    },
    /// No file yet.
    SeatEmpty {
        en: "No settings file yet: the seat bridge plays with its built-in defaults, and nothing caps its spend across games. Add a profile to make one.",
        de: "Noch keine Einstellungsdatei: Die Sitz-Brücke spielt mit ihren eingebauten Vorgaben, und nichts begrenzt ihre Ausgaben über Partien hinweg. Füge ein Profil hinzu, um eine anzulegen.",
    },
    /// A file with no profiles.
    SeatNoProfiles {
        en: "No profiles: without --mind or --profile, the bridge plays the house AI.",
        de: "Keine Profile: Ohne --mind oder --profile spielt die Brücke die Haus-KI.",
    },
    /// The file cannot be used. `{0}` why, in the file's own words.
    SeatFileRefused {
        en: "The settings file cannot be used and is left as it is: {0}. Mend it by hand; this panel reads it again when it changes.",
        de: "Die Einstellungsdatei ist nicht verwendbar und bleibt, wie sie ist: {0}. Korrigiere sie von Hand; dieser Bereich liest sie neu, sobald sie sich ändert.",
    },
    /// The file changed while there were edits here.
    SeatChangedOnDisk {
        en: "The file changed on disk since it was read. Saving writes over it; Discard changes shows it.",
        de: "Die Datei hat sich auf der Festplatte geändert, seit sie gelesen wurde. Speichern überschreibt sie; Änderungen verwerfen zeigt sie.",
    },
    /// Adds a profile.
    SeatAdd { en: "Add a profile", de: "Profil hinzufügen" },
    /// Adds a copy of the shown profile.
    SeatDuplicate { en: "Duplicate", de: "Duplizieren" },
    /// Removes the shown profile.
    SeatRemove { en: "Remove", de: "Entfernen" },
    /// The switch that makes the shown profile the default.
    SeatDefault { en: "Default", de: "Standard" },
    /// A profile's chip when it is the default. `{0}` its name.
    SeatChipDefault { en: "{0} · default", de: "{0} · Standard" },
    /// What the default is.
    SeatDefaultAbout {
        en: "The default profile plays when the bridge is given neither --mind nor --profile.",
        de: "Das Standardprofil spielt, wenn die Brücke weder --mind noch --profile bekommt.",
    },
    /// No default.
    SeatDefaultNone {
        en: "No default profile: given neither --mind nor --profile, the bridge plays the house AI.",
        de: "Kein Standardprofil: Ohne --mind und --profile spielt die Brücke die Haus-KI.",
    },
    /// A profile's name.
    SeatName { en: "Name", de: "Name" },
    /// A profile's provider.
    SeatProvider { en: "Protocol", de: "Protokoll" },
    /// The Anthropic API.
    SeatProviderAnthropic { en: "Anthropic", de: "Anthropic" },
    /// Any OpenAI-compatible endpoint.
    SeatProviderOpenAi { en: "OpenAI-compatible", de: "OpenAI-kompatibel" },
    /// An agent CLI on this machine, played on the player's subscription.
    SeatProviderCli { en: "Agent CLI (subscription)", de: "Agent-CLI (Abo)" },
    /// Under a CLI profile's model box.
    SeatCliModel {
        en: "A CLI's model is its tool, then the tool's own model if you name one: claude, or claude:opus.",
        de: "Das Modell einer CLI ist ihr Werkzeug, dann, wenn du eins nennst, dessen eigenes Modell: claude oder claude:opus.",
    },
    /// A profile's model.
    SeatModel { en: "Model", de: "Modell" },
    /// A priced model offered for the model box. `{0}` the model, `{1}` input and `{2}` output price.
    SeatPriced { en: "{0}: {1} in, {2} out", de: "{0}: {1} Eingabe, {2} Ausgabe" },
    /// Under the suggestions.
    SeatPricedAbout {
        en: "Prices in US dollars per million tokens, as this build knows them.",
        de: "Preise in US-Dollar je Million Tokens, wie dieser Build sie kennt.",
    },
    /// How hard the model thinks.
    SeatEffort { en: "Effort", de: "Denkaufwand" },
    /// How the model answers.
    SeatAnswer { en: "Answers with", de: "Antwortet mit" },
    /// The build's way of answering.
    SeatAnswerBuild { en: "Default", de: "Standard" },
    /// Answering by calling a tool.
    SeatAnswerTools { en: "Tools", de: "Werkzeugen" },
    /// Answering with one JSON object.
    SeatAnswerJson { en: "JSON", de: "JSON" },
    /// Answering with one JSON object held to a schema.
    SeatAnswerJsonSchema { en: "JSON schema", de: "JSON-Schema" },
    /// The most one reply may take.
    SeatMaxTokens { en: "Max output tokens", de: "Höchstens Ausgabe-Tokens" },
    /// Input price over the build's.
    SeatPriceIn { en: "Price in ($ per million)", de: "Preis Eingabe ($ je Million)" },
    /// Output price over the build's.
    SeatPriceOut { en: "Price out ($ per million)", de: "Preis Ausgabe ($ je Million)" },
    /// This build's price for the model. `{0}` input, `{1}` output.
    SeatBuildPrice {
        en: "This build's price: {0} in, {1} out per million tokens. A price given here replaces it.",
        de: "Preis dieses Builds: {0} Eingabe, {1} Ausgabe je Million Tokens. Ein hier angegebener Preis ersetzt ihn.",
    },
    /// No price for the model.
    SeatNoBuildPrice {
        en: "This build has no price for this model: give one here, or limit its games in tokens.",
        de: "Dieser Build kennt keinen Preis für dieses Modell: Gib hier einen an oder begrenze seine Partien in Tokens.",
    },
    /// The most a game may spend in dollars.
    SeatGameUsd { en: "Per game ($)", de: "Pro Partie ($)" },
    /// The most a game may spend in tokens.
    SeatGameTokens { en: "Per game (tokens)", de: "Pro Partie (Tokens)" },
    /// The longest one answer may take.
    SeatThinkSecs { en: "Think seconds", de: "Bedenkzeit (Sekunden)" },
    /// The variable the key is read from.
    SeatKeyEnv { en: "Key variable", de: "Schlüssel-Variable" },
    /// The variable is set. `{0}` its name.
    SeatKeySet {
        en: "{0} is set in this program's environment.",
        de: "{0} ist in der Umgebung dieses Programms gesetzt.",
    },
    /// The variable is not set. `{0}` its name.
    SeatKeyUnset {
        en: "{0} is not set in this program's environment. The bridge reads it from its own.",
        de: "{0} ist in der Umgebung dieses Programms nicht gesetzt. Die Brücke liest sie aus ihrer eigenen.",
    },
    /// Over the ready-made adapters a profile can be added from.
    SeatPresets {
        en: "Add an adapter (a protocol and its address, both editable afterwards):",
        de: "Adapter hinzufügen (Protokoll und Adresse, danach änderbar):",
    },
    /// The key box's caption.
    SeatKeyBox { en: "Key", de: "Schlüssel" },
    /// The empty key box, no key kept.
    SeatKeyHint { en: "paste the key here", de: "Schlüssel hier einfügen" },
    /// The empty key box, a key kept.
    SeatKeyReplaceHint { en: "paste a new key to replace it", de: "neuen Schlüssel einfügen, um ihn zu ersetzen" },
    /// Hands the typed key to the credential store.
    SeatKeyKeep { en: "Keep", de: "Speichern" },
    /// Forgets the kept key.
    SeatKeyForget { en: "Forget", de: "Vergessen" },
    /// The store is being asked.
    SeatKeyAsking { en: "Asking this machine's credential store…", de: "Frage den Schlüsselbund dieses Rechners…" },
    /// A key is kept. `{0}` the host it goes to.
    SeatKeyKept {
        en: "A key is kept in this machine's credential store, for {0} only. It is never shown; the variable wins where it is set.",
        de: "Im Schlüsselbund dieses Rechners liegt ein Schlüssel, nur für {0}. Er wird nie angezeigt; die Variable geht vor, wo sie gesetzt ist.",
    },
    /// No key is kept. `{0}` the host.
    SeatKeyNoneKept {
        en: "No key kept for {0}. One pasted here goes to this machine's credential store, never to the file.",
        de: "Kein Schlüssel für {0} gespeichert. Ein hier eingefügter geht in den Schlüsselbund dieses Rechners, nie in die Datei.",
    },
    /// No store to keep a key in. `{0}` why.
    SeatKeyStoreUnavailable {
        en: "No credential store to keep a key in ({0}): set the key's variable in the environment.",
        de: "Kein Schlüsselbund für einen Schlüssel ({0}): Setze die Variable des Schlüssels in der Umgebung.",
    },
    /// The key's variable or address is not one yet.
    SeatKeyNoEntry {
        en: "A key can be kept once the key variable and the address read.",
        de: "Ein Schlüssel kann gespeichert werden, sobald Schlüssel-Variable und Adresse stimmen.",
    },
    /// Where the API is.
    SeatBaseUrl { en: "Base URL", de: "Basis-URL" },
    /// A box's value when left empty. `{0}` the value.
    SeatByDefault { en: "{0} by default", de: "standardmäßig {0}" },
    /// The base URL when left empty. `{0}` the variable, `{1}` the address.
    SeatBaseByDefault { en: "{0}, else {1}", de: "{0}, sonst {1}" },
    /// An effort left to an OpenAI-compatible endpoint.
    SeatEffortEndpoint { en: "the endpoint's own", de: "wie der Endpunkt es vorgibt" },
    /// An effort left to a CLI.
    SeatEffortCli { en: "the CLI's own", de: "wie die CLI es vorgibt" },
    /// The most calls a game may make.
    SeatGameCalls { en: "Per game (calls)", de: "Pro Partie (Aufrufe)" },
    /// No call limit for an API's games.
    SeatNoCallLimit { en: "no limit", de: "keine Grenze" },
    /// The program a CLI profile runs.
    SeatCommand { en: "Program", de: "Programm" },
    /// The program when left empty. `{0}` the tool's name.
    SeatCommandByDefault { en: "{0}, found on PATH", de: "{0}, im PATH gesucht" },
    /// A CLI's games have no price.
    SeatCliNoPrice {
        en: "A CLI plays on your subscription, which has no price: its games are limited in tokens and calls, and count under the token caps.",
        de: "Eine CLI spielt mit deinem Abo, das keinen Preis hat: Ihre Partien sind in Tokens und Aufrufen begrenzt und zählen unter den Token-Obergrenzen.",
    },
    /// The caps' heading.
    SeatCaps { en: "Caps across all games", de: "Obergrenzen über alle Partien" },
    /// Under it.
    SeatCapsAbout {
        en: "Dollars count the games of models with a price, tokens the games of models without one. An empty box is no cap.",
        de: "Dollar zählen die Partien von Modellen mit Preis, Tokens die von Modellen ohne. Ein leeres Feld ist keine Grenze.",
    },
    /// Dollars a day.
    SeatDayUsd { en: "Per day ($)", de: "Pro Tag ($)" },
    /// Dollars a month.
    SeatMonthUsd { en: "Per month ($)", de: "Pro Monat ($)" },
    /// Tokens a day.
    SeatDayTokens { en: "Per day (tokens)", de: "Pro Tag (Tokens)" },
    /// Tokens a month.
    SeatMonthTokens { en: "Per month (tokens)", de: "Pro Monat (Tokens)" },
    /// A profile with no price under a daily dollar cap and no daily token cap.
    SeatUnpricedDay {
        en: "The caps count dollars per day, and this model has no price: the bridge refuses its games until it has a price or the caps a daily token limit.",
        de: "Die Obergrenzen zählen Dollar pro Tag, und dieses Modell hat keinen Preis: Die Brücke verweigert seine Partien, bis es einen Preis oder die Obergrenzen ein Tageslimit in Tokens haben.",
    },
    /// The same for a month.
    SeatUnpricedMonth {
        en: "The caps count dollars per month, and this model has no price: the bridge refuses its games until it has a price or the caps a monthly token limit.",
        de: "Die Obergrenzen zählen Dollar pro Monat, und dieses Modell hat keinen Preis: Die Brücke verweigert seine Partien, bis es einen Preis oder die Obergrenzen ein Monatslimit in Tokens haben.",
    },
    /// The spend's heading.
    SeatSpent { en: "Spent", de: "Ausgegeben" },
    /// Periods on the local clock. `{0}` its offset, "UTC+02:00".
    SeatZoneLocal { en: "Counted in local time, {0}.", de: "Gezählt in Ortszeit, {0}." },
    /// Periods on UTC.
    SeatZoneUtc { en: "Counted in UTC.", de: "Gezählt in UTC." },
    /// Today's spend. `{0}` the day, `{1}` what it counts.
    SeatSpentDay { en: "Today, {0}: {1}", de: "Heute, {0}: {1}" },
    /// This month's spend. `{0}` the month, `{1}` what it counts.
    SeatSpentMonth { en: "This month, {0}: {1}", de: "Diesen Monat, {0}: {1}" },
    /// Spent against a cap. `{0}` spent, `{1}` the cap.
    SeatOf { en: "{0} of {1}", de: "{0} von {1}" },
    /// One token.
    SeatTokensOne { en: "{0} token", de: "{0} Token" },
    /// Tokens.
    SeatTokensMany { en: "{0} tokens", de: "{0} Tokens" },
    /// How many games a sum is over. `{0}` games, `{1}` of them still open.
    SeatGames { en: "games: {0}, still open: {1}", de: "Partien: {0}, davon offen: {1}" },
    /// The book cannot be read. `{0}` why.
    SeatBookUnreadable { en: "The spend book cannot be read: {0}", de: "Das Ausgabenbuch kann nicht gelesen werden: {0}" },
    /// Writes the file.
    SeatSave { en: "Save", de: "Speichern" },
    /// Drops every edit.
    SeatRevert { en: "Discard changes", de: "Änderungen verwerfen" },
    /// There are edits.
    SeatUnsaved { en: "Unsaved changes.", de: "Ungespeicherte Änderungen." },
    /// Save waits for the faults.
    SeatFixFirst { en: "Mend what is marked, then save.", de: "Korrigiere das Markierte, dann speichere." },
    /// Written.
    SeatSavedNote { en: "Saved.", de: "Gespeichert." },
    /// Not written. `{0}` why.
    SeatNotSaved { en: "Not saved: {0}", de: "Nicht gespeichert: {0}" },
    /// A box for a whole number holds something else.
    SeatFaultWhole { en: "A whole number, in digits.", de: "Eine ganze Zahl, in Ziffern." },
    /// A box for dollars holds something else.
    SeatFaultDollars {
        en: "An amount of US dollars, not below zero, such as 2.5.",
        de: "Ein Betrag in US-Dollar, nicht unter null, etwa 2,5.",
    },
    /// Half a price.
    SeatFaultHalfPrice {
        en: "Give both prices, input and output, or neither.",
        de: "Gib beide Preise an, Eingabe und Ausgabe, oder keinen.",
    },
    /// Two profiles of one name.
    SeatFaultNameTaken { en: "Another profile has this name.", de: "Ein anderes Profil hat diesen Namen." },
    /// A field named like a key.
    SeatFaultKeyNamed {
        en: "Named like a key. A key never goes in the settings file: set it in the environment and name its variable under Key variable.",
        de: "Benannt wie ein Schlüssel. Ein Schlüssel gehört nie in die Einstellungsdatei: Setze ihn in der Umgebung und nenne seine Variable unter Schlüssel-Variable.",
    },
    /// A value shaped like a key.
    SeatFaultKeyShaped {
        en: "This looks like an API key. A key never goes in the settings file: set it in the environment and name its variable under Key variable.",
        de: "Das sieht aus wie ein API-Schlüssel. Ein Schlüssel gehört nie in die Einstellungsdatei: Setze ihn in der Umgebung und nenne seine Variable unter Schlüssel-Variable.",
    },
    /// The default names no profile.
    SeatFaultNoDefault { en: "The default names no profile.", de: "Der Standard nennt kein Profil." },
    /// Not a profile's name.
    SeatFaultName {
        en: "A name is up to 32 letters, digits, - and _, starting with a letter or digit.",
        de: "Ein Name hat bis zu 32 Buchstaben, Ziffern, - und _ und beginnt mit einem Buchstaben oder einer Ziffer.",
    },
    /// Not a model id.
    SeatFaultModel {
        en: "A model id is up to 100 letters, digits and - _ . : / @.",
        de: "Eine Modell-ID hat bis zu 100 Buchstaben, Ziffern und - _ . : / @.",
    },
    /// A CLI's model that names no tool this build plays.
    SeatFaultTool {
        en: "Name the tool first: claude, or claude:<model>.",
        de: "Nenne zuerst das Werkzeug: claude oder claude:<Modell>.",
    },
    /// A CLI asked to answer with tools.
    SeatFaultCliAnswer {
        en: "A CLI has none of our tools: it answers with JSON or a JSON schema.",
        de: "Eine CLI hat keins unserer Werkzeuge: Sie antwortet mit JSON oder einem JSON-Schema.",
    },
    /// A key or an address on a CLI profile.
    SeatFaultCliKey {
        en: "A CLI reads no key and has no address: it plays as you are signed in to it.",
        de: "Eine CLI liest keinen Schlüssel und hat keine Adresse: Sie spielt so, wie du bei ihr angemeldet bist.",
    },
    /// A price on a CLI profile.
    SeatFaultCliPrice {
        en: "A subscription has no price: limit a CLI's games in tokens and calls.",
        de: "Ein Abo hat keinen Preis: Begrenze die Partien einer CLI in Tokens und Aufrufen.",
    },
    /// A program on a profile that is not a CLI's.
    SeatFaultCliOnly {
        en: "Only a CLI profile runs a program.",
        de: "Nur ein CLI-Profil führt ein Programm aus.",
    },
    /// A program named by a path that is not absolute.
    SeatFaultCommand {
        en: "The program's absolute path, such as /opt/homebrew/bin/claude.",
        de: "Der absolute Pfad des Programms, etwa /opt/homebrew/bin/claude.",
    },
    /// No calls at all.
    SeatFaultGameCalls {
        en: "The most calls a game may make: at least 1.",
        de: "Die meisten Aufrufe, die eine Partie machen darf: mindestens 1.",
    },
    /// Not an effort.
    SeatFaultEffort {
        en: "An effort is one lowercase word, such as low, medium or high.",
        de: "Ein Denkaufwand ist ein kleingeschriebenes Wort, etwa low, medium oder high.",
    },
    /// JSON asked of Anthropic.
    SeatFaultJson {
        en: "JSON answers are for OpenAI-compatible endpoints and CLIs; Anthropic's models answer with tools.",
        de: "JSON-Antworten sind für OpenAI-kompatible Endpunkte und CLIs; Anthropics Modelle antworten mit Werkzeugen.",
    },
    /// No reply at all.
    SeatFaultMaxTokens {
        en: "The most one reply may take: at least 1.",
        de: "Das Höchste, was eine Antwort nehmen darf: mindestens 1.",
    },
    /// No time at all.
    SeatFaultThinkSecs {
        en: "The longest one answer may take: at least 1 second.",
        de: "Die längste Zeit für eine Antwort: mindestens 1 Sekunde.",
    },
    /// A dollar budget with no price.
    SeatFaultUnpriced {
        en: "This build has no price for this model, so a dollar limit cannot be held: give it a price, or make its tokens per game the limit.",
        de: "Dieser Build kennt keinen Preis für dieses Modell, daher lässt sich keine Dollar-Grenze halten: Gib ihm einen Preis oder mache die Tokens pro Partie zur Grenze.",
    },
    /// Not a variable's name. `{0}` an example.
    SeatFaultKeyEnv {
        en: "The name of an environment variable, such as {0}: capitals, digits and _. Never the key itself.",
        de: "Der Name einer Umgebungsvariable, etwa {0}: Großbuchstaben, Ziffern und _. Nie der Schlüssel selbst.",
    },
    /// An address a key may not go to.
    SeatFaultAddress {
        en: "An https:// address; http:// only on this machine.",
        de: "Eine https://-Adresse; http:// nur auf diesem Rechner.",
    },
    /// Settings that cannot be written at all.
    SeatFaultFile { en: "These settings cannot be written.", de: "Diese Einstellungen lassen sich nicht schreiben." },

    // ====================================================================
    // The shell redesign (`.claude/ux-b6/DESIGN-v5.md`): one append region
    // per package, so packages working side by side add their words without
    // meeting on the same lines. A region holds its screens' words; a word
    // two screens share goes in the shell's. Labels keep their component's
    // budget (§2.3: button 18, German 20; chip 14; tab 16; nav 12), and a
    // German label shortened to fit is chosen here, once.
    //
    // ---- the shell (WP0b): header, kit, keyboard, bell, strips -----------
    /// Navigation: the play screen.
    ShellPlay { en: "Play", de: "Spielen" },
    /// Navigation: the decks screen.
    ShellDecks { en: "Decks", de: "Decks" },
    /// A settings row kept on this device (`ClientSettings`).
    ShellThisDevice { en: "This device", de: "Dieses Gerät" },
    /// A settings row kept with the account, on every device (`Preferences`).
    ShellYourAccount { en: "Your account", de: "Dein Konto" },
    /// An error line's action.
    ShellRetry { en: "Retry", de: "Erneut versuchen" },
    /// A toast's action after a deletion.
    ShellUndo { en: "Undo", de: "Rückgängig" },
    /// A toast's action that opens more.
    ShellDetails { en: "Details", de: "Details" },
    /// A deck tile's primary.
    ShellUseForNextGame { en: "Use for next game", de: "Fürs nächste Spiel" },
    /// A deck tile's second action.
    ShellEdit { en: "Edit", de: "Bearbeiten" },
    /// A house deck: copy it and make it the next game's deck.
    ShellAddAndUse { en: "Add and use", de: "Hinzufügen & nutzen" },
    /// The gold primary while the player holds a seat.
    ShellReturnToGame { en: "Return to game", de: "Zurück zum Spiel" },
    /// Why a control is off while there is no gateway.
    ShellNeedsGateway { en: "Needs the gateway", de: "Braucht das Gateway" },
    /// Why the starts are off while the player holds a seat. `{0}` the table.
    ShellSeatedAt { en: "You are seated at {0}", de: "Du sitzt an {0}" },
    /// An error line. `{0}` the gateway's name.
    ShellCouldNotReach { en: "Couldn't reach {0}", de: "{0} ist nicht erreichbar" },
    /// Play's empty tables list.
    ShellNoOneWaiting { en: "No one is waiting", de: "Niemand wartet" },
    /// A list read a moment ago.
    ShellUpdatedJustNow { en: "updated just now", de: "gerade aktualisiert" },
    /// A table filter.
    ShellOpenSeats { en: "Open seats", de: "Freie Plätze" },
    /// A table filter.
    ShellNoPassword { en: "No password", de: "Ohne Passwort" },
    /// The decks screen's first tab.
    ShellMyDecks { en: "My decks", de: "Meine Decks" },
    /// The decks screen's second tab.
    ShellHouseDecks { en: "House decks", de: "Hausdecks" },
    /// A deck was deleted; the toast offers Undo.
    ShellDeckDeleted { en: "Deck deleted", de: "Deck gelöscht" },
    /// The account popover.
    ShellCopyHandle { en: "Copy my handle", de: "Mein Handle kopieren" },
    /// The text-size row (Display & Interface).
    ShellTextSize { en: "Text size", de: "Textgröße" },
    /// The text-size row's help.
    ShellTextSizeHelp { en: "Ctrl/Cmd + and − change it anywhere outside a game.", de: "Strg/Cmd + und − ändern sie überall außerhalb eines Spiels." },
    /// The master volume row.
    ShellMasterVolume { en: "Master volume", de: "Gesamtlautstärke" },
    /// An account row.
    ShellHoldStill { en: "Hold the table still", de: "Tisch ruhig halten" },
    /// A drawer of expert fields.
    ShellAdvanced { en: "Advanced", de: "Erweitert" },
    /// The dev gallery's title.
    ShellGallery { en: "Shell components", de: "Shell-Bausteine" },
    /// A shell key: focus the screen's search.
    ShellKeySearch { en: "Search", de: "Suchen" },
    /// A shell key: the shortcuts overlay.
    ShellKeyOverlay { en: "Keyboard shortcuts", de: "Tastenkürzel" },
    /// A shell key: the overlay in go-to mode.
    ShellKeyQuickSwitch { en: "Go to…", de: "Gehe zu …" },
    /// A shell key: read this screen's data again.
    ShellKeyRefresh { en: "Refresh", de: "Aktualisieren" },
    /// A shell key: text one step larger.
    ShellKeyTextLarger { en: "Larger text", de: "Größerer Text" },
    /// A shell key: text one step smaller.
    ShellKeyTextSmaller { en: "Smaller text", de: "Kleinerer Text" },
    /// A shell key: text back to the default size.
    ShellKeyTextReset { en: "Default text size", de: "Normale Textgröße" },
    /// A shell key: start the game (the room's host).
    ShellKeyStart { en: "Start the game", de: "Spiel starten" },
    /// A shell key: save the deck.
    ShellKeySave { en: "Save deck", de: "Deck speichern" },
    /// A shell key: export the deck.
    ShellKeyExport { en: "Export", de: "Exportieren" },
    /// The shortcuts overlay's search field, while empty.
    ShellOverlaySearch { en: "Search shortcuts", de: "Kürzel suchen" },
    /// The shortcuts overlay's way to Settings › Controls.
    ShellOverlayEditKeys { en: "Edit keys…", de: "Tasten ändern …" },
    /// Closes a sheet.
    ShellClose { en: "Close", de: "Schließen" },
    /// The shortcuts overlay, when nothing matches the search.
    ShellOverlayNoMatch { en: "No shortcut matches", de: "Kein Kürzel passt" },
    /// The seated strip at a waiting table. `{0}` the table, `{1}` chairs
    /// taken, `{2}` chairs, `{3}` ready.
    ShellSeatedStrip { en: "Seated at \u{201c}{0}\u{201d} · {1} of {2} seated · {3} ready", de: "Am Tisch \u{201e}{0}\u{201c} · {1} von {2} sitzen · {3} bereit" },
    /// The seated strip while the game is played. `{0}` the table.
    ShellPlayingStrip { en: "Playing at \u{201c}{0}\u{201d}", de: "Du spielst an \u{201e}{0}\u{201c}" },
    /// The strip's way back to the table.
    ShellReturn { en: "Return", de: "Zurück" },
    /// The strip's way to give the chair up (waiting tables only).
    ShellLeave { en: "Leave", de: "Verlassen" },
    /// The gateway pill's counts. `{0}` tables waiting, `{1}` players online.
    ShellTablesOnline { en: "{0} tables · {1} online", de: "{0} Tische · {1} online" },
    /// The gateway pill's count before `/lobby/stats` has answered. `{0}`
    /// tables waiting.
    ShellTablesWaiting { en: "{0} waiting", de: "{0} warten" },
    /// The lobby feed is down and being dialled again.
    ShellReconnecting { en: "Reconnecting\u{2026}", de: "Verbinde neu\u{2009}\u{2026}" },
    /// Dial the lobby feed again now.
    ShellRetryNow { en: "Retry now", de: "Jetzt versuchen" },
    /// The front door after a `401`: the session ended, nothing typed is lost.
    ShellSessionEnded { en: "Signed out \u{2014} your session ended · Sign in again", de: "Abgemeldet \u{2014} deine Sitzung ist abgelaufen · Melde dich neu an" },
    /// The account pill's menu: sign out.
    ShellSignOut { en: "Sign out", de: "Abmelden" },
    /// The account pill's menu: back to the gateway picker.
    ShellSwitchGateway { en: "Switch gateway", de: "Gateway wechseln" },
    /// The account pill's menu: the report form.
    ShellReportProblem { en: "Report a problem", de: "Problem melden" },
    /// The bell, with nothing in it.
    ShellBellEmpty { en: "Nothing new", de: "Nichts Neues" },
    /// A bell line: somebody sat down at the player's table. `{0}` who.
    ShellBellJoined { en: "{0} sat down at your table", de: "{0} hat sich an deinen Tisch gesetzt" },
    /// A bell line: somebody left the player's table. `{0}` who.
    ShellBellLeft { en: "{0} left your table", de: "{0} hat deinen Tisch verlassen" },
    /// A bell line: every chair at the player's table is taken.
    ShellBellFilled { en: "Your table is full", de: "Dein Tisch ist voll" },
    /// The gateway pill while there is no gateway.
    ShellOfflinePill { en: "Offline · local decks", de: "Offline · lokale Decks" },
    /// The gateway popover: this client's version follows.
    ShellThisClient { en: "This client", de: "Dieser Client" },
    /// The gateway popover's counts. `{0}` players online, `{1}` tables
    /// waiting, `{2}` games running.
    ShellStatsLine { en: "{0} online · {1} waiting · {2} playing", de: "{0} online · {1} warten · {2} spielen" },
    /// The gateway now refuses this client's protocol: an update is needed.
    ShellUpdateRequired { en: "Update required \u{2014} this gateway runs a newer version", de: "Update n\u{f6}tig \u{2014} dieses Gateway l\u{e4}uft mit einer neueren Version" },
    // ---- front door + terms (WP1) ----------------------------------------
    /// The badge beside a closed-beta gateway's name on the front door.
    FrontClosedBeta { en: "closed beta", de: "geschlossene Beta" },
    /// The front door's version verdict when nothing is wrong.
    FrontCompatible { en: "compatible", de: "kompatibel" },
    /// The front door's line naming this client's own version. `{0}`.
    FrontThisClient { en: "this client {0}", de: "dieser Client {0}" },
    /// Under Create account and Play as guest on a closed beta.
    FrontKeyHint { en: "This gateway needs a beta key \u{2014} the key box opens under the button you choose.", de: "Dieses Gateway braucht einen Beta-Schl\u{fc}ssel \u{2014} das Feld \u{f6}ffnet sich unter dem Knopf, den du w\u{e4}hlst." },
    /// Under the username when an account is made.
    FrontUsernameHint { en: "Signs you in; nobody else sees it", de: "Zum Anmelden; niemand sonst sieht ihn" },
    /// Under a password while Caps Lock looks on.
    FrontCapsLock { en: "Caps Lock is on", de: "Feststelltaste ist an" },
    /// The gateway did not answer: the head of the sign-in face. `{0}` the
    /// gateway.
    FrontUnreachable { en: "Can\u{2019}t reach {0}", de: "{0} ist nicht erreichbar" },
    /// Asks the gateway again.
    FrontRetry { en: "Retry", de: "Erneut" },
    /// The guest's face's title and its button.
    FrontGuestTitle { en: "Play as guest", de: "Als Gast spielen" },
    /// The text row's door to the About sheet.
    FrontAbout { en: "About", de: "\u{dc}ber" },
    /// The one-line colophon's notice: the policy's words, shortened; the
    /// full notice is on the About sheet one press away (`docs/legal.md`).
    ColophonNotice { en: "Unofficial Fan Content", de: "Inoffizieller Fan-Content" },
    /// The one-line colophon's Scryfall credit.
    ColophonScryfall { en: "Card data and images by Scryfall", de: "Kartendaten und -bilder von Scryfall" },
    /// The one-line colophon's source link.
    ColophonSource { en: "Source (AGPL-3.0)", de: "Quellcode (AGPL-3.0)" },
    /// The About sheet's title.
    AboutTitle { en: "About Baylee", de: "\u{dc}ber Baylee" },
    /// The About sheet: the licence of this program.
    AboutLicence { en: "Baylee is free software: you may run, study, share and change it under the GNU Affero General Public License, version 3. Whoever runs a changed gateway offers its players that version\u{2019}s source.", de: "Baylee ist freie Software: Du darfst sie unter der GNU Affero General Public License, Version 3, ausf\u{fc}hren, untersuchen, weitergeben und ver\u{e4}ndern. Wer ein ver\u{e4}ndertes Gateway betreibt, bietet seinen Spielern den Quellcode dieser Fassung an." },
    /// The About sheet: the heading over the third-party licences.
    AboutThirdParty { en: "Third-party licences", de: "Lizenzen Dritter" },
    /// The About sheet: the interface's and the card text's typefaces.
    AboutFonts { en: "Alegreya Sans (Juan Pablo del Peral, Huerta Tipogr\u{e1}fica) and Faustina (Omnibus-Type): SIL Open Font License 1.1.", de: "Alegreya Sans (Juan Pablo del Peral, Huerta Tipogr\u{e1}fica) und Faustina (Omnibus-Type): SIL Open Font License 1.1." },
    /// The About sheet: the icon font.
    AboutIcons { en: "Font Awesome Free 6.7.2 (Fonticons, Inc.): the font files under the SIL Open Font License 1.1.", de: "Font Awesome Free 6.7.2 (Fonticons, Inc.): die Schriftdateien unter der SIL Open Font License 1.1." },
    /// The About sheet: the mana font.
    AboutMana { en: "Mana 1.18 (Andrew Gioia): SIL Open Font License 1.1. The symbols it draws are Wizards of the Coast\u{2019}s.", de: "Mana 1.18 (Andrew Gioia): SIL Open Font License 1.1. Die Symbole, die sie zeichnet, geh\u{f6}ren Wizards of the Coast." },
    /// The About sheet: the engine and the libraries.
    AboutCrates { en: "Bevy and the other Rust crates this build links: each under its own licence (MIT, Apache-2.0 and the like), held to an allow-list by cargo-deny; NOTICE in the source names the rest.", de: "Bevy und die anderen Rust-Crates dieses Builds: jede unter ihrer eigenen Lizenz (MIT, Apache-2.0 und \u{e4}hnliche), von cargo-deny gegen eine Liste gepr\u{fc}ft; NOTICE im Quellcode nennt den Rest." },
    /// The About sheet: the preconstructed deck lists.
    AboutPrecons { en: "Preconstructed deck lists from MTGJSON (Zach Halpern): MIT License.", de: "Vorgefertigte Decklisten von MTGJSON (Zach Halpern): MIT-Lizenz." },
    /// Closes a sheet.
    SheetClose { en: "Close", de: "Schlie\u{df}en" },
    /// The terms sheet's title. `{0}` the gateway.
    TermsTitle { en: "Before you play at {0}", de: "Bevor du bei {0} spielst" },
    /// The terms sheet's caption. `{0}` the version.
    TermsVersion { en: "Terms of this gateway \u{b7} version {0}", de: "Bedingungen dieses Gateways \u{b7} Version {0}" },
    /// After the version: when the operator last changed them. `{0}`.
    TermsUpdated { en: "updated {0}", de: "aktualisiert {0}" },
    /// While the text is asked for.
    TermsLoading { en: "Loading the terms\u{2026}", de: "Die Bedingungen werden geladen \u{2026}" },
    /// The text could not be read; nothing is accepted.
    TermsLoadFailed { en: "Couldn\u{2019}t load the terms", de: "Die Bedingungen konnten nicht geladen werden" },
    /// Asks for the text again.
    TermsRetry { en: "Retry", de: "Erneut" },
    /// Signs out, nothing stored (a guest is asked first).
    TermsNotNow { en: "Not now", de: "Nicht jetzt" },
    /// Accepts the terms shown.
    TermsAccept { en: "Accept & continue", de: "Annehmen & weiter" },
    /// Why Accept waits.
    TermsScrollToAccept { en: "Read to the end to accept", de: "Bis zum Ende lesen, um anzunehmen" },
    /// While the acceptance is on its way.
    TermsSending { en: "Sending\u{2026}", de: "Wird gesendet \u{2026}" },
    /// What accepting stores, and where.
    TermsStoredNote { en: "Read to the end to accept. Your acceptance (version and time) is stored with your account on this gateway; this device remembers it too.", de: "Lies bis zum Ende, um anzunehmen. Deine Zustimmung (Version und Zeit) wird mit deinem Konto auf diesem Gateway gespeichert; dieses Ger\u{e4}t merkt sie sich auch." },
    /// A guest pressed Not now.
    TermsGuestAsk { en: "A guest that signs out loses its decks. Sign out anyway?", de: "Ein Gast, der sich abmeldet, verliert seine Decks. Trotzdem abmelden?" },
    /// The guest stays and reads on.
    TermsStay { en: "Stay", de: "Bleiben" },
    /// The guest signs out after all.
    TermsSignOut { en: "Sign out", de: "Abmelden" },
    /// Declines the terms and deletes the account (its own confirmation
    /// follows). The terms text names this button word for word.
    TermsDecline { en: "Decline and delete account", de: "Ablehnen und Konto l\u{f6}schen" },
    // ---- play + room (WP2) -----------------------------------------------
    /// A clock's span in minutes. `{0}` minutes.
    ClockMinutes { en: "{0} min", de: "{0} Min." },
    /// A clock's span in seconds. `{0}` seconds.
    ClockSeconds { en: "{0} s", de: "{0} s" },
    /// The `classic` clock's name: three minutes, the default.
    ClockClassic { en: "classic", de: "klassisch" },
    /// The `casual` clock's name.
    ClockCasual { en: "casual", de: "gemütlich" },
    /// The `standard` clock's name.
    ClockStandard { en: "standard", de: "normal" },
    /// The `blitz` clock's name.
    ClockBlitz { en: "blitz", de: "Blitz" },
    /// The `untimed` clock's name.
    ClockUntimed { en: "untimed", de: "ohne Uhr" },
    /// The `casual` clock's help line.
    ClockCasualHelp { en: "Ten minutes a decision.", de: "Zehn Minuten pro Entscheidung." },
    /// The `classic` clock's help line.
    ClockClassicHelp { en: "Three minutes a decision; the default.", de: "Drei Minuten pro Entscheidung; die Voreinstellung." },
    /// The `standard` clock's help line.
    ClockStandardHelp { en: "Two minutes a decision.", de: "Zwei Minuten pro Entscheidung." },
    /// The `blitz` clock's help line.
    ClockBlitzHelp { en: "Thirty seconds a decision.", de: "Dreißig Sekunden pro Entscheidung." },
    /// The `untimed` clock's help line.
    ClockUntimedHelp { en: "No decision clock; an absent player can still be stood in for.", de: "Keine Entscheidungsuhr; für Abwesende springt trotzdem jemand ein." },
    /// A table filter: leave out tables of house AIs only.
    PlayHideAiOnly { en: "Hide AI-only", de: "Ohne KI-Tische" },
    /// A table filter: two-chair tables.
    PlayDuel { en: "Duel", de: "Duell" },
    /// The tables' sort: the gateway's order.
    PlaySortNewest { en: "newest", de: "Neueste zuerst" },
    /// The tables' sort: most open chairs first.
    PlaySortOpenSeats { en: "open seats", de: "Freie Plätze" },
    /// The tables' sort: by name.
    PlaySortName { en: "name", de: "Name" },
    /// Before a Join: the next game's deck is of another format than the
    /// host's. `{0}` mine, `{1}` the host's.
    PlayFormatWarning { en: "Your next-game deck is {0}; this table's host plays {1}. You can pick another deck in the room.", de: "Dein Deck fürs nächste Spiel ist {0}; der Gastgeber spielt {1}. Im Raum kannst du ein anderes wählen." },
    /// A template card: Commander.
    TemplateCommander { en: "Commander", de: "Commander" },
    /// A template card: a twenty-life duel.
    TemplateDuel { en: "Duel 20", de: "Duell 20" },
    /// A template card: every seat starts with five lands.
    TemplateFiveLand { en: "Five-land start", de: "Fünf Länder" },
    /// The rules line: starting life. `{0}` life.
    RulesLife { en: "{0} life", de: "{0} Leben" },
    /// The rules line: one free mulligan. `{0}` the count.
    RulesMulliganOne { en: "{0} free mulligan", de: "{0} freier Mulligan" },
    /// The rules line: free mulligans. `{0}` the count.
    RulesMulliganMany { en: "{0} free mulligans", de: "{0} freie Mulligans" },
    /// The rules line: the five-land start.
    RulesFiveLands { en: "five lands in play", de: "fünf Länder im Spiel" },
    /// The footer hint: Enter on a table row.
    PlayHintJoin { en: "on a row to join", de: "auf einer Zeile: mitspielen" },
    /// Why the starts are off while the player's game runs. `{0}` the table.
    PlayPlayingAt { en: "You are playing at {0}", de: "Du spielst an {0}" },
    /// Why the starts are off without a game host (`POST /lobby/games` 503).
    PlayNoHost { en: "This gateway has no game host right now", de: "Dieses Gateway hat gerade keinen Spielhost" },
    /// The hero on first run, and why Play the house waits.
    PlayPickToStart { en: "Pick a deck to start", de: "Wähle ein Deck zum Start" },
    /// The hero's caption.
    PlayYourNextGame { en: "Your next game", de: "Dein nächstes Spiel" },
    /// The hero: choose another deck.
    PlayChangeDeck { en: "Change deck", de: "Deck wechseln" },
    /// Recent games' caption.
    PlayRecentGames { en: "Recent games · this session", de: "Letzte Spiele · diese Sitzung" },
    /// A recent game against the house alone.
    PlayVsHouse { en: "vs the house", de: "gegen das Haus" },
    /// A recent game's opponents. `{0}` their names.
    PlayVs { en: "vs {0}", de: "gegen {0}" },
    /// A recent game won.
    PlayWon { en: "won", de: "gewonnen" },
    /// A recent game lost.
    PlayLost { en: "lost", de: "verloren" },
    /// A recent game drawn.
    PlayDrawn { en: "draw", de: "unentschieden" },
    /// A recent game again, one press.
    PlayRematch { en: "Rematch", de: "Revanche" },
    /// A recent game's settings in the Create-table sheet.
    PlayEditAndRematch { en: "Edit and rematch", de: "Ändern & Revanche" },
    /// Over the tables while the player holds a chair. `{0}` the table.
    PlayJoinOff { en: "Join is off while you are seated at {0}", de: "Mitspielen ist aus, solange du an {0} sitzt" },
    /// A table being played.
    PlayPlaying { en: "playing", de: "läuft" },
    /// A table's chairs. `{0}` taken, `{1}` chairs.
    PlaySeats { en: "{0}/{1} seats", de: "{0}/{1} Plätze" },
    /// A table's house chairs. `{0}` the count.
    PlayAiCount { en: "AI ×{0}", de: "KI ×{0}" },
    /// A locked table's inline password box, while empty.
    PlayPassword { en: "Password", de: "Passwort" },
    /// The sheet's name field.
    SheetName { en: "Name", de: "Name" },
    /// The sheet's players control.
    SheetPlayers { en: "Players", de: "Spieler" },
    /// The sheet's templates.
    SheetTemplate { en: "Template", de: "Vorlage" },
    /// The sheet's rules drawer.
    SheetAdjust { en: "Adjust", de: "Anpassen" },
    /// The sheet's password field.
    SheetPassword { en: "Password · optional", de: "Passwort · optional" },
    /// The sheet's empty password box.
    SheetNoPassword { en: "No password", de: "Kein Passwort" },
    /// The sheet's clock control.
    SheetClock { en: "Clock", de: "Uhr" },
    /// The sheet's way out.
    SheetCancel { en: "Cancel", de: "Abbrechen" },
    /// The sheet's primary over a room.
    SheetApply { en: "Apply", de: "Übernehmen" },
    /// The sheet's primary.
    SheetOpenTable { en: "Open table", de: "Tisch öffnen" },
    /// The sheet's title over a room.
    SheetEditRules { en: "Edit rules", de: "Regeln ändern" },
    /// A new table's name. `{0}` the host's name.
    SheetDefaultName { en: "{0}\u{2019}s table", de: "Tisch von {0}" },
    /// The room's badge, and its rules line, for a locked room.
    RoomPasswordSet { en: "password set", de: "Passwort gesetzt" },
    /// The room's Copy invite.
    RoomCopyInvite { en: "Copy invite", de: "Einladung kopieren" },
    /// Copy invite, once copied.
    RoomInviteCopied { en: "Invite copied", de: "Kopiert" },
    /// The rules rail's first line. `{0}` the format, `{1}` the chairs.
    RoomRulesPlayers { en: "{0} · {1} players", de: "{0} · {1} Spieler" },
    /// The rail: no sides.
    RoomTeamsOn { en: "Teams: on", de: "Teams: an" },
    /// The rail: sides.
    RoomTeamsOff { en: "Teams: off", de: "Teams: aus" },
    /// The rail: sides for each chair.
    RoomSetTeams { en: "Set teams", de: "Teams festlegen" },
    /// A chair nobody holds.
    RoomEmptySeat { en: "\u{2014} empty \u{2014}", de: "\u{2014} frei \u{2014}" },
    /// Why Start waits. `{0}` who.
    RoomWaitingFor { en: "waiting for {0} to be ready", de: "wartet, bis {0} bereit ist" },
    /// Why Start waits, said by the gateway's listing.
    RoomNotStartable { en: "not every chair is settled yet", de: "noch ist nicht jeder Platz bereit" },
    /// A seat's menu: hand the room on.
    RoomMakeHost { en: "Make host", de: "Zum Gastgeber machen" },
    /// A seat's menu: the next side.
    RoomNextTeam { en: "Next team", de: "Nächstes Team" },
    /// A seat's menu: its own starting life.
    RoomLifeOverride { en: "Life override…", de: "Eigene Lebenspunkte …" },
    /// A seat's menu: its starting board.
    RoomStartingPosition { en: "Starting position…", de: "Startposition …" },
    /// A seat's menu: the house or a model out, a person's chair again.
    RoomMakeOpen { en: "Make it an open seat", de: "Zum freien Platz machen" },
    /// Under the house AI's name.
    RoomHouseAi { en: "house AI", de: "Haus-KI" },
    /// A language model's chair, and the button that seats one.
    RoomLanguageModel { en: "Language model", de: "Sprachmodell" },
    /// Under the host's name.
    RoomHost { en: "Host", de: "Gastgeber" },
    /// Under a delegated chair. `{0}` whose.
    RoomFor { en: "for {0}", de: "für {0}" },
    /// An open chair's AI menu button.
    RoomAi { en: "AI", de: "KI" },
    /// Before another player's deck.
    RoomDeckWord { en: "deck", de: "Deck" },
    /// The seat's deck plays the host's format.
    RoomFits { en: "fits", de: "passt" },
    /// The seat's deck plays another format. `{0}` the host's.
    RoomOtherFormat { en: "{0} table", de: "{0}-Tisch" },
    /// Over a seat's drawer.
    RoomForTesting { en: "For testing and puzzles", de: "Für Tests und Rätsel" },
    /// The chair sheet's primary.
    RoomSeat { en: "Seat", de: "Setzen" },
    /// The chair sheet without a model to seat.
    RoomNoModel { en: "No language model is set up on this computer yet.", de: "Auf diesem Rechner ist noch kein Sprachmodell eingerichtet." },
    /// The chair sheet's way to set one up.
    RoomSetUpModel { en: "Set up a model…", de: "Modell einrichten …" },
    /// The chair sheet's title. `{0}` the chair.
    RoomChairTitle { en: "Chair {0} · language model", de: "Platz {0} · Sprachmodell" },
    /// The host's Leave, asked (DESIGN-v5 §5). `{0}` the table's name.
    LeaveHostingQuestion { en: "Leave \u{201c}{0}\u{201d}?", de: "\u{201e}{0}\u{201c} verlassen?" },
    /// Under it: what leaving does to the table.
    LeaveHostingHint {
        en: "You host this table: it passes to the player seated longest, or closes if nobody else is seated.",
        de: "Du leitest diesen Tisch: Er geht an den, der am l\u{e4}ngsten sitzt, oder schlie\u{df}t, wenn sonst niemand sitzt.",
    },
    /// Why Start waits: one chair nobody holds. `{0}` the count.
    RoomOpenSeatOne { en: "{0} seat is open \u{2014} a player, the house or a model takes it", de: "{0} Platz ist frei \u{2014} ein Spieler, das Haus oder ein Modell nimmt ihn" },
    /// Why Start waits: chairs nobody holds. `{0}` the count.
    RoomOpenSeatMany { en: "{0} seats are open \u{2014} players, the house or models take them", de: "{0} Plätze sind frei \u{2014} Spieler, das Haus oder Modelle nehmen sie" },
    // ---- decks, house decks, history (WP3) -------------------------------
    /// The decks' sort: newest save first.
    DecksSortSaved { en: "last saved", de: "Speicherdatum" },
    /// The decks' sort: by name.
    DecksSortName { en: "name", de: "Name" },
    /// The decks' sort: by format.
    DecksSortFormat { en: "format", de: "Format" },
    /// The decks' sort: by colour identity.
    DecksSortColours { en: "colours", de: "Farben" },
    /// A deck's format: Commander.
    FormatCommander { en: "Commander", de: "Commander" },
    /// A deck's format: anything without a commander.
    FormatFreeform { en: "Freeform", de: "Frei" },
    /// A deck saved under a minute ago.
    SavedJustNow { en: "saved just now", de: "gerade gespeichert" },
    /// A deck saved minutes ago. `{0}` minutes.
    SavedMinutesAgo { en: "saved {0} min ago", de: "vor {0} Min. gespeichert" },
    /// A deck saved hours ago. `{0}` hours.
    SavedHoursAgo { en: "saved {0} h ago", de: "vor {0} Std. gespeichert" },
    /// A deck saved yesterday.
    SavedYesterday { en: "saved yesterday", de: "gestern gespeichert" },
    /// A deck saved days ago. `{0}` days.
    SavedDaysAgo { en: "saved {0} days ago", de: "vor {0} Tagen gespeichert" },
    /// A tile's meta line. `{0}` cards, `{1}` sideboard cards.
    DeckMeta { en: "{0} · SB {1}", de: "{0} · SB {1}" },
    /// A tile's badge: the deck the next game is played with.
    DecksNextGame { en: "next game", de: "nächstes Spiel" },
    /// A tile's badge: one card will not play. `{0}` the count.
    DecksUnplayableOne { en: "{0} card won\u{2019}t play", de: "{0} Karte spielt nicht" },
    /// A tile's badge: cards that will not play. `{0}` the count.
    DecksUnplayableMany { en: "{0} cards won\u{2019}t play", de: "{0} Karten spielen nicht" },
    /// The shelf's search box, while empty.
    DecksSearchHint { en: "Search decks…", de: "Decks suchen …" },
    /// Play's table sort button. `{0}` the order (German: the order alone,
    /// which says it).
    PlaySortBy { en: "Sort: {0}", de: "{0}" },
    /// The sort button. `{0}` the sort.
    DecksSortBy { en: "Sort: {0}", de: "Nach {0}" },
    /// The shelf, when the search matched nothing. `{0}` the search.
    DecksNoMatch { en: "No deck matches \u{201c}{0}\u{201d}", de: "Kein Deck passt zu \u{201e}{0}\u{201c}" },
    /// A tile's menu: a copy of the deck.
    DecksDuplicate { en: "Duplicate", de: "Duplizieren" },
    /// A tile's menu: the deck's saved versions.
    DecksHistory { en: "History…", de: "Verlauf …" },
    /// A tile's menu: star the deck.
    DecksFavourite { en: "Favourite", de: "Favorit" },
    /// A tile's menu: unstar it.
    DecksUnfavourite { en: "No longer a favourite", de: "Kein Favorit mehr" },
    /// The empty shelf.
    DecksEmptyTitle { en: "Your shelf is empty \u{2014} start from a house deck, build one, or import a list.", de: "Dein Regal ist leer \u{2014} beginne mit einem Hausdeck, baue eins oder importiere eine Liste." },
    /// The New deck tile's line under a pointer, natively.
    DecksNewTileHint { en: "or import a list \u{2014} Ctrl/Cmd+V or drop a file", de: "oder eine Liste importieren \u{2014} Strg/Cmd+V oder Datei ablegen" },
    /// The New deck tile's line under a finger, or in a browser.
    DecksNewTileTouch { en: "or import a list \u{203a}", de: "oder eine Liste importieren \u{203a}" },
    /// A house deck: a copy to the shelf, staying here.
    HouseAdd { en: "Add", de: "Hinzufügen" },
    /// A house deck: its cards, in a sheet.
    HousePreview { en: "Preview", de: "Ansehen" },
    /// The artist's credit under a picture. `{0}` the artist.
    ArtCredit { en: "Art · {0}", de: "Illus. · {0}" },
    /// The history sheet's title. `{0}` the deck.
    HistoryTitle { en: "History · {0}", de: "Verlauf · {0}" },
    /// A past version. `{0}` the number.
    HistoryRow { en: "v{0}", de: "v{0}" },
    /// The current version. `{0}` the number.
    HistoryCurrentRow { en: "v{0} · current", de: "v{0} · aktuell" },
    /// Over the changes: what this version holds against the deck now.
    HistoryAgainstCurrent { en: "Against the current version", de: "Gegenüber der aktuellen Version" },
    /// Over the changes: this version is the deck as it is.
    HistoryIsCurrent { en: "This is the deck as it is now", de: "So ist das Deck jetzt" },
    /// List every card of the version.
    HistoryShowAll { en: "Show all cards", de: "Alle Karten" },
    /// The history sheet's primary.
    HistoryRestore { en: "Restore version", de: "Wiederherstellen" },
    /// Why Restore is off: the version shown is the deck as it is.
    HistoryRestoreCurrent { en: "This is the current version", de: "Das ist die aktuelle Version" },
    /// The toast after a restore, with Undo.
    HistoryRestored { en: "Version restored", de: "Version wiederhergestellt" },
    // ---- the deck builder (WP4) ------------------------------------------
    /// The builder's title while the deck has no name yet.
    BuildUntitled { en: "New deck", de: "Neues Deck" },
    /// The header's save button.
    BuildSave { en: "Save", de: "Speichern" },
    /// Why Save is off: nothing has changed since the last save.
    BuildNothingToSave { en: "Nothing to save", de: "Nichts zu speichern" },
    /// The save state: one edit not saved yet.
    BuildUnsavedOne { en: "Unsaved · 1 change", de: "Ungespeichert · 1 Änderung" },
    /// The save state: `{0}` edits not saved yet.
    BuildUnsavedMany { en: "Unsaved · {0} changes", de: "Ungespeichert · {0} Änderungen" },
    /// The save state while the request is out.
    BuildSaving { en: "Saving\u{2026}", de: "Speichere\u{2009}\u{2026}" },
    /// The save state a moment after a save.
    BuildSavedNow { en: "Saved · just now", de: "Gespeichert · gerade eben" },
    /// The save state `{0}` minutes after a save.
    BuildSavedAgo { en: "Saved · {0} min ago", de: "Gespeichert · vor {0} Min." },
    /// The save state of a deck opened and not changed.
    BuildSaved { en: "Saved", de: "Gespeichert" },
    /// The save state of a new deck nothing was done to yet.
    BuildNotSaved { en: "Not saved yet", de: "Noch nicht gespeichert" },
    /// The save state after the gateway refused or did not answer.
    BuildSaveFailed { en: "Couldn't save", de: "Nicht gespeichert" },
    /// The header menu: rename the deck.
    BuildRename { en: "Rename", de: "Umbenennen" },
    /// The header menu: empty the deck (it asks first).
    BuildEmptyDeck { en: "Empty the deck\u{2026}", de: "Deck leeren\u{2009}\u{2026}" },
    /// The header menu: the settings screen.
    BuildSettings { en: "Builder settings", de: "Einstellungen" },
    /// Back with unsaved changes: the question.
    BuildDiscardQuestion { en: "Discard changes?", de: "Änderungen verwerfen?" },
    /// Back with unsaved changes: what is at stake.
    BuildDiscardBody { en: "This deck has unsaved changes. A draft stays on this device until you save or discard it.", de: "Dieses Deck hat ungespeicherte Änderungen. Ein Entwurf bleibt auf diesem Gerät, bis du speicherst oder verwirfst." },
    /// Back with unsaved changes: stay.
    BuildKeepEditing { en: "Keep editing", de: "Weiter bearbeiten" },
    /// Back with unsaved changes: leave without saving.
    BuildDiscard { en: "Discard", de: "Verwerfen" },
    /// A kept draft came back when its deck was opened again.
    BuildDraftRestored { en: "Your unsaved draft is back", de: "Dein ungespeicherter Entwurf ist zurück" },
    /// The search box's syntax help, beside the box.
    BuildSyntax { en: "syntax", de: "Syntax" },
    /// The syntax help's title.
    BuildSyntaxTitle { en: "What the search understands", de: "Was die Suche versteht" },
    /// Syntax help: `t:`.
    BuildSyntaxType { en: "type line contains", de: "Typzeile enthält" },
    /// Syntax help: `o:`.
    BuildSyntaxText { en: "rules text contains", de: "Regeltext enthält" },
    /// Syntax help: `c:`.
    BuildSyntaxColour { en: "at least these colours", de: "mindestens diese Farben" },
    /// Syntax help: `id<=`.
    BuildSyntaxIdentity { en: "colour identity within", de: "Farbidentität innerhalb" },
    /// Syntax help: `mv`.
    BuildSyntaxMana { en: "mana value", de: "Manawert" },
    /// Syntax help: `pow`.
    BuildSyntaxPower { en: "power (also tou, loy)", de: "Stärke (auch tou, loy)" },
    /// Syntax help: `m:`.
    BuildSyntaxCost { en: "mana cost holds these symbols", de: "Manakosten enthalten diese Symbole" },
    /// Syntax help: `is:commander`.
    BuildSyntaxCommander { en: "can lead a deck", de: "kann ein Deck anführen" },
    /// Syntax help: `is:partial`.
    BuildSyntaxPartial { en: "plays with a known gap", de: "spielbar mit bekannter Lücke" },
    /// Syntax help: a minus.
    BuildSyntaxNot { en: "not", de: "nicht" },
    /// Syntax help: `!"…"`.
    BuildSyntaxExact { en: "exactly this name", de: "genau dieser Name" },
    /// Syntax help: `or`.
    BuildSyntaxOr { en: "either", de: "eines von beiden" },
    /// The pool's filter disclosure. Chip budget 14.
    BuildFilters { en: "Filters", de: "Filter" },
    /// The filter rail's type section.
    BuildRailType { en: "Card type", de: "Kartentyp" },
    /// The filter rail's mana value section.
    BuildRailMana { en: "Mana value", de: "Manawert" },
    /// The filter rail's switches.
    BuildRailShow { en: "Show", de: "Zeigen" },
    /// The pool's count line, then what narrows it: `{0}`.
    BuildAlsoNarrowing { en: "also narrowing: {0}", de: "außerdem: {0}" },
    /// A pool row's menu: choose a printing.
    BuildChoosePrinting { en: "Choose printing\u{2026}", de: "Druck wählen\u{2009}\u{2026}" },
    /// A pool row's menu: open the card sheet.
    BuildOpenCard { en: "Open card", de: "Karte öffnen" },
    /// A pool row's menu, or the card sheet: add to the main deck.
    BuildAddToMain { en: "Add to main", de: "Ins Hauptdeck" },
    /// A pool row's menu, or the card sheet: add to the sideboard.
    BuildAddToSide { en: "Add to sideboard", de: "Ins Sideboard" },
    /// A deck row's menu: move one copy to the sideboard.
    BuildMoveToSide { en: "Move to sideboard", de: "Ins Sideboard schieben" },
    /// A deck row's menu: move one copy to the main deck.
    BuildMoveToMain { en: "Move to main", de: "Ins Hauptdeck schieben" },
    /// A deck row's menu: take the whole row out.
    BuildRemoveAll { en: "Remove all", de: "Alle entfernen" },
    /// The key-hint footer: arrows.
    BuildHintMove { en: "move", de: "bewegen" },
    /// The key-hint footer: Enter.
    BuildHintAdd { en: "add", de: "hinzufügen" },
    /// The key-hint footer: Shift+Enter.
    BuildHintOther { en: "other list", de: "andere Liste" },
    /// The key-hint footer: `/`.
    BuildHintSearch { en: "search", de: "suchen" },
    /// The key-hint footer: Space.
    BuildHintPreview { en: "card", de: "Karte" },
    /// The commander slot's label.
    BuildCommander { en: "Commander", de: "Commander" },
    /// The commander slot with nobody in it.
    BuildNoCommander { en: "none", de: "keiner" },
    /// The commander slot's way to choose one.
    BuildChoose { en: "Choose", de: "Wählen" },
    /// The commander slot's way to add a partner.
    BuildPartner { en: "Partner", de: "Partner" },
    /// Deck tab: the main deck. Tab budget 16.
    BuildTabMain { en: "Main", de: "Haupt" },
    /// Deck tab: the sideboard.
    BuildTabSide { en: "Sideboard", de: "Sideboard" },
    /// Deck tab: the statistics.
    BuildTabStats { en: "Stats", de: "Statistik" },
    /// The deck's grouping: by card type.
    BuildByType { en: "By type", de: "Nach Typ" },
    /// The deck's grouping: by mana value.
    BuildByManaValue { en: "By mana value", de: "Nach Manawert" },
    /// The deck's grouping: by colour.
    BuildByColour { en: "By colour", de: "Nach Farbe" },
    /// Fold every section of the deck list.
    BuildCollapseAll { en: "Collapse all", de: "Alle zuklappen" },
    /// Open every section of the deck list.
    BuildExpandAll { en: "Expand all", de: "Alle aufklappen" },
    /// A mana-value section. `{0}` the value.
    BuildManaValue { en: "Mana value {0}", de: "Manawert {0}" },
    /// The last mana-value section. `{0}` the value.
    BuildManaValueUp { en: "Mana value {0}+", de: "Manawert {0}+" },
    /// A colour section: two colours or more.
    BuildMulticolour { en: "Multicoloured", de: "Mehrfarbig" },
    /// The deck's footer: one card the engine does not fully play.
    BuildShakyOne { en: "1 card is not fully implemented", de: "1 Karte ist nicht vollständig umgesetzt" },
    /// The deck's footer: `{0}` cards the engine does not fully play.
    BuildShakyMany { en: "{0} cards are not fully implemented", de: "{0} Karten sind nicht vollständig umgesetzt" },
    /// Stats: the curve's heading.
    BuildManaCurve { en: "Mana curve", de: "Manakurve" },
    /// Stats: what the curve counts.
    BuildCurveNote { en: "nonland cards by mana value", de: "Nichtland-Karten nach Manawert" },
    /// Stats: which type the curve lights. `{0}` the type.
    BuildCurveLit { en: "{0} highlighted", de: "{0} hervorgehoben" },
    /// Stats: the colour pips' heading.
    BuildColourPips { en: "Colour pips", de: "Farbsymbole" },
    /// Stats: what the pips count.
    BuildPipsNote { en: "share of coloured symbols in the deck", de: "Anteil der farbigen Symbole im Deck" },
    /// Stats: the land share. `{0}` lands, `{1}` cards.
    BuildLandShare { en: "{0} of {1}", de: "{0} von {1}" },
    /// Stats: draw a sample hand.
    BuildDrawSeven { en: "Draw seven", de: "Sieben ziehen" },
    /// Stats: what Draw seven does.
    BuildDrawNote { en: "a fresh shuffle each press · no card leaves the deck", de: "jedes Mal neu gemischt · keine Karte verlässt das Deck" },
    /// Stats: nothing to count yet.
    BuildStatsEmpty { en: "Add cards to see the deck's numbers", de: "Füge Karten hinzu, um die Zahlen des Decks zu sehen" },
    /// The phone's deck rail: the newest additions.
    BuildLastAdded { en: "Last added", de: "Zuletzt hinzugefügt" },
    /// The narrow builder's tab bar: the pool.
    BuildPanePool { en: "Pool", de: "Pool" },
    /// The narrow builder's tab bar: the deck.
    BuildPaneDeck { en: "Deck", de: "Deck" },
    /// The builder with no card search on its gateway.
    BuildNoCatalog { en: "This gateway has no card search \u{2014} the pool shows the compiled cards only", de: "Dieses Gateway hat keine Kartensuche \u{2014} der Pool zeigt nur die eingebauten Karten" },
    // ---- settings, report, language models, updates (WP5) ---------------
    /// The settings section: graphics.
    SectionGraphics { en: "Graphics", de: "Grafik" },
    /// The settings section: sound.
    SectionAudio { en: "Audio", de: "Audio" },
    /// The settings section: language, text, preview.
    SectionDisplay { en: "Display & Interface", de: "Anzeige & Oberfl\u{e4}che" },
    /// The settings section: keys.
    SectionControls { en: "Controls", de: "Steuerung" },
    /// The settings section: automation.
    SectionGameplay { en: "Gameplay", de: "Spielablauf" },
    /// The settings section: the account.
    SectionAccount { en: "Account", de: "Konto" },
    /// The settings section: the gateway.
    SectionNetwork { en: "Network & Gateway", de: "Netzwerk & Gateway" },
    /// The settings section: the language-model seat.
    SectionLanguageModels { en: "Language models", de: "Sprachmodelle" },
    /// The settings section: the updater.
    SectionUpdates { en: "Updates", de: "Updates" },
    /// The settings section: what is kept and sent.
    SectionPrivacy { en: "Privacy & Data", de: "Datenschutz & Daten" },
    /// A row's storage tag: this device.
    TagDevice { en: "this device", de: "dieses Ger\u{e4}t" },
    /// A row's storage tag: the account, on every device.
    TagAccount { en: "your account", de: "dein Konto" },
    /// A row's storage tag: this machine's seat file.
    TagMachine { en: "this machine", de: "dieser Rechner" },
    /// The settings search field's hint.
    SettingsSearch { en: "Search settings\u{2026}", de: "Einstellungen suchen \u{2026}" },
    /// The settings search found nothing.
    SettingsNoMatch { en: "Nothing matches", de: "Nichts gefunden" },
    /// Puts the shown section's rows back.
    SettingsResetSection { en: "Reset this section", de: "Bereich zur\u{fc}cksetzen" },
    /// Display & Interface, on a chip.
    SectionDisplayShort { en: "Display", de: "Anzeige" },
    /// Network & Gateway, on a chip.
    SectionNetworkShort { en: "Network", de: "Netzwerk" },
    /// Language models, on a chip.
    SectionModelsShort { en: "Models", de: "Modelle" },
    /// Privacy & Data, on a chip.
    SectionPrivacyShort { en: "Privacy", de: "Datenschutz" },
    /// An account row before sign-in.
    SettingsSignInToChange { en: "Sign in to change", de: "Zum \u{c4}ndern anmelden" },
    /// The rule over Graphics' account rows.
    SettingsAccountRule { en: "Your account's rows hold on every device; a device row never raises them", de: "Die Zeilen deines Kontos gelten auf jedem Ger\u{e4}t; eine Ger\u{e4}tezeile hebt sie nie an" },
    /// The save line under a section.
    SettingsSaveLine { en: "Saved at once \u{b7} tagged rows go with your account", de: "Sofort gespeichert \u{b7} markierte Zeilen gehen mit deinem Konto" },
    /// The save line before sign-in.
    SettingsSaveLineOffline { en: "Saved on this device \u{b7} sign in and the tagged rows follow your account", de: "Auf diesem Ger\u{e4}t gespeichert \u{b7} melde dich an, und die markierten Zeilen folgen deinem Konto" },
    /// Graphics: the preset.
    RowPreset { en: "Preset", de: "Voreinstellung" },
    /// Graphics: what the preset does.
    HelpPreset { en: "Writes this device's rows below; Custom once one of them differs", de: "Setzt die Ger\u{e4}tezeilen darunter; Eigene, sobald eine davon abweicht" },
    /// Graphics: the GPU the preset was picked for. {0} its name.
    HelpPresetAuto { en: "Picked for {0}", de: "Gew\u{e4}hlt f\u{fc}r {0}" },
    /// Graphics: windowed, borderless or fullscreen.
    RowDisplayMode { en: "Display mode", de: "Anzeigemodus" },
    /// Graphics: the display mode's revert.
    HelpDisplayMode { en: "A change asks to be kept and reverts in 15 s", de: "Eine \u{c4}nderung fragt nach und wird nach 15 s zur\u{fc}ckgenommen" },
    /// A display mode.
    DisplayWindowed { en: "Windowed", de: "Fenster" },
    /// A display mode.
    DisplayBorderless { en: "Borderless", de: "Randlos" },
    /// A display mode.
    DisplayFullscreen { en: "Fullscreen", de: "Vollbild" },
    /// The 15-s revert's question. {0} seconds left.
    KeepDisplayMode { en: "Keep these settings? Reverts in {0} s", de: "Diese Einstellung behalten? Zur\u{fc}ck in {0} s" },
    /// Keeps a changed display mode.
    KeepIt { en: "Keep", de: "Behalten" },
    /// Puts the display mode back at once.
    RevertIt { en: "Revert", de: "Zur\u{fc}ck" },
    /// Graphics: which monitor.
    RowMonitor { en: "Monitor", de: "Bildschirm" },
    /// Graphics: the monitor row.
    HelpMonitor { en: "Which screen the window stands on", de: "Auf welchem Bildschirm das Fenster steht" },
    /// Graphics: edge smoothing's cost.
    HelpAntiAliasing { en: "Smooths card edges and lettering \u{b7} GPU \u{2191}", de: "Gl\u{e4}ttet Kartenkanten und Schrift \u{b7} GPU \u{2191}" },
    /// Graphics: anti-aliasing on a phone.
    AntiAliasingLocked { en: "Off on phones \u{2014} tilers pay most for it", de: "Auf Telefonen aus \u{2014} Kachel-GPUs zahlen am meisten daf\u{fc}r" },
    /// Graphics: vsync's effect.
    HelpVSync { en: "Waits for the display: no tearing, one frame of delay \u{b7} power \u{2193}", de: "Wartet auf den Bildschirm: kein Zerrei\u{df}en, ein Bild Verz\u{f6}gerung \u{b7} Strom \u{2193}" },
    /// Graphics: the frame limit's effect.
    HelpFrameLimit { en: "Caps frames per second \u{b7} lower is cooler and quieter", de: "Begrenzt die Bilder pro Sekunde \u{b7} weniger ist k\u{fc}hler und leiser" },
    /// Graphics: the rest limit's effect.
    HelpRestLimit { en: "Frames per second at a table where nothing has happened for two seconds \u{b7} power \u{2193}\u{2193}", de: "Bilder pro Sekunde am Tisch, wenn zwei Sekunden lang nichts geschieht \u{b7} Strom \u{2193}\u{2193}" },
    /// Graphics: the background limit's effect.
    HelpBackgroundLimit { en: "Frames per second while another window has the focus \u{b7} power \u{2193}\u{2193}", de: "Bilder pro Sekunde, w\u{e4}hrend ein anderes Fenster vorn ist \u{b7} Strom \u{2193}\u{2193}" },
    /// Graphics: the frame-time counter.
    RowShowFrameRate { en: "Show frame rate", de: "Bildrate anzeigen" },
    /// Graphics: the counter.
    HelpShowFrameRate { en: "A small counter in a corner; a report carries it too", de: "Ein kleiner Z\u{e4}hler in einer Ecke; ein Bericht tr\u{e4}gt ihn mit" },
    /// Graphics: the painting behind the panels.
    RowBackdrop { en: "Backdrop", de: "Hintergrund" },
    /// Graphics: the backdrop's effect.
    HelpBackdrop { en: "The painting behind every screen; Plain is the easiest to read and the cheapest frame", de: "Das Gem\u{e4}lde hinter jedem Bildschirm; Schlicht liest sich am leichtesten und kostet am wenigsten" },
    /// A backdrop.
    BackdropPainting { en: "Painting", de: "Gem\u{e4}lde" },
    /// A backdrop.
    BackdropDimmed { en: "Dimmed", de: "Gedimmt" },
    /// A backdrop.
    BackdropPlain { en: "Plain", de: "Schlicht" },
    /// Graphics: ambient detail's effect.
    HelpAmbient { en: "Drifting light and dust; never above your account's Atmosphere \u{b7} GPU \u{2191}", de: "Treibendes Licht und Staub; nie \u{fc}ber der Atmosph\u{e4}re deines Kontos \u{b7} GPU \u{2191}" },
    /// Ambient detail: standing still.
    AmbientStill { en: "Still", de: "Ruhig" },
    /// Ambient detail: the lighter one.
    AmbientSoft { en: "Soft", de: "Sanft" },
    /// Ambient detail: everything.
    AmbientFull { en: "Full", de: "Voll" },
    /// Graphics: the account's atmosphere.
    HelpAtmosphere { en: "The ceiling for ambient detail on every device", de: "Die Obergrenze f\u{fc}r Umgebungsdetails auf jedem Ger\u{e4}t" },
    /// Graphics: the account's reduce-motion switch.
    RowHoldStill { en: "Hold the table still", de: "Tisch ruhig halten" },
    /// Graphics: holding the table still.
    HelpHoldStill { en: "Cards jump instead of gliding, everywhere; every rule stays", de: "Karten springen statt zu gleiten, \u{fc}berall; jede Regel bleibt" },
    /// Graphics: the sky.
    HelpSky { en: "The light behind the table: follows your clock, or stays at day or night", de: "Das Licht hinter dem Tisch: folgt deiner Uhr oder bleibt bei Tag oder Nacht" },
    /// Audio: the master volume.
    HelpMaster { en: "Every sound, the music included", de: "Jeder Klang, die Musik eingeschlossen" },
    /// Audio: the music.
    RowMusic { en: "Music", de: "Musik" },
    /// Audio: the music's theme.
    RowMusicTheme { en: "Music theme", de: "Musik-Thema" },
    /// Audio: what the four themes are.
    HelpMusicTheme { en: "Ballad: a lyrical folk song · Dance: a driving minor dance · Epic: broad and heroic · Jig: a playful medieval jig · Rotating: a different one each game. It changes at the next bar, no restart", de: "Ballade: ein lyrisches Volkslied · Tanz: ein treibender Moll-Tanz · Episch: weit und heroisch · Jig: eine verspielte mittelalterliche Gigue · Wechselnd: jedes Spiel ein anderes. Es wechselt mit dem n\u{e4}chsten Takt, ohne Neustart" },
    /// Audio: theme A.
    MusicThemeBallad { en: "Ballad", de: "Ballade" },
    /// Audio: theme B.
    MusicThemeDance { en: "Dance", de: "Tanz" },
    /// Audio: theme C.
    MusicThemeEpic { en: "Epic", de: "Episch" },
    /// Audio: theme D.
    MusicThemeJig { en: "Jig", de: "Jig" },
    /// Audio: a different theme each game.
    MusicThemeRotating { en: "Rotating", de: "Wechselnd" },
    /// Audio: the music's volume.
    HelpMusic { en: "The lobby's own score; every lobby screen has its switch", de: "Die eigene Musik der Lobby; jeder Lobby-Bildschirm hat ihren Schalter" },
    /// Audio: the table's sounds.
    HelpEffects { en: "The table's sounds, under your account's Table sounds", de: "Die Kl\u{e4}nge des Tisches, unter den Tischkl\u{e4}ngen deines Kontos" },
    /// Audio: silence behind other windows.
    HelpMuteUnfocused { en: "Silence while another window has the focus", de: "Stille, w\u{e4}hrend ein anderes Fenster vorn ist" },
    /// Audio: the account's table sounds.
    RowTableSounds { en: "Table sounds", de: "Tischkl\u{e4}nge" },
    /// Audio: the account's ceiling.
    HelpTableSounds { en: "Off, half or full on every device; Off wins over every slider", de: "Aus, halb oder voll auf jedem Ger\u{e4}t; Aus gewinnt gegen jeden Regler" },
    /// Display: the language.
    HelpLanguage { en: "The interface's language, and the card text's where the gateway has it", de: "Die Sprache der Oberfl\u{e4}che und, wo das Gateway sie hat, der Kartentexte" },
    /// Display: the five text steps.
    RowTextSize { en: "Text size", de: "Textgr\u{f6}\u{df}e" },
    /// Display: the text steps.
    HelpTextSize { en: "Five steps for every screen outside the table \u{b7} Ctrl/Cmd + = \u{2212} 0", de: "F\u{fc}nf Stufen f\u{fc}r jeden Bildschirm au\u{df}erhalb des Tisches \u{b7} Strg/Cmd + = \u{2212} 0" },
    /// Display: the card preview's size.
    RowPreviewSize { en: "Card preview size", de: "Gr\u{f6}\u{df}e der Kartenvorschau" },
    /// Display: the preview.
    HelpPreviewSize { en: "How large a card shows when pointed at", de: "Wie gro\u{df} eine Karte erscheint, auf die gezeigt wird" },
    /// Display: the text face.
    RowTextFace { en: "Prefer the text face", de: "Textansicht bevorzugen" },
    /// Display: the text face's effect.
    HelpTextFace { en: "Draw cards as text instead of their print; the modifier key swaps for as long as it is held", de: "Karten als Text statt als Druck zeichnen; die Zusatztaste tauscht, solange sie gehalten wird" },
    /// Controls: the shell's shortcuts.
    RowShellKeys { en: "Shortcuts", de: "Tastenk\u{fc}rzel" },
    /// Controls: rebinding a shortcut.
    HelpShellKeys { en: "The keys of every screen outside the table; a key already in use is refused, a second press takes it", de: "Die Tasten jedes Bildschirms au\u{df}erhalb des Tisches; eine belegte Taste wird abgelehnt, ein zweiter Druck nimmt sie" },
    /// Controls: the table's keys.
    RowTableKeys { en: "Table keys", de: "Tasten am Tisch" },
    /// Controls: the table's keys.
    HelpTableKeys { en: "The keys while a game is open", de: "Die Tasten, w\u{e4}hrend ein Spiel offen ist" },
    /// Gameplay: automation.
    HelpAutomation { en: "What the client answers for you; no rule changes", de: "Was der Client f\u{fc}r dich beantwortet; keine Regel \u{e4}ndert sich" },
    /// Gameplay: kept ability answers.
    RowAbilityAnswers { en: "Ability answers", de: "Antworten auf F\u{e4}higkeiten" },
    /// Gameplay: kept ability answers.
    HelpAbilityAnswers { en: "Answers you asked the client to keep giving", de: "Antworten, die der Client f\u{fc}r dich weiter geben soll" },
    /// Account: the handle.
    RowHandle { en: "Handle", de: "Name" },
    /// Account: the handle.
    HelpHandle { en: "What other players see you as", de: "Wie andere Spieler dich sehen" },
    /// Account: copies the handle.
    CopyHandle { en: "Copy my handle", de: "Meinen Namen kopieren" },
    /// Account: signing out.
    RowSignOut { en: "Sign out", de: "Abmelden" },
    /// Account: what signing out does.
    HelpSignOut { en: "Ends this session here and on the gateway", de: "Beendet diese Sitzung hier und auf dem Gateway" },
    /// Account: what deleting does.
    HelpDeleteAccount { en: "The account, its decks and its settings, for good", de: "Das Konto, seine Decks und seine Einstellungen, f\u{fc}r immer" },
    /// Network: the gateway.
    RowGateway { en: "Gateway", de: "Gateway" },
    /// Network: the gateway row.
    HelpGateway { en: "Where this client plays, and which version it runs", de: "Wo dieser Client spielt und welche Version dort l\u{e4}uft" },
    /// Network: back to the list.
    HelpSwitchGateway { en: "Back to the list of gateways", de: "Zur\u{fc}ck zur Liste der Gateways" },
    /// Network: the connection's state.
    RowDiagnostics { en: "Diagnostics", de: "Diagnose" },
    /// Network: diagnostics.
    HelpDiagnostics { en: "The connection's state as text, to copy into a report", de: "Der Zustand der Verbindung als Text, zum Kopieren in einen Bericht" },
    /// Copies diagnostics.
    CopyAsText { en: "Copy as text", de: "Als Text kopieren" },
    /// Network: playing offline.
    NetworkOffline { en: "Offline \u{b7} local decks, no gateway", de: "Offline \u{b7} lokale Decks, kein Gateway" },
    /// Language models: the profiles.
    RowProfiles { en: "Profiles", de: "Profile" },
    /// Opens a profile's sheet.
    ProfileOpen { en: "Edit", de: "Bearbeiten" },
    /// The report's attachments, a disclosure.
    ReportAttachments { en: "Attachments", de: "Anh\u{e4}nge" },
    /// Beside it: how many are ticked of how many this report has.
    ReportAttachmentsCount { en: "{0} of {1}", de: "{0} von {1}" },
    /// Where a report goes, signed in: the gateway, by name or address.
    ReportGoesGateway { en: "Goes to {0} with your session", de: "Geht mit deiner Sitzung an {0}" },
    /// Updates: checking on its own.
    RowCheckUpdates { en: "Check automatically", de: "Automatisch pr\u{fc}fen" },
    /// Updates: what checking sends.
    HelpCheckUpdates { en: "Asks GitHub at start and every six hours; GitHub sees your IP address and this version. Off: no request at all", de: "Fragt GitHub beim Start und alle sechs Stunden; GitHub sieht deine IP-Adresse und diese Version. Aus: gar keine Anfrage" },
    /// Updates: installing on its own.
    RowInstallUpdates { en: "Install automatically", de: "Automatisch installieren" },
    /// Updates: installing.
    HelpInstallUpdates { en: "Downloads a signed update and installs it when you quit", de: "L\u{e4}dt ein signiertes Update und installiert es beim Beenden" },
    /// Updates: asking now.
    RowCheckNow { en: "Check now", de: "Jetzt pr\u{fc}fen" },
    /// Updates: what was found, and why it does not install.
    HelpCheckNow { en: "What the last check found, and why an update does not install itself", de: "Was die letzte Pr\u{fc}fung fand, und warum sich ein Update nicht selbst installiert" },
    /// Privacy: what is kept and sent.
    RowWhatIsKept { en: "What is kept and sent", de: "Was gespeichert und gesendet wird" },
    /// Privacy: the true list.
    HelpWhatIsKept { en: "The gateway keeps your account, decks, settings and which terms you accepted. This client sends, on its own, only update checks to GitHub (desktop, while checking is on), crash reports once you agree, and asks Scryfall for card images in a browser", de: "Das Gateway speichert dein Konto, deine Decks, Einstellungen und welche Bedingungen du angenommen hast. Dieser Client sendet von selbst nur Update-Pr\u{fc}fungen an GitHub (Desktop, solange gepr\u{fc}ft wird), Absturzberichte nach deiner Zustimmung und fragt im Browser Scryfall nach Kartenbildern" },
    /// Privacy: what a report may carry.
    RowReportConsent { en: "What a report may carry", de: "Was ein Bericht mitnehmen darf" },
    /// Privacy: consent.
    HelpReportConsent { en: "Ticked here, ticked on the report form; a report shows exactly what is sent", de: "Hier angehakt, im Berichtsformular angehakt; ein Bericht zeigt genau, was gesendet wird" },
    /// Privacy: crash reports.
    RowCrashReports { en: "Crash reports", de: "Absturzberichte" },
    /// Privacy: crash reports.
    HelpCrashReports { en: "After a crash: send a report, ask each time, or never", de: "Nach einem Absturz: Bericht senden, jedes Mal fragen oder nie" },
    /// Crash reports: always.
    CrashSend { en: "Send", de: "Senden" },
    /// Crash reports: ask.
    CrashAsk { en: "Ask", de: "Fragen" },
    /// Crash reports: never.
    CrashNever { en: "Never", de: "Nie" },
    /// Privacy: the report form.
    HelpReportProblem { en: "Opens the report form (F8)", de: "\u{d6}ffnet das Berichtsformular (F8)" },
    /// Opens the report form.
    ReportOpen { en: "Report a problem", de: "Problem melden" },
    /// Language models on a browser or phone build.
    LanguageModelsElsewhere { en: "Language models are set up on a desktop client", de: "Sprachmodelle werden in einem Desktop-Client eingerichtet" },
    /// Controls: a key no shortcut may take.
    KeyFixed { en: "This key is fixed and cannot be bound", de: "Diese Taste ist fest und l\u{e4}sst sich nicht belegen" },
    /// Controls: a key another shortcut holds. `{0}` the holder.
    KeyHeldBy { en: "Already {0} \u{2014} press the key again, or take it", de: "Schon {0} \u{2014} dr\u{fc}cke die Taste noch einmal oder nimm sie" },
    /// Controls: takes the refused key.
    KeyTake { en: "Take it", de: "Nehmen" },
    /// Gameplay: no ability answers kept.
    NoAbilityAnswers { en: "None kept yet", de: "Noch keine" },
    /// A volume at nothing.
    SliderOff { en: "off", de: "aus" },
}

#[cfg(test)]
mod tests;
