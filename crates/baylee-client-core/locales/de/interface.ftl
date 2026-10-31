### One thing the interface says.

NounArmiesToAmass =
    { $count ->
        [one] Armee für den Aufmarsch
       *[other] Armeen für den Aufmarsch
    }
EnterTappedForCounters = Getappt ins Spiel kommen und { $count } { $counter } erhalten?
ChooseEntryOrder = Die Ersatzeffekte welches Permanents möchtest du zuerst anwenden?
ChooseEntryReplacement = Welchen Ersatzeffekt möchtest du als Nächstes anwenden?
ChooseDredge = Eine Karte ausgraben, statt zu ziehen? Wähle eine aus deinem Friedhof oder bestätige ohne Auswahl, um normal zu ziehen.
ChooseDrawReplacement = Welchen Ersatzeffekt möchtest du als Nächstes auf dieses Ziehen anwenden?
SacrificeToEnter =
    { $count ->
        [one] { $count } passendes Permanent opfern, damit diese Karte ins Spiel kommt?
       *[other] { $count } passende Permanents opfern, damit diese Karte ins Spiel kommt?
    }
ChooseSacrificeCount = Wie viele Permanents möchtest du opfern? ({ $min }–{ $max })
ChooseEnergyPayment =
    Energie bezahlen ({ $min }–{ $max }); bis zu { $count } { $count ->
        [one] getapptes Permanent kann
       *[other] getappte Permanents können
    } Marken erhalten
NounTappedForCounters =
    { $count ->
        [one] getapptes Permanent für eine Marke
       *[other] getappte Permanents für je eine Marke
    }
ChooseCounterKind = Wähle eine Markensorte
LogCountersStun =
    { $count ->
        [one] Betäubungsmarke
       *[other] Betäubungsmarken
    }
LogCountersFinality =
    { $count ->
        [one] Endgültigkeitsmarke
       *[other] Endgültigkeitsmarken
    }
PreviewTurn = Umdrehen
PreviewAlternate = Andere Ansicht
PreviewTurnCompact = Wenden
PreviewRotate = Andere Hälfte
PreviewRotateCompact = Hälfte
PreviewAlternateCompact = Ansicht
SpiresCircles = Zwei Farben für dieses Exemplar einkreisen
SpiresNeedsCircles = Cryptic Spires benötigt zwei eingekreiste Farben. Wähle sie in der Druckauswahl.
SpiresOutsideIdentity = Die eingekreisten Farben von Cryptic Spires liegen außerhalb der Farbidentität des Commanders.
ApplyPrinting = Auf diese Zeile anwenden
AddMainShort = + Hauptdeck
AddSideShort = + Sideboard
HistoryAccountHint = Historie gibt es nach der Anmeldung für Account-Decks.
HistorySaveHint = Speichere das Deck, um seine Historie zu beginnen.
ImportDeck = Importieren
ExportDeck = Exportieren
ImportTitle = Deck importieren
ExportTitle = Dieses Deck exportieren
PasteFromClipboard = Einfügen
ImportClear = Leeren
ImportTake = Deck übernehmen
TransferClose = Schließen
CopyToClipboard = Kopieren
SaveToFile = Als Datei speichern
ImportBoxEmpty = Noch nichts eingefügt
FormatBaylee = Baylee-Text
ImportedDeckName = Importiertes Deck
ImportHowTo = Kopiere eine Deck-Liste, einen Baylee-, JSON- oder YAML-Export oder einen Moxfield-Export (More → Export → Copy for Moxfield) und füge den Text hier mit Einfügen oder Strg/Cmd+V ein. Das Format wird automatisch erkannt.
ImportReadAs = Erkannt als { $format }. Zeilen: { $count } · Karten: { $copies }
ImportReplaces = Der Import ersetzt das Deck im Editor durch ein neues, ungespeichertes Deck. Das gespeicherte Deck bleibt, wie es ist.
ImportMoxfieldLink = Das ist ein Link zu einem Deck auf { $site }, das Baylee nicht direkt lesen kann. Öffne das Deck auf { $site } → More → Export → Copy for Moxfield und füge es dann hier ein.
ImportNotFetched = Decks von { $site } kann diese Version noch nicht abrufen. Exportiere das Deck dort als Text und füge ihn hier ein.
ImportUnknownLink = Baylee kann keine Decks von { $host } lesen. Exportiere das Deck dort als Textliste und füge die Liste hier ein.
ImportTook = Aus { $format } importiert. Zeilen: { $count } · Karten: { $copies }
ImportWaitingForPool = Warte auf den Kartenpool, um die Namen zu prüfen…
ImportUnknownCards = Nicht im Kartenpool dieses Gateways: { $count }. Vor dem Speichern entfernen:
ImportSkipped = Zeilen, die keine Deckzeilen sind: { $count }
ImportSkippedLine = {"  "}Zeile { $line }: { $text }
ImportMaybeNotKept = Maybeboard-Zeilen nicht übernommen, da Baylee kein Maybeboard speichert: { $count }
ImportNotALeader = { $card } kann kein Deck anführen und ist deshalb nicht als Commander markiert.
ImportNamedForYou = Die Datei nennt keinen Decknamen, daher heißt es „{ $deck }“.
ImportDefaultPrinting = Zeilen ohne Druck nutzen den Standarddruck: { $count }
ImportDefaultLanguage = Zeilen ohne Sprache sind englisch: { $count }
ImportNoCommander = Die Datei nennt keinen Commander. Für ein Commander-Deck lege einen im Kartenmenü der Deck-Liste fest.
AndMore = {"  "}und { $count } weitere
ImportTooLarge = Der Text ist zu groß für ein Deck: höchstens 256 KiB.
ImportTooManyRows = Eine Deckdatei hat höchstens 1000 Zeilen; diese hat { $count }.
ImportNothing = In dem Text steht kein Deck.
ImportUnreadable = Keine Deckzeilen gefunden. Zeile { $line } lautet: { $text }
ImportSyntax = Die Datei ließ sich nicht lesen: { $error }
ImportVersion = Das ist ein Deckdokument der Version { $version }; dieser Build liest Version 1. Aktualisiere Baylee, um es zu importieren.
ImportDeckName = Der Deckname ist zu lang oder enthält einen Zeilenumbruch.
ImportCardRefused = Karte { $position } lässt sich nicht importieren: { $reason }
ImportCardName = sie hat keinen Namen.
ImportCardCount = ihre Anzahl muss zwischen 1 und 1000000 liegen.
ImportCardNote = ihre Notiz ist länger als 500 Zeichen.
ImportCardUnstorable = ihr Name oder Druck lässt sich so nicht speichern.
ExportComplete = Dieses Format behält alles am Deck.
ExportLeavesOut = { $format } kann nicht alles ausdrücken; weggelassen:
LossName = der Deckname
LossLang = Sprachen, Zeilen: { $count }
LossScryfallId = genaue Druck-IDs, Zeilen: { $count }
LossNote = Notizen, Zeilen: { $count }
LossFinish = Holo-, Glitzer- und Galaxy-Veredelungen, als Non-Foil geschrieben, Zeilen: { $count }
LossCollectorNumber = Sammlernummern ohne Set, Zeilen: { $count }
LossMaybeboard = das Maybeboard, Zeilen: { $count }
ExportCopied = In die Zwischenablage kopiert.
ExportCopyFailed = Die Zwischenablage ließ sich nicht beschreiben. Speichere den Export stattdessen als Datei.
ExportSavedTo = Gespeichert als { $path }
ExportDownloaded = { $file } wird heruntergeladen
ExportSaveFailed = Die Datei ließ sich nicht speichern: { $error }
ImportKeys = Strg/Cmd+V fügt ein · Enter importiert · Esc schließt
ExportKeys = ← → Format · Strg/Cmd+C kopiert · Strg/Cmd+S speichert · Esc schließt
DeleteDeckQuestion = „{ $name }“ löschen?
ClearDeckQuestion = „{ $name }“ leeren?
DestructiveHint = Bitte bestätigen. Leeren entfernt Hauptdeck und Sideboard; Löschen entfernt das gespeicherte Deck.
CommanderSection = Commander
ChooseCommander = Auswählen / ersetzen
ChoosePartner = + Partner
DoneChoosing = Zurück zu allen Karten
CommanderHint = Hier wählst du deinen Commander. Entfernen hebt die Rolle auf; die Karte bleibt im Deck.
PartnerHint = Hier erscheinen nur passende Partner. Manche Commander können keinen Partner haben.
DeckStatistics = Statistiken
HubPlay = Spielen
HubDecks = Meine Sammlung
SelectedDeck = Dein nächstes Spiel
ChooseDeck = Deck auswählen
CreateTable = Tisch erstellen
PlayerCount = { $count } Spieler
NoOwnDecks = Starte mit einem Hausdeck oder baue dein eigenes.
FirstDeck = Dein erstes Deck
EmptyTablesTitle = Platz für dein nächstes Spiel
ChooseDeckToBegin = Erstelle einen Tisch und wähle dein Deck im Raum. Freunde können über die Tischsuche beitreten.
NoMatches = Keine passenden Tische
ClearTableSearch = Alle Tische anzeigen
CollectionHint = Deine Decks, deine Ideen. Jede gespeicherte Änderung bleibt in deiner Historie.
ChooseGateway = Wähle dein Gateway
GatewayHint = Wo dein Account, deine Decks und Tische zu Hause sind.
ChooseGatewayFirst = Wähle ein Gateway, um dich anzumelden oder zu registrieren.
GatewayAddress = GATEWAY-ADRESSE
SaveGateway = Gateway speichern
GatewayUrlInvalid = Gib eine http://- oder https://-Adresse ohne Zugangsdaten, Abfrageparameter oder Fragment ein.
LanguageAndSettings = Sprache und Einstellungen
AllSettings = Alle Einstellungen
ForgetGateway = Aus dieser Liste entfernen
ForgetGatewayQuestion = „{ $name }“ aus dieser Liste entfernen?
ForgetGatewayHint = Nur dieses Gerät vergisst es. Dein Account und deine Decks bleiben auf dem Gateway.
GatewayAnswering = antwortet
SourceCode = Quellcode (AGPL-3.0): { $url }
ImpressumAction = Impressum
ImpressumLink = Impressum · { $host }
TileSource = Quellcode (AGPL-3.0)
TileDiscord = Community auf Discord
ScryfallCredit = Kartendaten und -bilder bereitgestellt von Scryfall.
GatewayChecking = Prüfe { $url }…
GatewaySaved = { $url } gespeichert.
GatewayNotFound = Unter { $url } hat kein Baylee-Gateway geantwortet. Prüfe die Adresse oder versuche es erneut, sobald es läuft.
LinkGatewayQuestion = { $url } zu deinen Gateways hinzufügen?
LinkGatewayHint = Ein Link fragt danach. Gateways betreiben andere Leute: Füge eines nur hinzu, wenn du weißt, wer es betreibt. Dein Baylee-Central-Passwort geht nie an ein Gateway; fragt ein Gateway nach einem Passwort, ist es sein eigenes.
LinkGatewayJoinsAccount = Es kommt zu den Gateways deines Accounts.
LinkGatewayJoinsDevice = Es kommt zu den Gateways dieses Geräts.
LinkGatewayAdd = Gateway hinzufügen
LinkNotNow = Nicht jetzt
LinkGatewaySignInFirst = Melde dich an, dann fragt Baylee, ob { $url } zu deinem Account kommt. Oder geh zurück, um es diesem Gerät hinzuzufügen.
LinkGatewayKnown = { $url } ist schon unter deinen Gateways.
LinkGatewayListFull = Dein Account hält so viele Gateways, wie er kann. Entferne eines, bevor du ein weiteres hinzufügst.
GatewayCheckingShort = wird geprüft…
GatewayVersionUnknown = Version unbekannt
GatewayNotAnswering = antwortet nicht
GatewayProtocolMismatch = Dieses Gateway spricht Protokoll { $gateway_version } und dieser Client { $client_version }, deshalb öffnen sich seine Partien hier nicht.
GatewayViewMismatch = Die Partien dieses Gateways senden Ansichtsversion { $gateway_version } und dieser Client liest { $client_version }, deshalb öffnen sie sich hier nicht.
GatewayOlder = Dieses Gateway ist älter als die Versionsprüfung. Ob sich seine Partien hier öffnen, zeigt sich erst, wenn eine beginnt.
GatewaySilent = Dieses Gateway antwortet gerade nicht.
AccountNameHint = Andere sehen ihn als Name#Tag. 3–16 Zeichen: A–Z, 0–9, _ oder -, an beiden Enden ein Buchstabe oder eine Zahl.
AccountPasswordHint = Mindestens 8 Zeichen, kein häufiges Passwort und keiner deiner beiden Namen.
AccountPasswordInvalid = Wähle ein Passwort mit 8–256 Zeichen, das sich von Benutzer- und Anzeigenamen unterscheidet und kein häufiges Passwort ist.
LibraryReadFailed = Die Daten deiner Sammlung ließen sich nicht lesen. Bitte erneut versuchen.
WelcomeTitle = Dein nächstes Spiel beginnt hier.
WelcomeNote = Baue dein Deck. Finde deinen Tisch. Spiele deinen Stil.
OfflineBenefit = Spiele ohne Account. Offline-Decks bleiben auf diesem Gerät.
LobbyGuide = Wähle dein Deck und tritt einem Tisch bei oder eröffne einen.
DeckHistory = Historie
HistoryHint = Jeder Stand bleibt erhalten. Wiederherstellen erzeugt eine neue Version; spätere Stände bleiben verfügbar.
WorkingDiff = Änderungen gegenüber der zuletzt gespeicherten Version
GrantedActions = Gewährte Aktionen
GrantedLifeCost =
    { $amount ->
        [one] { $amount } Lebenspunkt zahlen
       *[other] { $amount } Lebenspunkte zahlen
    }
GrantedRecipient = Empfänger: { $recipient }
GrantedCancelLifeLoss =
    Diesen bevorstehenden Verlust von { $amount } { $amount ->
        [one] Lebenspunkt
       *[other] Lebenspunkten
    } vor deinem nächsten Ziehsegment aufheben
GrantedConfirm = Aktion ausführen
RestoreVersion = Diesen Stand wiederherstellen
ConfirmRestore = Wiederherstellen bestätigen
RestoreWarning = Dies ersetzt dein Arbeitsdeck einschließlich ungespeicherter Änderungen. Gespeicherte Versionen bleiben erhalten.
CurrentVersion = Aktuell
ChoosePrintShort = Bild
VersionSavedAt = Gespeichert
VersionSupersededAt = Durch nächsten Stand ersetzt
VersionLabel = Version { $version }
LibraryEmpty = Noch keine Decks veröffentlicht. Du kannst ein eigenes erstellen.
NoPastVersions = Mit dem nächsten Speichern beginnt deine Historie.
LibraryLoading = Deine Sammlung wird geladen…
LibraryRetry = Erneut versuchen
LibraryBack = Zurück
LibraryMain = Hauptdeck
LibrarySide = Sideboard
LibraryCommanders = Commander
NoChanges = Keine Änderungen in diesem Bereich.
LibraryCounts =
    { $count ->
        [one] { $count } Karte · { $sideboard } Sideboard
       *[other] { $count } Karten · { $sideboard } Sideboard
    }
LobbyCounts =
    { $count } { $count ->
        [one] Deck
       *[other] Decks
    } · { $tables } { $tables ->
        [one] passender Tisch
       *[other] passende Tische
    }
