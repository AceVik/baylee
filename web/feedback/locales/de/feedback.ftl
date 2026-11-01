### The feedback service's web UI in German (web/feedback, docs/feedback.md).
### The glossary in docs/translations.md is binding: Magic's own terms as the
### German cards print them, the words admins say in English left English.

nav-reports = Berichte
nav-admin = Admin
nav-label = Bereiche
nav-signOut = Abmelden
nav-contact = Kontakt
nav-skip = Zum Inhalt springen
nav-signedIn = Angemeldet als { $name }

contact-title = Kontaktnachrichten
contact-summary = { $open } offen · aus dem Formular der Startseite
contact-filter = Anzeigen
contact-open = Offen
contact-done = Erledigt
contact-all = Alle
contact-none = Hier sind keine Nachrichten.
contact-anonymous = Ohne Namen
contact-noEmail = keine Antwortadresse
contact-reply = Per E-Mail antworten
contact-replySubject = Deine Nachricht an Baylee
contact-language = Sprache der Seite
contact-received = Eingegangen
contact-doneAt = Erledigt am
contact-markDone = Als erledigt markieren
contact-reopen = Wieder öffnen
contact-delete = Löschen
contact-deleteConfirm = Diese Nachricht endgültig löschen? Nur die Audit-Zeile bleibt.
contact-deleteYes = Löschen
contact-deleteNo = Behalten
contact-deleted = Die Nachricht wurde gelöscht.
contact-back = Alle Nachrichten
contact-history = Verlauf
contact-noHistory = Noch nichts damit gemacht.
contact-kept = Bleibt 90 Tage nach „erledigt“ erhalten, höchstens ein Jahr, und wird dann von selbst gelöscht.
contact-shown = { $n } von { $total }

admin-title = Übersicht
admin-updated = Stand { $time }
admin-paused = Pausiert, solange die Seite verborgen ist
admin-refresh = Aktualisieren
admin-loading = Lädt…
admin-noAnswer = Der Dienst hat nicht geantwortet.

stats-players = Spieler
stats-registered = Registrierte Accounts
stats-confirmed = mit bestätigter Adresse
stats-online = Jetzt online
stats-inLobby = in der Lobby
stats-seated = am Tisch
stats-sessions = Aktive Sitzungen
stats-signedIn = Accounts angemeldet
stats-guests = Gäste
stats-guestCap = von { $cap }
stats-guestsOff = Gäste aus
stats-noCap = keine Obergrenze
stats-newAccounts = Neue Accounts
stats-games = Partien
stats-running = Jetzt laufend
stats-local = auf diesem Rechner
stats-waiting = Wartende Tische
stats-started = Begonnene Partien
stats-finished = Beendete Partien
stats-total = insgesamt
stats-today = heute (UTC)
stats-week = 7 Tage
stats-month = 30 Tage
stats-server = Server
stats-agents = Agenten
stats-capacity = Kapazität
stats-unlimited = unbegrenzt
stats-agentGames = Partien auf Agenten
stats-version = Build
stats-registration = Registrierung
stats-keys = Beta-Schlüssel
stats-activeKeys = Gültige Schlüssel
stats-usesLeft = Einlösungen übrig
stats-admitted = Freigeschaltete Accounts
stats-usedUp = aufgebraucht
stats-expired = abgelaufen
stats-revoked = widerrufen

invites-title = Schlüssel für die geschlossene Beta
invites-create = Schlüssel erzeugen
invites-count = Anzahl
invites-uses = Accounts pro Schlüssel
invites-expires = Läuft ab
invites-never = Nie
invites-hours12 = In 12 Stunden
invites-day1 = In 1 Tag
invites-days7 = In 7 Tagen
invites-days30 = In 30 Tagen
invites-days90 = In 90 Tagen
invites-note = Notiz (für wen)
invites-notePlaceholder = z. B. Max
invites-making = Wird erzeugt…
invites-made =
    { $n ->
        [one] { $n } neuer Schlüssel: nur jetzt sichtbar
       *[other] { $n } neue Schlüssel: nur jetzt sichtbar
    }
invites-madeHint = Gespeichert wird von jedem Schlüssel nur ein Hash. Kopiere sie, bevor du die Seite verlässt.
invites-copy = Kopieren
invites-copyAll = Alle kopieren
invites-copied = Kopiert
invites-share = Teilen
invites-done = Fertig
invites-list = Alle Schlüssel
invites-none = Noch keine Schlüssel.
invites-created = Erzeugt
invites-state = Status
invites-usesLeft = Übrig
invites-admitted = Freigeschaltet
invites-expiresAt = Läuft ab
invites-noteColumn = Notiz
invites-revoke = Widerrufen…
invites-revokeConfirm = Diesen Schlüssel widerrufen? Danach kann ihn niemand mehr nutzen.
invites-revokeYes = Endgültig widerrufen
invites-revokeNo = Behalten
invites-revoked = Schlüssel widerrufen.
invites-neverExpires = nie
invites-noKeyShown = Schlüssel werden nicht aufgelistet: Gespeichert ist nur ihr Hash.

state-active = gültig
state-used_up = aufgebraucht
state-expired = abgelaufen
state-revoked = widerrufen

audit-title = Letzte Änderungen
audit-none = Hier wurde noch nichts geändert.

sec-label = Admin-Bereiche
sec-overview = Übersicht
sec-live = Live
sec-accounts = Accounts
sec-keys = Beta-Schlüssel

admin-auto = Automatisch aktualisieren
admin-retry = Erneut versuchen

kpi-online = Jetzt online
kpi-onlineSub = { $lobby } in der Lobby · { $table } am Tisch
kpi-games = Laufende Partien
kpi-gamesSub =
    { $waiting ->
        [one] { $waiting } Tisch wartet
       *[other] { $waiting } Tische warten
    }
kpi-accounts = Accounts
kpi-accountsSub = { $registered } registriert · { $guests } Gäste
kpi-today = Neu heute
kpi-todaySub = { $registered } registriert · { $guests } Gäste
kpi-played = Partien heute
kpi-playedSub = { $finished } beendet
kpi-reports = Neue Berichte
kpi-reportsSub = { $total } Berichte insgesamt
kpi-offline = Jetzt offline
kpi-offlineSub = anonym, nur Clients mit Zustimmung

