//! Localized, known server messages. Unknown diagnostics remain intact.
use super::Lang;

/// Translate server-owned status keys without changing card or player names.
#[must_use]
pub fn server_message(lang: Lang, message: &str) -> String {
    if lang == Lang::En {
        return message.to_owned();
    }
    let detail = message.strip_prefix("casting: ").unwrap_or(message);
    let detail = detail.strip_prefix("illegal action: ").unwrap_or(detail);
    if let Some((_, translated)) = MESSAGES.iter().find(|(english, _)| *english == detail) {
        return (*translated).to_owned();
    }
    message.to_owned()
}

const MESSAGES: &[(&str, &str)] = &[
    (
        "this gateway takes no guests",
        "Dieses Gateway nimmt keine Gäste auf.",
    ),
    (
        "no guest seats free, sign up or try later",
        "Keine Gastplätze frei — registriere dich oder versuch es später.",
    ),
    (
        "guests cannot upload images",
        "Gäste können keine Bilder hochladen.",
    ),
    ("game is over", "Das Spiel ist beendet."),
    (
        "action does not match the pending request",
        "Diese Aktion passt nicht zur aktuellen Entscheidung.",
    ),
    (
        "illegal action for your seat",
        "Diese Aktion ist für deinen Platz gerade nicht möglich.",
    ),
    (
        "not a human seat",
        "Dieser Platz wird nicht von einem Spieler gesteuert.",
    ),
    ("object vanished", "Das Objekt ist nicht mehr verfügbar."),
    (
        "card is not in your hand",
        "Die Karte ist nicht auf deiner Hand.",
    ),
    (
        "sorcery-speed timing not met",
        "Das ist nur in deiner Hauptphase bei leerem Stapel möglich.",
    ),
    (
        "variable costs not supported yet",
        "Variable Kosten werden hier noch nicht unterstützt.",
    ),
    (
        "a cost that has to ask reached the payer unanswered",
        "Für diese Kosten fehlt noch eine Entscheidung.",
    ),
    (
        "a draw offer needs priority",
        "Du brauchst Priorität, um ein Unentschieden anzubieten.",
    ),
    (
        "a granted ability cannot ask for its cost yet",
        "Diese verliehene Fähigkeit unterstützt noch keine Kostenauswahl.",
    ),
    (
        "ability not activatable",
        "Diese Fähigkeit kann gerade nicht aktiviert werden.",
    ),
    (
        "ability not usable from this zone",
        "Diese Fähigkeit kann aus dieser Zone nicht benutzt werden.",
    ),
    (
        "activated abilities of artifacts can't be activated (Karn)",
        "Karn verhindert das Aktivieren von Artefaktfähigkeiten.",
    ),
    (
        "activation condition not met",
        "Die Aktivierungsbedingung ist nicht erfüllt.",
    ),
    (
        "an activation cost is not a target choice",
        "Für diese Aktivierung müssen Kosten statt Ziele gewählt werden.",
    ),
    (
        "cannot pay the cost",
        "Die Kosten können nicht bezahlt werden.",
    ),
    (
        "cannot pay the spell's cost",
        "Die Kosten des Zauberspruchs können nicht bezahlt werden.",
    ),
    (
        "cannot pay the suspend cost",
        "Die Aussetzen-Kosten können nicht bezahlt werden.",
    ),
    (
        "cannot pay the total cost",
        "Die Gesamtkosten können nicht bezahlt werden.",
    ),
    (
        "card cannot be suspended",
        "Diese Karte kann nicht ausgesetzt werden.",
    ),
    ("card not in hand", "Die Karte ist nicht auf deiner Hand."),
    (
        "choose exactly one legendary permanent to keep",
        "Wähle genau eine legendäre bleibende Karte, die bleiben soll.",
    ),
    ("color not allowed", "Diese Farbe ist nicht erlaubt."),
    (
        "creature cannot attack",
        "Diese Kreatur kann nicht angreifen.",
    ),
    ("creature cannot block", "Diese Kreatur kann nicht blocken."),
    (
        "duplicate attacker",
        "Ein Angreifer wurde doppelt ausgewählt.",
    ),
    ("duplicate blocker", "Ein Blocker wurde doppelt ausgewählt."),
    (
        "invalid card selection",
        "Diese Kartenauswahl ist nicht erlaubt.",
    ),
    ("invalid defender", "Dieses Angriffsziel ist nicht erlaubt."),
    (
        "invalid target selection",
        "Diese Zielauswahl ist nicht erlaubt.",
    ),
    (
        "land not playable now",
        "Dieses Land kann gerade nicht gespielt werden.",
    ),
    (
        "loyalty already used this turn",
        "Eine Loyalitätsfähigkeit wurde in diesem Zug bereits aktiviert.",
    ),
    (
        "mana ability not activatable",
        "Diese Manafähigkeit kann gerade nicht aktiviert werden.",
    ),
    (
        "menace requires two blockers",
        "Bedrohlichkeit erfordert mindestens zwei Blocker.",
    ),
    (
        "must bottom exactly the required number of cards",
        "Lege genau die geforderte Anzahl Karten unter die Bibliothek.",
    ),
    (
        "must discard exactly the required number",
        "Wirf genau die geforderte Anzahl Karten ab.",
    ),
    (
        "no card to cast from exile",
        "Es gibt keine spielbare Karte im Exil.",
    ),
    (
        "no card to exile for the pitch cost",
        "Es gibt keine passende Karte zum Bezahlen durch Exilieren.",
    ),
    (
        "no granted ability",
        "Diese verliehene Fähigkeit ist nicht verfügbar.",
    ),
    ("no legal targets", "Es gibt keine erlaubten Ziele."),
    ("no prepared spell", "Es ist kein Zauberspruch vorbereitet."),
    (
        "no subtype choice pending",
        "Es wird gerade kein Untertyp gewählt.",
    ),
    ("no such ability", "Diese Fähigkeit ist nicht verfügbar."),
    ("no such attacker", "Dieser Angreifer ist nicht verfügbar."),
    ("no such card", "Diese Karte ist nicht verfügbar."),
    ("no such cast mode", "Diese Spielweise ist nicht verfügbar."),
    (
        "no such permanent",
        "Diese bleibende Karte ist nicht verfügbar.",
    ),
    (
        "no trigger awaiting a mode",
        "Keine ausgelöste Fähigkeit wartet auf eine Modusauswahl.",
    ),
    (
        "no way to cast this spell",
        "Dieser Zauberspruch kann gerade auf keine Weise gewirkt werden.",
    ),
    (
        "nobody left to offer a draw to",
        "Es ist niemand mehr da, dem ein Unentschieden angeboten werden kann.",
    ),
    ("not a creature type", "Das ist kein Kreaturentyp."),
    (
        "not a loyalty ability",
        "Das ist keine Loyalitätsfähigkeit.",
    ),
    ("not a miracle card", "Diese Karte hat kein Mirakulum."),
    (
        "not a permutation of the offered objects",
        "Die Reihenfolge muss genau die angebotenen Objekte enthalten.",
    ),
    ("not a suspend card", "Diese Karte hat kein Aussetzen."),
    (
        "not an activated ability",
        "Das ist keine aktivierte Fähigkeit.",
    ),
    (
        "not card-backed",
        "Dieses Objekt gehört zu keiner gedruckten Karte.",
    ),
    (
        "not enough counters to pay the cost",
        "Es sind nicht genug Marken zum Bezahlen vorhanden.",
    ),
    (
        "not enough legal targets",
        "Es gibt nicht genug erlaubte Ziele.",
    ),
    (
        "not enough loyalty",
        "Es ist nicht genug Loyalität vorhanden.",
    ),
    ("not enough mana", "Es ist nicht genug Mana vorhanden."),
    (
        "nothing can pay this cost",
        "Diese Kosten können mit nichts bezahlt werden.",
    ),
    (
        "number outside the offered range",
        "Die Zahl liegt außerhalb des erlaubten Bereichs.",
    ),
    (
        "player not among the options",
        "Dieser Spieler gehört nicht zur Auswahl.",
    ),
    (
        "spell not castable now",
        "Dieser Zauberspruch kann gerade nicht gewirkt werden.",
    ),
    (
        "the prepared spell cannot be cast right now",
        "Der vorbereitete Zauberspruch kann gerade nicht gewirkt werden.",
    ),
    (
        "the untap determination is not a target choice",
        "Wähle, welche bleibenden Karten enttappt werden sollen, statt Ziele zu wählen.",
    ),
    ("unknown card", "Unbekannte Karte."),
    ("unknown linked card", "Unbekannte verknüpfte Karte."),
    (
        "a deck has at most two commanders",
        "Ein Deck darf höchstens zwei Commander haben.",
    ),
    (
        "a handle carries a #tag",
        "Der Spielername muss einen #Tag enthalten.",
    ),
    (
        "a table seats between two and eight",
        "Ein Tisch braucht zwei bis acht Plätze.",
    ),
    ("account gone", "Dieses Konto ist nicht mehr verfügbar."),
    (
        "card catalog not configured",
        "Es ist kein Kartenkatalog eingerichtet.",
    ),
    (
        "card catalog request failed",
        "Der Kartenkatalog konnte nicht abgefragt werden.",
    ),
    ("card note too long", "Die Kartennotiz ist zu lang."),
    ("deck data missing", "Die Deckdaten fehlen."),
    ("deck too large", "Das Deck ist zu groß."),
    ("empty card pool", "Der Kartenvorrat ist leer."),
    (
        "every seat is on the same team; a game needs at least two sides",
        "Alle Plätze gehören zum selben Team. Es werden mindestens zwei Seiten benötigt.",
    ),
    ("game already started", "Das Spiel hat bereits begonnen."),
    (
        "hashing failed",
        "Das Passwort konnte nicht verarbeitet werden.",
    ),
    ("house deck missing", "Das vorgefertigte Deck fehlt."),
    (
        "invalid card count (1-4, unlimited for basic lands)",
        "Ungültige Kartenanzahl: 1–4, bei Standardländern unbegrenzt.",
    ),
    ("invalid card list", "Die Kartenliste ist ungültig."),
    (
        "invalid credentials",
        "Benutzername oder Passwort stimmen nicht.",
    ),
    ("invalid deck name", "Der Deckname ist ungültig."),
    ("invalid display name", "Der Anzeigename ist ungültig."),
    ("invalid username", "Der Benutzername ist ungültig."),
    (
        "invalid or expired token",
        "Die Anmeldung ist ungültig oder abgelaufen. Bitte melde dich erneut an.",
    ),
    (
        "invalid password",
        "Das Passwort erfüllt die Anforderungen nicht.",
    ),
    (
        "invalid seat token",
        "Die Berechtigung für diesen Platz ist ungültig.",
    ),
    ("invalid sideboard", "Das Sideboard ist ungültig."),
    (
        "kind must be human or ai",
        "Der Platz muss für einen Spieler oder die KI eingerichtet sein.",
    ),
    (
        "malformed card count",
        "Die Kartenanzahl ist falsch geschrieben.",
    ),
    (
        "malformed card line",
        "Die Kartenzeile hat ein ungültiges Format.",
    ),
    ("missing bearer token", "Bitte melde dich zuerst an."),
    (
        "no engine available",
        "Momentan ist keine Spielengine verfügbar.",
    ),
    ("no such AI", "Diese KI ist nicht verfügbar."),
    ("no such deck", "Dieses Deck ist nicht verfügbar."),
    ("no such game", "Dieses Spiel ist nicht verfügbar."),
    ("no such image", "Dieses Bild ist nicht verfügbar."),
    ("no such image kind", "Diese Bildart ist nicht verfügbar."),
    ("no such player", "Dieser Spieler wurde nicht gefunden."),
    ("no such seat", "Diesen Platz gibt es nicht."),
    ("no such team", "Dieses Team gibt es nicht."),
    ("no such version", "Diese Version ist nicht verfügbar."),
    ("nobody is sitting there", "Dieser Platz ist leer."),
    (
        "not everyone is ready",
        "Noch nicht alle Spieler sind bereit.",
    ),
    ("not your deck", "Dieses Deck gehört dir nicht."),
    ("not your room", "Dieser Raum gehört dir nicht."),
    ("not your seat", "Das ist nicht dein Platz."),
    (
        "only the host arranges seats",
        "Nur der Gastgeber kann die Plätze einrichten.",
    ),
    (
        "only the host starts the game",
        "Nur der Gastgeber kann das Spiel starten.",
    ),
    ("pick a deck first", "Wähle zuerst ein Deck."),
    (
        "registration is disabled",
        "Neue Konten können momentan nicht registriert werden.",
    ),
    (
        "settings must be an object",
        "Die Einstellungen haben ein ungültiges Format.",
    ),
    (
        "settings too large",
        "Die Einstellungen sind zu umfangreich.",
    ),
    (
        "someone is sitting there",
        "Dieser Platz ist bereits besetzt.",
    ),
    (
        "that card cannot be a commander",
        "Diese Karte kann kein Commander sein.",
    ),
    (
        "that game is not over yet",
        "Dieses Spiel ist noch nicht beendet.",
    ),
    ("that game is over", "Dieses Spiel ist beendet."),
    (
        "that is the deck's current state",
        "Das ist bereits die aktuelle Version des Decks.",
    ),
    ("that link has expired", "Dieser Link ist abgelaufen."),
    ("that link is not valid", "Dieser Link ist ungültig."),
    (
        "that seat is not an AI",
        "Dieser Platz wird nicht von einer KI gesteuert.",
    ),
    ("that seat is taken", "Dieser Platz ist bereits vergeben."),
    (
        "that username is taken",
        "Dieser Benutzername ist bereits vergeben.",
    ),
    (
        "the dev board could not be dealt",
        "Der Testtisch konnte nicht aufgebaut werden.",
    ),
    (
        "the gateway's database is unavailable",
        "Die Datenbank des Gateways ist nicht erreichbar.",
    ),
    (
        "the rematch has already started",
        "Die Revanche hat bereits begonnen.",
    ),
    (
        "the room is no longer ready",
        "Der Raum ist nicht mehr startbereit.",
    ),
    ("the table is full", "Der Tisch ist voll."),
    (
        "those two cards cannot lead one deck",
        "Diese beiden Karten können nicht gemeinsam ein Deck anführen.",
    ),
    ("too many attempts", "Zu viele Versuche. Bitte warte kurz."),
    (
        "too many remembered answers",
        "Es wurden zu viele Antworten gespeichert.",
    ),
    ("unknown commander", "Unbekannter Commander."),
    ("unknown finish", "Unbekannte Kartenveredelung."),
    ("unknown language", "Unbekannte Sprache."),
    ("upload failed", "Das Hochladen ist fehlgeschlagen."),
    (
        "verify failed",
        "Die Anmeldung konnte nicht geprüft werden.",
    ),
    ("wrong password", "Das Passwort stimmt nicht."),
    (
        "you are already at this table",
        "Du sitzt bereits an diesem Tisch.",
    ),
    (
        "you are not at this table",
        "Du sitzt nicht an diesem Tisch.",
    ),
    (
        "you were not at that table",
        "Du hast nicht an diesem Tisch gespielt.",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn known_errors_translate_without_dropping_unknown_diagnostics() {
        for (english, german) in MESSAGES {
            assert!(!german.is_empty());
            assert_eq!(server_message(Lang::En, english), *english);
            assert_eq!(server_message(Lang::De, english), *german);
        }
        assert_eq!(
            server_message(Lang::De, "casting: illegal action: not enough mana"),
            "Es ist nicht genug Mana vorhanden."
        );
        assert_eq!(
            server_message(Lang::De, "unrecognized server diagnostic 42"),
            "unrecognized server diagnostic 42"
        );
    }
}