Composition = Deckübersicht
UniqueCards = Verschiedene Karten
AverageMana = Ø Manabetrag · ohne Länder
LandShare = Länderanteil
OpeningLand = Land in der Starthand
ConfirmDeleteDeck = Endgültig löschen?
AppName = Baylee
Username = BENUTZERNAME
DisplayName = ANZEIGENAME
Password = PASSWORT
PasswordAgain = PASSWORT WIEDERHOLEN
Continue = Weiter
SignIn = Anmelden
CreateAccount = Registrieren
WantAnAccount = Account erstellen
HaveAnAccount = Ich habe schon einen Account
PlayOffline = Offline spielen
PlayingOffline = Offline: deine Decks und ein Tisch voll Haus-KI
Settings = Einstellungen
MusicPlaying = Musik
MusicSilent = Musik aus
Language = Sprache
Graphics = Grafik
GraphicsPreset = Qualität
QualityLow = Niedrig
QualityMedium = Mittel
QualityHigh = Hoch
QualityUltra = Ultra
QualityCustom = Angepasst
AntiAliasing = Kantenglättung
VSync = VSync
VSyncAdaptive = adaptiv
FrameLimit = Bildratenlimit
Unlimited = unbegrenzt
BackgroundFrames = Im Hintergrund
RowRestLimit = Bildrate in Ruhe
AmbientEffects = Umgebungseffekte
Audio = Lautstärke
MasterVolume = Gesamt
EffectsVolume = Spielklänge
MuteInBackground = Im Hintergrund stumm
UpdateReady = Update { $version } bereit – wird beim Beenden installiert
UpdateAvailable = Update { $version } verfügbar – hier herunterladen
UpdatedTo = Aktualisiert auf { $version }
UpdateWhyOff = Automatische Updates sind in den Einstellungen aus.
UpdateWhyDev = Das ist ein Entwicklungs-Build: Er sucht nach Updates, installiert sie aber nie selbst.
UpdateWhyFolder = Baylee darf seinen eigenen Ordner nicht beschreiben und sich darum nicht ersetzen.
UpdateWhyMoveApp = macOS startet Baylee aus einer schreibgeschützten Kopie. Verschiebe es nach „Programme“, dann kann es sich selbst aktualisieren.
UpdateWhyReadOnly = Baylee installiert hier keine Updates: Es darf { $folder } nicht beschreiben ({ $error }). Verschiebe es in deinen Programme-Ordner, dann kann es das.
UpdateTranslocated = macOS startet Baylee aus einer schreibgeschützten Kopie, weil es noch dort liegt, wo es entpackt wurde. Updates installieren sich trotzdem; nach „Programme“ verschoben, startet es ohne diesen Umweg.
UpdateMoveHome = In meinen Programme-Ordner verschieben
UpdateMoveSystem = Für alle nach „Programme“ verschieben
UpdateMoveWhat = Kopiert Baylee dorthin, startet die Kopie und beendet die laufende. Die alte Kopie wird nur gelöscht, wenn du es sagst.
UpdateMoveFailed = Baylee ließ sich nicht verschieben: { $error }
UpdateMoved = Baylee läuft jetzt aus { $path }.
UpdateOldCopy = Die alte Kopie liegt noch unter { $path }.
UpdateTrashOld = Alte Kopie in den Papierkorb
UpdateKeepOld = Behalten
UpdateTrashFailed = Sie ließ sich nicht in den Papierkorb legen ({ $error }); zieh sie selbst hinein, wenn du magst.
UpdateWhyNotVerified = Der Download ließ sich nicht als unserer bestätigen und wurde nicht installiert.
UpdateWhyOther = Dieses Update lässt sich nicht automatisch installieren.
ReleaseNotes = Versionshinweise
UpdateRestartOffer = Starte jetzt neu und mach genau hier weiter – oder später: Es wird beim Beenden installiert.
UpdateRestartNow = Jetzt neu starten
UpdateLater = Später
UpdateRestartFailed = Neustart nicht möglich: { $error }
ResumeFailed = Konnte nicht dort weitermachen, wo du warst – normaler Start.
ResumeGameDiffers = Die Partie ließ sich nicht genau so wiederherstellen und wurde nicht fortgesetzt.
ResumeHousePlayed = Zurück am Tisch – das Haus hat während des Neustarts für dich gespielt.
ResumeSeatHeld = Zurück am Tisch – dein Platz wurde während des Neustarts freigehalten.
UpdateAutoCheck = Automatisch nach Updates suchen
UpdateAutoCheckWhy = Fragt GitHub beim Start und alle sechs Stunden. GitHub sieht dabei deine IP-Adresse und diese Version. Aus: gar keine Anfrage.
UpdateAutoInstall = Automatisch aktualisieren
UpdateAutoInstallWhy = Lädt ein signiertes Update herunter und installiert es beim Beenden.
UpdateCheckNow = Nach Updates suchen
UpdateChecking = Suche läuft…
UpdateUpToDate = Baylee ist aktuell.
UpdateCheckFailed = Die Suche nach Updates hat nicht geklappt.
UpdateCardTitle = Update { $version }
UpdateCardReady = Update { $version } ist bereit
UpdateCardAvailable = Update { $version } verfügbar
UpdateDownloading = Wird heruntergeladen…
UpdateVerifying = Wird geprüft…
UpdateUnpacking = Wird entpackt…
UpdateBytesOf = { $received } von { $total }
UpdatePercent = { $percent } %
UpdateLeftSeconds = noch etwa { $count } s
UpdateLeftMinutes = noch etwa { $count } min
UpdateLeftMoment = nur noch wenige Sekunden
UpdateLeftHours = noch über eine Stunde
UpdateDownloaded = { $size } heruntergeladen
UpdateSignatureVerified = Signatur bestätigt
UpdateDownloadingLine = Update { $version } wird heruntergeladen
UpdateVerifyingLine = Update { $version } wird geprüft…
UpdateUnpackingLine = Update { $version } wird entpackt…
UpdateFailedLine = Update { $version } nicht heruntergeladen – neuer Versuch bei der nächsten Suche
UpdateFailedWhy = Der Download ist abgebrochen ({ $error }). Baylee versucht es bei der nächsten Suche erneut.
UpdateTryAgain = Erneut versuchen
SignOut = Abmelden
DeleteAccount = Account löschen
DeleteAccountQuestion = Den Account { $account } löschen?
DeleteAccountHint = Der Account, seine Decks und seine Einstellungen verschwinden endgültig von diesem Gateway, und jedes damit angemeldete Gerät wird abgemeldet. Gib zur Bestätigung dein Passwort ein.
DeleteGuestHint = Der Gast, seine Decks und seine Einstellungen verschwinden endgültig von diesem Gateway.
DeleteAccountConfirm = Endgültig löschen
DeleteAccountNeedsPassword = bitte dein Passwort
AccountDeleted = Account gelöscht
YourDecks = Deine Decks
NewDeck = Neues Deck
AddStarterDeck = Starterdeck hinzufügen
NoDecksYet = noch keine Decks – füge das Starterdeck hinzu
Edit = Bearbeiten
Delete = Löschen
Tables = Tische
Search = SUCHE
SearchTables = Tisch oder Gastgeber…
SearchCards = Name, Typ, Text…
DoSearch = Suchen
Refresh = Neu laden
PlayTheHouse = Gegen das Haus
SeatHouse = Haus-KI
RoomPassword = RAUM-PASSWORT
OpenATableFor = Tisch eröffnen für
NoTablesOpen = Noch ist es ruhig an den Tischen. Eröffne deinen und lade jemanden zum Spielen ein.
OfflineReadyToPlay = Dein Deck ist bereit. Spiele direkt gegen die Haus-KI oder erstelle einen Tisch mit eigener Platz- und Teamaufteilung.
GatewayUnreachable = Das Gateway antwortet nicht. Neuer Versuch läuft…
GatewayBack = Das Gateway ist wieder da.
NoNewGames = Gerade können keine neuen Spiele beginnen – vielleicht wird der Server aktualisiert. Laufende Spiele gehen weiter.
NoTableMatches = kein Tisch passt zu „{ $query }“
Join = Mitspielen
Ready = Bereit
NotReady = Nicht bereit
Start = Starten
Leave = Verlassen
UseMyDeck = mein Deck
MakeHost = zum Gastgeber
SitHere = hier sitzen
SeatToAi = → KI
SeatToOpen = → frei
SeatToLlm = → Sprachmodell
RoomLlmPlays = Sprachmodell: { $model }, Aufwand { $effort }
RoomLlmEffortOwn = wie das Modell es vorgibt
RoomLlmProfile = Profil (Einstellungsdatei)
RoomLlmNoEfforts = Für dieses Modell kennt dieser Build keine Aufwandsstufe.
RoomLlmDeck = Deck, das es mitbringt
RoomLlmSave = Ins Profil übernehmen
RoomLlmRemove = Modell entfernen
RoomLlmStarting = Seine Seat-Bridge besetzt den Platz, sobald der Raum ihn als frei führt.
HostedEntry = Gateway-Modell
HostedTitle = Gateway-Modell für Platz { $seat }
HostedIntro = Ein Modell, das das Gateway betreibt, spielt auf diesem Platz. Wähle eines:
HostedAsking = Frage das Gateway…
HostedNone = Dieses Gateway bietet gerade kein Modell an.
HostedBackAt = wieder ab { $time }
HostedNotNow = gerade nicht verfügbar
HostedStarting = Startet…
HostedReady = Bereit
HostedFailed = Das Gateway-Modell ist ausgefallen: { $reason }
HostedNoReason = ohne Angabe
HostedDataGoesTo = Spieldaten gehen an { $vendor }
HostedTakeBack = Zurücknehmen
GameLlmTitle = Sprachmodell-Plätze (Debug: ab der nächsten Entscheidung)
GameLlmHouse = Haus-KI
AiNovice = Anfänger
AiCasual = Locker
AiSteady = Solide
AiSharp = Scharf
AiExpert = Experte
PageBack = ‹ Zurück
PageMore = Mehr ›
PageOf = { $first }–{ $last } von { $total }
SeatOpen = Platz { $seat } · frei
SeatAi = Platz { $seat } · KI ({ $difficulty })
SeatTaken = Platz { $seat } · { $player }
SeatYours = Platz { $seat } · { $player } (du)
Seated = { $count }/{ $total } besetzt
SeatCount = { $count } Plätze
WaitingFor = wartet auf { $player }
WaitingForYou = wartet auf dich
AllReady = alle bereit
Locked = verschlossen
StateWaiting = wartet
StatePlaying = läuft
StateOver = beendet
TableOpenWaiting = dein Tisch { $table } ist offen – warte auf Gegner
TableOpenHouseWaiting = dein Tisch ist offen – richte die Plätze ein und starte
TakingYourSeat = nehme meinen Platz ein…
BackToLobby = Zurück zur Lobby
PlayAgain = Nochmal spielen
SignedIn = angemeldet
SignedOut = abgemeldet
NeedUsernameAndPassword = bitte Benutzername und Passwort
UsernameInvisible = dieser Benutzername enthält ein unsichtbares Zeichen
UsernameCharacters = ein Benutzername besteht nur aus A–Z, Ziffern und _ - .
UsernameLength = ein Benutzername hat 3 bis 24 Zeichen
UsernameEdge = ein Benutzername beginnt und endet mit einem Buchstaben oder einer Ziffer
UsernameDoubled = in einem Benutzernamen stehen _ - . nie zweimal hintereinander
YourUsernameIs = angemeldet – dein Benutzername ist { $username }; die Adresse funktioniert noch bis Ende 2026
PlayAsGuest = Als Gast spielen
ContinueAsGuest = Weiter als { $handle }
GuestName = GASTNAME
GuestDefaultName = Guest
JoiningAsGuest = trete als Gast bei…
PlayingAsGuest = du spielst als Gast
GuestNotice = Du spielst als Gast. Dieser Account und seine Decks werden etwa 30 Tage nach deinem letzten Besuch gelöscht.
GuestEnded = dieser Gast-Account ist abgelaufen – spiel als neuer Gast oder melde dich an
NoGuests = dieses Gateway nimmt keine Gäste auf
InviteKey = BETA-SCHLÜSSEL
InviteKeyShape = BAYLEE-XXXX-XXXX-XXXX-XXXX
InviteKeyHint = Dieses Gateway ist eine geschlossene Beta: Ein neuer Account oder ein neuer Gast braucht einen Schlüssel. Füg ihn einfach so ein, wie du ihn bekommen hast.
NeedInviteKey = bitte deinen Beta-Schlüssel
InviteKeyInvalid = dieser Beta-Schlüssel ist nicht gültig – prüf ihn oder frag nach einem neuen
InviteKeyNeeded = dieses Gateway ist eine geschlossene Beta: ein neuer Account oder Gast braucht einen Beta-Schlüssel
GuestSignOutQuestion = { $name } abmelden?
GuestSignOutHint = Ein Gast kann sich nicht wieder anmelden. Abmelden löscht diesen Gast und seine Decks sofort.
NeedDisplayName = bitte einen Anzeigenamen
PasswordsDiffer = die beiden Passwörter stimmen nicht überein
BannedUntil = Dieser Account ist bis { $until } gesperrt. Grund: { $reason }
BannedForGood = Dieser Account ist gesperrt. Grund: { $reason }
ChooseNewPassword = Neues Passwort wählen
PasswordWasReset = Ein Admin hat dein Passwort zurückgesetzt. Wähle ein neues, um dich anzumelden.
NewPassword = NEUES PASSWORT
SetNewPassword = Passwort setzen und anmelden
NewPasswordIsTheOld = wähle ein anderes Passwort als das, das du bekommen hast
SettingPassword = dein Passwort wird gesetzt…
NoSignUps = dieses Gateway nimmt keine neuen Accounts an
CreatingAccount = erstelle den Account…
SigningIn = melde mich an…
AccountCreated = Account erstellt – melde mich an…
LoadingPool = lade den Kartenpool…
SavingDeck = speichere das Deck…
DeckSaved = Deck gespeichert
OpeningDeck = öffne das Deck…
DeletingDeck = lösche das Deck…
DeckDeleted = Deck gelöscht
OpeningTable = eröffne einen Tisch…
SittingDown = setze mich…
PlayingAgain = spiele nochmal…
SayingReady = bereit…
SayingNotReady = nicht bereit…
Starting = starte…
HandingOver = übergebe den Raum…
ArrangingTable = richte den Tisch her…
LeavingTable = verlasse den Tisch…
PickADeckFirst = wähle zuerst ein Deck
TableOpen = Tisch offen – warte auf Gegner
TableOpenHouse = Tisch offen – richte die Plätze ein und starte
OpponentSatDown = ein Gegner hat sich gesetzt
YouAreSeated = du sitzt – melde dich bereit, dann startet der Gastgeber
TakingTheSeat = nehme den Platz ein…
GameEnded = das Spiel ist vorbei
Unreadable = konnte { $what } vom Gateway nicht lesen
TheGameList = die Tischliste
TheDeckList = die Deck-Liste
TheSignIn = die Anmeldung
TheSeat = den Platz
ThePool = den Kartenpool
ThePrintings = die Drucke
TheDeck = das Deck
SettingsTitle = Einstellungen
Back = Zurück
SettingsOnAccount = In deinem Account gespeichert – sie begleiten dich an jeden Tisch.
SettingsOnDevice = Auf diesem Rechner gespeichert. Melde dich an, und sie folgen deinem Account.
Keys = Tasten
Automation = Automatik
WhereToStop = Wo angehalten wird
PressAKey = drücke eine Taste…
Unbound = nicht belegt
ResetAll = Alles zurücksetzen
Reset = zurücksetzen
SwitchOn = an
SwitchOff = aus
RailExplain = Ein roter Schritt ist einer, den der Client für dich abgibt. Nichts ist rot, bis du es rot machst.
UsePreset = nutzen
RailPresetEveryStep = Überall anhalten
RailPresetEveryStepDetail = Frag mich in jedem Schritt beider Züge.
RailPresetQuietSteps = Stille Schritte überspringen
RailPresetQuietStepsDetail = Nicht Enttappen, Versorgung, Ziehen, Schaden oder Aufräumen – dort wird nichts entschieden.
RailPresetCompetitive = Turnier-Stopps
RailPresetCompetitiveDetail = Deine beiden Hauptphasen, beide Kampfansagen und das gegnerische Endsegment.
YourTurns = Deine Züge
TheirTurns = Gegnerische Züge
HoldTheTableStill = Tisch ruhig halten
HoldTheTableStillWhy = Karten und Kamera springen hin, statt sich zu bewegen.
Sky = Himmel
SkyWhy = Wetter hinter dem Tisch. Am Spiel ändert es nichts.
SkyAuto = Automatisch
SkyDay = Tag
SkyNight = Nacht
Atmosphere = Atmosphäre
AtmosphereWhy = Polarlichter und wandernde Sterne am Nachthimmel.
AtmosphereOff = Aus
AtmosphereSoft = Zart
AtmosphereFull = Voll
Sound = Ton
SoundWhy = Kurze Töne für Lebenspunkte, deinen Zug und das Spielende.
SoundFull = An
SoundHalf = Leiser
SoundOff = Aus
ActPrimary = Das Naheliegende tun
ActConfirmTargetBatch = Ziele für diese Serie bestätigen
ActConfirm = Bestätigen / Priorität abgeben
ActCancel = Abbrechen
ActActivateCard = Karte spielen oder wählen
ActActivateGroup = Alle Karten der Gruppe wählen
ActCursorLeft = Cursor nach links
ActCursorRight = Cursor nach rechts
ActCursorUp = Cursor nach oben
ActCursorDown = Cursor nach unten
ActCombatFocusNext = Auf den nächsten Verteidiger zielen
ActCombatFocusPrev = Auf den vorigen Verteidiger zielen
ActCombatNone = Nichts deklarieren
ActNextPhase = Zur nächsten Phase springen
ActNextTurn = Zum nächsten Zug springen
ActToggleTextView = Kartentext statt Bild lesen
ActToggleBrowser = Zonenbrowser öffnen, auch wenn die oberste Karte eines Friedhofs leuchtet
ActToggleLog = Spielprotokoll öffnen
ActHoldForStack = Stapel auflösen lassen
ActHoldForTurn = Nichts mehr in diesem Zug
ActMulliganKeep = Diese Hand behalten
ActMulliganTake = Mulligan
ActAnswerYes = Ja
AnswerYesBatch = Ja für alle { $count } gleichen Fähigkeiten
NoMaxHandSize = Keine maximale Handkartenzahl
AutoCombatDamage = Rest automatisch verteilen
ActAnswerNo = Nein
ActNumberUp = Zahl hoch
ActNumberDown = Zahl runter
ActTextPageUp = Text eines Stapeleintrags hochblättern
ActTextPageDown = Text eines Stapeleintrags runterblättern
ActRailUp = Phasenleiste hoch
ActRailDown = Phasenleiste runter
RowTableLean = Tischwinkel
HelpTableLean = Wie steil die Kamera auf einen Tisch mit drei oder mehr Plätzen blickt. Steil zeichnet dein eigenes Brett etwa ein Drittel größer und die fernen Plätze etwas kleiner; flach zeichnet alle Bretter gleich groß.
LeanSteep = Steil
LeanGentle = Flach
RowVisitCamera = Platz ansehen
HelpVisitCamera = Wo die Kamera steht, wenn du einen anderen Platz ansiehst (sein Button, F). Automatisch: hinter einem Mitspieler, als säßest du neben ihm, und gegenüber einem Gegner, dessen Brett du dann wie im Duell liest. Oder immer dahinter mit deiner Reihe oder der Uhr im Bild, dahinter immer mit der Uhr, oder gegenüber.
VisitAuto = Automatisch
VisitBehind = Dahinter
VisitBehindDial = Dahinter, Uhr
VisitAcross = Gegenüber
PriorityCueOn = Prioritätston: an
PriorityCueOff = Prioritätston: aus
RowPriorityCue = Prioritätston
HelpPriorityCue = Ein weicher Schlag, wenn der Tisch auf dich zu warten beginnt – nur auf deinem eigenen Gerät, nie für die anderen. Er hat einen eigenen Schalter; der Musik-Schalter schaltet ihn nie stumm.
ActFocusNextSeat = Nächsten Platz ansehen
ActFocusPrevSeat = Vorigen Platz ansehen
ActFocusHome = Zurück zu meinem Platz
ActArrangementMenu = Anordnung wählen
ActNextArrangement = Nächste Anordnung
ActHandDrawer = Hand auf- oder zuklappen (Handy)
TypesInDeck = Am häufigsten in deinem Deck
TypesAll = Alle Typen
CastFirstQuestion = { $spell } wirken: { $question } – bezahlt wird danach
CancelCast = Wirken abbrechen
ActFoldDecision = Entscheidung ein- oder ausklappen
ArrRing = Ring
ArrRingBlurb = Alle Plätze am Ring; die Kamera besucht sie.
ArrUprightRing = Aufrechter Ring
ArrUprightRingShort = Aufrecht
ArrUprightRingBlurb = Der Ring, jedes Brett aufrecht zu dir; ein Besuch zoomt heran.
ArrTurntable = Drehteller
ArrTurntableBlurb = Du und ein Gegner wie im Duell, die anderen klein an den Seiten; die Wahl eines Platzes dreht den Tisch.
ArrArcRail = Bogenschiene
ArrArcRailBlurb = Die Gegner auf einem Bogen über dir; die Kamera fährt die Schiene entlang.
ArrPods = Raster
ArrPodsBlurb = Jeder Gegner aufrecht in einem Raster, von oben gesehen; ein Besuch zoomt heran.
ArrSpotlight = Rampenlicht
ArrSpotlightBlurb = Ein Duell gegen einen Platz, die anderen als Chips in der Leiste; mit „Tisch folgt dem Zug“ folgt es dem Spiel.
ArrTurntableRows = Drehteller+
ArrTurntableRowsBlurb = Drehteller bis vier Plätze, Rampenlicht ab fünf oder auf dem Handy.
ArrResolvedTo = Hier: { $arrangement }
ArrFocusRing = Fokusring
ArrFocusRingBlurb = Ein Duell in der Mitte, die anderen Plätze als Ausschnitte an den Rändern.
ArrNotInADuel = Im Duell gibt es nur eine Anordnung
ArrComing = Kommt ({ $package })
ArrNotHereSeats = Nicht bei so vielen Plätzen in einem Fenster dieser Größe
ArrNotHereWindow = Nicht in einem so kleinen Fenster
ArrNotOnAPhone = Nicht auf dem Handy
ArrExperimental = experimentell
ArrRememberForSeats = Für { $count } Plätze merken
ArrLettersNote = Die Buchstaben stehen für Bildzeichen, die noch gezeichnet werden.
ArrGameMenuRow = Anordnung › { $arrangement }
RowArrangement = Tischanordnung
HelpArrangement = Wie die Plätze an einem Tisch für drei oder mehr angeordnet sind. P wählt am Tisch eine Anordnung, Shift+P die nächste; dies ist der Standard.
RowFollowTurn = Tisch folgt dem Zug
HelpFollowTurn = Beginnt der Zug eines anderen Spielers, zeigt der Tisch dessen Platz – eine Kamera-Anordnung besucht ihn, die anderen holen ihn herüber – und in deinem Zug kehrt er zu deinem zurück. Er wartet, während du eine Frage beantwortest; ein Platz, den du selbst ansiehst, bleibt bis zum nächsten Zugbeginn.
RowArrangementBySeats = Je Tischgröße
HelpArrangementBySeats = Eine für eine Tischgröße gemerkte Anordnung gilt an jedem Tisch dieser Größe statt des Standards.
ArrSeats = { $count } Plätze
ArrAsDefault = Standard
ArrKeysHint = P wählt die Anordnung · Shift+P die nächste
GroupAnswering = Antworten
GroupMovingAround = Bewegen
GroupCombat = Kampf
GroupPhases = Phasen
GroupQuestions = Fragen
GroupDisplay = Darstellung
GroupChat = Chat
ActChatCommand = Chat: Befehl tippen (/)
ActChatReply = Chat: letztem Flüstern antworten
ActChatOpen = Chat: Eingabe öffnen
ActFriends = Freundesliste
AutoPassLabel = Abgeben, wenn nichts zu tun ist
AutoPassDetail = Kein Land, kein Zauberspruch, keine Fähigkeit, keine Karte zum Aussetzen: ohne Nachfrage abgeben.
AutoSkipTurnsLabel = In gegnerischen Zügen abgeben
AutoSkipTurnsDetail = Nur Priorität. Aufs Blocken verzichtet es nie für dich.
AutoSkipAttacksLabel = Leere Angriffsansage überspringen
AutoSkipAttacksDetail = Nur wenn nichts, das du kontrollierst, angreifen kann.
AutoSkipBlocksLabel = Leere Blockansage überspringen
AutoSkipBlocksDetail = Nur wenn nichts, das du kontrollierst, blocken kann.
RailUntap = Enttappen
RailUpkeep = Versorgung
RailDraw = Ziehen
RailMain1 = Haupt 1
RailCombatBegin = Kampfbeginn
RailAttackers = Angreifer
RailBlockers = Blocker
RailDamage = Schaden
RailCombatEnd = Kampfende
RailMain2 = Haupt 2
RailEndStep = Endsegment
RailCleanup = Aufräumen
MutateOnTop = Obenauf legen
MutateUnderneath = Darunter legen
ChooseMutatePosition = Position beim Mutieren wählen
ChooseDungeon = Gewölbe wählen
ChooseDungeonRoom = Nächsten Raum wählen
ColorWhite = Weiß
ColorBlue = Blau
ColorBlack = Schwarz
ColorRed = Rot
ColorGreen = Grün
ColorColourless = Farblos
KindCreature = Kreatur
KindInstant = Spontanzauber
KindSorcery = Hexerei
KindArtifact = Artefakt
KindEnchantment = Verzauberung
KindPlaneswalker = Planeswalker
KindBattle = Schlacht
KindLand = Land
KindOther = Sonstiges
GroupCreatures = Kreaturen
GroupPlaneswalkers = Planeswalker
GroupInstants = Spontanzauber
GroupSorceries = Hexereien
GroupArtifacts = Artefakte
GroupEnchantments = Verzauberungen
GroupBattles = Schlachten
GroupLands = Länder
GroupOther = Sonstiges
SortName = A–Z
SortCost = Kosten
SortType = Typ
SortBy = Sortierung: { $order }
PaneCards = Karten ({ $count })
RoomDraft = Ungespeicherte Einstellungen · übernimm deine Änderungen vor dem Start.
RoomTitle = Dein Tisch
RoomName = Raumname
RoomRules = Regeln & Startaufstellung
RoomHostHelp = Du richtest den Tisch ein. Die Spieler wählen ihr Deck selbst und melden sich bereit.
RoomOfflineHelp = Wähle Decks und KI-Stärke. Passe bei Bedarf die Startregeln an und starte das Spiel.
RoomGuestHelp = Der Gastgeber richtet den Tisch ein. Wähle unten dein Deck und melde dich dann bereit.
RoomPlayers = Spieler & Plätze
RoomLife = Start-Lebenspunkte
RoomMulligans = Freie Mulligans
RoomBoard = Permanents suchen
RoomStartingCards = Startkarten · { $count }
RoomApply = Einstellungen übernehmen
RoomLockHelp = Leer lassen, um das bisherige Passwort beizubehalten. Änderungen gelten nach „Einstellungen übernehmen“.
RoomUnlock = Passwort entfernen
RoomTemplate = Startvorlage
RoomCardAppearance = Edition & Foil
RoomCardCounters = Startmarken
RoomCounterKind = Markenart
RoomCounterHint = +1/+1, -1/-1, loyalty, lore, time, charge, poison, energy, rad, lifelink, level, flying, shadow, indestructible, hone; beliebige +X/+Y- oder -X/-Y-Paare; custom:ID
RoomAddCounter = Marke hinzufügen
RoomRemoveCard = Karte entfernen
RoomNoCardMatches = Kein passendes Permanent gefunden.
RoomFast = Start mit fünf Ländern
RoomPlanechase = Planechase · nicht verfügbar: Weltenkarten und Weltenwürfel sind noch nicht umgesetzt.
RoomSuccession = Verlässt der Gastgeber den Raum, übernimmt der am längsten anwesende Spieler.
RoomPickDeck = Deck auswählen
RoomDefault = Standard verwenden
RoomNoDeck = Noch kein Deck gewählt
PaneDeck = Deck ({ $count } / { $sideboard })
LeaveWithoutSaving = Ohne Speichern verlassen
BackToDecks = ‹ Decks
EditingADeck = Deck bearbeiten
ANewDeck = Neues Deck
SaveDeck = Deck speichern
DeckIsSaved = Gespeichert
HideFilters = Filter ausblenden
ShowFilters = Filter
ClearFilters = Zurücksetzen
PlayableOnly = Nur spielbare
PoolTally = { $count } von { $total } Karten{ $suffix }
PoolNarrow = {" "}– { $count } angezeigt, tippe weiter zum Eingrenzen
NothingMatches = nichts gefunden – versuche es mit weniger Filtern
NoPrintings = keine Drucke
LookingForPrintings = suche weitere Drucke…
PrintingAt = { $position } von { $total }
NoCatalogOnlyThis = Weitere Drucke konnten nicht geladen werden. Referenzdruck wird angezeigt.
ReferencePrinting = Referenzdruck
AllSets = Alle
FinishPlain = Normal
FinishFoil = Foil
FinishEtched = Geätzt
AddPrinting = Hinzufügen
CountInZone = { $count } { $zone }
ZoneDeck = im Deck
ZoneSideboard = im Sideboard
NoArtForPrinting = kein Bild für diesen Druck
NoRulesText = kein Regeltext – dieses Gateway hat keinen Kartenkatalog
NotAsPrinted = { $coverage } – diese Karte funktioniert nicht wie gedruckt
AddToDeck = + Deck
AddToSideboard = + Sideboard
MoveToSideboard = → Sideboard
MoveToDeck = → Deck
RemoveCard = entfernen
IsCommander = Commander ✓
SetCommander = als Commander
HeldInDeck = { $count } im Deck
HeldInSideboard = { $count } im Sideboard
HeldInBoth = { $count } im Deck, { $sideboard } im Sideboard
CoverageGenerated = ungetestet
CoveragePartial = teilweise
CoverageStub = Platzhalter
DeckNameLabel = DECKNAME
TabMain = Deck { $count }
TabSide = Sideboard { $count }
DeckMakeup =
    { $lands } { $lands ->
        [one] Land
       *[other] Länder
    } · { $creatures } { $creatures ->
        [one] Kreatur
       *[other] Kreaturen
    } · { $spells } { $spells ->
        [one] anderer Zauberspruch
       *[other] andere Zaubersprüche
    }
DeckEmptyHint = leer – tippe links auf eine Karte, um sie hinzuzufügen
EmptyTheDeck = Deck leeren
DroppedCards = entfallen: { $card }
DeckNeedsName = Das Deck braucht einen Namen.
DeckNameTooLong = Der Name ist zu lang (höchstens 64 Zeichen).
DeckIsEmpty = Das Deck ist leer.
TooManyLines = Höchstens { $count } verschiedene Karten je Liste.
TooManyCards = Höchstens { $count } Karten je Liste.
CardGoneFromPool = { $card } ist nicht mehr im Kartenpool.
CommanderNoLongerEligible = { $card } kann kein Commander mehr sein – dieses Deck wird ohne einen gespeichert.
DeckTooSmall =
    { $count ->
        [one] { $count } Karte – ein Constructed-Deck braucht mindestens { $min }.
       *[other] { $count } Karten – ein Constructed-Deck braucht mindestens { $min }.
    }
SideboardTooBig = Ein Sideboard hat üblicherweise höchstens { $count } Karten.
ThinOnLands =
    { $count ->
        [one] { $count } Land bei { $total } Karten ist knapp für diese Kurve.
       *[other] { $count } Länder bei { $total } Karten sind knapp für diese Kurve.
    }