chart-games = Begonnene Partien pro Tag
chart-players = Spieler, die gespielt haben, pro Tag
chart-accounts = Neue Accounts pro Tag
chart-registered = Registriert
chart-guests = Gäste
chart-started = Partien
chart-finished = Beendet
chart-playersSeries = Spieler
chart-table = Als Tabelle zeigen
chart-day = Tag (UTC)
chart-sum = { $n } in 30 Tagen
chart-peak = Höchstwert { $n }
chart-hint = 30 Tage, UTC. Mit den Pfeiltasten gehst du durch die Tage.

stats-uptime = Läuft seit
stats-central = Baylee Central
stats-authMode = Wer hereinkommt
stats-terms = Nutzungsbedingungen
stats-on = an
stats-off = aus
stats-decks = Decks der Spieler
stats-records = Aufzeichnungen
stats-recordBytes = Gespeicherte Aufzeichnungen
stats-avgGame = Durchschnittliche Partie (30 Tage)
stats-data = Daten
stats-guestsNew = Neue Gäste
stats-withEmail = mit Adresse
stats-byKey = per Schlüssel freigeschaltet
stats-awaitingEngine = Plätze warten auf eine Engine
stats-reports = Berichte

live-players = Jetzt online
live-noPlayers = Gerade ist niemand online.
live-tables = Tische
live-noTables = Kein Tisch ist offen.
live-agents = Agenten
live-noAgents = Kein Agent verbunden: Es kann keine neue Partie starten.
live-waiting = wartet
live-playing = läuft
live-inLobby = Lobby
live-atTable = am Tisch
live-inGame = im Spiel
live-host = Gastgeber
live-noHost = kein Gastgeber
live-open = freier Platz
# { $level } is the house AI's level, as the gateway names it.
live-ai = Haus-KI · { $level }
live-ready = bereit
live-notReady = nicht bereit
# A tag on a waiting room that has a password.
live-locked = Passwort
live-rematch = Revanche
live-engine = Engine verbunden
live-noEngine = wartet auf seine Engine
live-local = dieser Rechner
live-opened = eröffnet { $when }
live-all = Alle
live-search = Spieler filtern
live-guest = Gast
live-bridge = LLM-Platz { $name }, für { $by }
live-capacity = Kapazität { $n }
live-unlimited = unbegrenzt
live-agentGames =
    { $n ->
        [one] { $n } Partie
       *[other] { $n } Partien
    }
live-clock = { $secs } s pro Entscheidung
live-noClock = ohne Uhr
live-team = Team { $n }
live-seats = { $taken } von { $total } Plätzen belegt
live-noMatch = Kein Spieler passt.

acc-search = Name, Benutzername oder #Tag
acc-searchLabel = Accounts suchen
acc-kind = Art
acc-kindAll = Alle
acc-kindRegistered = Registriert
acc-kindGuest = Gäste
acc-onlineOnly = Nur online
acc-sort = Sortierung

sort-newest = Neueste zuerst
sort-oldest = Älteste zuerst
sort-name = Name
sort-games = Meiste Partien
sort-active = Zuletzt aktiv

acc-count = { $from }–{ $to } von { $total }
acc-none = Kein Account passt.
acc-name = Account
acc-created = Erstellt
acc-active = Zuletzt aktiv
acc-games = Partien
acc-decks = Decks
acc-sessions = Sitzungen
acc-registered = registriert
acc-guest = Gast
acc-online = online
acc-inGame = im Spiel
acc-prev = Zurück
acc-next = Weiter
acc-never = —
acc-activeHint = Aus der letzten Verlängerung der Sitzung: auf 6 Stunden genau bei Accounts, auf einen Tag bei Gästen.
acc-byKey = Beta-Schlüssel
acc-clear = Zurücksetzen
acc-tagFilter = Nur der Account mit dem Tag #{ $tag }

det-back = Alle Accounts
det-username = Benutzername
det-handle = Angezeigt als
det-id = ID
det-copy = ID kopieren
det-copied = Kopiert

mod-title = Maßnahmen für diesen Account
mod-hint = Jede Maßnahme wird mit deinem Namen protokolliert. Einen Grund sieht der Spieler.
mod-cancel = Abbrechen
mod-ban = Sperren
mod-banTitle = { $who } sperren
mod-banReason = Grund (wird dem Spieler gezeigt)
mod-banForever = Dauerhaft
mod-banUntil = Bis
mod-banYes = Sperren
mod-banNeedsReason = Gib einen Grund an.
mod-banNeedsEnd = Wähle ein Ende der Sperre, das in der Zukunft liegt.
mod-banWhat = Die Sitzungen enden, und der Account verlässt sofort jeden Tisch. Ein Gast behält seine Sitzung, die aber bis zum Ende der Sperre abgewiesen wird.
mod-banned = gesperrt
mod-bannedUntil = Gesperrt bis { $when } durch { $by }: { $reason }
mod-bannedForever = Dauerhaft gesperrt durch { $by }: { $reason }
mod-unban = Sperre aufheben
mod-unbanAsk = Die Sperre für { $who } aufheben? Die Anmeldung ist sofort wieder möglich.
mod-delete = Account löschen
mod-deleteTitle = { $who } löschen
mod-deleteWhat = Der Account, seine Decks, Einstellungen und Bilder werden endgültig gelöscht; seine Partien bleiben ohne Namen in den Aufzeichnungen. Das lässt sich nicht rückgängig machen.
mod-deleteType = Gib zum Bestätigen { $handle } ein
mod-deleteYes = Endgültig löschen
mod-endGame = Partie beenden
mod-endTitle = Diese Partie beenden
mod-endWhat = Alle Spieler erfahren, dass die Partie beendet wurde und warum; niemand gewinnt oder verliert.
mod-endReason = Grund (wird den Spielern gezeigt)
mod-endDefault = Von einem Admin beendet
mod-endYes = Partie beenden
mod-endAsked = Die Engine wurde benachrichtigt; der Tisch schließt gleich.