ShakyCards =
    { $count ->
        [one] { $count } Karte ist noch nicht vollständig umgesetzt und funktioniert nicht wie gedruckt.
       *[other] { $count } Karten sind noch nicht vollständig umgesetzt und funktionieren nicht wie gedruckt.
    }
CardUnderneath = DARUNTER
CardAttached = ANGELEGT
WaitingForPlayer = Warte auf { $player }
WaitingForPlayers = Warte auf { $count } Spieler
WaitingForHeldSeat = Warte auf das Haus – { $player } ist abwesend
PayOrPass = Du schuldest Mana. Klicke Länder zum Vormerken oder wähle „Jetzt zahlen“; getappt wird erst beim Bezahlen.
PayStage = Jetzt zahlen ({ $count } tappen)
PayGo = Bezahlen
UnstagePayment = Alle zurücknehmen
DeclinePayment = Nicht zahlen
DeclinePaymentCast = Nicht zahlen – der Zauberspruch geht zurück
DeclinePact = Nicht zahlen – du verlierst das Spiel
PaymentShort = Nicht genug Mana zum Bezahlen. Merke weitere Länder vor oder wähle „Nicht zahlen“.
AiSaidHead = { $player } denkt
AiSaidThinking = Überlegung: { $thought }
AiSaidChose = Wahl: { $choice }
AiSaidCost =
    { $count ->
        [one] { $count } Token, { $seconds } s
       *[other] { $count } Tokens, { $seconds } s
    }
MaskChoose = Wähle eine Kreatur zum verdeckten Wirken (X = { $x }) oder lehne ab
MaskReceipt = Tatsächliche Manazahlung: { $mana }; feste Aktivierungskosten: { $fixed_cost }
CommandChoose = Wähle die Karte, die dieser Spieler nach Möglichkeit spielen muss
ConfirmFaceDown = Verdeckt wirken
DeclineCast = Nicht wirken
ChooseTextReplacement = Wähle das bisherige und das neue Wort
TextReplacementSummary = Wortänderung: { $from } → { $to }
TextOldWord = Bisheriges Wort
TextNewWord = Neues Wort
ChooseRequiredMana = Wähle eine Manafähigkeit eines Landes
RequiredManaProgress =
    { $count ->
        [one] Verpflichtend – { $count } Aktivierung ausgeführt
       *[other] Verpflichtend – { $count } Aktivierungen ausgeführt
    }
DecidingFor = Du entscheidest für { $player }
DamageTurnFaceUp = Vor dem Schaden aufdecken
GrantedPaymentHint = Mana erzeugen oder passen.
PrepareManaPayment = Du kannst Mana erzeugen, um bis zu { $amount } Schaden zu verhindern. Passe, um den Betrag zu wählen.
ChooseManaPayment = Wie viel Mana zahlen? ({ $min }–{ $max }; bis zu { $amount } Schaden verhindern)
ChooseDamageSource = Wähle eine Schadensquelle
LogFromStack = vom Stapel
LogIntoStack = auf den Stapel
SourceHand = Hand
SourceLibrary = Bibliothek
SourceCurrent = aktuell
SourceHistorical = früheres Objekt
SourceVersion = Objektversion { $version }
SourceReferences = { $count } auf dem Stapel
SourceFaceDown = Verdeckte Quelle
SourceUnknown = Unbekannte Quelle
DamageCombat = Kampfschaden
DamageNoncombat = Nichtkampfschaden
DamagePart = { $source } → { $recipient }: { $amount } ({ $kind })
DamageShield = Verhindere die nächsten Schadenspunkte; Schild übrig: { $remaining }
DamageReplaceAmount = Setze den Schaden jeder angegebenen Quelle an jedem Empfänger auf { $amount }; dies ist keine Schadensverhinderung
DamageReflect = Die Quelle dieses Effekts fügt { $player } zusätzlich den angegebenen Schaden zu
DamageShieldCounter = Entferne eine Schildmarke und verhindere den Schaden dieses Ereignisses
DamageEventShield = Verhindere Schaden dieses Ereignisses; übrig: { $remaining }; Rest verfällt
DamagePreventAll = Verhindere den gesamten angegebenen Schaden
DamagePreventCombat = Verhindere den gesamten angegebenen Kampfschaden
DamageProtection = Schutz verhindert den angegebenen Schaden
DamageRedirect = Leite den Schaden auf { $recipient } um
DamageRedirectNext = Leite den Schaden auf { $recipient } um; übrig: { $remaining }
DamageCounter = Entferne eine { $counter }-Marke und versuche, 1 Schaden zu verhindern; Marken übrig: { $remaining }
DamageSourceShield = Nutze den Schild gegen { $source }; { $amount } Schaden bleibt unverhindert
DamageShieldLife = ; { $player } erhält Lebenspunkte in Höhe des verhinderten Schadens
PreventionShare = { $part } – verhindere { $count } von höchstens { $max }
RedirectionShare = { $part } – leite { $count } von höchstens { $max } um
DamageRule = Regeleffekt
DamageUnpreventable = ; Schaden kann nicht verhindert werden
AllocateDamageCounters =
    Verteile { $count } { $count ->
        [one] Markenentfernung
       *[other] Markenentfernungen
    } – { $remaining } übrig. Wähle eine Quelle und gib ihren Anteil ein.
DamageCounterShare =
    { $part } – { $count } { $count ->
        [one] Markenentfernung
       *[other] Markenentfernungen
    } anwenden (höchstens { $max })
ChooseDamageEffect = Wähle, welcher Effekt den Schaden als Nächstes verändert
AllocatePrevention = Verteile { $count } Schadensverhinderung – { $remaining } übrig. Wähle eine Quelle und gib ihren Anteil ein.
AllocateRedirection = { $count } Schaden umleiten – { $remaining } übrig. Wähle eine Quelle und gib ihren Anteil ein.
OptionalPayment = Freiwillige Zahlung
SacrificeForPlayer = { $player } opfert die gewählten Permanents. { $selection }
Owed = Geschuldet
OwedLeft = offen
JustWaiting = Warte
MulliganFree = Diese Hand behalten? (der nächste Mulligan ist frei)
MulliganTaken = Diese Hand behalten? ({ $count } genommen)
PutOnBottom =
    { $count ->
        [one] Lege { $count } Karte nach unten
       *[other] Lege { $count } Karten nach unten
    }
DeclareAttackers = Angreifer deklarieren
AttackAim = Angriffsziel: { $defender } · { $count } zugewiesen
AttackRemaining = Mit allen übrigen angreifen → { $defender }
AttackWithdrawAll = Alle Angreifer zurückziehen
AttackListHide = Liste ausblenden ▾
AttackListShow = Liste zeigen ({ $count }) ▸
AttackSend = Angreifen: { $card } → { $defender }
AttackWithdraw = Zurückziehen: { $card } → { $defender }
AttackDraftHint = Angriffsziel wählen, dann Kreaturen. Zum Umverteilen hier zurückziehen. Zum Schluss bestätigen.
AttackPage = Seite { $page } von { $total }
DeclareBlockers = Blocker deklarieren
BlockDraftHint = Angreifer wählen, dann die Kreaturen, die ihn blocken. Erneut wählen nimmt einen Blocker zurück. Zum Schluss bestätigen.
BlockAim = Angreifer: { $attacker } · { $count } Blocker
BlockSend = Blocken: { $blocker } → { $attacker }
BlockWithdraw = Zurücknehmen: { $blocker } → { $attacker }
DiscardCards =
    { $count ->
        [one] Wirf { $count } Karte ab
       *[other] Wirf { $count } Karten ab
    }
LegendRule = Legendenregel: behalte eine
NounCards =
    { $count ->
        [one] Karte
       *[other] Karten
    }
NounTargets =
    { $count ->
        [one] Ziel
       *[other] Ziele
    }
NounCardsFromLibrary =
    { $count ->
        [one] Karte aus deiner Bibliothek
       *[other] Karten aus deiner Bibliothek
    }
NounCardsToTop =
    { $count ->
        [one] Karte, die oben auf deine Bibliothek kommt
       *[other] Karten, die oben auf deine Bibliothek kommen
    }
NounCardsToHand =
    { $count ->
        [one] Karte, die auf deine Hand kommt
       *[other] Karten, die auf deine Hand kommen
    }
NounCardsToBottom =
    { $count ->
        [one] Karte, die unter deine Bibliothek kommt
       *[other] Karten, die unter deine Bibliothek kommen
    }
NounCardsToPlay =
    { $count ->
        [one] Karte, die du in diesem Zug spielen darfst
       *[other] Karten, die du in diesem Zug spielen darfst
    }
NounCardsToBattlefield =
    { $count ->
        [one] Karte, die ins Spiel kommt
       *[other] Karten, die ins Spiel kommen
    }
NounCardsToGraveyard =
    { $count ->
        [one] Karte, die auf den Friedhof ihres Besitzers kommt
       *[other] Karten, die auf den Friedhof ihres Besitzers kommen
    }
InspectLibrary = Sieh dir die Bibliothekskarten an und bestätige ohne Auswahl.
BeholdInstructions = Wähle ein passendes Permanent unter deiner Kontrolle oder zeige eine passende Handkarte offen vor. Bestätige ohne Auswahl, um abzulehnen.
InspectHand = Sieh dir die Hand an und bestätige, wenn du fertig bist
NounCardsForFirstPile =
    { $count ->
        [one] Karte für den ersten Haufen (der Rest ist der zweite)
       *[other] Karten für den ersten Haufen (der Rest ist der zweite)
    }
NounCardsFromGraveyard =
    { $count ->
        [one] Karte aus deinem Friedhof, oder keine, um deine Bibliothek zu durchsuchen
       *[other] Karten aus deinem Friedhof, oder keine, um deine Bibliothek zu durchsuchen
    }
NounCardsOutside =
    { $count ->
        [one] Karte von außerhalb der Partie
       *[other] Karten von außerhalb der Partie
    }
NounPermanentsToSacrifice =
    { $count ->
        [one] Permanent, das geopfert wird
       *[other] Permanents, die geopfert werden
    }
NounLandsToRemoveCounters =
    { $count ->
        [one] Land, von dem die Marken entfernt werden
       *[other] Länder, von denen die Marken entfernt werden
    }
NounLandsToKeep =
    { $count ->
        [one] Land, das du behältst
       *[other] Länder, die du behältst
    }
NounCreaturesToKeep =
    { $count ->
        [one] Kreatur, die du behältst
       *[other] Kreaturen, die du behältst
    }
NounPermanentsToKeep =
    { $count ->
        [one] Permanent, das du behältst
       *[other] Permanents, die du behältst
    }
NounCardsToKeep =
    { $count ->
        [one] Handkarte, die du behältst
       *[other] Handkarten, die du behältst
    }
NounCardsToDiscard =
    { $count ->
        [one] Karte, die abgeworfen wird
       *[other] Karten, die abgeworfen werden
    }
NounPermanentsToTap =
    { $count ->
        [one] ungetapptes Permanent, das getappt wird
       *[other] ungetappte Permanents, die getappt werden
    }
NounPermanentsToReturn =
    { $count ->
        [one] Permanent, das auf die Hand seines Besitzers zurückgebracht wird
       *[other] Permanents, die auf die Hand ihrer Besitzer zurückgebracht werden
    }
NounCardsToExile =
    { $count ->
        [one] Karte aus deinem Friedhof, die ins Exil geschickt wird
       *[other] Karten aus deinem Friedhof, die ins Exil geschickt werden
    }
NounPermanentsToLeaveTapped =
    { $count ->
        [one] Permanent, das getappt bleibt
       *[other] Permanents, die getappt bleiben
    }
NounNewHostsFor =
    { $count ->
        [one] Objekt, das { $aura } verzaubert
       *[other] Objekte, die { $aura } verzaubert
    }
NounTheAura = die Aura
NounPermanentsToCopy =
    { $count ->
        [one] Permanent, das kopiert wird
       *[other] Permanents, die kopiert werden
    }
NounPermanentsToUntap =
    { $count ->
        [one] Permanent, das enttappt wird
       *[other] Permanents, die enttappt werden
    }
NounDiscardsToReveal =
    { $count ->
        [one] abgeworfene Karte, die du mit Nephalia Academy offen vorzeigst (andere bleiben verdeckt)
       *[other] abgeworfene Karten, die du mit Nephalia Academy offen vorzeigst (andere bleiben verdeckt)
    }
NounCardsToReveal =
    { $count ->
        [one] Karte, die offen vorgezeigt wird
       *[other] Karten, die offen vorgezeigt werden
    }
NounAttackersToBand =
    { $count ->
        [one] Angreifer, der sich dem Bündnis anschließt
       *[other] Angreifer, die sich dem Bündnis anschließen
    }
NounCreaturesForPile =
    { $count ->
        [one] Kreatur für diesen Haufen
       *[other] Kreaturen für diesen Haufen
    }
NounCreaturesForPileOf =
    { $count ->
        [one] Kreatur für Haufen { $pile } von { $total } (jeder Haufen blockt einen zufälligen Angreifer)
       *[other] Kreaturen für Haufen { $pile } von { $total } (jeder Haufen blockt einen zufälligen Angreifer)
    }
NounCreaturesForLeft =
    { $count ->
        [one] Kreatur für den „linken“ Haufen (der Rest geht nach rechts)
       *[other] Kreaturen für den „linken“ Haufen (der Rest geht nach rechts)
    }
ChooseRiverLabel = Wähle „links“ (zuerst) oder „rechts“: nur dieser Haufen und Flieger dürfen ihn blocken
ChooseRiverLabelFor = { $attacker }: wähle „links“ oder „rechts“; nur dieser Haufen und Flieger dürfen ihn blocken
RiverLeftRow = Links: { $cards }
RiverRightRow = Rechts: { $cards }
NounAttackersToBlock =
    { $count ->
        [one] Angreifer, der geblockt wird
       *[other] Angreifer, die geblockt werden
    }
NounAttackersForBlocker =
    { $count ->
        [one] Angreifer, den { $blocker } blockt
       *[other] Angreifer, die { $blocker } blockt
    }
NounAttackersToBandWith =
    { $count ->
        [one] Angreifer, der mit { $leader } ein Bündnis bildet
       *[other] Angreifer, die mit { $leader } ein Bündnis bilden
    }
TapToHelpPay = Tappe Permanents, um mitzubezahlen – jedes bezahlt ein Mana
DelveToHelpPay = Schicke Karten aus deinem Friedhof ins Exil, um mitzubezahlen – jede bezahlt ein Mana
CrewWithPower = Bemannen { $power }: Tappe beliebig viele deiner anderen ungetappten Kreaturen mit einer Gesamtstärke von { $power } oder mehr
SaddleWithPower = Aufsatteln { $power }: Tappe beliebig viele deiner anderen ungetappten Kreaturen mit einer Gesamtstärke von { $power } oder mehr
ChooseUpTo = Wähle bis zu { $count } { $noun }
ChooseNewTarget = Wähle ein neues Ziel
ChooseNewTargetOrKeep = Neues Ziel wählen oder ohne Auswahl dieses Ziel behalten
RetargetContext = Ziel { $position } von { $total }: { $target }. { $question }
TargetBeforeZoneChange = vor dem Zonenwechsel
TargetReturnedHint = Soll die zurückgekehrte Karte das Ziel sein, wähle sie unten erneut.
RetargetOriginal = Bisheriges Ziel { $position } von { $total }: { $target }
PreviousTarget = bisheriges Ziel
KeepCurrentTarget = Bisheriges Ziel behalten
ChangeTarget = Ziel ändern
RetargetKeepOrChange = Ziel { $position } von { $total }: behalten oder ein neues wählen
RetargetMustChange = Ziel { $position } von { $total }: wähle ein neues
KeepRemainingTargets = Alle übrigen behalten ({ $count })
ChooseExactlyOrNoTaps =
    { $count ->
        [one] Wähle genau { $count } Permanent zum Tappen oder keines
       *[other] Wähle genau { $count } Permanents zum Tappen oder keine
    }
ChooseExactly = Wähle { $count } { $noun }
ChooseBetween = Wähle { $min }–{ $max } { $noun }
ChooseCreatureType = Wähle einen Kreaturentyp
ChooseBasicLandType = Wähle einen Standardlandtyp
ChooseLandType = Wähle einen Landtyp
ChooseCardName = Wähle einen Kartennamen
ChooseColour = Wähle eine Farbe
ChooseColours = Wähle eine oder mehrere Farben
ChooseCounters = Wie viele { $counter }? ({ $min }–{ $max })
ChooseCountersNamed = Wie viele { $counter } auf { $target }? ({ $min }–{ $max })
ChooseNumberIn = Wähle eine Zahl ({ $min }–{ $max })
DamageShare = Schaden an Ziel { $position } von { $total }, noch { $remaining } zu verteilen ({ $min }–{ $max })
CombatDamageShare = Kampfschaden an Kreatur { $position } von { $total }, noch { $remaining } zu verteilen ({ $min }–{ $max })
CombatDamageShareNamed = Kampfschaden von { $source } an { $recipient } ({ $position } von { $total }), noch { $remaining } zu verteilen ({ $min }–{ $max })
ReplicateHowOften = Reproduktion { $cost }: wie oft zahlen? ({ $min }–{ $max })
ChoosePlayer = Wähle einen Spieler
ChooseHowToCast = Wähle, wie gewirkt wird
ChoosePileForHand = Wähle einen Haufen für deine Hand; der andere kommt auf deinen Friedhof
PileRow = Haufen { $pile }: { $cards }
EmptyPile = keine Karten
CastModeNumber = Modus { $number }
CastBackFace = Rückseite
CastLandFace = Als Land spielen
CastMiracle = Mirakulum
CastFlashback = Rückblende
CastBeamMeUp = Beam mich hoch
DifferentManaValues = Wähle Ziele mit unterschiedlichen Manabeträgen
CastDash = Sturmangriff
CastMutate = Mutation
CastEscape = Befreiung
CastPermission = Diese Erlaubnis zum Wirken nutzen
CastPermissionUnlimited = Eine unbeschränkte Erlaubnis zum Wirken nutzen
HintClickHand = Klicke eine Karte auf deiner Hand an
HintClickBoard = Klicke an, was du wählst
HintTypeToFilter = Tippe, um die Liste einzugrenzen
NoAbilities = Keine Fähigkeiten
LevelShort = St. { $level }
LevelLong = Stufe { $level }
FilterPlaceholder = zum Suchen tippen…
PutInOrder = Bringe diese in eine Reihenfolge
OrderGraveyard = Friedhof ordnen: erste Karte oben
OrderOnTop = Lege diese oben zurück: die erste ist die neue oberste Karte
OrderOnBottom = Lege diese unter die Bibliothek: die letzte ist die unterste Karte
ScryPrompt = Hellsicht: Karten oben lassen oder unter die Bibliothek legen
SurveilPrompt = Karten oben lassen oder auf deinen Friedhof legen
DiscardToLibraryPrompt = Abwerfen: auf deinen Friedhof oder stattdessen oben auf deine Bibliothek
TheGameIsOver = Das Spiel ist vorbei
PayLifeOrTapped =
    { $amount ->
        [one] { $amount } Lebenspunkt zahlen? Sonst kommt es getappt ins Spiel
       *[other] { $amount } Lebenspunkte zahlen? Sonst kommt es getappt ins Spiel
    }
UseManaAnyType = North Star nutzen, um die Manakosten dieses Zauberspruchs mit Mana beliebigen Typs zu bezahlen? Zusätzliche Kosten gelten unverändert.
PayAdditionalCost = Die zusätzlichen Kosten zahlen?
PayLife =
    { $amount ->
        [one] { $amount } Lebenspunkt zahlen?
       *[other] { $amount } Lebenspunkte zahlen?
    }
PayTax = {"{"}{ $mana }{"}"} zahlen?
PayMana = { $cost } zahlen?
PayManaAndLife =
    { $life ->
        [one] { $cost } und { $life } Lebenspunkt zahlen?
       *[other] { $cost } und { $life } Lebenspunkte zahlen?
    }
RemoveCountersInsteadOfUntapping = Statt zu enttappen { $count } { $counters } entfernen? „Nein“ wählt einen anderen Ersatzeffekt.
PayPact = Pakt: { $cost } zahlen. Jetzt Mana erzeugen? Wenn du ablehnst oder ohne genug Mana abschließt, verlierst du das Spiel.
CastForMiracle = Für die Mirakulum-Kosten wirken?
CastWithoutPaying = Wirken, ohne seine Manakosten zu bezahlen?
CastPaying = Jetzt wirken und seine Manakosten bezahlen? Du erzeugst zuerst das Mana.
DrawOfferedBy = { $player } bietet ein Unentschieden an. Annehmen?
CommanderToCommandZone = Deinen Commander in die Kommandozone legen?
CommanderInsteadOfHand = Dein Commander käme auf die Hand. Stattdessen in die Kommandozone?
CommanderInsteadOfLibrary = Dein Commander käme in die Bibliothek. Stattdessen in die Kommandozone?
UseRegenerationShield = Einen Regenerationsschild verbrauchen? „Nein“ nutzt die eigene Regeneration des Permanents und bewahrt den Schild.
UseShieldCounter = Eine Schildmarke statt Regeneration einsetzen?
UseDamageClearingReplacement = Zerstörung ersetzen und stattdessen allen markierten Schaden entfernen? „Nein“ nutzt einen anderen anwendbaren Ersatzeffekt.
ApplyAdditionalCountersNext =
    Als Nächstes { $count } { $count ->
        [one] zusätzliche Marke
       *[other] zusätzliche Marken
    } jeder Art hinzufügen? „Nein“ wendet zuerst einen anderen Ersatzeffekt an.
UseTheOptionalAbility = Diese Fähigkeit ist optional. Einsetzen?
TopOfLibraryOrBottom = Deine Karte kommt in deine Bibliothek. Oben drauf? („Nein“ legt sie nach unten.)
TakeOneOfType = Nimm bis zu eine Karte vom Typ { $card_type } auf deine Hand
CastDiscovered = Du hast diese Karte entdeckt. Wirken, ohne ihre Manakosten zu bezahlen? („Nein“ nimmt sie auf deine Hand.)
SkipTurnToUntap = Diesen Zug überspringen, um dieses Permanent zu enttappen?
YesOrNo = Ja oder nein?
OfferADraw = Unentschieden anbieten
Concede = Aufgeben
Watch = Zuschauen
StopWatching = Zuschauen beenden
SpectatorCount = { $count } Zuschauer
AllowSpectators = Zuschauer erlauben
StoppedWatching = Du schaust nicht mehr zu
YouAreWatching = Du schaust zu
ConcedeConfirm = Aufgeben? Nochmal drücken
LeaveTable = Tisch verlassen
LeftTheTable = Du hast den Tisch verlassen
LeaveTableConfirm = Verlassen? Nochmal drücken
LeaveTableCost = Das Haus spielt für dich weiter.
LeaveTableCostOffline = Die Partie endet.
GameLogTitle = Spielprotokoll
GameLogEmpty = Noch ist nichts passiert
GameLogNewBelow = Neue Zeilen unten
GameLogLatest = Zurück zum Neuesten
GameLogTurnLines = { $count ->
        [one] { $count } Eintrag
       *[other] { $count } Einträge
    }
RevealedBy = { $player } zeigt offen vor
RevealedWaiting = Noch { $count } weitere
RevealedHint = Zeige auf eine Karte, um sie zu vergrößern
RevealNext = Nächste
RevealDone = Fertig
HoldingPriority = Du wirst nicht gefragt
HoldRelease = Wieder fragen
ArmedPlay = Diese Karte spielen
ArmedActivate = Fähigkeit aktivieren
ArmedPayAndCast = { $cost } zahlen und wirken
ArmedPayAndActivate = { $cost } zahlen und aktivieren
ArmedSuspend = { $cost } zahlen und aussetzen
SuspendedCards = Ausgesetzte Karten
SuspendedCard =
    { $card } · { $owner }{"\u000A"}{ $count } { $count ->
        [one] Zeitmarke
       *[other] Zeitmarken
    } · ansehen
ArmedSuspendNow = Diese Karte aussetzen
ArmedCancel = Doch nicht
AimNext = Nächstes Ziel
Attack = Angreifen
Block = Blocken
DeclareNone = Keine
KeepHand = Behalten
TakeMulligan = Mulligan
TargetingFor = Ziele für { $source }
TargetingSecond = Zweites Ziel des Effekts
TargetingExactly =
    Ausgewählt: { $selected }/{ $min } • { $count } { $count ->
        [one] mögliches Ziel
       *[other] mögliche Ziele
    }
TargetingChoices =
    Wähle { $min }–{ $max } • { $count } { $count ->
        [one] gültiges Ziel
       *[other] gültige Ziele
    } • { $selected } ausgewählt
TargetingBatch = Für alle { $count } bestätigen
TargetingBatchHint = { $count } gleiche Auslöser warten. Einzeln bestätigen oder diese Auswahl für alle { $count } übernehmen.
TargetingSelected = Ausgewählt: { $targets }
TargetingAllSeats = Alle Spieler
TargetingShowFrom = Ziele zeigen von
TargetingTargets = Ziele
TargetingPage = Ziele { $first }–{ $last } von { $total }. Auswahl auch direkt am Tisch möglich.
CastKicked = Mit Bonuskosten
CastDisguise = Verkleidung – 2/2, Abwehr {"{"}2{"}"}
TurnFaceUp = Aufdecken
UnlockDoor = { $door } entsperren
CastPrototype = Prototyp
ConfirmOk = OK
PassPriority = Passen
SkipTheTurn = Zug überspringen
ResolveTheStack = Stapel abarbeiten
DesignationDay = Tag
DesignationNight = Nacht
SortByPlace = Der Reihe nach
SortByName = Nach Name
SortByCost = Nach Kosten
SortByType = Nach Typ
IsToken = Spielstein
YourMove = Du bist dran
YouMayRespond = Du kannst reagieren
StackRun = Stapel abarbeiten
StackRunTo = Bis zur Markierung abarbeiten
StackStopHint = Stopp vor der markierten Fähigkeit. Erneut klicken zum Aufheben.
StackSelectHint = Eintrag markieren, um davor anzuhalten.
StackAbilityPolicy = Für diese Kartenfähigkeit
StackAlwaysPass = Immer passen
StackAsk = Immer fragen
StackAlwaysYes = Immer Ja
StackAlwaysNo = Immer Nein
StackResetRules = Fähigkeitsautomatik zurücksetzen
StackNoPolicy = Dieser Eintrag hat keine wiederverwendbare Kartenfähigkeit.
StackPolicyHint = Ziele und Zahlungen entscheidest du weiterhin selbst.
StackRuleName = { $card } · Fähigkeit { $number }
FinishHolographic = Holografisch
FinishGlitter = Glitzer
FinishGalaxy = Galaxie
StackTitle = Stapel
StackSpell = Zauberspruch
StackAbilityBare = Fähigkeit
StackAbility = Fähigkeit · { $source }
StackMore = +{ $count } weitere
SeatNumbered = Platz { $seat }
YouNamed = Du ({ $player })
You = Du
AimedAt = Ziel: { $target } ({ $position } von { $total })
DeclaredCount = { $count } deklariert
IncomingAt = { $defender }: { $count } Angreifer, { $unblocked } Schaden ungeblockt
IncomingYou = Du
ASeat = ein Platz
APermanent = ein Permanent
AimingAtNothing = nichts
RailYou = DU
RailOpponent = GEGNER
LinkLost = Verbindung verloren – verbinde neu…
LinkStandIn = Verbinde noch immer neu – der Tisch wartet { $wait } auf dich, dann übernimmt das Haus deinen Platz, bis du zurück bist.
LinkPaused = Verbinde noch immer neu – das Spiel ist pausiert, bis du zurück bist.
LostWaiting = { $player } hat die Verbindung verloren – der Tisch wartet { $wait }
LostPaused = { $player } hat die Verbindung verloren – das Spiel ist pausiert, bis die Verbindung wieder steht
LostHousePlays = Die Haus-KI spielt für { $player }
LinkGaveUp = Der Tisch ist nicht erreichbar. Tritt aus der Lobby erneut bei.
LinkSignInEnded = Der Tisch nimmt diesen Platz von hier nicht mehr an: Deine Anmeldung ist abgelaufen, oder der Platz wurde auf einem anderen Gerät übernommen. Melde dich in der Lobby neu an, um zurückzukehren.
ClientOutdated = Dein Client ist veraltet: Aktualisiere ihn, um beizutreten.
TableOlder = Dieser Tisch läuft mit einer älteren Version als dein Client.
CardHasNoTarget = Für diese Karte gibt es kein gültiges Ziel mehr
CardWrongTime = Diese Karte lässt sich in diesem Zeitfenster nicht spielen
OnlyDearerModeLeft = Der normale Modus hat kein legales Ziel; nur der teurere Modus (etwa Überlast) ist möglich, und dein Mana reicht dafür nicht
CardCostsUnavailable = Die Mittel zum Bezahlen dieser Karte fehlen
DeedWithdrawn = Die Engine bietet das nicht mehr an
CastModeWithdrawn = Die Engine bietet diese Art, sie zu wirken, nicht mehr an
PlanLandGone = Ein Land, mit dem der Plan gerechnet hat, lässt sich nicht mehr tappen
PlanColourGone = Diese Quelle kann die Farbe nicht erzeugen, die der Plan wollte
PlanSpellRefused = Das Mana steht bereit, aber der Zauberspruch lässt sich nicht wirken
PlanSuspendRefused = Das Mana steht bereit, aber die Karte lässt sich nicht aussetzen
PlanCardGone = Der Ablauf hat die Karte verloren, für die er bezahlt hat
PlanQuestionChanged = Das Spiel hat etwas anderes gefragt
ManaPool = Manavorrat
PlateLife =
    { $amount ->
        [one] { $amount } Lebenspunkt
       *[other] { $amount } Lebenspunkte
    }
PlateHand = { $count } auf der Hand
PlateLibrary = { $count } in der Bibliothek
PlateGraveyard = { $count } im Friedhof
PlateExile = { $count } im Exil
PlatePoison = { $count } Gift
PlateRad = { $count } Strahlung
PayEnergy =
    { $count ->
        [one] { $count } Energiemarke zahlen?
       *[other] { $count } Energiemarken zahlen?
    }
ProliferatePlayer = { $player } eine weitere Marke jeder bereits vorhandenen Sorte geben?
NounProliferatePermanents =
    { $count ->
        [one] Permanent, das eine weitere Marke jeder Sorte erhält
       *[other] Permanents, die eine weitere Marke jeder Sorte erhalten
    }
PlateSpeed = Tempo { $speed }/4
PlateMaxSpeed = Maximaltempo (4/4)
PlateEnergy = { $count } Energie
PlateCommander = { $amount } Commander-Schaden
PlateMonarch = Monarch
PlateLost = Ausgeschieden
PlateTurn = Am Zug
PlateWaiting = Der Tisch wartet
ActiveTurn = Am Zug
ChosenType = Gewählt: { $subtype }
ChosenName = Genannt: { $card }
LockedDoors = Versperrt: { $doors }
TapForRestricted = Tappen für { $mana } (eingeschränkt)
TapForVariable = Tappen für X Mana ({ $mana })
SheetPressAgain = zum Bestätigen
SheetDigitPicks = Mit Ziffer wählen
SheetCloses = schließen
SheetMorePage = Weitere – Seite { $page } von { $total }
TapFor = Tappen für { $mana }
AnyColor = beliebige Farbe
OrLast = { $list } oder { $last }
GrantedAbility = Verliehene Fähigkeit
GrantedAbilities = Verliehene Fähigkeiten
NoRulesTextHere = Regeltext nicht verfügbar
ArrivalLogin = An der Schwelle
ArrivalTable = Ein Tisch. Eine neue Geschichte.
ArrivalHeld = Die Reise wartet
ArrivalPortal = Das Portal öffnet sich
ArrivalPlayers = Warte auf die Runde  ·  { $count } / { $total } bereit
ArrivalWorld = Die Welt erwacht
ArrivalGateways = Gateways werden geprüft
ArrivalCards = Karten und Tisch werden vorbereitet
ArrivalGraphics = Der letzte Feinschliff
ArrivalReady = Alles bereit
ArrivalTableSteps = VORBEREITEN    ·    VERSAMMELN    ·    EINTRETEN
ArrivalLoginSteps = WELT    ·    VERBINDUNG    ·    ANKUNFT
ArrivalLeave = Zur Lobby  ·  Esc
ArrivalLeaveTable = Tisch verlassen  ·  Esc
ArrivalTimeout = Das Laden konnte nicht abgeschlossen werden. Prüfe die Verbindung und versuche es erneut.
ArrivalAssetsFailed = Benötigte Dateien für die Oberfläche konnten nicht geladen werden. Bitte starte den Client neu oder installiere ihn erneut.
ArrivalGraphicsFailed = Die Grafik konnte nicht vorbereitet werden. Bitte starte den Client neu.
VeilTakingSeat = Du nimmst Platz
VeilTalking = Im Gespräch mit dem Gateway
VeilWorking = Einen Moment
SeatSideNone = kein Team
SeatSide = Team { $team }
YouWon = Du hast gewonnen
YouLost = Du hast verloren
YourTeamWon = Team { $team } gewinnt – dein Team
TheirTeamWon = Team { $team } gewinnt
TheGameIsADraw = Das Spiel endet unentschieden
GameClosedVerdict = Das Spiel wurde beendet
GameClosedReason = Ein Admin hat es beendet: { $reason }
EndedLastPlayer = Nur noch ein Spieler im Spiel
EndedLastTeam = Nur noch ein Team im Spiel
EndedByEffect = Ein Effekt hat entschieden
LostLifeYou = Deine Lebenspunkte fielen auf 0 oder weniger
LostLifeOther = Die Lebenspunkte von { $player } fielen auf 0 oder weniger
LostEmptyDrawYou = Du wolltest aus einer leeren Bibliothek ziehen
LostEmptyDrawOther = { $player } wollte aus einer leeren Bibliothek ziehen
LostPoisonYou = Du hattest zehn oder mehr Giftmarken
LostPoisonOther = { $player } hatte zehn oder mehr Giftmarken
LostCommanderDamageYou = Du hast 21 oder mehr Kampfschaden von einem Commander erhalten
LostCommanderDamageOther = { $player } hat 21 oder mehr Kampfschaden von einem Commander erhalten
LostConcededYou = Du hast aufgegeben
LostConcededOther = { $player } hat aufgegeben
LostEffectYou = Ein Effekt ließ dich verlieren
LostEffectOther = Ein Effekt ließ { $player } verlieren
HouseClockYou = Deine Zeit lief ab, und das Haus traf deine letzte Entscheidung
HouseClockOther = Die Zeit von { $player } lief ab, und das Haus traf die letzte Entscheidung
HouseStandInYou = Du warst nicht verbunden, und das Haus traf deine letzte Entscheidung
HouseStandInOther = { $player } war nicht verbunden, und das Haus traf die letzte Entscheidung
GatewayNoAnswer = das Gateway antwortete nicht: { $error }
GatewayAnswered = das Gateway antwortete mit { $status }
NoOfflineDuel = Offline-Duell konnte nicht gestartet werden
UnsavedChanges = ungespeicherte Änderungen – nochmal drücken zum Verlassen
NoRoomForCopy = kein Platz für noch eine Kopie davon
CouldNotReachTable = Tisch nicht erreichbar: { $error }
BrowseLooking = Gezeigt
BrowseHand = Hand
BrowseBattlefield = Spielfeld
BrowseGraveyard = Friedhof
BrowseExile = Exil
BrowseCommand = Kommandozone
BrowseAll = Alle
BrowseEmpty = nichts hier
BrowseFilter = Name, Typ…
BrowseTallyUpTo = { $count } von bis zu { $max } gewählt
BrowseTallyExact = { $count } von { $max } gewählt
BrowseConfirm = Bestätigen
FilterBuild = Filter einstellen
FilterAdd = Bedingung hinzufügen…
FilterDone = Fertig
FilterClear = Leeren
FilterAsTyped = wie getippt
FilterUnanswerable = Eine Bedingung kann hier nicht geprüft werden – die Liste bleibt leer, solange sie steht.
FilterAlsoChips = Zusätzlich grenzen Chips diese Liste ein: { $chips }
FilterChipPlayable = nur spielbare
FilterChipCmc = Manabetrag { $mana_value }
FilterChipCmcUp = Manabetrag { $mana_value } oder mehr
FilterKindText = Text
FilterKindColor = Farbe
FilterKindNumber = Zahl
FilterKindFlag = Eigenschaft
FilterKindCost = Kosten
FilterKeyLoose = Überall
FilterKeyName = Name
FilterKeyExact = Genauer Name
FilterKeyOracle = Regeltext
FilterKeyType = Typ
FilterKeyColor = Farbe
FilterKeyIdentity = Farbidentität
FilterKeyMana = Manakosten
FilterKeyManaValue = Manabetrag
FilterKeyPower = Stärke
FilterKeyToughness = Widerstandskraft
FilterKeyLoyalty = Loyalität
FilterAtLeast = mindestens
FilterExactly = genau
FilterAtMost = höchstens
FilterContains = enthält
FilterCount = Anzahl
FilterMulticolor = mehrfarbig
FilterEven = gerade
FilterOdd = ungerade
FlagPlayable = spielbar
FlagCommander = Commander
FlagBasic = Standardland
FlagDfc = doppelseitig
BrowseNone = Keine
BrowseOrderHint = Karte antippen, dann die, vor die sie soll, oder ein Stapelende
ArrangeLibraryTop = Oben auf die Bibliothek
ArrangeLibraryBottom = Unter die Bibliothek
ArrangeGraveyard = Auf den Friedhof
ArrangeUnplaced = Noch nicht gelegt
ArrangePutHere = Hierher legen
ArrangeEmptyPile = Leer
BrowseZoneOf = { $zone } · { $player }
BrowseTabCount = { $label } ({ $count })
LogACard = eine Karte
LogAFaceDownCard = eine verdeckte Karte
LogAPlaneswalker = einen Planeswalker
LogABattle = eine Schlacht
BattleProtectedBy = Beschützt von { $player }
BattleDefense = Verteidigung { $defense }
LogAnd = { $list } und { $last }
LogYouInList = du
LogRepeated = { $line } (×{ $count })
LogTurnYou = Zug { $turn } · dein Zug
LogTurn = Zug { $turn } · { $player }
LogMulliganYou = { $you_subject } hast einen Mulligan genommen
LogMulligan = { $player } hat einen Mulligan genommen
LogKeptCardsYou =
    { $count ->
        [one] { $you_subject } hast { $count } Karte behalten
       *[other] { $you_subject } hast { $count } Karten behalten
    }
LogKeptCards =
    { $count ->
        [one] { $player } hat { $count } Karte behalten
       *[other] { $player } hat { $count } Karten behalten
    }
LogTimedPassedYou = { $you_subject } hattest keine Zeit mehr und hast gepasst
LogTimedPassed = { $player } hatte keine Zeit mehr und hat gepasst
LogTimedKeptYou = { $you_subject } hattest keine Zeit mehr und hast deine Hand behalten
LogTimedKept = { $player } hatte keine Zeit mehr und hat die Hand behalten
LogTimedNoAttackersYou = { $you_subject } hattest keine Zeit mehr und hast nicht angegriffen
LogTimedNoAttackers = { $player } hatte keine Zeit mehr und hat nicht angegriffen
LogTimedNoBlockersYou = { $you_subject } hattest keine Zeit mehr und hast nicht geblockt
LogTimedNoBlockers = { $player } hatte keine Zeit mehr und hat nicht geblockt
LogTimedDeclinedYou = { $you_subject } hattest keine Zeit mehr und hast abgelehnt
LogTimedDeclined = { $player } hatte keine Zeit mehr und hat abgelehnt
LogTimedChosenYou = { $you_subject } hattest keine Zeit mehr, und das Haus hat für { $you_object } gewählt
LogTimedChosen = { $player } hatte keine Zeit mehr, und das Haus hat gewählt
LogStandInYou = { $you_subject } bist weggegangen, und das Haus hat deinen Platz übernommen
LogStandIn = { $player } ist weggegangen, und das Haus hat den Platz übernommen
LogConnectionLostYou = { $you_subject } hast die Verbindung verloren – der Tisch wartete { $wait }, bevor das Haus übernommen hätte
LogConnectionLost = { $player } hat die Verbindung verloren – der Tisch wartet { $wait } auf die Rückkehr
LogConnectionLostPausedYou = { $you_subject } hast die Verbindung verloren – das Spiel war pausiert, bis die Verbindung zurück war
LogConnectionLostPaused = { $player } hat die Verbindung verloren – das Spiel ist pausiert, bis die Verbindung zurück ist
LogReturnedYou = { $you_subject } bist zurückgekommen
LogReturned = { $player } ist zurückgekommen
LogLandPlayedYou = { $you_subject } hast { $land }{ $from } gespielt
LogLandPlayed = { $player } hat { $land }{ $from } gespielt
LogCastYou = { $you_subject } hast { $spell }{ $from } gewirkt
LogCast = { $player } hat { $spell }{ $from } gewirkt
LogAbilityYou = { $you_subject } hast eine Fähigkeit auf den Stapel gelegt: { $source }
LogAbility = { $player } hat eine Fähigkeit auf den Stapel gelegt: { $source }
LogActivatedYou = { $you_subject } hast { $source } aktiviert
LogActivated = { $player } hat { $source } aktiviert
LogTriggeredYou = { $source } hat für { $you_object } ausgelöst
LogTriggered = { $source } hat für { $player } ausgelöst
LogLifeNowPoints =
    { $count ->
        [one] jetzt { $life } Lebenspunkt (vorher { $before })
       *[other] jetzt { $life } Lebenspunkte (vorher { $before })
    }
LogCountered = { $spell } wurde neutralisiert
LogDidNotResolve = { $object } wurde nicht verrechnet
LogDrewYou = { $you_subject } hast { $cards } gezogen
LogDrew = { $player } hat { $cards } gezogen
LogDrewCardsYou =
    { $count ->
        [one] { $you_subject } hast { $count } Karte gezogen
       *[other] { $you_subject } hast { $count } Karten gezogen
    }
LogDrewCards =
    { $count ->
        [one] { $player } hat { $count } Karte gezogen
       *[other] { $player } hat { $count } Karten gezogen
    }
LogDiscardedYou = { $you_subject } hast { $card } abgeworfen
LogDiscarded = { $player } hat { $card } abgeworfen
LogMovedShuffled = { $object } wurde { $from } { $to } gemischt
LogOntoLibraryTopYou = oben auf deine Bibliothek
LogOntoLibraryTop = oben auf die Bibliothek von { $player }
LogOntoLibraryBottomYou = unter deine Bibliothek
LogOntoLibraryBottom = unter die Bibliothek von { $player }
LogIntoLibraryAtYou = als { $position }. Karte von oben in deine Bibliothek
LogIntoLibraryAt = als { $position }. Karte von oben in die Bibliothek von { $player }
LogFromOwnHand = aus der eigenen Hand
LogFromOwnLibrary = aus der eigenen Bibliothek
LogFromOwnGraveyard = aus dem eigenen Friedhof
LogYouSubject = du
LogYouObject = dich
LogYouIndirect = dir
LogLostEmptyDrawYou = { $you_subject } wolltest aus einer leeren Bibliothek ziehen
LogLostPoisonYou = { $you_subject } hattest zehn oder mehr Giftmarken
LogLostCommanderDamageYou = { $you_subject } hast 21 oder mehr Kampfschaden von einem einzigen Commander erhalten
LogLostConcededYou = { $you_subject } hast aufgegeben
LogLostEffectYou = Ein Effekt ließ { $you_object } verlieren
LogMoved = { $object } ist { $from } { $to } gelangt
LogFromLibraryYou = aus deiner Bibliothek
LogFromLibrary = aus der Bibliothek von { $player }
LogFromHandYou = aus deiner Hand
LogFromHand = aus der Hand von { $player }
LogFromGraveyardYou = aus deinem Friedhof
LogFromGraveyard = aus dem Friedhof von { $player }
LogFromBattlefield = vom Spielfeld
LogFromExile = aus dem Exil
LogFromCommand = aus der Kommandozone
LogIntoLibraryYou = in deine Bibliothek
LogIntoLibrary = in die Bibliothek von { $player }
LogIntoHandYou = auf deine Hand
LogIntoHand = auf die Hand von { $player }
LogIntoGraveyardYou = auf deinen Friedhof
LogIntoGraveyard = auf den Friedhof von { $player }
LogIntoBattlefield = ins Spiel
LogIntoExile = ins Exil
LogIntoCommand = in die Kommandozone
LogCreatedYou = { $you_subject } hast { $token } erzeugt
LogCreated = { $player } hat { $token } erzeugt
LogCombatDamageYou = { $source } hat { $you_indirect } { $amount } Kampfschaden zugefügt
LogDamageYou = { $source } hat { $you_indirect } { $amount } Schaden zugefügt
LogCombatDamage = { $recipient } hat { $amount } Kampfschaden durch { $source } erlitten
LogDamage = { $recipient } hat { $amount } Schaden durch { $source } erlitten
LogDamageUnsourcedYou = { $you_subject } hast { $amount } Schaden erlitten
LogDamageUnsourced = { $recipient } hat { $amount } Schaden erlitten
LogLifePointsYou =
    { $count ->
        [one] { $you_subject } hattest danach { $life } Lebenspunkt (vorher { $before })
       *[other] { $you_subject } hattest danach { $life } Lebenspunkte (vorher { $before })
    }
LogLifePoints =
    { $count ->
        [one] { $player } hatte danach { $life } Lebenspunkt (vorher { $before })
       *[other] { $player } hatte danach { $life } Lebenspunkte (vorher { $before })
    }
LogPlayerCountersYou = { $you_subject } hattest danach { $count } { $counter } (vorher { $before })
LogPlayerCounters = { $player } hatte danach { $count } { $counter } (vorher { $before })
LogCountersGot = { $object } hat { $count } { $counter } bekommen
LogCounters = { $object } hatte danach { $count } { $counter } (vorher { $before })
LogCountersPlus =
    { $count ->
        [one] +{ $power }/+{ $toughness }-Marke
       *[other] +{ $power }/+{ $toughness }-Marken
    }
LogCountersMinus =
    { $count ->
        [one] -{ $power }/-{ $toughness }-Marke
       *[other] -{ $power }/-{ $toughness }-Marken
    }
LogCountersLoyalty =
    { $count ->
        [one] Loyalitätsmarke
       *[other] Loyalitätsmarken
    }
LogCountersBore =
    { $count ->
        [one] Bohrungsmarke
       *[other] Bohrungsmarken
    }
LogCountersShield =
    { $count ->
        [one] Schildmarke
       *[other] Schildmarken
    }
LogCountersDefense =
    { $count ->
        [one] Verteidigungsmarke
       *[other] Verteidigungsmarken
    }
LogCountersLore =
    { $count ->
        [one] Sagenmarke
       *[other] Sagenmarken
    }
LogCountersTime =
    { $count ->
        [one] Zeitmarke
       *[other] Zeitmarken
    }