det-lang = Sprache
det-email = E-Mail
det-emailNone = keine
det-emailUnconfirmed = angegeben, nicht bestätigt
det-emailConfirmed = bestätigt am { $when }
det-terms = Nutzungsbedingungen
det-termsNone = nicht angenommen
det-termsAccepted = { $version }, { $when }
det-key = Beta-Schlüssel
det-keyNone = keiner
det-keyUsed = ja
det-now = Jetzt
det-offline = offline
det-openTable = In Live zeigen
det-decks = Decks
det-noDecks = Keine Decks.
det-games = Letzte Partien
det-noGames = Keine aufgezeichneten Partien.
det-cards = { $n } Karten
det-version = Version { $n }
det-complete = beendet
det-incomplete = kein Ende aufgezeichnet
det-seat = Platz { $n }
det-house = Haus-KI oder kein Account
det-houseShort = Haus-KI
det-notFound = Diesen Account gibt es nicht (mehr).
det-lasted = dauerte { $d }
det-updated = geändert { $when }

keys-summary = { $active } gültig · { $left } Einlösungen übrig · { $admitted } freigeschaltet

status-new = Neu
status-triaged = Gesichtet
status-in_progress = In Arbeit
status-resolved = Erledigt
status-wont_fix = Wird nicht behoben
status-duplicate = Duplikat

kind-bug = Fehler
kind-crash = Absturz
kind-improvement = Verbesserung
kind-feedback = Feedback
kind-other = Sonstiges
kind-chat = Chatnachricht

rep-title = Berichte
rep-view-inbox = Eingang
rep-view-triaged = Gesichtet
rep-view-in_progress = In Arbeit
rep-view-resolved = Erledigt
rep-view-all = Alle
rep-views = Ansichten
# The search box's placeholder; / is the key that moves to it.
rep-search = Text durchsuchen  ( / )
rep-searchLabel = Text
rep-filters = Filter
rep-filtersOn = Filter ({ $n })
rep-kind = Art
rep-status = Status
rep-gateway = Gateway
rep-reporter = Verfasser
rep-reporterHint = Pseudonym
rep-sender = Absender
rep-senderUnverified = nicht geprüft, daher nicht genannt
rep-findSender = Account des Absenders suchen
rep-from = Von
rep-to = Bis
rep-record = Aufzeichnung
rep-any = Alle
rep-withRecord = Mit Aufzeichnung
rep-withoutRecord = Ohne
rep-clear = Zurücksetzen
rep-apply = Suchen
rep-refresh = Aktualisieren
rep-refreshing = Wird aktualisiert…
rep-updated = Stand { $time }
rep-arrived =
    { $n ->
        [one] { $n } neuer Bericht seit dem letzten Blick
       *[other] { $n } neue Berichte seit dem letzten Blick
    }
rep-show = Anzeigen
rep-loading = Lädt…
rep-none = Kein Bericht passt.
rep-range = { $from }–{ $to } von { $total }
rep-col-received = Eingegangen
rep-col-kind = Art
rep-col-status = Status
rep-col-text = Text
rep-col-gateway = Gateway
rep-col-reporter = Verfasser
rep-col-record = Aufzeichnung
rep-col-issue = Issue
rep-noText = (kein Text)
rep-unread = ungelesen
rep-select = { $what } auswählen
rep-selectAll = Alle Berichte dieser Seite auswählen
rep-selected = { $n } ausgewählt
rep-setStatus = Status setzen
rep-copyIds = IDs kopieren
rep-exportCsv = CSV exportieren
rep-exported =
    { $n ->
        [one] { $n } Bericht exportiert.
       *[other] { $n } Berichte exportiert.
    }
rep-markRead = Als gelesen markieren
rep-deselect = Auswahl aufheben
rep-bulkDone =
    { $n ->
        [one] { $n } Bericht auf { $status } gesetzt.
       *[other] { $n } Berichte auf { $status } gesetzt.
    }
rep-bulkFailed = { $failed } von { $total } konnten nicht geändert werden: { $why }
rep-shortcuts = Tastenkürzel
rep-key-move = Cursor bewegen
rep-key-open = in der Seitenleiste öffnen
rep-key-page = Seite öffnen
rep-key-select = auswählen
rep-key-selectAll = alle auf dieser Seite auswählen
rep-key-status = Status setzen (neu … Duplikat)
rep-key-search = suchen
rep-key-refresh = aktualisieren
rep-key-copy = Link kopieren
rep-key-export = CSV exportieren
rep-key-close = Seitenleiste schließen, Auswahl aufheben
rep-key-help = diese Liste
rep-open = Öffnen
rep-openPage = Seite öffnen
rep-close = Schließen
rep-copyLink = Link kopieren
rep-copyId = ID kopieren
rep-copySummary = Zusammenfassung kopieren
rep-copied = Kopiert.
rep-copyFailed = Kopieren fehlgeschlagen.
rep-onlyBy = Nur Berichte von { $who }
rep-cardImage = { $name }, wie Scryfall sie zeigt
rep-scryfallCredit = Bild: Scryfall
rep-seatN = Platz { $n }
rep-dismiss = Ausblenden
rep-direct = direkt von einem Client, nicht angemeldet
rep-clientRecord = vom Client geliefert, ungeprüft
rep-received = Eingegangen
rep-changed = Geändert
rep-gatewayBuild = Gateway-Build
rep-clientBuild = Client-Build
rep-game = Partie
rep-wrote = Was der Spieler schrieb
rep-refs = Verweise
rep-card = Karte { $n }
rep-table = Am Tisch
rep-noTable = Der Bericht wurde nicht an einem Tisch geschrieben.
rep-turn = Zug
rep-turnOf = Zug { $n }, { $who }
# rep-ownTurn and rep-seatsTurn fill { $who } in rep-turnOf.
rep-ownTurn = der eigene
rep-seatsTurn = von Platz { $n }
rep-phase = Phase
rep-seat = Platz
rep-view = Ansicht
rep-viewSeq = Sequenz { $n }
rep-pending = Frage
rep-noPending = keine offen
rep-awaiting = wartet auf Platz { $n }
rep-holding = In der Hand
rep-armed = bereit: { $what }
rep-selectedN = { $n } ausgewählt
rep-outbox = { $n } nicht gesendet
rep-manaRun = tappt Länder
rep-lastError = Letzte Ablehnung
rep-players = { $n } Spieler
rep-crash = Absturz
rep-where = Wo
rep-thread = Thread
rep-platform = Plattform
rep-when = Wann
rep-backtrace = Backtrace
rep-screenshot = Screenshot
rep-screenshotAlt = Das Fenster des Spielers, als der Bericht geschrieben wurde
rep-log = Das Spielprotokoll des Spielers
rep-logEmpty = Das Protokoll ist leer.
rep-tableJson = Der Tisch, wie der Spieler ihn sah
rep-everything = Alles, was der Client gesendet hat
rep-missing = Der Spieler hat { $what } nicht mitgeschickt.
# rep-aScreenshot … rep-theSettings fill { $what } in rep-missing.
rep-aScreenshot = den Screenshot
rep-theLog = das Spielprotokoll
rep-theTable = den Tisch
rep-systemDetails = die Systemangaben
rep-theSettings = die Einstellungen
rep-recordTitle = Aufzeichnung
rep-download = Herunterladen
rep-complete = vollständig
rep-partial = unvollständig
rep-noRecord = Keine Aufzeichnung angehängt.
rep-recordHint = gzip, darin JSON Lines; abspielbar mit baylee_gamehost::record::replay auf dem Build, den ihr Header nennt.
rep-system = System
rep-settings = Einstellungen
rep-history = Verlauf
rep-noHistory = Noch niemand hat ihn geändert.
rep-triage = Sichtung
rep-issue = GitHub-Issue
rep-notLinked = Nicht verknüpft.
rep-unlink = Trennen
rep-link = Verknüpfen
rep-issueLabel = Issue-Nummer oder URL
rep-issuePlaceholder = #311 oder Issue-URL
rep-badIssue = Gib eine Issue-Nummer (311, #311) oder ihre URL in AceVik/baylee an.
rep-statusSet = Status: { $status }.
rep-linked = Mit #{ $n } verknüpft.
rep-unlinked = Verknüpfung getrennt.
rep-summaryLabel = Zusammenfassung für ein neues Issue
rep-summaryHint = Öffentlich auf GitHub: keine personenbezogenen Daten, keine Zitate aus dem Bericht. Beschreibe das Problem mit eigenen Worten.
rep-category = Kategorie
rep-openIssue = Neues Issue auf GitHub öffnen
rep-delete = Bericht löschen…
rep-deleteAsk = Diesen Bericht und seine Aufzeichnung endgültig löschen?
rep-deleteYes = Endgültig löschen
rep-deleteNo = Behalten
rep-deleted = Bericht gelöscht.
rep-back = ← Alle Berichte
rep-reportersTop = Verfasser nach Zahl der Berichte
rep-reportsN =
    { $n ->
        [one] { $n } Bericht
       *[other] { $n } Berichte
    }
rep-drawerLabel = Bericht
rep-unknownBuild = unbekannter Build

prio-critical = Kritisch
prio-high = Hoch
prio-normal = Normal
prio-low = Niedrig

rep-priority = Priorität
rep-noPriority = Keine
rep-view-open = Offen
rep-view-mine = Meine
rep-view-unassigned = Nicht zugewiesen
rep-view-done = Erledigt
rep-view-trash = Papierkorb
rep-assignee = Zuständig
rep-nobody = Niemand
rep-label = Label
rep-labels = Labels
rep-channel = Kanal
rep-channelGateway = über ein Gateway
rep-channelDirect = direkt von einem Client
rep-source = Absender
rep-noSource = keine Angabe
rep-sourceOthers = alle außer { $source }
rep-sourceTitle = Was laut Client die Meldung geschickt hat (seine Angabe, ungeprüft)
rep-onlyUnread = Nur ungelesene
rep-signature = Absturz
rep-duplicatesOf = Duplikate von { $id }
rep-inTrashFilter = Im Papierkorb
rep-removeFilter = Filter { $what } entfernen
rep-sort = Sortieren nach
rep-sortBy = Nach { $what } sortieren
rep-ascending = aufsteigend
rep-descending = absteigend

sort-received = Eingang
sort-changed = Letzte Änderung
sort-priority = Priorität
sort-status = Status
sort-kind = Art
sort-reporter = Verfasser
sort-gateway = Gateway

rep-pageSize = Pro Seite
rep-page = Seite { $n } von { $total }
rep-pages = Seiten
rep-first = Erste
rep-prev = Vorige
rep-next = Nächste
rep-last = Letzte
rep-saved = Gespeicherte Ansichten
rep-saveView = Diese Ansicht speichern…
rep-viewName = Name der Ansicht
rep-save = Speichern
rep-cancel = Abbrechen
rep-viewSaved = Ansicht „{ $name }“ gespeichert.
rep-viewRemoved = Ansicht entfernt.
rep-removeView = Ansicht { $name } entfernen
rep-noSaved = Noch keine gespeicherten Ansichten.
rep-groups = Absturzgruppen
rep-listMode = Berichte
rep-shownAs = Anzeigen als
rep-noGroups = Kein Absturz passt.
rep-showGroup = Zugehörige Berichte zeigen
rep-col-crash = Absturz
rep-col-count = Berichte
rep-col-openN = Offen
rep-col-first = Zuerst
rep-col-last = Zuletzt
rep-col-priority = Priorität
rep-col-assignee = Zuständig
rep-notesN =
    { $n ->
        [one] { $n } Notiz
       *[other] { $n } Notizen
    }
rep-dupBadge = Duplikat von { $id }
rep-setPriority = Priorität setzen
rep-assign = Zuweisen
rep-assignMe = Mir zuweisen
# An entry in the assignee lists: the admin who is signed in.
rep-me = ich ({ $name })
rep-addLabel = Label hinzufügen
rep-removeLabel = Label entfernen
rep-mergeInto = Duplikate von…
rep-mergeHint = Die anderen als Duplikate des gewählten markieren
rep-toTrash = In den Papierkorb
rep-restore = Wiederherstellen
rep-purge = Endgültig löschen…
rep-purgeAsk =
    { $n ->
        [one] { $n } Bericht samt Aufzeichnung endgültig löschen?
       *[other] { $n } Berichte samt Aufzeichnungen endgültig löschen?
    }
rep-markUnread = Als ungelesen markieren
rep-trashed =
    { $n ->
        [one] { $n } Bericht in den Papierkorb gelegt.
       *[other] { $n } Berichte in den Papierkorb gelegt.
    }
rep-restored =
    { $n ->
        [one] { $n } Bericht wiederhergestellt.
       *[other] { $n } Berichte wiederhergestellt.
    }
rep-purged =
    { $n ->
        [one] { $n } Bericht endgültig gelöscht.
       *[other] { $n } Berichte endgültig gelöscht.
    }
rep-undo = Rückgängig
rep-changedN =
    { $n ->
        [one] { $n } Bericht geändert.
       *[other] { $n } Berichte geändert.
    }
rep-bulkRefused = Nichts wurde geändert: { $why }
rep-exportAll = Alle { $n } Treffer exportieren
rep-exportSelected = Ausgewählte exportieren
rep-trashHint = Berichte im Papierkorb werden nach { $days } Tagen endgültig gelöscht.
rep-key-assign = mir zuweisen oder wieder niemandem
rep-key-read = als gelesen oder ungelesen markieren
rep-key-trash = in den Papierkorb legen
rep-key-pages = vorige oder nächste Seite
rep-key-neighbours = voriger oder nächster Bericht der Liste
rep-duplicateOf = Duplikat von
rep-notDuplicate = Kein Duplikat
rep-markDuplicate = Als Duplikat markieren
rep-duplicateLabel = ID oder Link des Originals
rep-badDuplicate = Gib die ganze ID oder den Link des Originals an.
rep-duplicateSet = Als Duplikat markiert.
rep-duplicateCleared = Kein Duplikat mehr.
rep-labelLabel = Neues Label
rep-labelHint = Kleinbuchstaben, Ziffern, Bindestriche
rep-badLabel = Ein Label besteht aus Kleinbuchstaben, Ziffern und Bindestrichen, höchstens 32 Zeichen.
rep-removeLabelN = Label { $label } entfernen
rep-labelsSet = Labels gespeichert.
rep-assigned = { $name } zugewiesen.
rep-unassigned = Niemandem zugewiesen.
rep-prioritySet = Priorität: { $priority }.
rep-conflict = Jemand hat den Bericht inzwischen geändert; er wird so gezeigt, wie er jetzt ist. Versuch es noch einmal.
rep-notes = Notizen
rep-notesHint = Intern: Nur Admins lesen sie. Keine personenbezogenen Daten, keine Zitate aus dem Bericht.
rep-addNote = Notiz hinzufügen
rep-noteLabel = Neue Notiz (Strg+Enter fügt sie hinzu)
rep-noNotes = Noch keine Notizen.
rep-removeNote = Notiz entfernen
rep-related = Zusammenhang
rep-sameGame = Berichte aus dieser Partie
rep-sameCrash = Berichte zu diesem Absturz
rep-itsDuplicates = Seine Duplikate
rep-theOriginal = Das Original
rep-nothingRelated = Nichts anderes hängt damit zusammen.
rep-inTrashSince = Im Papierkorb seit
rep-trashDays = Er wird { $days } Tage danach endgültig gelöscht.
rep-trashedOne = Bericht in den Papierkorb gelegt.
rep-restoredOne = Bericht wiederhergestellt.
rep-position = { $n } von { $total }
rep-outsideList = nicht in der Liste, aus der er geöffnet wurde
rep-enlarge = Screenshot vergrößern

# A report's history: each audit-* line follows the admin's name and a colon,
# and the ones ending without an object are followed by the value (a status,
# an issue, a name, a priority, labels, a report's id).
audit-status = setzte den Status auf
audit-issue = verknüpfte
audit-unlinked = trennte das Issue
audit-assign = übertrug ihn an
audit-unassign = wies ihn niemandem zu
audit-priority = setzte die Priorität auf
audit-unprioritised = entfernte die Priorität
audit-labels = setzte die Labels auf
audit-nolabels = entfernte alle Labels
audit-duplicate = markierte ihn als Duplikat von
audit-original = nahm die Duplikat-Markierung zurück
audit-trash = legte ihn in den Papierkorb
audit-restore = stellte ihn wieder her
audit-delete = löschte ihn endgültig

sec-sets = Sets
sec-models = Modelle
sec-access = Wer hereinkommt
entry-title = Wer hereinkommt
entry-lead = Wen dieses Gateway hereinlässt. Eine Änderung gilt für jede Anmeldung ab jetzt.
entry-legend = Dieses Gateway lässt herein
entry-central = Nur Baylee-Konten
entry-centralHint = Spieler melden sich mit ihrem Baylee-Konto an, Gäste nicht. Die Voreinstellung.
entry-mixed = Baylee-Konten und Gäste
entry-mixedHint = Spieler melden sich mit ihrem Baylee-Konto an oder spielen als Gast.
entry-guests = Nur Gäste
entry-guestsHint = Jeder spielt mit einem Anzeigenamen, kein Baylee-Konto meldet sich an.
entry-fromEnvironment = Vom Betreiber festgelegt (BAYLEE_AUTH_MODE): dort ändern und das Gateway neu starten.
entry-fromDefault = Noch hat niemand gewählt, also gilt die Voreinstellung: nur Baylee-Konten.
entry-fromConsole = Gewählt von { $who }, { $when }.
entry-applies = Wer schon angemeldet ist, bleibt, bis er geht.
entry-save = Speichern
entry-saved = Gespeichert. Die nächste Anmeldung folgt dem.
entry-nobody = Niemand kommt herein: Dieses Gateway lässt nur Baylee-Konten herein und ist nicht mit Baylee Central verbunden.
entry-pasteToken = Central-Token einfügen
entry-allowGuests = Gäste zulassen
sec-central = Baylee Central
central-title = Verbindung zu Baylee Central
central-lead = Mit einem Token von baylee.app meldet dieses Gateway seine Spieler über Baylee Central an und pflegt seinen Eintrag in der öffentlichen Gateway-Liste.
central-fromFile = Die Betreiberdatei (BAYLEE_CENTRAL_CONFIG_FILE) verbindet dieses Gateway; ändere sie dort.
central-none = Nicht verbunden. Bis ein Token dieses Gateway verbindet, meldet sich hier kein Baylee-Konto an; Gäste kommen herein, wo es sie hereinlässt.
central-rejected = Baylee Central nimmt dieses Token nicht mehr an: Es wurde widerrufen oder ist abgelaufen. Erzeuge auf baylee.app ein neues.
central-unavailable = Baylee Central hat gerade nicht geantwortet.
central-token = Token
central-setBy = Eingefügt von
central-gateway = Gateway
central-state = Freigabe
central-pending = Wartet auf Freigabe
central-approved = Freigegeben
central-disabled = Gesperrt
central-allows = Erlaubt
central-sso = Anmeldung
central-listing = Listeneintrag
central-expires = Gültig bis
central-pendingHint = Bis ein Admin von Baylee Central das Gateway freigibt, steht es nicht in der öffentlichen Liste. Sein Token meldet Spieler aber schon an.
central-paste = Token einfügen
central-replace = Token ersetzen
central-step1 = Melde dich auf baylee.app an und öffne Deine Gateways.
central-step2 = Registriere dort die Adresse dieses Gateways und erzeuge ein Token.
central-step3 = Füge das Token hier ein. Das Gateway prüft es bei Baylee Central, bevor es es behält.
central-link = Verbinden
central-notToken = Ein Token beginnt mit bgt_ und hat danach 64 Hex-Ziffern.
central-linked = Verbunden. Das Gateway benutzt das Token jetzt.
central-unlink = Token entfernen
central-unlinkConfirm = Über Baylee Central angemeldete Spieler werden bei der nächsten Prüfung abgemeldet.
central-keep = Behalten
central-unlinked = Das Token ist entfernt. Widerrufe es auch auf baylee.app, wenn es niemand mehr braucht.
sec-cards = Gesperrte Karten

cards-title = Gesperrte Karten
cards-summary = { $n } derzeit gesperrt. Ein Deck mit einer gesperrten Karte lässt sich auf diesem Gateway nicht wählen, nicht bereit melden und startet keine Partie; laufende Partien gehen weiter, lokale Partien sind nicht betroffen.
cards-search = Karte suchen
cards-searchLabel = Kartenname oder Index
cards-searchPlaceholder = z. B. Lightning Bolt
cards-searching = Sucht…
cards-noHits = Keine Karte im Pool dieses Gateways passt.
cards-hint = Gesucht wird nur im Pool des Gateways: Eine Karte, die es nicht spielen kann, steckt hier in keinem Deck.
cards-block = Sperren…
cards-blockedNow = gesperrt
cards-reason = Grund (Spieler sehen ihn)
cards-reasonPlaceholder = z. B. lässt die Engine abstürzen, Fix in beta.9
cards-duration = Wie lange
cards-forever = Unbefristet
cards-day1 = 1 Tag
cards-days7 = 7 Tage
cards-days30 = 30 Tage
cards-untilDate = Bis zu einem Zeitpunkt…
cards-until = Bis (deine Ortszeit)
cards-blockConfirm = { $name } { $when } sperren? Decks mit dieser Karte können auf diesem Gateway keine Partie mehr starten.
# cards-whenForever and cards-whenUntil fill { $when } in cards-blockConfirm.
cards-whenForever = unbefristet
cards-whenUntil = bis { $at }
cards-blockYes = Sperren
cards-blockNo = Abbrechen
cards-blocking = Sperrt…
cards-blockedDone = { $name } ist gesperrt.
cards-untilPast = Wähle einen Zeitpunkt in der Zukunft.
cards-active = In Kraft
cards-noneActive = Keine Karte ist gesperrt.
cards-history = Frühere Sperren
cards-noHistory = Noch keine.
cards-by = Von
cards-made = Gesperrt
cards-ends = Endet
cards-never = nie
cards-lifted = Aufgehoben
cards-unblock = Entsperren…
cards-unblockConfirm = Die Sperre für { $name } aufheben? Decks mit dieser Karte sind sofort wieder spielbar.
cards-unblockYes = Sperre aufheben
cards-unblockNo = Behalten
cards-unblocked = Sperre aufgehoben.

cardstate-active = in Kraft
cardstate-expired = abgelaufen
cardstate-lifted = aufgehoben

llm-title = Sprachmodelle
llm-summary =
    { $hosts ->
        [one] { $hosts } Seat-Agent
       *[other] { $hosts } Seat-Agenten
    } · { $profiles ->
        [one] { $profiles } Profil
       *[other] { $profiles } Profile
    } · { $games ->
        [one] { $games } Partie läuft
       *[other] { $games } Partien laufen
    }
llm-hosts = Seat-Agenten
llm-noHosts = Kein Seat-Agent verbunden: Es wird kein Gateway-Modell angeboten.
llm-connected = verbunden
llm-local = lokal
llm-load = Last
llm-since = Verbunden seit
llm-profiles = Profile
llm-noProfiles = Noch hat kein Seat-Agent ein Profil.
llm-profile = Profil
llm-games = Partien
llm-today = Heute
llm-month = Dieser Monat
llm-key = Schlüssel
llm-key-kept = hinterlegt
llm-key-absent = fehlt
llm-key-none_needed = nicht nötig
llm-key-unavailable = kein Schlüsselspeicher
llm-kind-api = API
llm-kind-cli = CLI
llm-state-available = verfügbar
llm-state-busy = ausgelastet
llm-state-probing = wird geprüft
llm-state-exhausted = ausgeschöpft
llm-state-needs_login = Anmeldung nötig
llm-state-failing = gestört
llm-state-disabled = abgeschaltet
llm-until = bis { $when }
llm-lastOk = Zuletzt erfolgreich geprüft { $ago }
llm-on = An
llm-off = Aus
llm-enabled = Eingeschaltet
llm-canary = Canary
llm-probe = Prüfen
llm-edit = Bearbeiten
llm-delete = Löschen
llm-setKey = Schlüssel…
llm-keyFor = Neuer Schlüssel für { $id } auf { $host }
llm-keySend = Schlüssel senden
llm-keyForget = Schlüssel entfernen
llm-keyHint = Nur zum Schreiben: einmal gesendet, hier geleert, nie wieder angezeigt.
llm-add = Profil hinzufügen
llm-editTitle = { $id } auf { $host } bearbeiten
llm-host = Seat-Agent
llm-id = ID
llm-label = Name
llm-vendor = Anbieter (erhält die Spieldaten)
llm-maxGames = Partien gleichzeitig
llm-capDay = Obergrenze pro Tag (USD)
llm-capMonth = Obergrenze pro Monat (USD)
llm-noBound = keine Grenze
llm-profileHint = Anbieter, Modell, Adresse und der Rest des Profils im Format der Einstellungsdatei (docs/llm-seat.md). Das Budget steht in den Feldern oben, nicht hier. Nie ein Schlüssel: Schlüssel gehen über „Schlüssel…“.
llm-section-profile = Profil
llm-section-load = Partien und Ausweichprofil
llm-section-game = Budget pro Partie
llm-section-caps = Ausgabengrenzen
llm-section-model = Modell
llm-hint-enabled = Aus: Dieses Modell startet keine Partie; mit Ausweichprofil starten neue Partien dort, sonst wird es niemandem angeboten. Laufende Partien gehen weiter.
llm-hint-canary = Eine Prüfung stellt dem Modell auch eine bezahlte Frage: Nur so zeigt sich ein aufgebrauchtes Guthaben.
llm-hint-maxGames = Ab so vielen startet eine neue Partie auf dem Ausweichprofil, wenn es verfügbar ist; ohne Ausweichprofil ist das Profil ausgelastet.
llm-fallback = Ausweichprofil
llm-fallbackNone = keines
llm-fallbackMissing = { $id } (nicht auf diesem Seat-Agenten)
llm-fallbackUnfit = { $id } (anderer Anbieter oder keine API)
llm-badNumber = Jede Zahl ist 0 oder größer; ein leeres Feld bedeutet keine Grenze.
llm-hint-fallback = Übernimmt eine Partie, wenn dieses Modell gedrosselt oder ausgelastet ist, und neue Partien starten darauf, solange dieses Profil sie nicht annehmen kann (voll, gedrosselt, gestört, aus). Nur API-Profile dieses Seat-Agenten mit demselben Anbieter. Wer es bestellen darf, wird für dieses Profil allein festgelegt.
llm-hint-fallbackCli = Ein Agenten-CLI hat kein Ausweichprofil.
llm-gameUsd = Pro Partie (USD)
llm-hint-gameUsd = Was eine Partie höchstens ausgeben darf; braucht den Preis des Modells. Standard 5 $.
llm-gameTokens = Tokens pro Partie
llm-hint-gameTokens = Input und Output zusammen; die Grenze für ein Modell ohne Preis. Standard 5.000.000 (20.000.000 für ein CLI).
llm-gameCalls = Aufrufe pro Partie
llm-hint-gameCalls = Darüber hinaus spielt die Haus-KI die Partie zu Ende. Standard: keine Grenze für eine API, 500 für ein CLI.
llm-price = Preis (USD / Mio. Tokens, Input · Output)
llm-priceIn = Preis Input (USD / Mio. Tokens)
llm-priceOut = Preis Output (USD / Mio. Tokens)
llm-hint-price = Beide oder keiner: für ein Modell, dessen Preis dieser Build nicht kennt, oder für einen besseren Preis.
llm-capDayTokens = Tokens pro Tag
llm-capMonthTokens = Tokens pro Monat
llm-hint-caps = Alle Partien dieses Profils zusammen, gezählt in seinem eigenen Ausgabenbuch (UTC-Tag und -Monat). Ist eine Obergrenze erreicht, ist das Profil bis zum nächsten Tag oder Monat ausgeschöpft; mit Ausweichprofil starten neue Partien dort.
llm-badPrice = Ein Preis braucht beides: Input und Output.
llm-ask-save = { $what } speichern? Laufende Partien behalten, womit sie begonnen haben.
llm-refused = Nicht gespeichert: { $why }
llm-fallbackIs = Ausweichprofil: { $id }
llm-diverts = neue Partien auf { $id }
llm-save = Speichern
llm-saving = Speichert…
llm-cancel = Abbrechen
llm-badId = Seat-Agent und ID: 1–64 Zeichen aus A–Z, a–z, 0–9, '.', '_' oder '-'.
llm-badNames = Name und Anbieter: je 1–40 Zeichen.
llm-badProfile = Das Profil ist kein JSON-Objekt.
llm-assign = Zuordnung
llm-assignHint = Welche Profile auf welchem Seat-Agenten laufen. Ein Haken kopiert die Definition des Profils dorthin; ein Schlüssel wird nicht mitkopiert.
llm-unassign = Entfernen
llm-ask-delete = { $what } löschen? Ein hinterlegter Schlüssel bleibt im Speicher des Seat-Agenten.
llm-ask-unassign = { $what } von diesem Seat-Agenten entfernen?
llm-ask-forget = Den Schlüssel von { $what } entfernen?
llm-done-write = Profil gespeichert.
llm-done-delete = Profil entfernt.
llm-done-enable = Profil eingeschaltet.
llm-done-disable = Profil abgeschaltet.
llm-done-probe = Prüfung gestartet.
llm-done-key = Schlüssel gesendet.
llm-done-keyForgot = Schlüssel entfernt.
llm-done-assign = Profil auf den Seat-Agenten kopiert.

access-title = Wer welches Modell einsetzen darf
access-hint = Pro Profil: alle registrierten Accounts, nur die Accounts auf seiner Liste oder niemand. Gäste nie. Ein neues Profil ist für niemanden offen, bis du es öffnest. Eine Änderung beendet keine Partie und keinen schon bestellten Platz.
access-noProfiles = Noch meldet kein Seat-Agent ein Profil, und keines hat eine Regel.
access-unset = nicht gesetzt: niemand
access-listedN = { $n } auf der Liste
access-isDefault = Standard für registrierte Spieler
access-notReported = kein Seat-Agent meldet es gerade
access-reported = von einem Seat-Agenten gemeldet
access-manage = Zugang verwalten
access-editTitle = Wer { $name } einsetzen darf
access-policy = Regel
access-policy-registered = Alle registrierten Spieler
access-policy-listed = Nur die Spieler auf der Liste
access-policy-nobody = Niemand
access-policyHint-registered = Jeder angemeldete Account, der kein Gast ist, sieht es und darf es einsetzen.
access-policyHint-listed = Nur die Accounts unten sehen es; allen anderen wird nichts gezeigt.
access-policyHint-nobody = Vor allen Spielern verborgen; niemand darf es einsetzen.
access-default = Das Standardmodell für registrierte Spieler
access-defaultHint = Höchstens ein Profil ist der Standard; markierst du dieses, verliert jedes andere die Markierung. Der Standard ist immer für alle registrierten Spieler offen.
access-list = Zugelassene Spieler
access-listIdle = Die Liste bleibt erhalten und zählt nur, solange die Regel „Nur die Spieler auf der Liste“ lautet.
access-find = Spieler hinzufügen
access-findPlaceholder = Benutzername oder Anzeigename…
access-typeMore = Gib mindestens { $n } Zeichen eines Benutzer- oder Anzeigenamens ein.
access-searching = Sucht…
access-noMatch = Kein registrierter Spieler beginnt so.
access-matches = { $n } gefunden: ↑ ↓ zum Wählen, Enter zum Hinzufügen.
access-already = schon auf der Liste
access-toAdd = Hinzuzufügen ({ $n }):
access-dropAdd = { $who } nicht hinzufügen
access-toRemove = Von der Liste zu nehmen ({ $n }):
access-keep = { $who } behalten
access-empty = Noch steht niemand auf der Liste.
access-filter = Liste filtern
access-sort = Reihenfolge
access-sortName = nach Name
access-sortNewest = neueste zuerst
access-sortOldest = älteste zuerst
access-selectShown = Alle angezeigten auswählen
access-select = { $who } auswählen
access-removeSelected = Ausgewählte entfernen ({ $n })
access-noneShown = Kein Spieler auf der Liste passt zum Filter.
access-addedBy = { $ago } von { $by } hinzugefügt
access-remove = Entfernen
access-undo = Behalten
access-saveEllipsis = Speichern…
access-review = Diese Änderungen übernehmen?
access-reviewPolicy = Regel: { $policy }.
access-reviewDefault = Es wird zum Standard für registrierte Spieler.
access-reviewNotDefault = Es ist nicht mehr der Standard.
access-reviewAdd = { $n } Spieler hinzufügen:
access-reviewRemove = { $n } Spieler von der Liste nehmen:
access-reviewNote = Laufende Partien und schon bestellte Plätze sind nicht betroffen.
access-confirm = Übernehmen
access-back = Zurück
access-saved = Zugang zu { $name } gespeichert.

srv-title = Server
srv-coresN = { $n } Kerne
srv-lastHour = letzte Stunde
srv-lastMinutes = letzte { $n } min
srv-noSample = Noch keine Messung: Das Gateway ist gerade erst gestartet.
srv-cpu = CPU
srv-load = Last { $l }
srv-memory = Arbeitsspeicher
srv-disk = Festplatte
srv-ofTotal = von { $total }
srv-network = Netzwerk
srv-requests = Anfragen
srv-perSecond = { $n }/s
srv-sockets = { $n } offene Sockets
srv-socketsLabel = offene Sockets
srv-uptime = Gateway läuft seit
srv-gatewayProcess = dieser Gateway-Prozess
srv-hostUp = Rechner läuft seit { $d }
srv-engines = Engine-Prozesse auf diesem Rechner
srv-noGames = Auf diesem Rechner läuft gerade keine Partie.
srv-noProc = Nur unter Linux: Hier kann das Gateway die Prozesse seiner Engines nicht lesen.
srv-cores = { $n } Kerne
srv-sparkOf = { $what }, letzte Stunde
srv-gameSpark = CPU von Partie { $id }, letzte Stunde
srv-services = Dienste
srv-feedback = Feedback-Dienst
srv-gateway = Gateway
srv-reachable = antwortete in { $ms } ms
srv-unreachable = hat nicht geantwortet
srv-checking = prüft…

sets-title = Fortschritt im Pool
sets-all = Alle Sets
sets-wholePool = Der ganze Pool
sets-ofCorpus = von { $total } Karten spielbar ({ $pct } %)
sets-playable = Spielbar
sets-playableParts = { $implemented } umgesetzt + { $generated } generiert
sets-latestClient = Neuester Client
sets-implemented = Umgesetzt
sets-generated = Generiert (ungetestet)
sets-partial = Teilweise
sets-unimplemented = Stub
sets-absent = Nicht begonnen
sets-howCounted = Ein Set zählt die Karten, die zuerst in ihm erschienen sind, aus dem Ledger und dem kompilierten Pool dieses Builds; Nachdrucke zählen bei ihrem ersten Set.
sets-search = Set-Code
sets-sort = Reihenfolge
sets-sortRelease = Erscheinungsreihenfolge
sets-sortSize = meiste Karten
sets-sortDone = am vollständigsten
sets-sortOpen = am meisten offen
sets-onlyOpen = Nur unfertige
sets-count = { $n } von { $total } Sets
sets-set = Set
sets-cards = Karten
sets-progress = Fortschritt
sets-filter = Anzeigen
sets-allCards = Alle
sets-searchCards = Kartenname
sets-cardCount = { $n } von { $total } Karten
sets-noCards = Keine Karte passt.
sets-position = Set { $n } in Erscheinungsreihenfolge
sets-onScryfall = Auf Scryfall

chart-reports = Berichte pro Tag
chart-bugs = Fehler und Abstürze
chart-otherReports = Andere Berichte