LogCountersAge =
    { $count ->
        [one] Altersmarke
       *[other] Altersmarken
    }
LogCountersCharge =
    { $count ->
        [one] Ladungsmarke
       *[other] Ladungsmarken
    }
LogCountersPoison =
    { $count ->
        [one] Giftmarke
       *[other] Giftmarken
    }
LogCountersEnergy =
    { $count ->
        [one] Energiemarke
       *[other] Energiemarken
    }
LogCountersRad =
    { $count ->
        [one] Verstrahlungsmarke
       *[other] Verstrahlungsmarken
    }
LogCountersFlying =
    { $count ->
        [one] Fliegend-Marke
       *[other] Fliegend-Marken
    }
LogCountersShadow =
    { $count ->
        [one] Irrealität-Marke
       *[other] Irrealität-Marken
    }
LogCountersIndestructible =
    { $count ->
        [one] Unzerstörbar-Marke
       *[other] Unzerstörbar-Marken
    }
LogCountersHone =
    { $count ->
        [one] Feinschliffmarke
       *[other] Feinschliffmarken
    }
LogCountersLifelink =
    { $count ->
        [one] Lebensverknüpfung-Marke
       *[other] Lebensverknüpfung-Marken
    }
LogCountersLevel =
    { $count ->
        [one] Stufenmarke
       *[other] Stufenmarken
    }
LogCountersOther =
    { $count ->
        [one] Marke
       *[other] Marken
    }
LogAttackedYou = { $attacker } hat { $you_object } angegriffen
LogAttacked = { $attacker } hat { $defender } angegriffen
LogBlocked = { $blocker } hat { $attacker } geblockt
LogBanded = { $attacker } greift in einem Bündnis mit { $leader } an
LogControlYou = { $you_subject } hast die Kontrolle über { $object } übernommen
LogControl = { $player } hat die Kontrolle über { $object } übernommen
LogTurnedFaceUp = { $object } wurde aufgedeckt
LogTransformed = { $object } wurde transformiert
LogCardsKeptYou = { $you_subject } hast gewählt, { $cards } zu behalten
LogCardsKept = { $player } hat gewählt, { $cards } zu behalten
LogRevealedYou = { $you_subject } hast { $cards } offen vorgezeigt
LogRevealed = { $player } hat { $cards } offen vorgezeigt
LogShuffledYou = { $you_subject } hast deine Bibliothek gemischt
LogShuffled = { $player } hat die eigene Bibliothek gemischt
LogRolledYou = { $you_subject } hast mit einem W{ $sides } eine { $result } gewürfelt
LogRolled = { $player } hat mit einem W{ $sides } eine { $result } gewürfelt
LogFlipWonYou = { $you_subject } hast eine Münze geworfen und den Wurf gewonnen
LogFlipLostYou = { $you_subject } hast eine Münze geworfen und den Wurf verloren
LogFlipWon = { $player } hat eine Münze geworfen und den Wurf gewonnen
LogFlipLost = { $player } hat eine Münze geworfen und den Wurf verloren
LogWonYou = { $you_subject } hast das Spiel gewonnen
LogWonOne = { $player } hat das Spiel gewonnen
LogWonMany = { $players } haben das Spiel gewonnen
LogDrawn = Das Spiel endete unentschieden
LogLoopBroken = Eine Endlosschleife wurde erkannt und beendet
LogLoop = Eine Endlosschleife wurde erkannt
LogDay = Es wurde Tag
LogNight = Es wurde Nacht
LogMonarchYou = { $you_subject } bist zum Monarchen geworden
LogMonarch = { $player } ist zum Monarchen geworden
LogAimedMode = Modus: { $modes }
LogAimedModes = Modi: { $modes }
LogAimedTarget = Ziel: { $targets }
LogAimedTargets = Ziele: { $targets }
LogAimedSpell = { $source }: { $details }
LogAimedAbility = Fähigkeit von { $source }: { $details }
LogQuoted = „{ $text }“
LogPhaseBeginning = Beginn
LogPhaseFirstMain = Hauptphase
LogPhaseCombat = Kampf
LogPhaseSecondMain = Zweite Hauptphase
LogPhaseEnding = Ende des Zuges
ReportButton = Problem melden
ReportKindBug = Fehler
ReportKindImprovement = Vorschlag
ReportKindFeedback = Feedback
ReportKindCrash = Absturz
ReportKindOther = Sonstiges
ReportTextHint = Was ist passiert, und was hast du erwartet?
ReportChars = { $count } / { $max } Zeichen
ReportTextTooLong = Der Text ist länger als { $count } Zeichen.
ReportIncludeHeading = Außerdem senden (auf diesem Gerät gemerkt):
ReportAlways = Immer gesendet: dein Text, die Version dieses Clients und die Kennung der Partie. Das Gateway fügt seine eigene Aufzeichnung der Partie hinzu, die niemanden namentlich nennt.
ReportAlwaysLocal = Immer gesendet: dein Text und die Version dieses Clients.
ReportCatSystem = System und Hardware
ReportCatSystemHint = Plattform, Prozessoranzahl, Grafikadapter, Fenstergröße, Sprache.
ReportCatGame = Der Tisch, wie du ihn siehst
ReportCatGameHint = Deine Sicht aufs Spielfeld, die offene Frage und was du ausgewählt hattest. Nichts, was vor dir verborgen ist.
ReportCatLog = Dein Spielprotokoll
ReportCatLogHint = Die Namen anderer Spieler werden durch Player A, Player B, … ersetzt.
ReportCatSettings = Einstellungen
ReportCatSettingsHint = Sprache, Anzeige, Tastenbelegung und Daueranweisungen. Keine Namen, Adressen oder Tokens.
ReportCatScreenshot = Bild des Baylee-Fensters
ReportCatScreenshotHint = Nur das Baylee-Fenster selbst, wie es beim Öffnen dieses Formulars aussah: nie dein Desktop, andere Programme oder andere Fenster. Es kann Namen anderer Spieler zeigen.
ReportCatNothing = (hier gibt es nichts zu senden)
ReportCatNoShotOnWeb = (die Browser-Version macht kein Bild)
ReportCatScreenshotHintWeb = Die Browser-Version des Clients kann kein Bild ihres Fensters machen, ein Bericht von hier enthält also nie eines. Ist das Kästchen angehakt, gilt es, wenn du aus dem Desktop-Client meldest.
ReportShotSize = { $width } × { $height }, { $kilobytes } KB
ReportCrashesBox = Absturzberichte automatisch senden
ReportPreviewShow = Was gesendet wird
ReportPreviewHide = Vorschau aus
ReportSend = Senden
ReportClose = Schließen
ReportSending = Wird gesendet…
ReportPreparing = Anhänge werden vorbereitet…
ReportSent = Danke. Bericht { $id } ist angekommen.
ReportsUnavailable = Dieses Gateway nimmt gerade keine Berichte an.
ReportSignInAgain = Deine Sitzung ist abgelaufen. Melde dich erneut an und sende den Bericht dann.
ReportTooLarge = Der Bericht ist zu groß. Lass das Fensterbild oder das Protokoll weg und versuch es erneut.
ReportTooMany = Zu viele Berichte in kurzer Zeit. Warte etwas und sende dann erneut.
ReportUnreachable = Das Gateway war nicht erreichbar. Dein Bericht ist noch da.
ReportNotPassedOn = Das Gateway konnte den Bericht nicht weitergeben. Dein Bericht ist noch da; versuch es später erneut.
ReportFailed = Der Bericht konnte nicht gesendet werden. Versuch es später erneut.
ReportNeedsSession = Melde dich bei einem Gateway an, um einen Bericht zu senden. Eine andere Adresse, an die er gehen könnte, kennt dieser Client nicht.
ReportGoesDirect = Du bist nicht angemeldet, darum geht dieser Bericht direkt an Baylee unter { $url }, ohne Konto.
ReportsDirectUnavailable = Dort werden gerade keine Berichte direkt von einem Client angenommen. Melde dich bei einem Gateway an, um ihn zu senden.
ReportDirectUnreachable = Die Adresse für Berichte war nicht erreichbar. Dein Bericht ist noch da.
ReportRecordBox = Die vollständige Aufzeichnung dieses Spiels anhängen
ReportRecordHint = Jede Aktion des Spiels, das dieses Gerät ausgerichtet hat, ab dem Mischen ({ $kilobytes } KB). Sie zeigt die Karten aller Plätze, auch die verdeckten: Hände, Bibliotheken, verdeckte Karten. Sie nennt niemanden. Wird bei jedem Bericht neu gefragt.
ReportRecordNever = Nie anbieten, die Aufzeichnung eines Spiels anzuhängen
ReportConfirmTitle = Diesen Bericht senden?
ReportConfirmTo = An: { $url }
ReportConfirmText = Dein Text ({ $count } Zeichen) und die Version dieses Builds
ReportConfirmPart = und: { $label }
ReportConfirmRecord = und die vollständige Aufzeichnung des Spiels, das dieses Gerät ausgerichtet hat ({ $kilobytes } KB): die Karten aller Plätze, auch die verdeckten
ReportConfirmDevice = und eine zufällige Kennung, die dieses Gerät für Berichte erzeugt hat, damit sich Berichte eines Geräts zuordnen lassen; nie dein Name, dein Account oder deine Adresse
ReportConfirmSend = Jetzt senden
ReportConfirmBack = Zurück
ReportRecordLeftOut = Die Aufzeichnung des Spiels war zu groß zum Anhängen und wurde weggelassen.
ReportLeaked = Nicht gesendet: Der Bericht enthält einen geheimen Wert ({ $secret }). Entferne ihn und versuch es erneut.
ReportTrimmed = Um die Größengrenze einzuhalten, wurden das Fensterbild oder ältere Protokollzeilen weggelassen.
CrashAskTitle = Baylee wurde beim letzten Mal unerwartet beendet.
CrashAskBody = Darf der Client Absturzberichte an das Gateway senden, auf dem du spielst? Ein Absturzbericht enthält den Fehler, die Stelle im Programm, an der er auftrat (den Backtrace), die Version dieses Clients und, wenn du sie im Berichtsformular erlaubst, Systemangaben. Nichts über deine Partien oder deinen Account. Du kannst das jederzeit im Berichtsformular ändern.
CrashAskSend = Absturzberichte senden
CrashAskNever = Nicht senden
SeatPanelTitle = Sprachmodell-Plätze
SeatPanelAbout = Welches Modell die Seat-Bridge (baylee-seat) am Tisch spielt und was sie ausgeben darf. Kein Schlüssel steht je in dieser Datei: Jedes Profil nennt die Umgebungsvariable, aus der sein Schlüssel gelesen wird, oder legt ihn im Schlüsselbund dieses Rechners ab.
SeatFileAt = Datei: { $path }
SeatDesktopOnly = Sprachmodell-Plätze werden am Desktop eingerichtet, wo die Seat-Bridge läuft.
SeatNoConfigDir = Dieses System nennt kein Konfigurationsverzeichnis, daher gibt es keine Einstellungsdatei für die Seat-Bridge.
SeatEmpty = Noch keine Einstellungsdatei: Die Seat-Bridge spielt mit ihren eingebauten Vorgaben, und nichts begrenzt ihre Ausgaben über Partien hinweg. Füge ein Profil hinzu, um eine anzulegen.
SeatNoProfiles = Keine Profile: Ohne --mind oder --profile spielt die Bridge die Haus-KI.
SeatFileRefused = Die Einstellungsdatei ist nicht verwendbar und bleibt, wie sie ist: { $error }. Korrigiere sie von Hand; dieser Bereich liest sie neu, sobald sie sich ändert.
SeatChangedOnDisk = Die Datei hat sich auf der Festplatte geändert, seit sie gelesen wurde. Speichern überschreibt sie; Änderungen verwerfen zeigt sie.
SeatAdd = Profil hinzufügen
SeatDuplicate = Duplizieren
SeatRemove = Entfernen
SeatDefault = Standard
SeatChipDefault = { $profile } · Standard
SeatDefaultAbout = Das Standardprofil spielt, wenn die Bridge weder --mind noch --profile bekommt.
SeatDefaultNone = Kein Standardprofil: Ohne --mind und --profile spielt die Bridge die Haus-KI.
SeatName = Name
SeatProvider = Protokoll
SeatProviderAnthropic = Anthropic
SeatProviderOpenAi = OpenAI-kompatibel
SeatProviderCli = Agent-CLI (Abo)
SeatCliModel = Bei einer CLI ist das Modell ihr Werkzeug, dahinter auf Wunsch dessen eigenes Modell: claude oder claude:opus.
SeatModel = Modell
SeatPriced = { $model }: { $input } Eingabe, { $output } Ausgabe
SeatPricedAbout = Preise in US-Dollar je Million Tokens, wie dieser Build sie kennt.
SeatEffort = Denkaufwand
SeatAnswer = Antwortet mit
SeatSessionApi = Verwaltete Spielsitzungen
SeatSessionApiAbout = Nur aktivieren, wenn dieser Server die Sitzungserweiterung unterstützt. Behält das Spielgespräch und beendet es beim Verlassen des Platzes.
SeatFaultSessionApi = Verwaltete Sitzungen benötigen einen OpenAI-kompatiblen Server mit Sitzungsunterstützung.
SeatSessionOff = Aus
SeatSessionOn = Ein
SeatAnswerBuild = Standard
SeatAnswerTools = Werkzeuge
SeatAnswerJson = JSON
SeatAnswerJsonSchema = JSON-Schema
SeatMaxTokens = Max. Ausgabe-Tokens
SeatPriceIn = Preis Eingabe ($ je Million)
SeatPriceOut = Preis Ausgabe ($ je Million)
SeatBuildPrice = Preis dieses Builds: { $input } Eingabe, { $output } Ausgabe je Million Tokens. Ein hier angegebener Preis ersetzt ihn.
SeatNoBuildPrice = Dieser Build kennt keinen Preis für dieses Modell: Gib hier einen an oder begrenze seine Partien in Tokens.
SeatGameUsd = Pro Partie ($)
SeatGameTokens = Pro Partie (Tokens)
SeatThinkSecs = Bedenkzeit (Sekunden)
SeatKeyEnv = Schlüssel-Variable
SeatKeySet = { $variable } ist in der Umgebung dieses Programms gesetzt.
SeatKeyUnset = { $variable } ist in der Umgebung dieses Programms nicht gesetzt. Die Bridge liest sie aus ihrer eigenen.
SeatPresets = Adapter hinzufügen (Protokoll und Adresse, danach änderbar):
SeatKeyBox = Schlüssel
SeatKeyHint = Schlüssel hier einfügen
SeatKeyReplaceHint = neuen Schlüssel einfügen, um ihn zu ersetzen
SeatKeyKeep = Merken
SeatKeyForget = Vergessen
SeatKeyAsking = Schlüsselbund dieses Rechners wird abgefragt…
SeatKeyKept = Im Schlüsselbund dieses Rechners liegt ein Schlüssel, nur für { $host }. Er wird nie angezeigt; die Variable geht vor, wo sie gesetzt ist.
SeatKeyNoneKept = Kein Schlüssel für { $host } gespeichert. Ein hier eingefügter geht in den Schlüsselbund dieses Rechners, nie in die Datei.
SeatKeyStoreUnavailable = Kein Schlüsselbund verfügbar ({ $reason }): Setze die Schlüssel-Variable in der Umgebung.
SeatKeyNoEntry = Ein Schlüssel kann gespeichert werden, sobald Schlüssel-Variable und Adresse stimmen.
SeatBaseUrl = Basis-URL
SeatByDefault = standardmäßig { $value }
SeatBaseByDefault = { $variable }, sonst { $url }
SeatEffortEndpoint = wie der Endpunkt es vorgibt
SeatEffortCli = wie die CLI es vorgibt
SeatGameCalls = Pro Partie (Aufrufe)
SeatNoCallLimit = keine Grenze
SeatCommand = Programm
SeatCommandByDefault = { $tool }, im PATH gesucht
SeatCliNoPrice = Eine CLI spielt mit deinem Abo, das keinen Preis hat: Ihre Partien sind in Tokens und Aufrufen begrenzt und werden auf die Token-Obergrenzen angerechnet.
SeatCaps = Obergrenzen über alle Partien
SeatCapsAbout = In Dollar zählen die Partien von Modellen mit Preis, in Tokens die von Modellen ohne. Ein leeres Feld heißt: keine Grenze.
SeatDayUsd = Pro Tag ($)
SeatMonthUsd = Pro Monat ($)
SeatDayTokens = Pro Tag (Tokens)
SeatMonthTokens = Pro Monat (Tokens)
SeatUnpricedDay = Die Obergrenzen zählen Dollar pro Tag, und dieses Modell hat keinen Preis: Die Bridge verweigert seine Partien, bis es einen Preis hat oder die Obergrenzen ein Tageslimit in Tokens haben.
SeatUnpricedMonth = Die Obergrenzen zählen Dollar pro Monat, und dieses Modell hat keinen Preis: Die Bridge verweigert seine Partien, bis es einen Preis hat oder die Obergrenzen ein Monatslimit in Tokens haben.
SeatSpent = Ausgegeben
SeatZoneLocal = Gezählt in Ortszeit, { $offset }.
SeatZoneUtc = Gezählt in UTC.
SeatSpentDay = Heute, { $period }: { $spent }
SeatSpentMonth = Diesen Monat, { $period }: { $spent }
SeatOf = { $spent } von { $cap }
SeatTokens =
    { $count ->
        [one] { $tokens } Token
       *[other] { $tokens } Tokens
    }
SeatGames = Partien: { $count }, davon offen: { $open }
SeatBookUnreadable = Das Ausgabenbuch kann nicht gelesen werden: { $error }
SeatSave = Speichern
SeatRevert = Änderungen verwerfen
SeatUnsaved = Ungespeicherte Änderungen.
SeatFixFirst = Korrigiere das Markierte, dann speichere.
SeatSavedNote = Gespeichert.
SeatNotSaved = Nicht gespeichert: { $error }
SeatFaultWhole = Eine ganze Zahl, in Ziffern.
SeatFaultDollars = Ein Betrag in US-Dollar, nicht unter null, etwa 2,5.
SeatFaultHalfPrice = Gib beide Preise an, Eingabe und Ausgabe, oder keinen.
SeatFaultNameTaken = Ein anderes Profil hat diesen Namen.
SeatFaultKeyNamed = Benannt wie ein Schlüssel. Ein Schlüssel gehört nie in die Einstellungsdatei: Setze ihn in der Umgebung und nenne seine Variable unter Schlüssel-Variable.
SeatFaultKeyShaped = Das sieht aus wie ein API-Schlüssel. Ein Schlüssel gehört nie in die Einstellungsdatei: Setze ihn in der Umgebung und nenne seine Variable unter Schlüssel-Variable.
SeatFaultNoDefault = Der Standard nennt kein Profil.
SeatFaultName = Ein Name hat bis zu 32 Buchstaben, Ziffern, - und _ und beginnt mit einem Buchstaben oder einer Ziffer.
SeatFaultModel = Eine Modell-ID hat bis zu 100 Buchstaben, Ziffern und - _ . : / @.
SeatFaultTool = Nenne zuerst das Werkzeug: claude oder claude:<Modell>.
SeatFaultCliAnswer = Eine CLI hat keins unserer Werkzeuge: Sie antwortet mit JSON oder einem JSON-Schema.
SeatFaultCliKey = Eine CLI liest keinen Schlüssel und hat keine Adresse: Sie spielt so, wie du bei ihr angemeldet bist.
SeatFaultCliPrice = Ein Abo hat keinen Preis: Begrenze die Partien einer CLI in Tokens und Aufrufen.
SeatFaultCliOnly = Nur ein CLI-Profil führt ein Programm aus.
SeatFaultCommand = Der absolute Pfad des Programms, etwa /opt/homebrew/bin/claude.
SeatFaultGameCalls = Höchstzahl der Aufrufe pro Partie: mindestens 1.
SeatFaultEffort = Ein Denkaufwand ist ein kleingeschriebenes Wort, etwa low, medium oder high.
SeatFaultJson = JSON-Antworten sind für OpenAI-kompatible Endpunkte und CLIs; Anthropics Modelle antworten mit Werkzeugen.
SeatFaultMaxTokens = Höchstzahl an Tokens für eine Antwort: mindestens 1.
SeatFaultThinkSecs = Höchstdauer einer Antwort: mindestens 1 Sekunde.
SeatFaultUnpriced = Dieser Build kennt keinen Preis für dieses Modell, daher lässt sich keine Dollar-Grenze einhalten: Gib ihm einen Preis oder begrenze stattdessen die Tokens pro Partie.
SeatFaultKeyEnv = Der Name einer Umgebungsvariable, etwa { $example }: Großbuchstaben, Ziffern und _. Nie der Schlüssel selbst.
SeatFaultAddress = Eine https://-Adresse; http:// nur auf diesem Rechner.
SeatFaultFile = Diese Einstellungen lassen sich nicht schreiben.
ShellPlay = Lobby
ShellDecks = Decks
ShellThisDevice = Dieses Gerät
ShellYourAccount = Dein Account
ShellRetry = Erneut versuchen
ShellUndo = Rückgängig
TourChapterLine = Kapitel { $position } von { $total } · { $chapter }
TourNext = Weiter
TourDone = Fertig
TourBack = Zurück
TourSkip = Überspringen
TourSkipReason = Überspringt dieses Kapitel
TourTryFirst = Erst ausprobieren oder überspringen
TourKeysFree = Deine Tasten gelten jetzt dem Bildschirm
TourKeysTable = Deine Tasten gelten jetzt dem Tisch
TourNeverAgain = Touren nicht mehr zeigen
TourFold = Einklappen
TourTableWaits = Der Tisch wartet auf dich
TourOffToast = Touren aus · Einstellungen › Anzeige
TourRestartedToast = Touren beginnen von vorn
TourStartPractice = Übungsspiel starten
TourLater = Später
TourMenuRow = Tour
TourMenuRowAway = Tour (nur im Übungsspiel)
RowTourLobby = Lobby & Raum
HelpTourLobby = Zeigt dir Eingang, Spielen-Seite und Raum, jeweils beim ersten Besuch.
RowTourBuilder = Deckbauer
HelpTourBuilder = Zeigt dir deine Decks, den Deckbauer, Import, Export und die Historie.
RowTourTable = Tisch
HelpTourTable = Zeigt dir den Tisch, in einem Übungsspiel gegen das Haus.
RowTourTips = Hinweise im Spiel
HelpTourTips = Ein kurzer Hinweis, wenn du etwas zum ersten Mal siehst, einmal auf diesem Gerät.
RowTourRestart = Touren neu starten
HelpTourRestart = Vergisst, was du hier schon gesehen hast, und zeigt alle Touren noch einmal.
TourChapterDoor = Der Eingang
TourChapterPlay = Spielen
TourChapterCreate = Tisch erstellen
TourChapterRoom = Der Raum
TourChapterClosing = Zum Schluss
TourChapterShelf = Deine Decks
TourChapterEditing = Bearbeiten
TourChapterLists = Import und Export
TourChapterHistory = Historie
TourChapterArrival = Los geht’s
TourChapterPlace = Dein Platz
TourChapterCasting = Karten spielen
TourChapterStack = Der Stapel
TourChapterDecisions = Fragen
TourChapterPhases = Phasen und Passen
TourChapterDial = Die Tischmitte
TourChapterSeeing = Die Kamera
TourChapterReveals = Vorzeigen, Protokoll, Zonen
TourChapterKeys = Tasten und Menü
TourChapterReporting = Ein Problem melden
TourL1Title = Das Gateway
TourL1Body = Ein Gateway ist der Server, auf dem deine Decks und Tische liegen. Der farbige Punkt zeigt, ob es erreichbar ist, die Zeile darunter, ob deine Version zu ihm passt. Mit ‹ kommst du zur Liste der Gateways; dort kannst du auch eine eigene Adresse eingeben.
TourL2Title = Account oder Gast
TourL2Body = Als Gast brauchst du nur einen Namen. Dafür kannst du keine Bilder hochladen, und rund 30 Tage nach deinem letzten Besuch wird der Gast samt Decks gelöscht. Mit einem Account bleiben deine Decks und Einstellungen auf dem Gateway. In einer geschlossenen Beta fragt der Button, den du wählst, nach deinem Schlüssel.
TourL3Title = Offline spielen
TourL3Body = Offline spielen braucht kein Gateway: Du spielst dann direkt hier auf deinem Rechner gegen die Haus-KI. Daneben stehen Einstellungen und Über, wo du Lizenzen und Quellcode findest.
TourL4Title = Drei Bereiche
TourL4Body = Oben wechselst du zwischen Spielen, Decks und Einstellungen, auch mit den Tasten 1, 2 und 3. Rechts siehst du dein Gateway mit der Zahl der Tische und Spieler, die Glocke mit dem, was du verpasst hast, und deinen Account. Mit ? siehst du auf jedem Bildschirm alle Tasten, die gerade gelten.
TourL4TitleDesktop = Vier Bereiche
TourL4BodyDesktop = Oben wechselst du zwischen Spielen, Decks, Replays und Einstellungen, auch mit den Tasten 1, 2, 4 und 3. Rechts siehst du dein Gateway mit der Zahl der Tische und Spieler, die Glocke mit dem, was du verpasst hast, und deinen Account. Mit ? siehst du auf jedem Bildschirm alle Tasten, die gerade gelten.
TourL5Title = Dein nächstes Spiel
TourL5Body = Das ist das Deck, mit dem du an den nächsten Tisch gehst. Deck wechseln öffnet die Auswahl, Bearbeiten den Deckbauer. Hast du noch kein eigenes Deck, stehen hier drei Hausdecks: Hinzufügen & nutzen kopiert eins in deinen Account und wählt es gleich aus.
TourL6Title = Gegen das Haus
TourL6Body = Gegen das Haus startet ein Spiel gegen die Haus-KI, direkt hier im Client und ohne Uhr. Über den kleinen Pfeil am Button wählst du, wie stark sie spielt. In so einem Spiel läuft auch die Tisch-Tour.
TourL7Title = Die Tische
TourL7Body = Hier siehst du live alle Tische auf dem Gateway: Gastgeber, Format, freie Plätze und ob ein Passwort nötig ist. Mitspielen setzt dich an einen Tisch; ein Passwort tippst du direkt in der Zeile ein. Mit / suchst du, die Chips darüber filtern. Sitzt du schon irgendwo, steht hier Zurück zum Spiel.
TourL8Title = Tisch erstellen
TourL8Body = Mit Tisch erstellen machst du einen eigenen Tisch auf, auch mit der Taste C. Öffne den Dialog jetzt.
TourL9Title = Tisch einstellen
TourL9Body = Wähle, wie viele mitspielen (2 bis 8), und eine Vorlage: Commander, Duell 20 oder Fünf Länder. Mit Anpassen änderst du Lebenspunkte, Mulligan und Handgröße. Ein Passwort ist freiwillig. Die Uhr ist die Bedenkzeit pro Entscheidung, von Blitz bis ohne Uhr. Tisch öffnen bringt dich in den Raum.
TourL10Title = Die Plätze
TourL10Body = Jeder Platz ist eine Zeile. In deiner wählst du dein Deck und sagst Bereit. Auf einen freien Platz setzt du mit KI die Haus-KI und wählst ihre Stärke, oder mit Sprachmodell ein Modell mit deinem eigenen Schlüssel. Die Zeilen der anderen kannst du nur lesen.
TourL11Title = Platzmenü und Teams
TourL11Body = Als Gastgeber findest du hinter den drei Punkten am Platz mehr: Zum Gastgeber machen, Nächstes Team, Eigene Lebenspunkte, Startposition zum Testen und Zum freien Platz machen. Teams festlegen links bei den Regeln schaltet Teams ein; dann zeigt jeder Platz sein Team.
TourL12Title = Regeln und Einladung
TourL12Body = Links stehen die Regeln des Tischs; Regeln ändern öffnet den Dialog noch einmal. Einladung kopieren legt Tischname, Gateway und einen Hinweis aufs Passwort in die Zwischenablage. Den Raum verlässt du nur über Verlassen, nie mit Esc. Gehst du woanders hin, bleibt dein Platz; die Leiste oben oder R bringt dich zurück.
TourL13Title = Bereit und Starten
TourL13Body = Bereit klickt jeder für sich. Starten gehört dem Gastgeber (auch Strg/Cmd+Enter) und bleibt grau, bis alle bereit sind; warum, steht daneben. Der Tisch geht auf, sobald alle ihn geladen haben, und erst dann laufen die Uhren.
TourLeaveTable = Tisch verlassen
TourL14Title = Übungsspiel?
TourL14Body = Das war die Lobby. Den Tisch zeige ich dir am besten in einem Übungsspiel: du gegen drei Haus-KIs, ohne Uhr, mit deinem Deck. Jetzt starten? Später findest du es unter Spielen › Gegen das Haus und dann im Menü unter Tour.
TourD1Title = Deine Decks
TourD1Body = Hier liegen deine Decks und die Hausdecks. Das Bild einer Kachel ist dein Commander, sonst die teuerste Karte, die kein Land ist. Darunter stehen Format, Kartenzahl, Sideboard und wann du zuletzt gespeichert hast. Abzeichen markieren dein nächstes Deck und Karten, die noch nicht spielbar sind.
TourD2Title = Was eine Kachel kann
TourD2Body = Fürs nächste Spiel macht das Deck zu deinem nächsten. Bearbeiten öffnet den Deckbauer. Hinter den drei Punkten: Duplizieren, Umbenennen, Exportieren, Historie, Favorit und Löschen. Löschen fragt nicht nach; du hast sechs Sekunden für Rückgängig, auch mit Strg/Cmd+Z.
TourD3Title = Hausdecks
TourD3Body = Fertige Decks vom Gateway, jede Karte darin ist spielbar. Hinzufügen kopiert eins zu deinen Decks, Hinzufügen & nutzen wählt es gleich fürs nächste Spiel. Ansehen zeigt die Liste; fahr mit der Maus über eine Karte oder drück die Leertaste, um sie groß zu sehen.
TourD4Title = Neu oder importiert
TourD4Body = Neues Deck öffnet einen leeren Deckbauer. Importieren liest Baylee-Text, JSON, YAML oder Moxfield-Text und erkennt das Format selbst; Strg/Cmd+V oder eine hierher gezogene Datei geht auch. Einen Moxfield-Link lädt Baylee nicht; der Dialog sagt dir, was du kopieren musst. Öffne ein Deck, dann geht’s im Deckbauer weiter.
TourD5Title = Die Kopfzeile
TourD5Body = ‹ Decks bringt dich zurück und fragt vorher, wenn etwas nicht gespeichert ist. Der Stift benennt das Deck um. Speichern (Strg/Cmd+S) zeigt einen Punkt, solange etwas ungespeichert ist. Esc wirft dich nie aus dem Deckbauer; es schließt nur, was gerade obenauf offen ist.
TourD6Title = Der Kartenpool
TourD6Body = Links stehen alle Karten, die Baylee schon spielen kann, nicht der ganze Katalog. Nur spielbare ist an und blendet Platzhalter aus. Karten, die erst teilweise gehen, bleiben drin und sind markiert; den Grund zeigt die Kartenansicht. Die Zeile unter der Zahl sagt, was die Liste gerade noch eingrenzt.
TourD7Title = Suchen
TourD7Body = Das Suchfeld versteht die Scryfall-Schreibweise, etwa t:creature, o:draw, c:rg oder mv<=3, auch mit or, Klammern und Minus. Ein deutscher Kartenname findet die Karte genauso. Das Zahnrad baut dir den Filter als Formular. Tipp jetzt etwas ein.
TourD8Title = Hinzufügen
TourD8Body = Das + legt eine Karte in den Tab, der gerade offen ist, mit Shift in den anderen; Enter und Shift+Enter ebenso. Hinter den drei Punkten: Sideboard, Druck wählen und Karte öffnen. Fahr mit der Maus über eine Zeile, und du siehst die Karte groß. Füg jetzt eine Karte hinzu.
TourD9Title = Die Deckseite
TourD9Body = Rechts ist dein Deck: oben der Commander, darunter die Tabs Haupt, Sideboard und Statistik, gruppiert nach Typ, Manabetrag oder Farbe. In jeder Zeile zählen − und + runter und hoch; die drei Punkte verschieben die Karte, wählen den Druck oder entfernen alle Exemplare. Statistik zeigt Kurve, Farben und Sieben ziehen.
TourD10Title = Hinweise unten
TourD10Body = Unten stehen zwei Arten von Hinweisen. Was das Gateway ablehnt, macht Speichern grau: kein Name, ein leeres Deck, mehr als 250 Zeilen oder Karten, die es nicht mehr gibt. Alles andere ist nur ein Satz, zum Beispiel 60 Karten oder 15 im Sideboard. Leuchtet der Button, kannst du speichern.
TourD11Title = Speichern
TourD11Body = Jedes Speichern legt eine Version an, die du später in der Historie wiederfindest. Änderst du nichts, wird auch nichts geschrieben. Drück jetzt Strg/Cmd+S.
TourD11bTitle = Handy: eine Spalte
TourD11bBody = Auf dem Handy siehst du Pool oder Deck; der Schalter oben wechselt. Filter sammelt die Chips hinter einem Button. Enter im Suchfeld fügt den ersten Treffer hinzu.
TourD12Title = Import
TourD12Body = Füg deine Liste mit Strg/Cmd+V ein; Baylee erkennt das Format selbst, im Zweifel wählst du es mit den Pfeiltasten. Enter importiert, danach schließt Enter. Zeilen, die nicht gefunden wurden, bleiben stehen und werden genannt; nichts fällt still unter den Tisch.
TourD13Title = Export
TourD13Body = Mit den Pfeiltasten wählst du das Format. Strg/Cmd+C kopiert die Liste, Strg/Cmd+S speichert sie als Datei (nicht im Browser). Baylee-Text merkt sich Druck und Finish jeder Karte, Moxfield-Text nur Anzahl und Name.
TourD14Title = Historie
TourD14Body = Links stehen alle gespeicherten Versionen, rechts siehst du, was sich gegenüber der vorigen oder der aktuellen geändert hat. Wiederherstellen macht aus der alten Version eine neue; nichts geht verloren, und sechs Sekunden lang kannst du es rückgängig machen.
TourT1Title = Das Übungsspiel
TourT1Body = Das hier ist ein Übungsspiel: drei Haus-KIs, keine Uhr, nichts zählt. Jede Sprechblase hat Weiter, Zurück und Überspringen; mit Esc klappst du sie oben zu einem kleinen Streifen ein, über die Kapitelzeile springst du. Wartet der Tisch auf dich, klappt sie sich von selbst ein. ? zeigt dir jederzeit alle Tasten.
TourT2Title = Deine Matte
TourT2Body = Deine Matte hat drei Reihen: vorne zur Tischmitte hin die Kreaturen, dahinter der Rest, hinten die Länder. Am Rand liegen Bibliothek, Friedhof, Exil und Kommandozone, jeweils mit Zahl. Gleiche Karten liegen übereinander, mit ×N; ein Klick fächert sie auf. Wird eine Reihe zu voll, scrollt sie seitlich.
TourT3Title = Die Hand
TourT3Body = Deine Hand liegt unten. Du kannst sie nach Kosten, A–Z oder Typ sortieren und mit dem Mausrad scrollen.
TourT4Title = Eine Karte lesen
TourT4Body = Fahr mit der Maus über eine Karte, und sie hebt sich und wird groß. Hältst du dabei Cmd oder Alt, siehst du ihren Text als Textseite; T lässt das an, V öffnet die Vorschau per Tastatur. Du siehst immer den Text, den die Karte gerade im Spiel hat; eine Kopie zeigt den Text der kopierten Karte.
TourT5Title = Die Leiste unten
TourT5Body = Die Leiste unten sagt dir, was der Tisch gerade von dir will, und zeigt die Antworten mit ihren Tasten: Leertaste passt, Enter klickt, Tab geht zur nächsten Phase, Shift+Tab zum nächsten Zug; jede Entscheidung bleibt trotzdem deine. Rechts findest du Protokoll, Zonen und Menü.
TourT6Title = Leuchtende Karten
TourT6Body = Was du jetzt spielen kannst, leuchtet: Gold heißt, dein Mana reicht schon; Blau heißt, der Client würde dafür Länder tappen. Der erste Klick wählt die Karte an, der zweite (oder Enter, E) spielt sie; Esc wählt ab. Manafähigkeiten gehen mit einem Klick. Lebenspunkte zahlt der Client nie von selbst, ein X rät er nie.
TourT7Title = Spielen, dann zahlen
TourT7Body = Anders als bei Arena: Ein Klick spielt die Karte erst und zahlt danach. Die Karte geht auf den Stapel, dann kommen ihre Fragen (Modus, X, Ziele), dann das Bezahlen. Der Client tappt die Länder, die er sich vorgenommen hat; du kannst aber jedes Land auch selbst tappen. Was noch fehlt, steht als Geschuldet im Manavorrat.
TourT8Title = Wirken abbrechen
TourT8Body = Solange die Fragen oder das Bezahlen offen sind, steht ganz rechts in der Leiste Wirken abbrechen (Esc): Die Karte kommt zurück auf die Hand, getappte Länder werden wieder enttappt. Spiel jetzt eine Karte und brich ab; ist noch nichts spielbar, klick auf Weiter.
TourT9Title = Mehrere Fähigkeiten
TourT9Body = Hat eine Karte mehrere Fähigkeiten oder mehrere Wege, sie zu spielen, geht neben ihr ein Zettel auf: Kosten, der gedruckte Text und eine Taste von 1 bis 9; 0 blättert um. Eine Mana-Zeile geht sofort; jede andere wählst du an und bestätigst dann. Alternative Kosten werden hier abgefragt, bevor ein Land getappt wird.
TourT10Title = Der Stapel
TourT10Body = Oben rechts siehst du den Stapel, jeden Eintrag als Karte: der nächste groß mit seinen Zielen, die übrigen kleiner darunter. Die Zeile oben sagt, auf wen der Tisch gerade wartet. Ein langer Text scrollt in seinem Kasten. Mit dem Button klappst du das Ganze ein.
TourT11Title = Antworten im Voraus
TourT11Body = Ein Klick macht einen Eintrag zum Halt: Bis zur Markierung abarbeiten passt, bis er oben liegt. Für diese Kartenfähigkeit wählst du Immer passen, Immer fragen, Immer Ja oder Immer Nein; das gilt für jede Kopie und bleibt im Account (zurücksetzen: Einstellungen › Spielablauf). Ziele und Kosten werden immer abgefragt.
TourT12Title = F6 und F7
TourT12Body = F6 lässt den Stapel durchlaufen, F7 stellt die Fragen für diesen Zug ab. Beides endet von selbst, sobald etwas Neues passiert, und steht bis dahin in der Leiste; dieselbe Taste noch einmal beendet es auch. Dauerhaft automatisch läuft nur, was du unter Einstellungen › Spielablauf einschaltest; am Anfang ist nichts an.
TourT13Title = Fragen mit Auswahl
TourT13Body = Hat eine Frage mehrere Antworten, klappt aus der Leiste ein Fenster hoch: oben die Frage und das Bild der Karte, um die es geht (mit der Maus darüber siehst du es groß), unten die Antworten mit ihren Tasten. Was du wählen darfst, leuchtet auf dem Tisch. Das Fenster schließt nie von allein.
TourT14Title = Fenster einklappen
TourT14Body = Das Fenster verdeckt den Tisch. Mit X oder dem Button oben rechts klappst du es zu einem kleinen Streifen am rechten Rand ein; dann siehst du den ganzen Tisch und kannst jedes Ziel anklicken. Der Streifen oder X holt es zurück, die Frage bleibt. Klapp es jetzt ein und wieder auf.
TourT15Title = Andere Fragen
TourT15Body = Eine Farbe wählst du als Symbol, nicht als Wort. Eine Zahl tippst du ein oder stellst sie mit den Pfeiltasten; Rücktaste löscht. Bei einem Kreaturentyp stehen zuerst die Typen aus deinem Deck, dann alle alphabetisch; tippen filtert. Ein Kartenname kommt aus demselben Feld, über alle Karten.
TourT16Title = Die Phasen
TourT16Body = Am Rand jeder Matte stehen die zwölf Schritte eines Zugs, der aktuelle leuchtet. Grün: Hier fragt dich der Tisch. Rot: Hier passt er für dich. Am Anfang ist nichts rot. Ein Klick schaltet um, Shift+W und Shift+S wandern. Vorlagen: Einstellungen › Spielablauf. Deine eigenen Angriffs- und Blockschritte werden nie rot.
TourT17Title = Passen
TourT17Body = Leertaste passt. Tab läuft bis zur nächsten Phase, Shift+Tab bis zum nächsten Zug; jede Frage dazwischen bekommst du trotzdem. Hast du einen möglichen Angreifer, heißt Leertaste nicht „kein Angriff“; dafür ist O da. Am Gateway steht deine Uhr im Button: ab 60 Sekunden rot und hörbar; läuft sie ab, drückt sie für dich.
TourT18Title = Angreifen und blocken
TourT18Body = Ein Klick auf eine Kreatur schickt sie gegen den Gegner, der gerade im Fokus ist; noch ein Klick nimmt sie zurück. Gibt es mehrere mögliche Gegner, wechselst du mit C und Shift+C. Angreifen, Blocken oder Nichts deklarieren schickt deine Antwort ab. Bei deinen Kampffragen fährt die Kamera von selbst zu dir nach Hause.
TourT19Title = Die Uhr in der Mitte
TourT19Body = In der Mitte des Tischs steht die Zugzahl, am Rand drumherum ein Edelstein je Spieler in dessen Farbe. Der lange helle Zeiger zeigt, wer am Zug ist, der kurze blaue, auf wen der Tisch wartet. Kommt der blaue Zeiger zu dir, pulsiert die Mitte und du hörst einen Ton; abstellen kannst du ihn im Menü unter Prioritätston.
TourT20Title = Namensschilder
TourT20Body = Jede Matte hat ein Schild: Name, Lebenspunkte, Handkarten, Bibliothek, Gift, Commander-Schaden; eine Krone zeigt den Monarchen, ein Abzeichen die Haus-KI. Die Linie oben am Schild leuchtet hell, wenn der Spieler am Zug ist, und blau, wenn der Tisch auf ihn wartet. Unten links stehen alle Spieler noch einmal als Chips.
TourT21Title = Die Anordnung
TourT21Body = Oben links wählst du, wie der Tisch aufgebaut ist: Ring, Aufrechter Ring, Drehteller, Bogenschiene, Raster, Rampenlicht, Drehteller+ oder Fokusring. P öffnet die Liste, Shift+P nimmt die nächste. Das Häkchen merkt sich deine Wahl für diese Spielerzahl. Im Duell gibt es nur eine. Drück jetzt Shift+P, dann noch einmal.
TourT22Title = Einen Spieler besuchen
TourT22Body = Klick unten links auf einen Chip oder drück F, und die Kamera schwenkt hinter diesen Spieler; sein Brett steht dann aufrecht und groß vor dir, auf dem Tisch bewegt sich nichts. Shift+F geht zum vorigen; H, Esc oder dein eigener Chip bringt dich zurück. Zielen geht auch von dort. Drück jetzt F, dann H.
TourT23Title = Dem Zug folgen
TourT23Body = Von selbst folgt die Kamera nur deinem Zug und deinen Kampffragen. Soll sie immer zum Spieler am Zug schwenken, schalte unter Einstellungen › Grafik Tisch folgt dem Zug ein. Sie schwenkt aber nie, solange eine Frage an dich offen ist oder dein Mauszeiger auf dem Tisch liegt.
TourT24Title = Auf dem Handy
TourT24Body = Auf dem Handy siehst du in der Grundansicht deine Matte und die Tischmitte. Die anderen Spieler besuchst du über die Chips; die Anordnung findest du im Menü.
TourT25Title = Vorzeigen
TourT25Body = Zeigt ein anderer Spieler Karten offen vor, siehst du sie groß in einem Fenster: wer sie vorzeigt, die Karten, Schließen. Mit der Maus darüber wird jede noch größer. Du kannst es oben zu einem Streifen einklappen; Esc oder ein Klick schließt es. Es bleibt ein paar Sekunden und verschwindet nicht, nur weil du passt.
TourT26Title = Das Protokoll
TourT26Body = L oder die Schriftrolle in der Leiste öffnet rechts das Protokoll: Uhrzeit, Spieler fett und jede Karte als Link, der die Vorschau öffnet, außer Karten, die du nicht sehen durftest. Es springt zur neuesten Zeile, bis du hochscrollst. Am Ende des Spiels steht es komplett im Ergebnisfenster. Drück jetzt L.
TourT27Title = Die Zonen
TourT27Body = G, die Zonen-Tür in der Leiste oder ein Klick auf einen Kartenstapel am Mattenrand öffnet die Zonen: Friedhöfe, Exil und den Stapel. Das Suchfeld versteht dieselbe Schreibweise wie der Deckbauer. Fragt dich der Tisch nach einer Karte aus einem Kartenstapel, geht er in der Mitte auf: Leertaste hakt an, Enter schickt ab.
TourT28Title = Alle Tasten
TourT28Body = ? zeigt dir alle Tasten, die am Tisch gerade gelten. F8 meldet ein Problem, Strg/Cmd mit +, − und 0 ändert die Schriftgröße. Jede Taste kannst du unter Einstellungen › Steuerung umbelegen; das bleibt in deinem Account. Mit W, A, S, D und E spielst du ganz ohne Maus.
TourT29Title = Das Menü
TourT29Body = Das Menü unten rechts hat alles, was du im Spiel brauchst: Unentschieden anbieten, Aufgeben, Anordnung, Prioritätston, Musik, Problem melden und Tour. Weitere Einstellungen gibt es am Tisch nicht; die warten in der Lobby.
TourT30Title = Aufgeben und Ende
TourT30Body = Aufgeben fragt noch einmal nach: Enter heißt Zurück; zum Aufgeben drückst du → und dann Enter, oder du klickst. Am Ende siehst du ein Fenster mit dem Ergebnis, dem Grund je Spieler und dem ganzen Protokoll. Am Gateway gibt es Nochmal spielen, offline Zurück zur Lobby; Esc bringt dich immer in die Lobby.
TourT31Title = Ein Problem melden
TourT31Body = Die Ecke oben rechts, F8 oder Problem melden im Menü öffnet ein Formular über dem Tisch. Wähle die Art, also Fehler, Vorschlag, Feedback oder Sonstiges, und schreib, was los ist. Gesendet werden immer dein Text und die Version des Clients; alles andere nur, wenn du es anhakst. Öffne es jetzt.
TourT32Title = Anhänge
TourT32Body = Jede Zeile unter Anhänge ist eine Wahl, die dein Gerät sich merkt: System, dein Blick auf den Tisch, dein Protokoll (Namen ersetzt), Einstellungen (ohne Schlüssel), ein Bild nur vom Baylee-Fenster, nie vom Desktop. Alles ist angehakt; nimm vor dem Senden heraus, was du behältst. „Zeigen, was gesendet wird“ zeigt alles.
TourT33Title = Wohin es geht
TourT33Body = Dein Bericht geht an das Gateway, bei dem du angemeldet bist. Bist du nirgends angemeldet, geht er direkt an Baylees Adresse für Berichte; kennt diese Version keine, kannst du nur Kopieren. Mit Aufzeichnung fragt ein Fenster noch einmal nach. Am Gateway läuft deine Uhr derweil weiter; melde also lieber nach der Frage.
TourT34Title = Fertig
TourT34Body = Das war der Tisch. Jedes Kapitel kannst du in jedem Spiel gegen das Haus noch einmal ansehen: Menü › Tour. Touren aus- oder wieder einschalten: Einstellungen › Anzeige & Oberfläche › Touren. Viel Spaß!
ShellDetails = Details
ShellUseForNextGame = Fürs nächste Spiel
ShellEdit = Bearbeiten
ShellAddAndUse = Hinzufügen & nutzen
ShellReturnToGame = Zurück zum Spiel
ShellNeedsGateway = Braucht das Gateway
ShellSeatedAt = Du sitzt an { $table }
ShellCouldNotReach = { $gateway } ist nicht erreichbar
ShellNoOneWaiting = Niemand wartet
DiscoveryGateway = DEIN GATEWAY
DiscoveryOnline = Online
DiscoveryWaiting = Offene Tische
DiscoveryPlaying = Im Spiel
DiscoveryLive = Live-Updates verbunden
DiscoveryConnecting = Live-Updates werden verbunden…
DiscoveryRefresh = Tische aktualisieren
DiscoveryDetails = Gateway-Details
DiscoveryChoose = Finde deinen nächsten Tisch
DiscoveryHelp = Tritt einem Tisch bei und wähle dort dein Deck. Der Gastgeber legt Format und Spieltempo fest.
DiscoveryLoading = Tische werden aktualisiert
DiscoveryResults =
    { $count ->
        [one] { $count } Tisch angezeigt
       *[other] { $count } Tische angezeigt
    }
DiscoveryProtected = Mit Passwort
DiscoveryOpen = Beitritt möglich
ShellUpdatedJustNow = gerade aktualisiert
ShellOpenSeats = Freie Plätze
ShellNoPassword = Ohne Passwort
ShellMyDecks = Meine Decks
ShellHouseDecks = Hausdecks
ShellDeckDeleted = Deck gelöscht
ShellCopyHandle = Mein Handle kopieren
ShellTextSize = Textgröße
ShellTextSizeHelp = Strg/Cmd + und − ändern sie überall außerhalb eines Spiels.
ShellMasterVolume = Gesamtlautstärke
ShellHoldStill = Tisch ruhig halten
ShellAdvanced = Erweitert
ShellGallery = Shell-Bausteine
ShellKeySearch = Suchen
ShellKeyOverlay = Tastenkürzel
ShellKeyQuickSwitch = Gehe zu…
ShellKeyRefresh = Aktualisieren
ShellKeyTextLarger = Größerer Text
ShellKeyTextSmaller = Kleinerer Text
ShellKeyTextReset = Normale Textgröße
ShellKeyStart = Spiel starten
ShellKeySave = Deck speichern
ShellKeyExport = Exportieren
ShellOverlaySearch = Kürzel suchen
ShellOverlayEditKeys = Tasten ändern…
ShellClose = Schließen
ShellOverlayNoMatch = Kein Kürzel passt
ShellSeatedStrip = Am Tisch „{ $table }“ · { $count } von { $total } besetzt · { $ready } bereit
ShellPlayingStrip = Du spielst an „{ $table }“
ShellReturn = Zurück
ShellLeave = Verlassen
ShellTablesOnline =
    { $count ->
        [one] { $count } Tisch · { $online } online
       *[other] { $count } Tische · { $online } online
    }
ShellTablesWaiting =
    { $count ->
        [one] { $count } Tisch offen
       *[other] { $count } Tische offen
    }
ShellReconnecting = Verbinde neu…
ShellRetryNow = Jetzt versuchen
ShellSessionEnded = Abgemeldet – deine Sitzung ist abgelaufen · Melde dich neu an
ShellSignOut = Abmelden
ShellSwitchGateway = Gateway wechseln
ShellReportProblem = Problem melden
ShellKeyFriends = Freunde
ShellKeyChat = Chat öffnen
ShellChat = Chat
ShellChatPrivacy = Chat & Privatsphäre
ShellBellEmpty = Nichts Neues
ShellBellJoined = { $player } hat sich an deinen Tisch gesetzt
ShellBellLeft = { $player } hat deinen Tisch verlassen
ShellBellFilled = Dein Tisch ist voll
ShellOfflinePill = Offline · lokale Decks
ShellThisClient = Dieser Client
ShellStatsLine = { $online } online · { $waiting } offen · { $playing } laufend
ShellUpdateRequired = Update nötig – dieses Gateway läuft mit einer neueren Version
FrontClosedBeta = geschlossene Beta
FrontCompatible = kompatibel
FrontThisClient = dieser Client { $version }
FrontKeyHint = Dieses Gateway braucht einen Beta-Schlüssel – das Feld öffnet sich unter dem Button, den du wählst.
FrontUsernameHint = Zum Anmelden; niemand sonst sieht ihn
FrontCapsLock = Feststelltaste ist an
FrontUnreachable = { $gateway } ist nicht erreichbar
FrontRetry = Erneut
FrontGuestTitle = Als Gast spielen
FrontAbout = Über
ColophonNotice = Inoffizieller Fan-Content
ColophonScryfall = Kartendaten und -bilder von Scryfall
ColophonSource = Quellcode (AGPL-3.0)
AboutTitle = Über Baylee
AboutLicence = Baylee ist freie Software: Du darfst sie unter der GNU Affero General Public License, Version 3, ausführen, untersuchen, weitergeben und verändern. Wer ein verändertes Gateway betreibt, bietet seinen Spielern den Quellcode dieser Fassung an. Eine veränderte Fassung sagt das und gibt sich nicht als das offizielle Baylee aus (NOTICE).
AboutThirdParty = Lizenzen Dritter
AboutFonts = Alegreya Sans (Juan Pablo del Peral, Huerta Tipográfica) und Faustina (Omnibus-Type): SIL Open Font License 1.1.
AboutIcons = Font Awesome Free 6.7.2 (Fonticons, Inc.): die Schriftdateien unter der SIL Open Font License 1.1.
AboutMana = Mana 1.18 (Andrew Gioia): SIL Open Font License 1.1. Die Symbole, die sie zeichnet, gehören Wizards of the Coast.
AboutCrates = Bevy und die anderen Rust-Crates dieses Builds: jede unter ihrer eigenen Lizenz (MIT, Apache-2.0 und ähnliche), von cargo-deny gegen eine Positivliste geprüft; NOTICE im Quellcode nennt den Rest.
AboutPrecons = Vorgefertigte Deck-Listen von MTGJSON (Zach Halpern): MIT-Lizenz.
SheetClose = Schließen
TermsTitle = Bevor du bei { $gateway } spielst
TermsVersion = Bedingungen dieses Gateways · Version { $version }
TermsUpdated = aktualisiert { $date }
TermsLoading = Die Bedingungen werden geladen…
TermsLoadFailed = Die Bedingungen konnten nicht geladen werden
TermsRetry = Erneut
TermsNotNow = Nicht jetzt
TermsAccept = Annehmen & weiter
TermsScrollToAccept = Bis zum Ende lesen, um anzunehmen
TermsSending = Wird gesendet…
TermsStoredNote = Lies bis zum Ende, um anzunehmen. Deine Zustimmung (Version und Zeit) wird mit deinem Account auf diesem Gateway gespeichert; dieses Gerät merkt sie sich auch.
TermsGuestAsk = Ein Gast, der sich abmeldet, verliert seine Decks. Trotzdem abmelden?
TermsStay = Bleiben
TermsSignOut = Abmelden
TermsDecline = Ablehnen und Account löschen
TermsLater = Später (abmelden)
TermsDeleteHere = Meine Daten auf diesem Gateway löschen
TermsStoredCentral = Lies bis zum Ende, um anzunehmen. Deine Zustimmung (Version, Text und Zeit) wird mit deinem Baylee-Account bei Baylee Central gespeichert, damit ein anderes Gerät nicht erneut fragt.
TermsChatTitle = Chat
TermsChatNote = Wo ein Gateway Chat anbietet, sind Nachrichten an Freunde und an deinen Tisch Ende-zu-Ende-verschlüsselt: Dein Gerät verschlüsselt sie, nur die Geräte der Empfänger entschlüsseln sie, und das Gateway leitet sie weiter, ohne sie lesen zu können. Es sieht nur, wer wem wann wie viel schreibt. Meldest du eine Nachricht, schickt dein Gerät ihren Text an die Moderation dieses Gateways. Computergegner und Zuschauer nehmen am Chat nicht teil.
CentralTermsTitle = Nutzungsbedingungen von Baylee Central
CentralTermsVersion = Dein Baylee-Account · Version { $version }
DeleteGatewayDataHint = Deine Decks, Einstellungen und Freunde auf diesem Gateway werden endgültig gelöscht. Dein Baylee-Account bleibt.
ClockMinutes = { $count } Min.
RunningMinutes = { $count } Min.
RunningHours = { $count } Std. { $minutes } Min.
ClockSeconds = { $count } s
ClockClassic = klassisch
ClockCasual = gemütlich
ClockStandard = normal
ClockBlitz = Blitz
ClockUntimed = ohne Uhr
ClockBullet = Bullet
ClockRapid = rasch
ClockQuick = flott
ClockBrisk = zügig
ClockRelaxed = entspannt
ClockLeisurely = gemächlich
ClockPatient = geduldig
ClockUnhurried = ohne Eile
ClockMarathon = Marathon
ClockBulletHelp = Fünfzehn Sekunden pro Entscheidung: keine Zeit zum Zögern.
ClockRapidHelp = Fünfundvierzig Sekunden pro Entscheidung.
ClockQuickHelp = Eine Minute pro Entscheidung.
ClockBriskHelp = Neunzig Sekunden pro Entscheidung.
ClockRelaxedHelp = Fünf Minuten pro Entscheidung.
ClockLeisurelyHelp = Fünfzehn Minuten pro Entscheidung.
ClockPatientHelp = Zwanzig Minuten pro Entscheidung.
ClockUnhurriedHelp = Eine halbe Stunde pro Entscheidung.
ClockMarathonHelp = Eine Stunde pro Entscheidung, die längste Uhr.
ClockCasualHelp = Zehn Minuten pro Entscheidung.
ClockClassicHelp = Drei Minuten pro Entscheidung; die Voreinstellung.
ClockStandardHelp = Zwei Minuten pro Entscheidung.
ClockBlitzHelp = Dreißig Sekunden pro Entscheidung.
ClockUntimedHelp = Keine Entscheidungsuhr; für Abwesende springt trotzdem jemand ein.
PlayHideAiOnly = Ohne KI-Tische
PlayDuel = Duell
PlaySortNewest = Neueste zuerst
PlaySortOpenSeats = Freie Plätze
PlaySortName = Name
PlayFormatWarning = Dein Deck fürs nächste Spiel ist { $format }; der Gastgeber spielt { $host_format }. Im Raum kannst du ein anderes wählen.
TemplateCommander = Commander
TemplateDuel = Duell 20
TemplateFiveLand = Fünf Länder
TemplateMixed = Gemischt
RulesLife =
    { $amount ->
        [one] { $amount } Lebenspunkt
       *[other] { $amount } Lebenspunkte
    }
RulesMulligans =
    { $count ->
        [one] { $count } freier Mulligan
       *[other] { $count } freie Mulligans
    }
RulesFiveLands = fünf Länder im Spiel
RulesMixed = Decks jedes Formats
PlayHintJoin = auf einer Zeile: mitspielen
PlayPlayingAt = Du spielst an { $table }
PlayNoHost = Dieses Gateway hat gerade keinen Spielserver
PlayPickToStart = Wähle ein Deck zum Start
PlayYourNextGame = Dein nächstes Spiel
PlayChangeDeck = Deck wechseln
PlayRecentGames = Letzte Spiele · diese Sitzung
PlayVsHouse = gegen das Haus
PlayVs = gegen { $opponents }
PlayWon = gewonnen
PlayLost = verloren
PlayDrawn = unentschieden
PlayRematch = Revanche
PlayEditAndRematch = Ändern & Revanche
PlayJoinOff = Mitspielen ist aus, solange du an { $table } sitzt
PlayPlaying = läuft
PlaySeats = { $count }/{ $total } Plätze
PlayAiCount = KI ×{ $count }
PlayPassword = Passwort
SheetName = Name
SheetPlayers = Spieler
SheetTemplate = Vorlage
SheetAdjust = Anpassen
SheetPassword = Passwort · optional
SheetNoPassword = Kein Passwort
SheetClock = Uhr
SheetCancel = Abbrechen
SheetApply = Übernehmen
SheetOpenTable = Tisch öffnen
SheetEditRules = Regeln ändern
SheetDefaultName = Tisch von { $player }
RoomPasswordSet = Passwort gesetzt
RoomCopyInvite = Einladung kopieren
RoomInviteCopied = Kopiert
RoomRulesPlayers = { $format } · { $count } Spieler
RoomTeamsOn = Teams: an
RoomTeamsOff = Teams: aus
RoomSetTeams = Teams festlegen
RoomEmptySeat = – frei –
RoomWaitingFor = noch nicht bereit: { $players }
RoomNotStartable = noch ist nicht jeder Platz bereit
RoomMakeHost = Zum Gastgeber machen
RoomNextTeam = Nächstes Team
RoomLifeOverride = Eigene Lebenspunkte…
RoomStartingPosition = Startposition…
RoomMakeOpen = Zum freien Platz machen
RoomHouseAi = Haus-KI
RoomLanguageModel = Sprachmodell
RoomHost = Gastgeber
RoomFor = für { $player }
RoomAi = KI
RoomDeckWord = Deck
RoomFits = passt
RoomOtherFormat = { $format }-Tisch
RoomForTesting = Für Tests und Rätsel
RoomSeat = Setzen
RoomNoModel = Auf diesem Rechner ist noch kein Sprachmodell eingerichtet. Lege unter Einstellungen › Sprachmodelle ein Profil an: Anbieter und Modell, mit denen ein Platz spielt, und seinen Schlüssel, der im Schlüsselbund dieses Rechners bleibt.
RoomNoBridge = Dieser Installation fehlt die Seat-Bridge (baylee-seat) neben dem Spiel; installiere das neueste Release erneut.
RoomSetUpModel = Modell einrichten…
RoomChairTitle = Platz { $seat } · Sprachmodell
LeaveHostingQuestion = „{ $name }“ verlassen?
LeaveHostingHint = Du leitest diesen Tisch: Er geht an den, der am längsten sitzt, oder schließt, wenn sonst niemand sitzt.
RoomOpenSeats =
    { $count ->
        [one] { $count } Platz ist frei – ein Spieler, das Haus oder ein Modell nimmt ihn
       *[other] { $count } Plätze sind frei – Spieler, das Haus oder Modelle nehmen sie
    }
DecksSortSaved = Speicherdatum
DecksSortName = Name
DecksSortFormat = Format
DecksSortColours = Farben
FormatCommander = Commander
FormatFreeform = Freiform
FormatMixed = Gemischt
SavedJustNow = gerade gespeichert
SavedMinutesAgo = vor { $count } Min. gespeichert
SavedHoursAgo = vor { $count } Std. gespeichert
SavedYesterday = gestern gespeichert
SavedDaysAgo = vor { $count } Tagen gespeichert
DeckMeta = { $count } · SB { $sideboard }
DecksNextGame = nächstes Spiel
DecksUnplayable =
    { $count ->
        [one] { $count } Karte nicht spielbar
       *[other] { $count } Karten nicht spielbar
    }
DecksBlockedOne = { $card } hier gesperrt
DecksBlockedMany = { $count } Karten hier gesperrt
CardBlocked = { $card } ist auf diesem Server gesperrt: { $reason }
CardBlockedUntil = { $card } ist auf diesem Server bis { $until } gesperrt: { $reason }
BuildBlocked = hier gesperrt
DecksSearchHint = Decks suchen…
PlaySortBy = { $order }
DecksSortBy = Nach { $order }
DecksNoMatch = Kein Deck passt zu „{ $query }“
DecksDuplicate = Duplizieren
DecksHistory = Historie…
DecksFavourite = Favorit
DecksUnfavourite = Kein Favorit mehr
DecksEmptyTitle = Dein Regal ist leer – beginne mit einem Hausdeck, baue eins oder importiere eine Liste.
DecksNewTileHint = oder eine Liste importieren – Strg/Cmd+V oder Datei ablegen
DecksNewTileTouch = oder eine Liste importieren ›
HouseAdd = Hinzufügen
HousePreview = Ansehen
ArtCredit = Illus. · { $artist }
HistoryTitle = Historie · { $deck }
HistoryRow = v{ $version }
HistoryCurrentRow = v{ $version } · aktuell
HistoryAgainstCurrent = Gegenüber der aktuellen Version
HistoryIsCurrent = So ist das Deck jetzt
HistoryShowAll = Alle Karten
HistoryRestore = Wiederherstellen
HistoryRestoreCurrent = Das ist die aktuelle Version
HistoryRestored = Version wiederhergestellt
HistoryPrevious = Vorige
HistoryCurrent = Aktuell
HistoryPick = Wählen…
HistoryPicking = Wähle die andere Version in der Liste.
HistoryFirstSave = erster Stand
HistoryToday = heute
HistoryYesterday = gestern
HistoryDeltaCopies = +{ $added } −{ $removed }
HistoryDeltaPrinting = { $count }× Druck
HistoryDeltaFinish = { $count } Foil
HistoryDeltaLanguage = { $count }× Sprache
HistoryDeltaNote = { $count }× Notiz
HistoryTagAdded = hinzugefügt
HistoryTagRemoved = entfernt
HistoryTagCount = Anzahl
HistoryTagPrinting = Druck
HistoryTagFinish = Finish
HistoryTagLanguage = Sprache
HistoryTagNote = Notiz
HistoryNoteChanged = { $card } · Notiz geändert
HistoryHouseTag = Hausdeck · nur lesen
HistoryExport = Exportieren…
HistoryNeedsGateway = Die Historie braucht das Gateway
BuildUntitled = Neues Deck
BuildSave = Speichern
BuildNothingToSave = Nichts zu speichern
BuildUnsavedOne = Ungespeichert · 1 Änderung
BuildUnsavedMany = Ungespeichert · { $count } Änderungen
BuildSaving = Speichere…
BuildSavedNow = Gespeichert · gerade eben
BuildSavedAgo = Gespeichert · vor { $count } Min.
BuildSaved = Gespeichert
BuildNotSaved = Noch nicht gespeichert
BuildSaveFailed = Speichern fehlgeschlagen
BuildRename = Umbenennen
BuildEmptyDeck = Deck leeren…
BuildSettings = Einstellungen
BuildDiscardQuestion = Änderungen verwerfen?
BuildDiscardBody = Dieses Deck hat ungespeicherte Änderungen. Ein Entwurf bleibt auf diesem Gerät, bis du speicherst oder verwirfst.
BuildKeepEditing = Weiter bearbeiten
BuildDiscard = Verwerfen
BuildDraftRestored = Dein ungespeicherter Entwurf ist zurück
BuildSyntax = Syntax
BuildSyntaxTitle = Was die Suche versteht
BuildSyntaxType = Typzeile enthält
BuildSyntaxText = Regeltext enthält
BuildSyntaxColour = mindestens diese Farben
BuildSyntaxIdentity = Farbidentität innerhalb
BuildSyntaxMana = Manabetrag
BuildSyntaxPower = Stärke (auch tou, loy)
BuildSyntaxCost = Manakosten enthalten diese Symbole
BuildSyntaxCommander = kann ein Deck anführen
BuildSyntaxPartial = spielbar mit bekannter Lücke
BuildSyntaxNot = nicht
BuildSyntaxExact = genau dieser Name
BuildSyntaxOr = eines von beiden
BuildFilters = Filter
BuildRailType = Kartentyp
BuildRailMana = Manabetrag
BuildRailShow = Zeigen
BuildAlsoNarrowing = außerdem: { $chips }
BuildChoosePrinting = Druck wählen…
BuildOpenCard = Karte öffnen
CardAddToDeck = Ins Deck
CardApplyPrinting = Druck übernehmen
CardSetCommander = Als Commander
CardIsCommander = Ist Commander
CardFlip = Wenden
CardAnyFinish = jedes Finish
CardEnglishText = Englischer Text
CardMorePrintings = Weitere Drucke brauchen das Gateway
CardHeadPrinting = DRUCK
CardHeadInDeck = IN DIESEM DECK
CardMain = Hauptdeck
CardSide = Sideboard
CardLanguage = Sprache
CardFinish = Finish
BuildAddToMain = Ins Hauptdeck
BuildAddToSide = Ins Sideboard
BuildMoveToSide = Ins Sideboard schieben
BuildMoveToMain = Ins Hauptdeck schieben
BuildRemoveAll = Alle entfernen
BuildHintMove = bewegen
BuildHintAdd = hinzufügen
BuildHintOther = andere Liste
BuildHintSearch = suchen
BuildHintPreview = Karte
BuildCommander = Commander
BuildNoCommander = keiner
BuildChoose = Wählen
BuildPartner = Partner
BuildTabMain = Haupt
BuildTabSide = Sideboard
BuildTabStats = Statistik
BuildByType = Nach Typ
BuildByManaValue = Nach Manabetrag
BuildByColour = Nach Farbe
BuildCollapseAll = Alle zuklappen
BuildExpandAll = Alle aufklappen
BuildManaValue = Manabetrag { $mana_value }
BuildManaValueUp = Manabetrag { $mana_value }+
BuildMulticolour = Mehrfarbig
BuildShakyOne = 1 Karte ist nicht vollständig umgesetzt
BuildShakyMany = { $count } Karten sind nicht vollständig umgesetzt
BuildManaCurve = Manakurve
BuildCurveNote = Nichtland-Karten nach Manabetrag
BuildCurveLit = { $card_type } hervorgehoben
BuildColourPips = Farbsymbole
BuildPipsNote = Anteil der farbigen Symbole im Deck
BuildLandShare = { $count } von { $total }
BuildDrawSeven = Sieben ziehen
BuildDrawNote = jedes Mal neu gemischt · keine Karte verlässt das Deck
BuildStatsEmpty = Füge Karten hinzu, um die Zahlen des Decks zu sehen
BuildLastAdded = Zuletzt hinzugefügt
BuildPanePool = Pool
BuildPaneDeck = Deck
BuildNoCatalog = Dieses Gateway hat keine Kartensuche – der Pool zeigt nur die eingebauten Karten
SectionGraphics = Grafik
SectionAudio = Audio
SectionDisplay = Anzeige & Oberfläche
SectionControls = Steuerung
SectionGameplay = Spielablauf
SectionAccount = Account
SectionNetwork = Netzwerk & Gateway
SectionLanguageModels = Sprachmodelle
SectionUpdates = Updates
SectionPrivacy = Datenschutz & Daten
TagDevice = dieses Gerät
TagAccount = dein Account
TagMachine = dieser Rechner
SettingsSearch = Einstellungen suchen…
SettingsNoMatch = Nichts gefunden
SettingsResetSection = Bereich zurücksetzen
SectionDisplayShort = Anzeige
SectionNetworkShort = Netzwerk
SectionModelsShort = Modelle
SectionPrivacyShort = Datenschutz
SettingsSignInToChange = Zum Ändern anmelden
SettingsAccountRule = Die Zeilen deines Accounts gelten auf jedem Gerät; eine Gerätezeile hebt sie nie an
SettingsSaveLine = Sofort gespeichert · markierte Zeilen folgen deinem Account
SettingsSaveLineOffline = Auf diesem Gerät gespeichert · melde dich an, und die markierten Zeilen folgen deinem Account
RowPreset = Voreinstellung
HelpPreset = Setzt die Gerätezeilen darunter; Angepasst, sobald eine davon abweicht
HelpPresetAuto = Gewählt für { $gpu }
RowDisplayMode = Anzeigemodus
HelpDisplayMode = Eine Änderung fragt nach und wird nach 15 s zurückgenommen
DisplayWindowed = Fenster
DisplayBorderless = Randlos
DisplayFullscreen = Vollbild
KeepDisplayMode = Diese Einstellung behalten? Zurück in { $count } s
KeepIt = Behalten
RevertIt = Zurück
RowMonitor = Bildschirm
HelpMonitor = Auf welchem Bildschirm das Fenster steht
HelpAntiAliasing = Glättet Kartenkanten und Schrift · GPU ↑
AntiAliasingLocked = Auf Handys aus – Kachel-GPUs zahlen am meisten dafür
HelpVSync = Wartet auf den Bildschirm: kein Tearing, ein Bild Verzögerung · Strom ↓
HelpFrameLimit = Begrenzt die Bilder pro Sekunde · weniger ist kühler und leiser
HelpRestLimit = Bilder pro Sekunde am Tisch, wenn zwei Sekunden lang nichts geschieht · Strom ↓↓
HelpBackgroundLimit = Bilder pro Sekunde, während ein anderes Fenster vorn ist · Strom ↓↓
RowShowFrameRate = Bildrate anzeigen
HelpShowFrameRate = Ein kleiner Zähler in einer Ecke; ein Bericht enthält ihn auch
RowBackdrop = Hintergrund
HelpBackdrop = Das Gemälde hinter jedem Bildschirm; Schlicht liest sich am leichtesten und kostet am wenigsten
BackdropPainting = Gemälde
BackdropDimmed = Gedimmt
BackdropPlain = Schlicht
HelpAmbient = Treibendes Licht und Staub; nie über der Atmosphäre deines Accounts · GPU ↑
AmbientStill = Ruhig
AmbientSoft = Sanft
AmbientFull = Voll
HelpAtmosphere = Die Obergrenze für Umgebungsdetails auf jedem Gerät
RowHoldStill = Tisch ruhig halten
HelpHoldStill = Karten springen statt zu gleiten, überall; jede Regel bleibt
HelpSky = Das Licht hinter dem Tisch: folgt deiner Uhr oder bleibt bei Tag oder Nacht
HelpMaster = Jeder Klang, die Musik eingeschlossen
RowMusic = Musik
RowMusicTheme = Musikthema in Partien
HelpMusicTheme = Fünfzehn eigene Suiten für deine Partien. Wechselnd wählt für jede Partie eine andere Suite. Login und Lobby teilen Baylees Erinnerung: Clair de lune und Gymnopédie Nr. 1, mit Flügel, Geige und Cello.
MusicThemeEmber = Glutpfad
MusicThemeGlass = Mondglas
MusicThemeThorn = Dornenkrone
MusicThemeTide = Nebelhafen
MusicThemeVelvet = Samtnacht · Klavier
MusicThemeVelvetShort = Samtnacht
MusicThemeCopper = Kupferwerk · Barock
MusicThemeCopperShort = Kupferwerk
MusicThemeJuniper = Wacholder · Folk
MusicThemeJuniperShort = Wacholder
MusicThemeLagoon = Lagunenlicht · Bossa
MusicThemeLagoonShort = Lagunenlicht
MusicThemeLantern = Laternen · Jazzwalzer
MusicThemeLanternShort = Laternen
MusicThemeNeon = Neonpfad · Synthwave
MusicThemeNeonShort = Neonpfad
MusicThemeCircuit = Pixelsturm · Chiptune
MusicThemeCircuitShort = Pixelsturm
MusicThemeMosaic = Mosaik · Marimba
MusicThemeMosaicShort = Mosaik
MusicThemeOrbit = Umlaufbahn · Ambient
MusicThemeOrbitShort = Umlaufbahn
MusicThemeIron = Eisenpuls · Breakbeat
MusicThemeIronShort = Eisenpuls
MusicThemeStar = Sternfall
RowMusicSamples = Instrumentenklang
HelpMusicSamples = Für die Spielmusik bereitet Studio die CC0-Aufnahmen für 48 kHz auf; Original nutzt die Aufnahmen mit 44,1 kHz. Baylees Erinnerung spielt immer mit ihrem Stereo-Flügel und den Streichern bei 48 kHz.
MusicSamplesStudio = Studio
MusicSamplesOriginal = Original
MusicThemeRotating = Wechselnd
HelpMusic = Musik für Titelbildschirm, Lobby und Spiel
HelpEffects = Die Klänge des Tisches; nie über den Tischklängen deines Accounts
HelpMuteUnfocused = Stille, während ein anderes Fenster vorn ist
RowTableSounds = Tischklänge
HelpTableSounds = Aus, halb oder voll auf jedem Gerät; Aus hat Vorrang vor jedem Regler
HelpLanguage = Die Sprache der Oberfläche und, wo das Gateway sie hat, der Kartentexte
RowTextSize = Textgröße
HelpTextSize = Fünf Stufen für jeden Bildschirm außerhalb des Tisches · Strg/Cmd + = − 0
RowPreviewSize = Größe der Kartenvorschau
HelpPreviewSize = Wie groß eine Karte erscheint, auf die gezeigt wird
RowTextFace = Textansicht bevorzugen
HelpTextFace = Karten als Text statt als Druck zeichnen; die Zusatztaste schaltet um, solange du sie hältst
RowShellKeys = Tastenkürzel
HelpShellKeys = Die Tasten jedes Bildschirms außerhalb des Tisches; eine belegte Taste wird abgelehnt, ein zweiter Druck nimmt sie
RowTableKeys = Tasten am Tisch
HelpTableKeys = Die Tasten, während ein Spiel offen ist
HelpAutomation = Was der Client für dich beantwortet; keine Regel ändert sich
RowAbilityAnswers = Antworten auf Fähigkeiten
HelpAbilityAnswers = Antworten, die der Client weiterhin für dich geben soll
RowHandle = Handle
HelpHandle = Wie andere Spieler dich sehen
CopyHandle = Mein Handle kopieren
RowSignOut = Abmelden
HelpSignOut = Beendet diese Sitzung hier und auf dem Gateway
HelpDeleteAccount = Der Account, seine Decks und seine Einstellungen, für immer
RowGateway = Gateway
HelpGateway = Wo dieser Client spielt und welche Version dort läuft
HelpSwitchGateway = Zurück zur Liste der Gateways
RowDiagnostics = Diagnose
HelpDiagnostics = Der Zustand der Verbindung als Text, zum Kopieren in einen Bericht
CopyAsText = Als Text kopieren
NetworkOffline = Offline · lokale Decks, kein Gateway
RowProfiles = Profile
ProfileOpen = Bearbeiten
ReportAttachments = Anhänge
ReportAttachmentsCount = { $count } von { $total }
ReportSmallLine = { $count } / { $max } Zeichen · # eine Karte · @ ein Spieler
ReportNowhere = Kein Ziel bekannt
ReportWhatIsSent = Was gesendet wird
ReportRefHand = Hand
ReportRefBattlefield = Spielfeld
ReportRefStack = Stapel
ReportRefGraveyard = Friedhof
ReportRefExile = Exil
ReportRefCommand = Kommandozone
ReportRefShown = Gezeigt
ReportRefLibraryTop = Bibliothek oben
ReportRefLogged = In dieser Partie gesehen
ReportRefYours = deine
ReportConfirmCardRefs =
    { $count ->
        [one] { $count } Kartenverweis
       *[other] { $count } Kartenverweise
    }
ReportConfirmPlayerNames =
    { $count ->
        [one] { $count } Spielername
       *[other] { $count } Spielernamen
    }
ReportGoesGateway = Geht mit deiner Sitzung an { $gateway }
RowCheckUpdates = Automatisch prüfen
HelpCheckUpdates = Fragt GitHub beim Start und alle sechs Stunden; GitHub sieht deine IP-Adresse und diese Version. Aus: gar keine Anfrage
RowInstallUpdates = Automatisch installieren
HelpInstallUpdates = Lädt ein signiertes Update und installiert es beim Beenden
RowCheckNow = Jetzt prüfen
HelpCheckNow = Was die letzte Prüfung fand, und warum sich ein Update nicht selbst installiert
RowWhatIsKept = Was gespeichert und gesendet wird
HelpWhatIsKept = Das Gateway speichert deinen Account, deine Decks, Einstellungen und welche Bedingungen du angenommen hast. Von sich aus sendet dieser Client nur Update-Prüfungen an GitHub (Desktop, solange die Prüfung an ist), Absturzberichte nach deiner Zustimmung und während eines Offline-Spiels eine anonyme „spielt noch“-Zählung (außer du schaltest sie unten aus); im Browser fragt er außerdem Scryfall nach Kartenbildern
RowReportConsent = Was ein Bericht mitnehmen darf
HelpReportConsent = Hier angehakt, im Berichtsformular angehakt; ein Bericht zeigt genau, was gesendet wird
RowCrashReports = Absturzberichte
HelpCrashReports = Nach einem Absturz: Bericht senden, jedes Mal fragen oder nie
RowUsageCount = Anonyme Nutzungszahl senden
HelpUsageCount = Meldet beim Offline-Spielen einmal pro Minute, dass ein Spiel läuft: kein Account, keine Geräte-ID, nichts über das Spiel
CrashSend = Senden
CrashAsk = Fragen
CrashNever = Nie
HelpReportProblem = Öffnet das Berichtsformular (F8)
HelpChatPrivacy = Wer dir flüstern darf, Freundschaftsanfragen, Chat-Verlauf, Töne und die Chat-Geräte dieses Accounts
ReportOpen = Problem melden
LanguageModelsElsewhere = Sprachmodelle werden in einem Desktop-Client eingerichtet
KeyFixed = Diese Taste ist fest vergeben und lässt sich nicht belegen
KeyHeldBy = Schon für { $shortcut } belegt – drücke die Taste noch einmal oder nimm sie
KeyTake = Nehmen
NoAbilityAnswers = Noch keine
SliderOff = aus
GroupReplay = Replays
ActReplayTurnBack = Replay: einen Zug zurück
ActReplayTurnForward = Replay: einen Zug weiter
ActReplayStart = Replay: zum Anfang
ActReplayEnd = Replay: zum Ende
ActReplaySlower = Replay: langsamer
ActReplayFaster = Replay: schneller
ActReplayPerspective = Replay: Perspektive wechseln
ToastTitleInfo = Hinweis
ToastTitleAlert = Warnung
ToastTitleTable = Tisch
ToastTitleSocial = Kontakt & Chat
GatewayOnlinePlayers = Spieler online
GatewayInLobby = In der Lobby
GatewayInGames = Im Spiel
GatewayWaitingTables = Offene Tische
GatewayRunningGames = Laufende Spiele
GatewayServerVersion = Server-Version
ShellGatewayInlineStats =
    { $tables ->
        [one] { $online } online ({ $lobby } Lobby · { $playing } im Spiel) · { $tables } Tisch
       *[other] { $online } online ({ $lobby } Lobby · { $playing } im Spiel) · { $tables } Tische
    }
SettingsQuickTitle = Schnelleinstellungen
SettingsAudioSection = Audio
SettingsDisplaySection = Anzeige & Bewegung
SettingsTextSize = Textgröße
HandOrderDraw = Zugfolge
HandOrderType = Typ
HandOrderColor = Farbe
CentralRegister = Noch kein Account? Leg einen auf baylee.app an
CentralSignInFailed = Anmeldung oder Wiederherstellung fehlgeschlagen. Bitte prüfe deine Zugangsdaten und die Verbindung.
CentralSyncFailed = Account-Daten konnten nicht synchronisiert werden. Deine lokalen Änderungen bleiben erhalten. Bitte melde dich erneut an.
CentralSignOut = Vom Baylee-Account abmelden
GatewayHintCentral = Wähle ein Gateway für deine Spiele. Dein Account wird zentral verwaltet.
ShellQuit = Beenden
PickerAllLanguages = Alle Sprachen
PickerForceFinish = Foil / Etched erzwingen
PickerHint = Shift halten: Rückseite · ← → Artwork wechseln
PickerSetSearch = Set · Kürzel oder Name suchen

# Table status and device display switches.
SeatEliminated = Du bist ausgeschieden · Der Tisch spielt weiter
CardViewText = Kartenansicht: Text
CardViewArt = Kartenansicht: Bild

# Baylees lokaler Erinnerungsort und Fotoalbum.
MemorialOpen = Baylees Erinnerungen öffnen
MemorialInvitation = Baylee · Für immer geliebt
MemorialDedication = Diese Welt ist einer kleinen Katze gewidmet, die unendlich geliebt wurde. Schlaf friedlich im Sonnenschein, Baylee. · 26. Oktober 2026
MemorialPrevious = Zurück
MemorialNext = Weiter

# Baylee’s story, approved by her family on 27 October 2026.
MemorialStoryTitle = Für Baylee. Für immer im Sonnenlicht.
MemorialTogether = Bei uns · 17. November 2025 – 26. Oktober 2026
MemorialChapters = Ihre Geschichte
MemorialChapterPrevious = Vorheriges Kapitel
MemorialChapterNext = Nächstes Kapitel
MemorialPhotos = Unsere Erinnerungen
MemorialRead = Zum Lesen scrollen

MemorialChapter1Title = Ein Zuhause für Baylee
MemorialChapter1Body =
    Am 17. November 2025 kam Baylee zu uns nach Hause.

    Meine Frau und ich hatten sie aus dem Tierheim in Krefeld geholt. Ihr Alter wurde auf ungefähr dreizehn Jahre geschätzt. Sie hatte bösartige Mammatumoren, und der Krebs hatte bereits Metastasen in ihrer Leber gebildet. Baylee war eine Palliativkatze. Wir wussten, dass unsere gemeinsame Zeit begrenzt sein würde. Sie sollte bei uns ein Zuhause haben, in dem sie geliebt wurde.

    Wie schnell sie dieses Zuhause für sich beanspruchen würde, zeigte sie uns gleich selbst.

    Schon am ersten Tag schlich sie sich auf meinen Schoß, während ich arbeitete. Und in ihrer ersten Nacht riss sie ein Loch in einen Beutel Katzenfutter, den ich eigentlich für die Katzen meiner Eltern bestellt hatte, und bediente sich daraus. Natürlich hatte sie ihr eigenes Futter bekommen und war am Abend gut gefüttert worden. Aber Baylee hatte offenbar beschlossen, dass dieser Vorrat trotzdem eine nähere Untersuchung verdiente.

    Die kleine Maus hatte einen erstaunlichen Appetit – und unglaublich viel Energie.

MemorialChapter2Title = Vertrauen wächst
MemorialChapter2Body =
    Weil uns auffiel, dass sie sehr viel Wasser trank, gingen wir mit ihr zum Tierarzt. Eine Blutuntersuchung zeigte eine Erkrankung der Schilddrüse, die auch ihren großen Hunger und ihre lebhafte Art erklärte. Sie bekam Schilddrüsenmedikamente und vorübergehend ein Antibiotikum, weil ihre Entzündungswerte erhöht waren.

    Als sie sich plötzlich überall rieb, viel maunzte und ungewöhnlich verschmust und anhänglich wurde, machten wir uns zunächst Sorgen, dass sie eines der neuen Medikamente nicht vertrug. Bei der erneuten Untersuchung stellte sich jedoch heraus: Unsere kleine Mausi war rollig. Sie war also nicht kastriert worden.

    Auch diese Zeit standen wir gemeinsam durch. Baylee vertraute uns immer mehr, und wir schlossen uns gegenseitig immer tiefer ins Herz.

MemorialChapter3Title = Unsere kleine Prinzessin
MemorialChapter3Body =
    Ich erinnere mich noch genau daran, wie sehr sie sich über ihren Kratzbaum freute. Sie setzte sich sofort ganz oben auf den Sitz und schaute zufrieden in die Wohnung. Erstaunlicherweise benutzte sie ausgerechnet diesen Platz danach nie wieder. Warum, blieb ihr Geheimnis.

    Andere Plätze ließ sie dafür kaum aus: meinen Schoß, meine Beine und unser Bett. Dort schlief sie meistens auf mir oder auf meiner Frau. Am liebsten schaffte sie es, auf uns beiden gleichzeitig zu liegen.

    Mit erhobenem Schwanz stolzierte sie durch die Wohnung, als wüsste sie ganz genau, dass sie hier die Prinzessin war. Sie hatte Temperament und konnte deutlich zeigen, wo ihre Grenzen lagen. Sie durfte bei uns ganz sie selbst sein. Auch mit ihren Eigenheiten, ihren Forderungen und ihrem kleinen Dickkopf wurde sie geliebt.

    Noch nie hatte ich zu einem Tier eine so tiefe Beziehung aufgebaut. Baylee wuchs meiner Frau und mir sehr tief ins Herz.

MemorialChapter4Title = Sonnenstunden
MemorialChapter4Body =
    Als es draußen wärmer wurde, entdeckte sie den Balkon für sich. Sie liebte es, die ersten Sonnenstrahlen des Tages in ihrem Fell zu spüren. Bald verbrachte sie dort beinahe den ganzen Tag.

    Besonders die Tauben und Gartenvögel hatten es ihr angetan. Sie beobachtete sie aufmerksam, lag ihnen auf der Lauer und versuchte immer wieder, sie zu erschrecken. Zwischen Sonnenbaden und Vogelbeobachtung hatte unsere kleine Prinzessin einen gut gefüllten Tagesplan.

MemorialChapter5Title = Ihr kleiner Dickkopf
MemorialChapter5Body =
    Währenddessen schritt ihre Krankheit weiter voran.

    Der Tumor entzündete sich, und Baylee leckte die betroffene Stelle immer wieder auf. Beim Tierarzt bekam sie ihren ersten Body, einen roten. Von da an musste sie täglich einen tragen, damit sie die Wunde nicht weiter verletzte.

    An das Tragen gewöhnte sie sich schnell. Das Anziehen konnte sie allerdings bis zum letzten Tag nicht leiden. Selbst mit einer Schale Futter ließ sie sich dann nur bedingt hervorlocken. So groß ihr Appetit auch war: Ihre Meinung zum Body blieb eindeutig.

    Die Wunde wurde schlimmer. Sie eiterte, begann irgendwann zu bluten und roch unangenehm. Gleichzeitig zeigte Baylee uns weiterhin, wie viel Freude sie an ihrem Alltag hatte. Sie spielte, fraß, erkundete ihre Umgebung und kletterte auf ihr Kratzbrett und auf die Schränke.

    Manchmal lag sie stundenlang auf der Bettkante neben uns und wartete darauf, dass wir endlich aufwachten und ihr Frühstück machten. Wenn ihr das zu lange dauerte, kletterte sie kurzerhand über uns hinweg. Unsere Gesichter waren dabei kein Hindernis.

    Beim Essen hatte sie genaue Vorstellungen. Geschmacklich gehörte sie eher zur „Fraktion Tiefkühlpizza“: Hochwertiges Katzenfutter konnte sie nach einer Weile nicht mehr begeistern, und Abwechslung war Pflicht. Zweimal hintereinander dasselbe Futter kam für sie nicht infrage. Huhn mit Leber, alles mit Käse, Thunfisch oder Lachs – damit konnte man ihr eine Freude machen.

MemorialChapter6Title = Mit erhobenem Schwanz
MemorialChapter6Body =
    Der Tumor wurde größer und nässte zunehmend. Baylee hatte sichtbar Schmerzen. Als diese nachließen, war sie wieder unsere kleine Katze, die mit erhobenem Schwanz durch die Räume zog und ihre Menschen für sich beanspruchte.

    Wir sahen, dass ihre Kräfte langsam weniger wurden. Doch ihr Appetit, ihre Neugier, ihre verschmuste Art und ihr lautes Schnurren waren noch da.

MemorialChapter7Title = Abschied nehmen
MemorialChapter7Body =
    Am Freitagabend, dem 23. Oktober 2026, bekam sie Symptome, die zunächst wie eine Erkältung wirkten. Ihre Nase lief, und sie atmete tief und angestrengt, als müsse sie um Luft ringen.

    Am Samstag wirkte sie wieder lebhafter. Die auffällige Atmung und der Schnupfen blieben, aber sie zeigte noch einmal die Energie und Lebensfreude, die wir von ihr kannten.

    Der Sonntagmorgen war schwer. Ihre Augen tränten, sie atmete mühsam und hatte Schmerzen. Ich gab ihr Schmerzmittel. Nach ungefähr einer Stunde ging es ihr wieder besser, doch ihre Atmung blieb verändert. Für uns war klar, dass wir mit ihr zum Tierarzt mussten.

    Am Montagmorgen verschlechterte sich ihr Zustand erneut. Zum Glück hatte meine Frau an diesem Tag frei. Wir riefen in der Praxis an, bekamen einen Termin und fuhren mit Baylee dorthin.

    Die Röntgenaufnahme zeigte, wie weit sich der Krebs inzwischen ausgebreitet hatte. Uns wurde erklärt, dass ihr nur noch ungefähr ein Drittel ihrer Lunge zum Atmen zur Verfügung stand. Sie weiterleben zu lassen hätte bedeutet, dass sie immer weniger Luft bekommen würde.

    Wir mussten Abschied nehmen.

    Am Montag, dem 26. Oktober 2026, gegen zwölf Uhr wurde unsere liebste Baylee eingeschläfert.

MemorialChapter8Title = Für immer im Sonnenlicht
MemorialChapter8Body =
    Sie war knapp ein Jahr bei uns. In dieser Zeit war sie ein Teil unserer Familie geworden und eine sehr enge Freundin. Sie gehörte zu unseren Arbeitstagen, zu unseren Nächten, zum Aufwachen am Morgen. Zu den Sonnenstunden auf dem Balkon und zu den kleinen Gewohnheiten, aus denen ein gemeinsames Leben besteht.

    Jetzt fehlt sie in all diesen Momenten.

    Die Leere ist noch frisch und tief. Uns fehlen ihr Schnurren, ihr Gewicht auf unseren Beinen, ihr erhobener Schwanz und ihre selbstverständliche Erwartung, dass wir uns endlich um ihr Frühstück kümmern.

    Wenn es etwas nach diesem Leben gibt, was auch immer es sein mag, dann wünsche ich mir, dieser Seele eines Tages wiederzubegegnen.

    Bis dahin tragen wir unser kleines Mäuschen für immer im Herzen.

    Deshalb ist dieses Hobbyprojekt Baylee gewidmet. Ihrem Temperament, ihrer Nähe, ihrer Freude an der Sonne und den Vögeln. Und der Liebe, die in unserer gemeinsamen Zeit gewachsen ist.

    Hier darf sie friedlich auf ihrer Wolke schlafen, im warmen Sonnenlicht, umgeben von Blumen, Tauben und Gartenvögeln.

    Ruhe in Frieden, meine liebste Baylee.

MemorialFirstChapter = Du bist am Anfang ihrer Geschichte.
MemorialLastChapter = Du bist am Ende ihrer Geschichte.

MemorialPhotoOpen = Foto groß ansehen
MemorialZoomIn = Vergrößern
MemorialZoomOut = Verkleinern
MemorialFit = Einpassen
MemorialZoomMinimum = Das Foto ist vollständig sichtbar.
MemorialZoomMaximum = Maximale Vergrößerung erreicht.
