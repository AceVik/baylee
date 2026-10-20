### One thing the interface says.

NounArmiesToAmass =
    { $count ->
        [one] Armee zum Aufmarschieren
       *[other] Armeen zum Aufmarschieren
    }
EnterTappedForCounters = Getappt ins Spiel kommen und { $count } { $counter } erhalten?
ChooseEntryOrder = Für welche bleibende Karte möchtest du die Eintrittseffekte zuerst anwenden?
ChooseEntryReplacement = Welchen Ersatzeffekt möchtest du als Nächstes anwenden?
ChooseDredge = Statt zu ziehen eine Karte ausgraben? Wähle sie aus deinem Friedhof oder bestätige ohne Auswahl, um normal zu ziehen.
ChooseDrawReplacement = Welchen Ersatzeffekt möchtest du als Nächstes auf dieses Ziehen anwenden?
SacrificeToEnter = { $count } passende bleibende Karten opfern, damit diese Karte ins Spiel kommt?
ChooseSacrificeCount = Wie viele bleibende Karten möchtest du opfern? ({ $min }–{ $max })
ChooseEnergyPayment = Energie bezahlen ({ $min }–{ $max }); bis zu { $count } getappte bleibende Karten können Marken erhalten
NounTappedForCounters =
    { $count ->
        [one] getappte bleibende Karte für eine Marke
       *[other] getappte bleibende Karten für je eine Marke
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
PreviewTurn = Drehen
PreviewAlternate = Andere Ansicht
PreviewTurnCompact = Drehen
PreviewRotate = Andere Hälfte
PreviewRotateCompact = Hälfte
PreviewAlternateCompact = Ansicht
SpiresCircles = Zwei Farben für dieses Exemplar einkreisen
SpiresNeedsCircles = Cryptic Spires benötigt zwei eingekreiste Farben. Wähle sie in der Druckauswahl.
SpiresOutsideIdentity = Die eingekreisten Farben von Cryptic Spires liegen außerhalb der Farbidentität des Kommandeurs.
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
ImportHowTo = Kopiere eine Deckliste, einen Baylee-, JSON- oder YAML-Export oder einen Moxfield-Export (More → Export → Copy for Moxfield) und füge ihn hier mit Einfügen oder Strg/Cmd+V ein. Das Format wird selbst erkannt.
ImportReadAs = Gelesen als { $format }. Zeilen: { $count } · Karten: { $copies }
ImportReplaces = Der Import ersetzt das Deck im Editor durch ein neues, ungespeichertes Deck. Das gespeicherte Deck bleibt, wie es ist.
ImportMoxfieldLink = Das ist ein Link zu einem Deck auf { $site }, das Baylee nicht direkt lesen kann. Öffne das Deck auf { $site } → More → Export → Copy for Moxfield und füge es dann hier ein.
ImportNotFetched = Decks von { $site } kann diese Version noch nicht abrufen. Exportiere das Deck dort als Text und füge ihn hier ein.
ImportUnknownLink = Baylee kann keine Decks von { $host } lesen. Exportiere das Deck dort als Textliste und füge die Liste hier ein.
ImportTook = Aus { $format } importiert. Zeilen: { $count } · Karten: { $copies }
ImportWaitingForPool = Warte auf den Kartenpool, um die Namen zu prüfen …
ImportUnknownCards = Nicht im Kartenpool dieses Gateways: { $count }. Vor dem Speichern entfernen:
ImportSkipped = Zeilen, die keine Deckzeilen sind: { $count }
ImportSkippedLine = {"  "}Zeile { $line }: { $text }
ImportMaybeNotKept = Vielleicht-Zeilen nicht übernommen, da Baylee keine speichert: { $count }
ImportNotALeader = { $card } kann kein Deck anführen und ist deshalb nicht als Commander markiert.
ImportNamedForYou = Die Datei nennt keinen Decknamen, daher heißt es „{ $deck }“.
ImportDefaultPrinting = Zeilen ohne Druck spielen den Standarddruck: { $count }
ImportDefaultLanguage = Zeilen ohne Sprache sind englisch: { $count }
ImportNoCommander = Die Datei nennt keinen Commander. Für ein Commander-Deck lege ihn im Kartenmenü der Deckliste fest.
AndMore = {"  "}und { $count } weitere
ImportTooLarge = Der Text ist zu groß für ein Deck: höchstens 256 KiB.
ImportTooManyRows = Eine Deckdatei hat höchstens 1000 Zeilen; diese hat { $count }.
ImportNothing = In dem Text steht kein Deck.
ImportUnreadable = Keine Deckzeilen gefunden. Zeile { $line } lautet: { $text }
ImportSyntax = Die Datei ließ sich nicht lesen: { $error }
ImportVersion = Das ist ein Deckdokument der Version { $version }; diese Version liest Version 1. Aktualisiere Baylee, um es zu importieren.
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
LossFinish = Holo-, Glitzer- und Galaxy-Veredelungen, als nicht-foil geschrieben, Zeilen: { $count }
LossCollectorNumber = Sammlernummern ohne Set, Zeilen: { $count }
LossMaybeboard = das Vielleicht-Board, Zeilen: { $count }
ExportCopied = In die Zwischenablage kopiert.
ExportCopyFailed = Die Zwischenablage ließ sich nicht beschreiben. Speichere es stattdessen als Datei.
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
ChooseGateway = Wähle deinen Gateway
GatewayHint = Wo dein Konto, deine Decks und Tische zu Hause sind.
ChooseGatewayFirst = Wähle einen Gateway, um dich anzumelden oder zu registrieren.
GatewayAddress = GATEWAY-ADRESSE
SaveGateway = Gateway speichern
GatewayUrlInvalid = Gib eine http://- oder https://-Adresse ohne Zugangsdaten, Abfrageparameter oder Fragment ein.
LanguageAndSettings = Sprache und Einstellungen
AllSettings = Alle Einstellungen
ForgetGateway = Aus dieser Liste entfernen
ForgetGatewayQuestion = „{ $name }“ aus dieser Liste entfernen?
ForgetGatewayHint = Nur dieses Gerät vergisst ihn. Dein Konto und deine Decks bleiben auf dem Gateway.
GatewayAnswering = antwortet
SourceCode = Quellcode (AGPL-3.0): { $url }
ImpressumAction = Impressum
ImpressumLink = Impressum · { $host }
TileSource = Quellcode (AGPL-3.0)
TileDiscord = Community auf Discord
ScryfallCredit = Kartendaten und -bilder bereitgestellt von Scryfall.
GatewayChecking = Prüfe { $url } …
GatewaySaved = { $url } gespeichert.
GatewayNotFound = Unter { $url } hat kein Baylee-Gateway geantwortet. Prüfe die Adresse oder versuche es erneut, sobald er läuft.
GatewayCheckingShort = wird geprüft …
GatewayVersionUnknown = Version unbekannt
GatewayNotAnswering = antwortet nicht
GatewayProtocolMismatch = Dieser Gateway spricht Protokoll { $gateway_version } und dieser Client { $client_version }, deshalb öffnen sich seine Partien hier nicht.
GatewayViewMismatch = Die Partien dieses Gateways senden Ansichtsversion { $gateway_version } und dieser Client liest { $client_version }, deshalb öffnen sie sich hier nicht.
GatewayOlder = Dieser Gateway ist älter als die Versionsprüfung. Ob sich seine Partien hier öffnen, zeigt erst eine Partie.
GatewaySilent = Dieser Gateway antwortet gerade nicht.
AccountNameHint = Andere sehen ihn als Name#Tag. 3–16 Zeichen: A–Z, 0–9, _ oder -, an beiden Enden ein Buchstabe oder eine Zahl.
AccountPasswordHint = Mindestens 8 Zeichen, kein häufiges Passwort und keiner deiner beiden Namen.
AccountPasswordInvalid = Wähle ein Passwort mit 8–256 Zeichen, verschieden von Benutzer- und Anzeigename und kein häufiges Passwort.
LibraryReadFailed = Die Bibliotheksantwort konnte nicht gelesen werden. Bitte erneut versuchen.
WelcomeTitle = Dein nächstes Spiel beginnt hier.
WelcomeNote = Baue dein Deck. Finde deinen Tisch. Spiele deinen Stil.
OfflineBenefit = Spiele ohne Konto. Offline-Decks bleiben auf diesem Gerät.
LobbyGuide = Wähle dein Deck und tritt einem Tisch bei oder eröffne einen.
DeckHistory = Historie
HistoryHint = Jeder Stand bleibt erhalten. Wiederherstellen erzeugt eine neue Version; spätere Stände bleiben verfügbar.
WorkingDiff = Änderungen gegenüber der zuletzt gespeicherten Version
GrantedActions = Gewährte Aktionen
GrantedLifeCost = { $amount } Leben bezahlen
GrantedRecipient = Empfänger: { $recipient }
GrantedCancelLifeLoss = Diesen bevorstehenden Verlust von { $amount } Leben vor deinem nächsten Ziehsegment aufheben
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
LibraryLoading = Bibliothek wird geladen…
LibraryRetry = Erneut versuchen
LibraryBack = Zurück
LibraryMain = Hauptdeck
LibrarySide = Sideboard
LibraryCommanders = Kommandeure
NoChanges = Keine Änderungen in diesem Bereich.
LibraryCounts = { $count } Karten · { $sideboard } Sideboard
LobbyCounts = { $count } Decks · { $tables } passende Tische
Composition = Deckübersicht
UniqueCards = Verschiedene Karten
AverageMana = Ø Mana · ohne Länder
LandShare = Länderanteil
OpeningLand = Land in den ersten sieben
ConfirmDeleteDeck = Endgültig löschen?
AppName = Baylee
Username = BENUTZERNAME
DisplayName = ANZEIGENAME
Password = PASSWORT
PasswordAgain = PASSWORT WIEDERHOLEN
Continue = Weiter
SignIn = Anmelden
CreateAccount = Registrieren
WantAnAccount = Konto erstellen
HaveAnAccount = Ich habe schon ein Konto
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
QualityCustom = Eigene
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
UpdateWhyDev = Das ist ein Entwicklungs-Build: Er sucht, installiert aber nie selbst.
UpdateWhyFolder = Baylee darf seinen eigenen Ordner nicht beschreiben und sich darum nicht ersetzen.
UpdateWhyMoveApp = macOS startet Baylee aus einer schreibgeschützten Kopie. Verschiebe es nach „Programme“, dann kann es sich selbst aktualisieren.
UpdateWhyReadOnly = Baylee installiert hier keine Updates: Es darf { $folder } nicht beschreiben ({ $error }). Verschiebe es in deinen Programme-Ordner, dann kann es das.
UpdateTranslocated = macOS startet Baylee aus einer schreibgeschützten Kopie, weil es noch dort liegt, wo es entpackt wurde. Updates installieren sich trotzdem; nach „Programme“ verschoben, startet es ohne diesen Umweg.
UpdateMoveHome = In meinen Programme-Ordner verschieben
UpdateMoveSystem = Für alle nach „Programme“ verschieben
UpdateMoveWhat = Kopiert Baylee dorthin, startet die Kopie und schließt diese. Die alte Kopie wird nur gelöscht, wenn du es sagst.
UpdateMoveFailed = Baylee ließ sich nicht verschieben: { $error }
UpdateMoved = Baylee läuft jetzt aus { $path }.
UpdateOldCopy = Die alte Kopie liegt noch unter { $path }.
UpdateTrashOld = Alte Kopie in den Papierkorb
UpdateKeepOld = Behalten
UpdateTrashFailed = Sie ließ sich nicht in den Papierkorb legen ({ $error }); zieh sie selbst hinein, wenn du magst.
UpdateWhyNotVerified = Der Download ließ sich nicht als unserer bestätigen und wurde nicht installiert.
UpdateWhyOther = Dieses Update lässt sich nicht automatisch installieren.
ReleaseNotes = Versionshinweise
UpdateHide = Ausblenden
UpdateRestartOffer = Update bereit – jetzt neu starten und genau hier weitermachen.
UpdateRestartNow = Jetzt neu starten
UpdateLater = Später
UpdateRestartFailed = Neustart nicht möglich: { $error }
ResumeFailed = Konnte nicht dort weitermachen, wo du warst – normaler Start.
ResumeGameDiffers = Die Partie ließ sich nicht genau so wiederherstellen und wurde nicht fortgesetzt.
ResumeHousePlayed = Zurück am Tisch – das Haus hat während des Neustarts für dich gespielt.
ResumeSeatHeld = Zurück am Tisch – dein Platz wurde während des Neustarts gehalten.
UpdateAutoCheck = Automatisch nach Updates suchen
UpdateAutoCheckWhy = Fragt GitHub beim Start und alle sechs Stunden. GitHub sieht dabei deine IP-Adresse und diese Version. Aus: gar keine Anfrage.
UpdateAutoInstall = Automatisch aktualisieren
UpdateAutoInstallWhy = Lädt ein signiertes Update herunter und installiert es beim Beenden.
UpdateCheckNow = Nach Updates suchen
UpdateChecking = Suche läuft…
UpdateUpToDate = Baylee ist aktuell.
UpdateCheckFailed = Die Suche nach Updates hat nicht geklappt.
SignOut = Abmelden
DeleteAccount = Konto löschen
DeleteAccountQuestion = Das Konto { $account } löschen?
DeleteAccountHint = Das Konto, seine Decks und seine Einstellungen verschwinden endgültig von diesem Gateway, und jedes angemeldete Gerät wird abgemeldet. Gib zur Bestätigung dein Passwort ein.
DeleteGuestHint = Der Gast, seine Decks und seine Einstellungen verschwinden endgültig von diesem Gateway.
DeleteAccountConfirm = Endgültig löschen
DeleteAccountNeedsPassword = bitte dein Passwort
AccountDeleted = Konto gelöscht
YourDecks = Deine Decks
NewDeck = Neues Deck
AddStarterDeck = Starterdeck hinzufügen
NoDecksYet = noch keine Decks — füge das Starterdeck hinzu
Edit = Bearbeiten
Delete = Löschen
Tables = Tische
Search = SUCHE
SearchTables = Tisch oder Gastgeber …
SearchCards = Name, Typ, Text …
DoSearch = Suchen
Refresh = Neu laden
PlayTheHouse = Gegen das Haus
SeatHouse = Haus-KI
RoomPassword = RAUM-PASSWORT
OpenATableFor = Tisch eröffnen für
NoTablesOpen = Noch sind alle Tische frei. Eröffne deinen und lade jemanden zum Spielen ein.
OfflineReadyToPlay = Dein Deck ist bereit. Spiele direkt gegen die Haus-KI oder erstelle einen Tisch mit eigener Sitz- und Teamaufteilung.
GatewayUnreachable = Das Gateway antwortet nicht. Neuer Versuch läuft…
GatewayBack = Das Gateway ist wieder da.
NoNewGames = Gerade können keine neuen Spiele beginnen — vielleicht wird der Server aktualisiert. Laufende Spiele gehen weiter.
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
RoomLlmNoEfforts = Dieses Modell nimmt keinen Aufwand, den dieser Build benennen kann.
RoomLlmDeck = Deck, das es mitbringt
RoomLlmSave = Ins Profil übernehmen
RoomLlmRemove = Modell entfernen
RoomLlmStarting = Seine Brücke nimmt den Stuhl, sobald der Raum ihn als frei führt.
HostedEntry = Gateway-Modell
HostedTitle = Gateway-Modell für Platz { $seat }
HostedIntro = Ein Modell, das das Gateway betreibt, spielt diesen Stuhl. Wähle eines:
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
GameLlmTitle = Sprachmodell-Stühle (Debug: ab der nächsten Entscheidung)
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
Locked = abgeschlossen
StateWaiting = wartet
StatePlaying = läuft
StateOver = beendet
TableOpenWaiting = dein Tisch { $table } ist offen — warte auf Gegner
TableOpenHouseWaiting = dein Tisch ist offen — richte die Plätze ein und starte
TakingYourSeat = nehme deinen Platz ein…
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
YourUsernameIs = angemeldet — dein Benutzername ist { $username }; die Adresse geht noch bis Ende 2026
PlayAsGuest = Als Gast spielen
ContinueAsGuest = Weiter als { $handle }
GuestName = GASTNAME
GuestDefaultName = Guest
JoiningAsGuest = trete als Gast bei …
PlayingAsGuest = du spielst als Gast
GuestNotice = Du spielst als Gast. Dieses Konto und seine Decks werden etwa 30 Tage nach deinem letzten Besuch gelöscht.
GuestEnded = dieses Gastkonto ist abgelaufen — spiel als neuer Gast oder melde dich an
NoGuests = dieses Gateway nimmt keine Gäste auf
InviteKey = BETA-SCHLÜSSEL
InviteKeyShape = BAYLEE-XXXX-XXXX-XXXX-XXXX
InviteKeyHint = Dieses Gateway ist eine geschlossene Beta: Ein neues Konto oder ein neuer Gast braucht einen Schlüssel. Füg ihn einfach so ein, wie du ihn bekommen hast.
NeedInviteKey = bitte deinen Beta-Schlüssel
InviteKeyInvalid = dieser Beta-Schlüssel ist nicht gültig — prüf ihn oder frag nach einem neuen
InviteKeyNeeded = dieses Gateway ist eine geschlossene Beta: ein neues Konto oder ein neuer Gast braucht einen Beta-Schlüssel
GuestSignOutQuestion = { $name } abmelden?
GuestSignOutHint = Ein Gast kann sich nicht wieder anmelden. Abmelden löscht diesen Gast und seine Decks sofort.
NeedDisplayName = bitte einen Anzeigenamen
PasswordsDiffer = die beiden Passwörter sind verschieden
BannedUntil = Dieses Konto ist bis { $until } gesperrt. Grund: { $reason }
BannedForGood = Dieses Konto ist gesperrt. Grund: { $reason }
ChooseNewPassword = Neues Passwort wählen
PasswordWasReset = Ein Admin hat dein Passwort zurückgesetzt. Wähle ein neues, um dich anzumelden.
NewPassword = NEUES PASSWORT
SetNewPassword = Passwort setzen und anmelden
NewPasswordIsTheOld = wähle ein anderes Passwort als das, das du bekommen hast
SettingPassword = dein Passwort wird gesetzt…
NoSignUps = dieses Gateway nimmt keine neuen Konten an
CreatingAccount = erstelle das Konto…
SigningIn = melde an…
AccountCreated = Konto erstellt — melde an…
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
TableOpen = Tisch offen — warte auf Gegner
TableOpenHouse = Tisch offen — richte die Plätze ein und starte
OpponentSatDown = ein Gegner hat sich gesetzt
YouAreSeated = du sitzt — geh auf Bereit, dann startet der Host
TakingTheSeat = nehme den Platz ein…
GameEnded = das Spiel ist vorbei
Unreadable = konnte { $what } vom Gateway nicht lesen
TheGameList = die Tischliste
TheDeckList = die Deckliste
TheSignIn = die Anmeldung
TheSeat = den Platz
ThePool = den Kartenpool
ThePrintings = die Drucke
TheDeck = das Deck
SettingsTitle = Einstellungen
Back = Zurück
SettingsOnAccount = In deinem Konto gespeichert — sie begleiten dich an jeden Tisch.
SettingsOnDevice = Auf diesem Rechner gespeichert. Melde dich an, und sie folgen deinem Konto.
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
RailPresetQuietStepsDetail = Nicht Enttappen, Versorgung, Ziehen, Schaden oder Aufräumen — dort wird nichts entschieden.
RailPresetCompetitive = Turnier-Stopps
RailPresetCompetitiveDetail = Beide eigenen Hauptphasen, beide Kampfansagen und ihr Endsegment.
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
SoundWhy = Kurze Töne für Leben, deinen Zug und das Spielende.
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
ActHoldForStack = Stack auflösen lassen
ActHoldForTurn = Nichts mehr in diesem Zug
ActMulliganKeep = Diese Hand behalten
ActMulliganTake = Mulligan
ActAnswerYes = Ja
AnswerYesBatch = Ja für alle { $count } gleichen Fähigkeiten
NoMaxHandSize = Kein Handkartenlimit
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
RowVisitCamera = Sitz ansehen
HelpVisitCamera = Wo die Kamera steht, wenn du einen anderen Platz ansiehst (sein Knopf, F). Automatisch: hinter einem Mitspieler, als säßest du neben ihm, und gegenüber einem Gegner, dessen Brett du dann wie im Duell liest. Oder immer dahinter mit deiner Reihe oder dem Zifferblatt im Bild, dahinter immer mit dem Zifferblatt, oder gegenüber.
VisitAuto = Automatisch
VisitBehind = Dahinter
VisitBehindDial = Dahinter, Uhr
VisitAcross = Gegenüber
PriorityCueOn = Prioritätston: an
PriorityCueOff = Prioritätston: aus
RowPriorityCue = Prioritätston
HelpPriorityCue = Ein weicher Schlag, wenn der Tisch auf dich zu warten beginnt — nur auf deinem eigenen Gerät, nie für die anderen. Er hat einen eigenen Schalter; der Musik-Schalter schaltet ihn nie stumm.
ActFocusNextSeat = Sitz ansehen
ActFocusPrevSeat = Vorigen Sitz ansehen
ActFocusHome = Zurück zu meinem Platz
ActArrangementMenu = Anordnung wählen
ActNextArrangement = Nächste Anordnung
ActHandDrawer = Hand auf- oder zuklappen (Telefon)
TypesInDeck = Am häufigsten in deinem Deck
TypesAll = Alle Typen
CastFirstQuestion = { $spell } wirken: { $question } — bezahlt wird danach
CancelCast = Abbrechen
ActFoldDecision = Entscheidung ein- oder ausklappen
ArrRing = Ring
ArrRingBlurb = Alle Sitze am Ring; die Kamera besucht.
ArrUprightRing = Aufrechter Ring
ArrUprightRingShort = Aufrecht
ArrUprightRingBlurb = Der Ring, jedes Brett aufrecht zu dir; ein Besuch zoomt heran.
ArrTurntable = Drehteller
ArrTurntableBlurb = Du und ein Gegner wie im Duell, die anderen klein an den Seiten; einen Sitz wählen dreht den Tisch.
ArrArcRail = Bogenschiene
ArrArcRailBlurb = Die Gegner auf einem Bogen über dir; die Kamera fährt die Schiene entlang.
ArrPods = Raster
ArrPodsBlurb = Jeder Gegner aufrecht in einem Raster, von oben gesehen; ein Besuch zoomt heran.
ArrSpotlight = Rampenlicht
ArrSpotlightBlurb = Ein Duell gegen einen Sitz, die anderen als Chips in der Leiste; mit „Tisch folgt dem Zug“ folgt es dem Spiel.
ArrTurntableRows = Drehteller+
ArrTurntableRowsBlurb = Drehteller bis vier Sitze, Rampenlicht ab fünf oder auf dem Handy.
ArrResolvedTo = Hier: { $arrangement }
ArrFocusRing = Fokusring
ArrFocusRingBlurb = Ein Duell in der Mitte, die anderen Sitze als Spalten an den Rändern.
ArrNotInADuel = Im Duell gibt es nur eine Anordnung
ArrComing = Kommt ({ $package })
ArrNotHereSeats = Nicht bei so vielen Sitzen in einem Fenster dieser Größe
ArrNotHereWindow = Nicht in einem so kleinen Fenster
ArrNotOnAPhone = Nicht auf dem Handy
ArrExperimental = experimentell
ArrRememberForSeats = Für { $count } Sitze merken
ArrLettersNote = Die Buchstaben stehen für Bildzeichen, die noch gezeichnet werden.
ArrGameMenuRow = Anordnung › { $arrangement }
RowArrangement = Tischanordnung
HelpArrangement = Wie die Sitze an einem Tisch mit drei oder mehr Plätzen stehen. P wählt am Tisch eine, Umschalt+P die nächste; dies ist der Standard.
RowFollowTurn = Tisch folgt dem Zug
HelpFollowTurn = Beginnt der Zug eines anderen Spielers, zeigt der Tisch dessen Sitz — eine Kamera-Anordnung besucht ihn, die anderen holen ihn herüber — und in deinem Zug kehrt er zu deinem zurück. Er wartet, während du eine Frage beantwortest; ein Sitz, den du selbst ansiehst, bleibt bis zum nächsten Zugbeginn.
RowArrangementBySeats = Je Sitzzahl
HelpArrangementBySeats = Eine für eine Sitzzahl gemerkte Anordnung gilt an einem Tisch dieser Größe statt des Standards.
ArrSeats = { $count } Sitze
ArrAsDefault = Standard
ArrKeysHint = P wählt die Anordnung · Umschalt+P die nächste
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
AutoPassDetail = Kein Land, kein Zauber, keine Fähigkeit, nichts zu suspendieren: ohne Nachfrage abgeben.
AutoSkipTurnsLabel = Durch gegnerische Züge abgeben
AutoSkipTurnsDetail = Nur Priorität. Ein Blocken lehnt es nie für dich ab.
AutoSkipAttacksLabel = Leeren Angriffsschritt überspringen
AutoSkipAttacksDetail = Nur wenn nichts, das du kontrollierst, angreifen kann.
AutoSkipBlocksLabel = Leeren Blockschritt überspringen
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
RailEndStep = Endschritt
RailCleanup = Aufräumen
MutateOnTop = Obenauf legen
MutateUnderneath = Darunter legen
ChooseMutatePosition = Position beim Mutieren wählen
ChooseDungeon = Dungeon wählen
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
RoomTitle = Dein Spielraum
RoomName = Raumname
RoomRules = Regeln & Startaufstellung
RoomHostHelp = Du konfigurierst den Tisch. Jeder Spieler wählt sein eigenes Deck und bestätigt seine Bereitschaft.
RoomOfflineHelp = Wähle Decks und KI-Stärke. Passe bei Bedarf die Startregeln an und starte das Spiel.
RoomGuestHelp = Der Host konfiguriert den Tisch. Wähle unten dein Deck und bestätige anschließend mit Bereit.
RoomPlayers = Spieler & Sitze
RoomLife = Start-Lebenspunkte
RoomMulligans = Freie Mulligans
RoomBoard = Start-Permanents suchen
RoomStartingCards = Startkarten · { $count }
RoomApply = Einstellungen übernehmen
RoomLockHelp = Leer lassen, um das bisherige Kennwort beizubehalten. Änderungen gelten nach dem Übernehmen.
RoomUnlock = Kennwort entfernen
RoomTemplate = Startvorlage
RoomCardAppearance = Edition & Foil
RoomCardCounters = Startcounter
RoomCounterKind = Counter-Typ
RoomCounterHint = +1/+1, -1/-1, loyalty, lore, time, charge, poison, energy, rad, lifelink, level, flying, shadow, indestructible, hone; beliebige +X/+Y- oder -X/-Y-Paare; custom:ID
RoomAddCounter = Counter hinzufügen
RoomRemoveCard = Karte entfernen
RoomNoCardMatches = Kein passendes Permanent gefunden.
RoomFast = Start mit fünf Ländern
RoomPlanechase = Planechase · noch nicht verfügbar: Weltenkarten und Weltenwürfel sind noch nicht implementiert.
RoomSuccession = Verlässt der Host den Raum, übernimmt der am längsten anwesende Spieler.
RoomPickDeck = Deck auswählen
RoomDefault = Standard verwenden
RoomNoDeck = Noch kein Deck gewählt
PaneDeck = Deck ({ $count } / { $sideboard })
LeaveWithoutSaving = Ohne Speichern verlassen
BackToDecks = ‹ Decks
EditingADeck = Deck bearbeiten
ANewDeck = Ein neues Deck
SaveDeck = Deck speichern
DeckIsSaved = Gespeichert
HideFilters = Filter ausblenden
ShowFilters = Filter
ClearFilters = Zurücksetzen
PlayableOnly = Nur spielbare
PoolTally = { $count } von { $total } Karten{ $suffix }
PoolNarrow = {" "}— { $count } gezeigt, tippe weiter zum Eingrenzen
NothingMatches = nichts gefunden — versuche weniger Filter
NoPrintings = keine Drucke
LookingForPrintings = suche weitere Drucke…
PrintingAt = { $position } von { $total }
NoCatalogOnlyThis = Weitere Drucke konnten nicht geladen werden. Referenzdruck wird angezeigt.
ReferencePrinting = Referenzdruck
AllSets = Alle
FinishPlain = Normal
FinishFoil = Folie
FinishEtched = Geätzt
AddPrinting = Hinzufügen
CountInZone = { $count } { $zone }
ZoneDeck = im Deck
ZoneSideboard = im Sideboard
NoArtForPrinting = kein Bild für diesen Druck
NoRulesText = kein Regeltext — dieses Gateway hat keinen Kartenkatalog
NotAsPrinted = { $coverage } — diese Karte spielt nicht wie gedruckt
AddToDeck = + Deck
AddToSideboard = + Sideboard
MoveToSideboard = → Sideboard
MoveToDeck = → Deck
RemoveCard = entfernen
IsCommander = Kommandeur ✓
SetCommander = als Kommandeur
HeldInDeck = { $count } im Deck
HeldInSideboard = { $count } im Sideboard
HeldInBoth = { $count } im Deck, { $sideboard } im Sideboard
CoverageGenerated = ungetestet
CoveragePartial = teilweise
CoverageStub = Rumpf
DeckNameLabel = DECKNAME
TabMain = Deck { $count }
TabSide = Sideboard { $count }
DeckMakeup = { $lands } Länder · { $creatures } Kreaturen · { $spells } andere Zauber
DeckEmptyHint = leer — tippe links auf eine Karte, um sie hinzuzufügen
EmptyTheDeck = Deck leeren
DroppedCards = entfallen: { $card }
DeckNeedsName = Das Deck braucht einen Namen.
DeckNameTooLong = Der Name ist zu lang (höchstens 64 Zeichen).
DeckIsEmpty = Das Deck ist leer.
TooManyLines = Höchstens { $count } verschiedene Karten je Liste.
TooManyCards = Höchstens { $count } Karten je Liste.
CardGoneFromPool = { $card } ist nicht mehr im Kartenpool.
CommanderNoLongerEligible = { $card } kann kein Kommandeur mehr sein — dieses Deck wird ohne einen gespeichert.
DeckTooSmall = { $count } Karten — ein Constructed-Deck will mindestens { $min }.
SideboardTooBig = Ein Sideboard hat üblicherweise höchstens { $count } Karten.
ThinOnLands = { $count } Länder auf { $total } Karten sind dünn für diese Kurve.
ShakyCards =
    { $count ->
        [one] { $count } Karte ist noch nicht vollständig umgesetzt und spielt nicht wie gedruckt.
       *[other] { $count } Karten sind noch nicht vollständig umgesetzt und spielen nicht wie gedruckt.
    }
CardUnderneath = DARUNTER
CardAttached = ANGELEGT
WaitingForPlayer = Warte auf { $player }
WaitingForPlayers = Warte auf { $count } Spieler
WaitingForHeldSeat = Warte auf das Haus — { $player } ist abwesend
PayOrPass = Du schuldest Mana. Klicke Länder zum Vormerken oder wähle „Jetzt zahlen“; getappt wird erst beim Bezahlen.
PayStage = Jetzt zahlen ({ $count } tappen)
PayGo = Bezahlen
UnstagePayment = Alle zurücknehmen
DeclinePayment = Nicht zahlen
DeclinePaymentCast = Nicht zahlen — der Zauber geht zurück
DeclinePact = Nicht zahlen — du verlierst das Spiel
PaymentShort = Nicht genug Mana zum Bezahlen. Merke weitere Länder vor oder wähle „Nicht zahlen“.
AiSaidHead = { $player } denkt
AiSaidThinking = Überlegung: { $thought }
AiSaidChose = Wahl: { $choice }
AiSaidCost = { $count } Tokens, { $seconds } s
MaskChoose = Wähle eine Kreatur zum verdeckten Wirken (X = { $x }), oder lehne ab
MaskReceipt = Tatsächliche Manazahlung: { $mana }; feste Aktivierungskosten: { $fixed_cost }
CommandChoose = Wähle die Karte, die dieser Spieler nach Möglichkeit spielen muss
ConfirmFaceDown = Verdeckt wirken
DeclineCast = Nicht wirken
ChooseTextReplacement = Wähle das bisherige und das neue Wort
TextReplacementSummary = Wortänderung: { $from } → { $to }
TextOldWord = Bisheriges Wort
TextNewWord = Neues Wort
ChooseRequiredMana = Wähle eine Manafähigkeit eines Landes
RequiredManaProgress = Verpflichtend — { $count } Aktivierungen ausgeführt
DecidingFor = Du entscheidest für { $player }
DamageTurnFaceUp = Vor dem Schaden aufdecken
GrantedPaymentHint = Mana erzeugen oder passen.
PrepareManaPayment = Erzeuge bei Bedarf Mana gegen bis zu { $amount } Schaden. Passe zur Wahl des Betrags.
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
DamageRedirectNext = Schaden umleiten → { $recipient }; noch { $remaining }
DamageCounter = Entferne eine { $counter }-Marke und versuche, 1 Schaden zu verhindern; Marken übrig: { $remaining }
DamageSourceShield = Nutze das Schild gegen { $source }; { $amount } Schaden bleibt unverhindert
DamageShieldLife = ; { $player } erhält Lebenspunkte in Höhe des verhinderten Schadens
PreventionShare = { $part } — verhindere { $count } von höchstens { $max }
RedirectionShare = { $part } — leite { $count } von höchstens { $max } um
DamageRule = Regeleffekt
DamageUnpreventable = ; Schaden kann nicht verhindert werden
AllocateDamageCounters = Verteile { $count } Markenentfernungen — { $remaining } übrig. Wähle eine Quelle und gib ihren Anteil ein.
DamageCounterShare = { $part } — { $count } Markenentfernungen anwenden (höchstens { $max })
ChooseDamageEffect = Wähle, welcher Effekt den Schaden als Nächstes verändert
AllocatePrevention = Verteile { $count } Schadensverhinderung — { $remaining } übrig. Wähle eine Quelle und gib ihren Anteil ein.
AllocateRedirection = { $count } Schaden umleiten — { $remaining } übrig. Wähle eine Quelle und gib ihren Anteil ein.
OptionalPayment = Freiwillige Zahlung
SacrificeForPlayer = { $player } opfert die gewählten bleibenden Karten. { $selection }
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
DeclareBlockers = Blocker deklarieren
BlockDraftHint = Angreifer wählen, dann die Kreaturen, die ihn blocken. Erneut wählen nimmt einen Blocker zurück. Zum Schluss bestätigen.
BlockAim = Angreifer: { $attacker } · { $count } blocken
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
        [one] Karte, die aufs Spielfeld kommt
       *[other] Karten, die aufs Spielfeld kommen
    }
NounCardsToGraveyard =
    { $count ->
        [one] Karte, die in den Friedhof ihres Besitzers kommt
       *[other] Karten, die in den Friedhof ihres Besitzers kommen
    }
InspectLibrary = Sieh dir die Bibliothekskarten an und bestätige ohne Auswahl.
BeholdInstructions = Wähle eine passende bleibende Karte unter deiner Kontrolle oder zeige eine passende Handkarte vor. Bestätige ohne Auswahl, um abzulehnen.
InspectHand = Sieh dir die Hand an und bestätige, wenn du fertig bist
NounCardsForFirstPile =
    { $count ->
        [one] Karte für den ersten Stapel (der Rest ist der zweite)
       *[other] Karten für den ersten Stapel (der Rest ist der zweite)
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
        [one] bleibende Karte, die geopfert wird
       *[other] bleibende Karten, die geopfert werden
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
        [one] bleibende Karte, die du behältst
       *[other] bleibende Karten, die du behältst
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
        [one] ungetappte bleibende Karte, die getappt wird
       *[other] ungetappte bleibende Karten, die getappt werden
    }
NounPermanentsToReturn =
    { $count ->
        [one] bleibende Karte, die auf die Hand ihres Besitzers zurückgenommen wird
       *[other] bleibende Karten, die auf die Hand ihres Besitzers zurückgenommen werden
    }
NounCardsToExile =
    { $count ->
        [one] Karte aus deinem Friedhof, die ins Exil geschickt wird
       *[other] Karten aus deinem Friedhof, die ins Exil geschickt werden
    }
NounPermanentsToLeaveTapped =
    { $count ->
        [one] bleibende Karte, die getappt bleibt
       *[other] bleibende Karten, die getappt bleiben
    }
NounNewHostsFor =
    { $count ->
        [one] Objekt, das { $aura } verzaubert
       *[other] Objekte, die { $aura } verzaubert
    }
NounTheAura = die Aura
NounPermanentsToCopy =
    { $count ->
        [one] bleibende Karte, die kopiert wird
       *[other] bleibende Karten, die kopiert werden
    }
NounPermanentsToUntap =
    { $count ->
        [one] bleibende Karte, die enttappt wird
       *[other] bleibende Karten, die enttappt werden
    }
NounDiscardsToReveal =
    { $count ->
        [one] abgeworfene Karte, die Nephalia Academy aufdeckt (andere bleiben verdeckt)
       *[other] abgeworfene Karten, die Nephalia Academy aufdeckt (andere bleiben verdeckt)
    }
NounCardsToReveal =
    { $count ->
        [one] Karte, die aufgedeckt wird
       *[other] Karten, die aufgedeckt werden
    }
NounAttackersToBand =
    { $count ->
        [one] Angreifer, der sich der Gruppe anschließt
       *[other] Angreifer, die sich der Gruppe anschließen
    }
NounCreaturesForPile =
    { $count ->
        [one] Kreatur für diesen Stapel
       *[other] Kreaturen für diesen Stapel
    }
NounCreaturesForPileOf =
    { $count ->
        [one] Kreatur für Stapel { $pile } von { $total } (jeder Stapel blockt einen zufälligen Angreifer)
       *[other] Kreaturen für Stapel { $pile } von { $total } (jeder Stapel blockt einen zufälligen Angreifer)
    }
NounCreaturesForLeft =
    { $count ->
        [one] Kreatur für den "linken" Stapel (der Rest geht nach rechts)
       *[other] Kreaturen für den "linken" Stapel (der Rest geht nach rechts)
    }
ChooseRiverLabel = Wähle "links" (zuerst) oder "rechts": nur dieser Stapel und Flieger dürfen blocken
ChooseRiverLabelFor = { $attacker }: wähle "links" oder "rechts"; nur dieser Stapel und Flieger dürfen blocken
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
        [one] Angreifer, der mit { $leader } eine Gruppe bildet
       *[other] Angreifer, die mit { $leader } eine Gruppe bilden
    }
TapToHelpPay = Tappe bleibende Karten, um mitzubezahlen — jede zahlt eins
DelveToHelpPay = Schicke Karten aus deinem Friedhof ins Exil, um mitzubezahlen — jede zahlt eine
CrewWithPower = Besatzung { $power }: Tappe beliebig viele deiner anderen ungetappten Kreaturen mit einer Gesamtstärke von { $power } oder mehr
SaddleWithPower = Satteln { $power }: Tappe beliebig viele deiner anderen ungetappten Kreaturen mit einer Gesamtstärke von { $power } oder mehr
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
ChooseExactlyOrNoTaps = Wähle genau { $count } bleibende Karten zum Tappen oder keine
ChooseExactly = Wähle { $count } { $noun }
ChooseBetween = Wähle { $min }–{ $max } { $noun }
ChooseCreatureType = Wähle einen Kreaturtyp
ChooseBasicLandType = Wähle einen Standardlandtyp
ChooseCardName = Wähle einen Kartennamen
ChooseColour = Wähle eine Farbe
ChooseColours = Wähle die Farbe oder Farben
ChooseCounters = Wie viele { $counter }? ({ $min }–{ $max })
ChooseCountersNamed = Wie viele { $counter } auf { $target }? ({ $min }–{ $max })
ChooseNumberIn = Wähle eine Zahl ({ $min }–{ $max })
DamageShare = Schaden an Ziel { $position } von { $total }, noch { $remaining } zu verteilen ({ $min }–{ $max })
CombatDamageShare = Kampfschaden an Kreatur { $position } von { $total }, noch { $remaining } zu verteilen ({ $min }–{ $max })
CombatDamageShareNamed = Kampfschaden von { $source } an { $recipient } ({ $position } von { $total }), noch { $remaining } zu verteilen ({ $min }–{ $max })
ReplicateHowOften = Replikation { $cost }: wie oft zahlen? ({ $min }–{ $max })
ChoosePlayer = Wähle einen Spieler
ChooseHowToCast = Wähle, wie gewirkt wird
ChoosePileForHand = Wähle einen Stapel für deine Hand; der andere kommt in deinen Friedhof
PileRow = Stapel { $pile }: { $cards }
EmptyPile = keine Karten
CastModeNumber = Modus { $number }
CastBackFace = Rückseite
CastLandFace = Als Land spielen
CastMiracle = Wunder
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
PayLifeOrTapped = { $amount } Leben zahlen? Sonst kommt es getappt ins Spiel
UseManaAnyType = North Star nutzen, um die Manakosten dieses Spruchs mit beliebigen Manatypen zu zahlen? Zusätzliche Kosten gelten unverändert.
PayAdditionalCost = Die zusätzlichen Kosten zahlen?
PayLife = { $amount } Lebenspunkte bezahlen?
PayTax = {"{"}{ $mana }{"}"} zahlen?
PayMana = { $cost } zahlen?
PayPact = Pakt: { $cost } bezahlen. Jetzt Mana erzeugen? Wenn du ablehnst oder ohne genug Mana abschließt, verlierst du das Spiel.
CastForMiracle = Für die Wunderkosten wirken?
CastWithoutPaying = Ohne Zahlung seiner Manakosten wirken?
CastPaying = Jetzt wirken und seine Manakosten bezahlen? Du erzeugst zuerst das Mana.
DrawOfferedBy = { $player } bietet ein Remis an. Annehmen?
CommanderToCommandZone = Deinen Kommandeur in die Kommandozone legen?
CommanderInsteadOfHand = Dein Kommandeur käme auf die Hand. Stattdessen in die Kommandozone?
CommanderInsteadOfLibrary = Dein Kommandeur käme in die Bibliothek. Stattdessen in die Kommandozone?
UseRegenerationShield = Einen Regenerationsschild verbrauchen? Nein nutzt die eigene Regeneration der bleibenden Karte und bewahrt den Schild.
UseShieldCounter = Eine Schildmarke statt Regeneration einsetzen?
UseDamageClearingReplacement = Zerstörung durch Entfernen allen markierten Schadens ersetzen? Nein nutzt einen anderen anwendbaren Ersatzeffekt.
ApplyAdditionalCountersNext = Als Nächstes { $count } zusätzliche Marken jeder Art hinzufügen? Nein wendet zuerst einen anderen Ersatzeffekt an.
UseTheOptionalAbility = Diese Fähigkeit ist optional. Einsetzen?
TopOfLibraryOrBottom = Deine Karte kommt in deine Bibliothek. Oben drauf? (Nein legt sie unter.)
TakeOneOfType = Nimm bis zu eine Karte vom Typ { $card_type } auf deine Hand
CastDiscovered = Du hast diese Karte entdeckt. Ohne ihre Manakosten zu bezahlen wirken? (Nein nimmt sie auf deine Hand.)
SkipTurnToUntap = Diesen Zug überspringen, um diese bleibende Karte zu enttappen?
YesOrNo = Ja oder nein?
OfferADraw = Remis anbieten
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
LeaveTableConfirm = Verlassen? Das Haus spielt für dich weiter. Nochmal drücken
LeaveTableConfirmOffline = Verlassen? Die Partie endet. Nochmal drücken
GameLogTitle = Spielprotokoll
GameLogEmpty = Noch ist nichts passiert
GameLogNewBelow = Neue Zeilen unten
GameLogLatest = Zurück zum Neuesten
RevealedBy = { $player } zeigt offen vor
RevealedWaiting = { $count } weitere folgen
RevealedHint = Zeige auf eine Karte, um sie zu vergrößern
RevealNext = Nächste
RevealDone = Fertig
HoldingPriority = Du wirst nicht gefragt
HoldRelease = Wieder fragen
ArmedPlay = Diese Karte spielen
ArmedActivate = Fähigkeit aktivieren
ArmedPayAndCast = { $cost } zahlen und zaubern
ArmedPayAndActivate = { $cost } zahlen und aktivieren
ArmedSuspend = { $cost } zahlen und aussetzen
SuspendedCards = Ausgesetzte Karten
SuspendedCard = { $card } · { $owner }{"\u000A"}{ $count } Zeitmarken · ansehen
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
TargetingExactly = Ausgewählt: { $selected }/{ $min } • { $count } mögliche Ziele
TargetingChoices = Wähle { $min }–{ $max } • { $count } gültige Ziele • { $selected } ausgewählt
TargetingBatch = Für alle { $count } bestätigen
TargetingBatchHint = { $count } gleiche Auslöser warten. Einzeln bestätigen oder diese Auswahl für alle { $count } übernehmen.
TargetingSelected = Ausgewählt: { $targets }
TargetingAllSeats = Alle Spieler
TargetingPage = Ziele { $first }–{ $last } von { $total }. Auswahl auch direkt am Tisch möglich.
CastKicked = Mit Bonuskosten
CastDisguise = Verkleidung — 2/2, Abwehr {"{"}2{"}"}
TurnFaceUp = Aufdecken
UnlockDoor = { $door } aufschließen
CastPrototype = Prototyp
ConfirmOk = OK
PassPriority = Passen
SkipTheTurn = Zug überspringen
ResolveTheStack = Stack abarbeiten
DesignationDay = Tag
DesignationNight = Nacht
SortByPlace = Nach Lage
SortByName = Nach Name
SortByCost = Nach Kosten
SortByType = Nach Typ
IsToken = Spielstein
YourMove = Du bist dran
YouMayRespond = Du kannst reagieren
StackRun = Stack abarbeiten
StackRunTo = Bis zur Markierung abarbeiten
StackStopHint = Stopp vor der markierten Fähigkeit. Erneut klicken zum Aufheben.
StackSelectHint = Eintrag markieren, um davor anzuhalten.
StackAbilityPolicy = Für diese Kartenfähigkeit
StackAlwaysPass = Immer passen
StackAsk = Immer fragen
StackAlwaysYes = Immer Ja
StackAlwaysNo = Immer Nein
StackResetRules = Fähigkeitsautomatik zurücksetzen
StackNoPolicy = Für diesen Eintrag ist keine dauerhafte Kartenfähigkeit verfügbar.
StackPolicyHint = Ziele und Zahlungen entscheidest du weiterhin selbst.
StackRuleName = { $card } · Fähigkeit { $number }
FinishHolographic = Holografisch
FinishGlitter = Glitzer
FinishGalaxy = Galaxie
StackTitle = Stapel
StackSpell = Zauber
StackAbilityBare = Fähigkeit
StackAbility = Fähigkeit · { $source }
StackMore = +{ $count } weitere
SeatNumbered = Platz { $seat }
YouNamed = Du ({ $player })
You = Du
AimedAt = Zielt auf { $target } ({ $position } von { $total })
DeclaredCount = { $count } deklariert
IncomingAt = { $defender }: { $count } greifen an, { $unblocked } ungeblockt
IncomingYou = dich
ASeat = ein Platz
APermanent = eine bleibende Karte
AimingAtNothing = nichts
RailYou = DU
RailOpponent = GEGNER
LinkLost = Verbindung verloren — verbinde neu …
LinkStandIn = Verbinde weiter — der Tisch wartet { $wait } auf dich, dann übernimmt das Haus deinen Platz, bis du zurück bist.
LinkPaused = Verbinde weiter — das Spiel ist pausiert, bis du zurück bist.
LostWaiting = { $player } hat die Verbindung verloren – warte { $wait }
LostPaused = { $player } hat die Verbindung verloren – das Spiel ist pausiert, bis die Verbindung zurück ist
LostHousePlays = Die Haus-KI spielt für { $player }
LinkGaveUp = Der Tisch ist nicht erreichbar. Tritt aus der Lobby erneut bei.
ClientOutdated = Dein Client ist veraltet: Aktualisiere ihn, um beizutreten.
TableOlder = Dieser Tisch läuft mit einer älteren Version als dein Client.
CardHasNoTarget = Für diese Karte gibt es kein gültiges Ziel mehr
CardWrongTime = Diese Karte lässt sich in diesem Zeitfenster nicht spielen
OnlyDearerModeLeft = Der normale Modus hat kein legales Ziel; nur der teurere Modus (etwa Überladen) ist möglich, und dein Mana reicht dafür nicht
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
PlateLife = { $amount } Leben
PlateHand = { $count } auf der Hand
PlateLibrary = { $count } in der Bibliothek
PlateGraveyard = { $count } im Friedhof
PlateExile = { $count } im Exil
PlatePoison = { $count } Gift
PlateRad = { $count } Strahlung
PayEnergy = { $count } Energiemarken bezahlen?
ProliferatePlayer = { $player } eine weitere Marke jeder bereits vorhandenen Sorte geben?
NounProliferatePermanents =
    { $count ->
        [one] bleibende Karte, die eine weitere Marke jeder Sorte erhält
       *[other] bleibende Karten, die eine weitere Marke jeder Sorte erhalten
    }
PlateSpeed = Tempo { $speed }/4
PlateMaxSpeed = Maximaltempo (4/4)
PlateEnergy = { $count } Energie
PlateCommander = { $amount } Kommandeurschaden
PlateMonarch = Monarch
PlateLost = Ausgeschieden
PlateTurn = Am Zug
PlateWaiting = Der Tisch wartet
ActiveTurn = Am Zug
ChosenType = Gewählt: { $subtype }
ChosenName = Genannt: { $card }
LockedDoors = Verschlossen: { $doors }
TapForRestricted = Tappen für { $mana } (eingeschränkt)
TapForVariable = Tappen für X Mana ({ $mana })
SheetPressAgain = zum Bestätigen
SheetDigitPicks = Eine Ziffer wählen
SheetCloses = schließen
SheetMorePage = Weitere — Seite { $page } von { $total }
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
VeilTakingSeat = Nehme deinen Platz ein
VeilTalking = Spreche mit dem Gateway
VeilWorking = Einen Moment
SeatSideNone = kein Team
SeatSide = Team { $team }
YouWon = Du hast gewonnen
YouLost = Du hast verloren
YourTeamWon = Team { $team } gewinnt — deins
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
LostCommanderDamageYou = Du hast 21 oder mehr Kampfschaden von einem Kommandeur erhalten
LostCommanderDamageOther = { $player } hat 21 oder mehr Kampfschaden von einem Kommandeur erhalten
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
NoOfflineDuel = Offline-Partie konnte nicht starten
UnsavedChanges = ungespeicherte Änderungen — nochmal drücken zum Verlassen
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
BrowseFilter = Name, Typ …
BrowseTallyUpTo = { $count } von bis zu { $max } gewählt
BrowseTallyExact = { $count } von { $max } gewählt
BrowseConfirm = Bestätigen
FilterBuild = Filter einstellen
FilterAdd = Bedingung hinzufügen …
FilterDone = Fertig
FilterClear = Leeren
FilterAsTyped = wie getippt
FilterUnanswerable = Eine Bedingung kann hier nicht geprüft werden — die Liste bleibt leer, solange sie steht.
FilterAlsoChips = Zusätzlich grenzen Chips diese Liste ein: { $chips }
FilterChipPlayable = nur spielbare
FilterChipCmc = Manawert { $mana_value }
FilterChipCmcUp = Manawert { $mana_value } oder mehr
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
FilterKeyManaValue = Manawert
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
FlagCommander = Kommandeur
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
LogConnectionLost = { $player } hat die Verbindung verloren – warte { $wait } auf die Rückkehr
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
LogLostCommanderDamageYou = { $you_subject } hast 21 oder mehr Kampfschaden von einem Kommandeur erhalten
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
LogIntoGraveyardYou = in deinen Friedhof
LogIntoGraveyard = in den Friedhof von { $player }
LogIntoBattlefield = aufs Spielfeld
LogIntoExile = ins Exil
LogIntoCommand = in die Kommandozone
LogCreatedYou = { $you_subject } hast { $token } erschaffen
LogCreated = { $player } hat { $token } erschaffen
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
        [one] Bohrmarke
       *[other] Bohrmarken
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
        [one] Kapitelmarke
       *[other] Kapitelmarken
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
        [one] Strahlungsmarke
       *[other] Strahlungsmarken
    }
LogCountersFlying =
    { $count ->
        [one] Flugfähigkeitsmarke
       *[other] Flugfähigkeitsmarken
    }
LogCountersShadow =
    { $count ->
        [one] Irrealitätsmarke
       *[other] Irrealitätsmarken
    }
LogCountersIndestructible =
    { $count ->
        [one] Unzerstörbarkeitsmarke
       *[other] Unzerstörbarkeitsmarken
    }
LogCountersHone =
    { $count ->
        [one] Schleifmarke
       *[other] Schleifmarken
    }
LogCountersLifelink =
    { $count ->
        [one] Lebensverknüpfungsmarke
       *[other] Lebensverknüpfungsmarken
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
LogBanded = { $attacker } greift in einer Gruppe mit { $leader } an
LogControlYou = { $you_subject } hast die Kontrolle über { $object } übernommen
LogControl = { $player } hat die Kontrolle über { $object } übernommen
LogTurnedFaceUp = { $object } wurde aufgedeckt
LogTransformed = { $object } hat sich verwandelt
LogCardsKeptYou = { $you_subject } behältst { $cards }
LogCardsKept = { $player } behält { $cards }
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
LogMonarchYou = { $you_subject } bist der Monarch geworden
LogMonarch = { $player } ist der Monarch geworden
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
ReportKindFeedback = Rückmeldung
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
ReportCatScreenshotHintWeb = Die Browser-Version des Clients kann kein Bild ihres Fensters machen, ein Bericht von hier enthält also nie eines. Angekreuzt gilt das Kästchen, wenn du aus dem Desktop-Client meldest.
ReportShotSize = { $width } × { $height }, { $kilobytes } KB
ReportCrashesBox = Absturzberichte automatisch senden
ReportPreviewShow = Zeigen, was gesendet wird
ReportPreviewHide = Vorschau schließen
ReportSend = Senden
ReportClose = Schließen
ReportSending = Wird gesendet …
ReportSent = Danke. Bericht { $id } ist angekommen.
ReportsUnavailable = Dieses Gateway nimmt gerade keine Berichte an.
ReportSignInAgain = Deine Sitzung ist abgelaufen. Melde dich erneut an und sende den Bericht dann.
ReportTooLarge = Der Bericht ist zu groß. Lass das Fensterbild oder das Protokoll weg und versuch es erneut.
ReportTooMany = Zu viele Berichte in kurzer Zeit. Warte etwas und sende dann erneut.
ReportUnreachable = Das Gateway war nicht erreichbar. Dein Bericht ist noch da.
ReportNotPassedOn = Das Gateway konnte den Bericht nicht weitergeben. Dein Bericht ist noch da; versuch es später erneut.
ReportFailed = Der Bericht konnte nicht gesendet werden. Versuch es später erneut.
ReportNeedsSession = Melde dich bei einem Gateway an, um einen Bericht zu senden. Einen anderen Feedback-Dienst, an den er gehen könnte, kennt dieser Client nicht.
ReportGoesDirect = Du bist nicht angemeldet, darum geht dieser Bericht direkt an den Baylee-Feedback-Dienst unter { $url }.
ReportsDirectUnavailable = Der Feedback-Dienst nimmt gerade keine Berichte direkt von einem Client an. Melde dich bei einem Gateway an, um ihn zu senden.
ReportDirectUnreachable = Der Feedback-Dienst war nicht erreichbar. Dein Bericht ist noch da.
ReportRecordBox = Die vollständige Aufzeichnung dieses Spiels anhängen
ReportRecordHint = Jeder Zug des Spiels, das dieses Gerät ausgerichtet hat, ab dem Mischen ({ $kilobytes } KB). Sie zeigt die Karten aller Plätze, auch die verdeckten: Hände, Bibliotheken, verdeckte Karten. Sie nennt niemanden. Wird bei jedem Bericht neu gefragt.
ReportRecordNever = Nie anbieten, die Aufzeichnung eines Spiels anzuhängen
ReportConfirmTitle = Diesen Bericht senden?
ReportConfirmTo = An: { $url }
ReportConfirmText = Dein Text ({ $count } Zeichen) und die Version dieses Builds
ReportConfirmPart = und: { $label }
ReportConfirmRecord = und die vollständige Aufzeichnung des Spiels, das dieses Gerät ausgerichtet hat ({ $kilobytes } KB): die Karten aller Plätze, auch die verdeckten
ReportConfirmDevice = und eine zufällige Kennung, die dieses Gerät für Berichte erzeugt hat, damit sich Berichte eines Geräts zuordnen lassen; nie dein Name, Konto oder deine Adresse
ReportConfirmSend = Jetzt senden
ReportConfirmBack = Zurück
ReportRecordLeftOut = Die Aufzeichnung des Spiels war zu groß zum Anhängen und wurde weggelassen.
ReportLeaked = Nicht gesendet: Der Bericht enthält „{ $secret }“. Entferne es und versuch es erneut.
ReportTrimmed = Um die Größengrenze einzuhalten, wurden das Fensterbild oder ältere Protokollzeilen weggelassen.
CrashAskTitle = Baylee wurde beim letzten Mal unerwartet beendet.
CrashAskBody = Darf der Client Absturzberichte an das Gateway senden, auf dem du spielst? Ein Absturzbericht enthält den Fehler, die Stelle im Programm, an der er auftrat (den Backtrace), und die Version dieses Clients, und Systemangaben, wenn du sie im Meldeformular erlaubst. Nichts über deine Partien oder dein Konto. Du kannst das jederzeit im Meldeformular ändern.
CrashAskSend = Absturzberichte senden
CrashAskNever = Nicht senden
SeatPanelTitle = Sprachmodell-Sitze
SeatPanelAbout = Welches Modell die Sitz-Brücke (baylee-seat) am Tisch spielt und was sie ausgeben darf. Kein Schlüssel steht je in dieser Datei: Jedes Profil nennt die Umgebungsvariable, aus der sein Schlüssel gelesen wird, oder legt ihn im Schlüsselbund dieses Rechners ab.
SeatFileAt = Datei: { $path }
SeatDesktopOnly = Sprachmodell-Sitze werden am Desktop eingerichtet, wo die Sitz-Brücke läuft.
SeatNoConfigDir = Dieses System nennt kein Konfigurationsverzeichnis, daher gibt es keine Einstellungsdatei für die Sitz-Brücke.
SeatEmpty = Noch keine Einstellungsdatei: Die Sitz-Brücke spielt mit ihren eingebauten Vorgaben, und nichts begrenzt ihre Ausgaben über Partien hinweg. Füge ein Profil hinzu, um eine anzulegen.
SeatNoProfiles = Keine Profile: Ohne --mind oder --profile spielt die Brücke die Haus-KI.
SeatFileRefused = Die Einstellungsdatei ist nicht verwendbar und bleibt, wie sie ist: { $error }. Korrigiere sie von Hand; dieser Bereich liest sie neu, sobald sie sich ändert.
SeatChangedOnDisk = Die Datei hat sich auf der Festplatte geändert, seit sie gelesen wurde. Speichern überschreibt sie; Änderungen verwerfen zeigt sie.
SeatAdd = Profil hinzufügen
SeatDuplicate = Duplizieren
SeatRemove = Entfernen
SeatDefault = Standard
SeatChipDefault = { $profile } · Standard
SeatDefaultAbout = Das Standardprofil spielt, wenn die Brücke weder --mind noch --profile bekommt.
SeatDefaultNone = Kein Standardprofil: Ohne --mind und --profile spielt die Brücke die Haus-KI.
SeatName = Name
SeatProvider = Protokoll
SeatProviderAnthropic = Anthropic
SeatProviderOpenAi = OpenAI-kompatibel
SeatProviderCli = Agent-CLI (Abo)
SeatCliModel = Das Modell einer CLI ist ihr Werkzeug, dann, wenn du eins nennst, dessen eigenes Modell: claude oder claude:opus.
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
SeatAnswerTools = Werkzeugen
SeatAnswerJson = JSON
SeatAnswerJsonSchema = JSON-Schema
SeatMaxTokens = Höchstens Ausgabe-Tokens
SeatPriceIn = Preis Eingabe ($ je Million)
SeatPriceOut = Preis Ausgabe ($ je Million)
SeatBuildPrice = Preis dieses Builds: { $input } Eingabe, { $output } Ausgabe je Million Tokens. Ein hier angegebener Preis ersetzt ihn.
SeatNoBuildPrice = Dieser Build kennt keinen Preis für dieses Modell: Gib hier einen an oder begrenze seine Partien in Tokens.
SeatGameUsd = Pro Partie ($)
SeatGameTokens = Pro Partie (Tokens)
SeatThinkSecs = Bedenkzeit (Sekunden)
SeatKeyEnv = Schlüssel-Variable
SeatKeySet = { $variable } ist in der Umgebung dieses Programms gesetzt.
SeatKeyUnset = { $variable } ist in der Umgebung dieses Programms nicht gesetzt. Die Brücke liest sie aus ihrer eigenen.
SeatPresets = Adapter hinzufügen (Protokoll und Adresse, danach änderbar):
SeatKeyBox = Schlüssel
SeatKeyHint = Schlüssel hier einfügen
SeatKeyReplaceHint = neuen Schlüssel einfügen, um ihn zu ersetzen
SeatKeyKeep = Speichern
SeatKeyForget = Vergessen
SeatKeyAsking = Frage den Schlüsselbund dieses Rechners…
SeatKeyKept = Im Schlüsselbund dieses Rechners liegt ein Schlüssel, nur für { $host }. Er wird nie angezeigt; die Variable geht vor, wo sie gesetzt ist.
SeatKeyNoneKept = Kein Schlüssel für { $host } gespeichert. Ein hier eingefügter geht in den Schlüsselbund dieses Rechners, nie in die Datei.
SeatKeyStoreUnavailable = Kein Schlüsselbund für einen Schlüssel ({ $reason }): Setze die Variable des Schlüssels in der Umgebung.
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
SeatCliNoPrice = Eine CLI spielt mit deinem Abo, das keinen Preis hat: Ihre Partien sind in Tokens und Aufrufen begrenzt und zählen unter den Token-Obergrenzen.
SeatCaps = Obergrenzen über alle Partien
SeatCapsAbout = Dollar zählen die Partien von Modellen mit Preis, Tokens die von Modellen ohne. Ein leeres Feld ist keine Grenze.
SeatDayUsd = Pro Tag ($)
SeatMonthUsd = Pro Monat ($)
SeatDayTokens = Pro Tag (Tokens)
SeatMonthTokens = Pro Monat (Tokens)
SeatUnpricedDay = Die Obergrenzen zählen Dollar pro Tag, und dieses Modell hat keinen Preis: Die Brücke verweigert seine Partien, bis es einen Preis oder die Obergrenzen ein Tageslimit in Tokens haben.
SeatUnpricedMonth = Die Obergrenzen zählen Dollar pro Monat, und dieses Modell hat keinen Preis: Die Brücke verweigert seine Partien, bis es einen Preis oder die Obergrenzen ein Monatslimit in Tokens haben.
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
SeatFaultGameCalls = Die meisten Aufrufe, die eine Partie machen darf: mindestens 1.
SeatFaultEffort = Ein Denkaufwand ist ein kleingeschriebenes Wort, etwa low, medium oder high.
SeatFaultJson = JSON-Antworten sind für OpenAI-kompatible Endpunkte und CLIs; Anthropics Modelle antworten mit Werkzeugen.
SeatFaultMaxTokens = Das Höchste, was eine Antwort nehmen darf: mindestens 1.
SeatFaultThinkSecs = Die längste Zeit für eine Antwort: mindestens 1 Sekunde.
SeatFaultUnpriced = Dieser Build kennt keinen Preis für dieses Modell, daher lässt sich keine Dollar-Grenze halten: Gib ihm einen Preis oder mache die Tokens pro Partie zur Grenze.
SeatFaultKeyEnv = Der Name einer Umgebungsvariable, etwa { $example }: Großbuchstaben, Ziffern und _. Nie der Schlüssel selbst.
SeatFaultAddress = Eine https://-Adresse; http:// nur auf diesem Rechner.
SeatFaultFile = Diese Einstellungen lassen sich nicht schreiben.
ShellPlay = Lobby
ShellDecks = Decks
ShellThisDevice = Dieses Gerät
ShellYourAccount = Dein Konto
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
TourChapterArrival = Los geht's
TourChapterPlace = Dein Platz
TourChapterCasting = Karten spielen
TourChapterStack = Der Stapel
TourChapterDecisions = Fragen
TourChapterPhases = Phasen und Passen
TourChapterDial = Die Tischmitte
TourChapterSeeing = Die Kamera
TourChapterReveals = Aufdecken, Log, Zonen
TourChapterKeys = Tasten und Menü
TourChapterReporting = Ein Problem melden
TourL1Title = Das Gateway
TourL1Body = Ein Gateway ist der Server, auf dem deine Decks und Tische liegen. Der farbige Punkt zeigt, ob er erreichbar ist, die Zeile darunter, ob deine Version zu ihm passt. Mit ‹ kommst du zur Liste der Gateways; dort kannst du auch eine eigene Adresse eingeben.
TourL2Title = Konto oder Gast
TourL2Body = Als Gast brauchst du nur einen Namen. Dafür kannst du keine Bilder hochladen, und rund 30 Tage nach deinem letzten Besuch wird der Gast samt Decks gelöscht. Mit einem Konto bleiben deine Decks und Einstellungen auf dem Gateway. In einer geschlossenen Beta fragt der Button, den du wählst, nach deinem Schlüssel.
TourL3Title = Offline spielen
TourL3Body = Offline spielen braucht kein Gateway: Du spielst dann direkt hier auf deinem Rechner gegen die Haus-KI. Daneben stehen Einstellungen und Über, wo du Lizenzen und Quellcode findest.
TourL4Title = Drei Bereiche
TourL4Body = Oben wechselst du zwischen Spielen, Decks und Einstellungen, auch mit den Tasten 1, 2 und 3. Rechts siehst du dein Gateway mit der Zahl der Tische und Spieler, die Glocke mit dem, was du verpasst hast, und dein Konto. Mit ? siehst du auf jedem Bildschirm alle Tasten, die gerade gelten.
TourL4TitleDesktop = Vier Bereiche
TourL4BodyDesktop = Oben wechselst du zwischen Spielen, Decks, Wiederholungen und Einstellungen, auch mit den Tasten 1, 2, 4 und 3. Rechts siehst du dein Gateway mit der Zahl der Tische und Spieler, die Glocke mit dem, was du verpasst hast, und dein Konto. Mit ? siehst du auf jedem Bildschirm alle Tasten, die gerade gelten.
TourL5Title = Dein nächstes Spiel
TourL5Body = Das ist das Deck, mit dem du an den nächsten Tisch gehst. Deck wechseln öffnet die Auswahl, Bearbeiten den Deckbauer. Hast du noch kein eigenes Deck, stehen hier drei Hausdecks: Hinzufügen & nutzen kopiert eins in dein Konto und wählt es gleich aus.
TourL6Title = Gegen das Haus
TourL6Body = Gegen das Haus startet ein Spiel gegen die Haus-KI, direkt hier im Client und ohne Uhr. Über den kleinen Pfeil am Button wählst du, wie stark sie spielt. In so einem Spiel läuft auch die Tisch-Tour.
TourL7Title = Die Tische
TourL7Body = Hier siehst du live alle Tische auf dem Gateway: Gastgeber, Format, freie Plätze und ob ein Passwort nötig ist. Mitspielen setzt dich an einen Tisch; ein Passwort tippst du direkt in der Zeile ein. Mit / suchst du, die Chips darüber filtern. Sitzt du schon irgendwo, steht hier Zurück zum Spiel.
TourL8Title = Tisch erstellen
TourL8Body = Mit Tisch erstellen machst du einen eigenen Tisch auf, auch mit der Taste C. Öffne es jetzt.
TourL9Title = Tisch einstellen
TourL9Body = Wähle, wie viele mitspielen (2 bis 8) und eine Vorlage: Commander, Duell 20 oder Fünf Länder. Mit Anpassen änderst du Leben, Mulligan und Handgröße. Ein Passwort ist freiwillig. Die Uhr ist die Bedenkzeit pro Entscheidung, von Blitz bis ohne Uhr. Tisch öffnen bringt dich in den Raum.
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
TourD1Body = Hier liegen deine Decks und die Hausdecks. Das Bild einer Kachel ist dein Commander, sonst die teuerste Karte, die kein Land ist. Darunter stehen Format, Kartenzahl, Sideboard und der letzte Stand. Marken zeigen dein nächstes Deck und Karten, die noch nicht spielbar sind.
TourD2Title = Was eine Kachel kann
TourD2Body = Fürs nächste Spiel macht das Deck zu deinem nächsten. Bearbeiten öffnet den Deckbauer. Hinter den drei Punkten: Duplizieren, Umbenennen, Exportieren, Verlauf, Favorit und Löschen. Löschen fragt nicht nach; du hast sechs Sekunden für Rückgängig, auch mit Strg/Cmd+Z.
TourD3Title = Hausdecks
TourD3Body = Fertige Decks vom Gateway, jede Karte darin ist spielbar. Hinzufügen kopiert eins zu deinen Decks, Hinzufügen & nutzen wählt es gleich fürs nächste Spiel. Ansehen zeigt die Liste; fahr mit der Maus über eine Karte oder drück die Leertaste, um sie groß zu sehen.
TourD4Title = Neu oder importiert
TourD4Body = Neues Deck öffnet einen leeren Deckbauer. Import liest Baylee-Text, JSON, YAML oder Moxfield-Text und erkennt das Format selbst; Strg/Cmd+V oder eine hierher gezogene Datei geht auch. Einen Moxfield-Link lädt Baylee nicht; der Dialog sagt dir, was du kopierst. Öffne ein Deck, dann geht's im Deckbauer weiter.
TourD5Title = Die Kopfzeile
TourD5Body = ‹ Decks bringt dich zurück und fragt vorher, wenn etwas nicht gespeichert ist. Der Stift benennt das Deck um. Speichern (Strg/Cmd+S) zeigt einen Punkt, solange etwas ungespeichert ist. Esc wirft dich nie aus dem Deckbauer; es schließt nur, was gerade obenauf offen ist.
TourD6Title = Der Kartenpool
TourD6Body = Links stehen alle Karten, die Baylee schon spielen kann, nicht der ganze Katalog. Nur spielbare ist an und blendet Platzhalter aus. Karten, die erst teilweise gehen, bleiben drin und sind markiert; den Grund zeigt die Kartenansicht. Die Zeile unter der Zahl sagt, was die Liste gerade noch eingrenzt.
TourD7Title = Suchen
TourD7Body = Das Suchfeld versteht die Scryfall-Schreibweise, etwa t:creature, o:draw, c:rg oder mv<=3, auch mit or, Klammern und Minus. Ein deutscher Kartenname findet die Karte genauso. Das Zahnrad baut dir den Filter als Formular. Tipp jetzt etwas ein.
TourD8Title = Hinzufügen
TourD8Body = Das + legt eine Karte in den Tab, der gerade offen ist, mit Shift in den anderen; Enter und Shift+Enter ebenso. Hinter den drei Punkten: Sideboard, Druck wählen und Karte öffnen. Fahr mit der Maus über eine Zeile, und du siehst die Karte groß. Füg jetzt eine Karte hinzu.
TourD9Title = Die Deckseite
TourD9Body = Rechts ist dein Deck: oben der Commander, darunter die Tabs Haupt, Sideboard und Statistik, gruppiert nach Typ, Manawert oder Farbe. In jeder Zeile zählen − und + runter und hoch; die drei Punkte verschieben die Karte, wählen den Druck oder nehmen alle raus. Statistik zeigt Kurve, Farben und Sieben ziehen.
TourD10Title = Hinweise unten
TourD10Body = Unten stehen zwei Arten von Hinweisen. Was das Gateway ablehnt, macht Speichern grau: kein Name, ein leeres Deck, mehr als 250 Zeilen oder Karten, die es nicht mehr gibt. Alles andere ist nur ein Satz, zum Beispiel 60 Karten oder 15 im Sideboard. Leuchtet der Button, kannst du speichern.
TourD11Title = Speichern
TourD11Body = Jedes Speichern legt eine Version an, die du später in der Historie wiederfindest. Änderst du nichts, wird auch nichts geschrieben. Drück jetzt Strg/Cmd+S.
TourD11bTitle = Telefon: eine Spalte
TourD11bBody = Auf dem Telefon siehst du Pool oder Deck; der Schalter oben wechselt. Filter sammelt die Chips hinter einem Button. Enter im Suchfeld fügt den ersten Treffer hinzu.
TourD12Title = Import
TourD12Body = Füg deine Liste mit Strg/Cmd+V ein; das Format erkennt Baylee selbst, und wenn es unsicher ist, wählst du es mit den Pfeiltasten. Enter importiert, danach schließt Enter. Zeilen, die nicht gefunden wurden, bleiben stehen und werden genannt; nichts fällt still unter den Tisch.
TourD13Title = Export
TourD13Body = Mit den Pfeiltasten wählst du das Format. Strg/Cmd+C kopiert die Liste, Strg/Cmd+S speichert sie als Datei (nicht im Browser). Baylee-Text merkt sich Druck und Finish jeder Karte, Moxfield-Text nur Anzahl und Name.
TourD14Title = Historie
TourD14Body = Links stehen alle gespeicherten Versionen, rechts siehst du, was sich gegenüber der vorigen oder der aktuellen geändert hat. Wiederherstellen macht aus der alten Version eine neue; nichts geht verloren, und sechs Sekunden lang kannst du es rückgängig machen.
TourT1Title = Das Übungsspiel
TourT1Body = Das hier ist ein Übungsspiel: drei Haus-KIs, keine Uhr, nichts zählt. Jede Sprechblase hat Weiter, Zurück und Überspringen; mit Esc klappst du sie oben zu einem kleinen Streifen ein, über die Kapitelzeile springst du. Wartet der Tisch auf dich, klappt sie sich von selbst ein. ? zeigt dir jederzeit alle Tasten.
TourT2Title = Deine Matte
TourT2Body = Deine Matte hat drei Reihen: vorne zur Tischmitte hin die Kreaturen, dahinter der Rest, hinten die Länder. Am Rand liegen Bibliothek, Friedhof, Exil und Kommandozone, jeweils mit Zahl. Gleiche Karten liegen als Stapel mit ×N; ein Klick zieht sie auseinander. Wird eine Reihe zu voll, scrollt sie seitlich.
TourT3Title = Die Hand
TourT3Body = Deine Hand liegt unten. Du kannst sie nach Kosten, A–Z oder Typ sortieren und mit dem Mausrad scrollen.
TourT4Title = Eine Karte lesen
TourT4Body = Fahr mit der Maus über eine Karte, und sie hebt sich und wird groß. Hältst du dabei Cmd oder Alt, siehst du ihren Text als Textseite; T lässt das an, V öffnet die Vorschau per Tastatur. Du siehst immer den Text, den die Karte gerade im Spiel hat; eine Kopie zeigt den Text des Kopierten.
TourT5Title = Die Leiste unten
TourT5Body = Die Leiste unten sagt dir, was der Tisch gerade von dir will, und zeigt die Antworten mit ihren Tasten: Leertaste passt, Enter klickt, Tab geht zur nächsten Phase, Shift+Tab zum nächsten Zug; jede Entscheidung bleibt trotzdem deine. Rechts findest du Log, Zonen und Menü.
TourT6Title = Leuchtende Karten
TourT6Body = Karten, die du jetzt spielen kannst, leuchten: Gold heißt, dein Mana reicht schon; Blau heißt, der Client würde dafür Länder tippen. Der erste Klick wählt die Karte an, der zweite (oder Enter, E) spielt sie; Esc wählt ab. Mana-Fähigkeiten gehen mit einem Klick. Leben zahlt der Client nie von selbst, ein X rät er nie.
TourT7Title = Spielen, dann zahlen
TourT7Body = Anders als bei Arena: Ein Klick spielt die Karte erst und zahlt danach. Die Karte geht auf den Stapel, dann kommen ihre Fragen (Modus, X, Ziele), dann das Bezahlen. Der Client tippt die Länder, die er sich vorgenommen hat; du kannst aber jedes Land auch selbst tippen. Was noch fehlt, steht als Geschuldet im Manapool.
TourT8Title = Abbrechen
TourT8Body = Solange die Fragen oder das Bezahlen offen sind, steht ganz rechts in der Leiste Abbrechen (Esc): Die Karte kommt zurück auf die Hand, getippte Länder enttappen. Spiel jetzt eine Karte und brich ab; ist noch nichts spielbar, geht es mit Weiter.
TourT9Title = Mehrere Fähigkeiten
TourT9Body = Hat eine Karte mehrere Fähigkeiten oder mehrere Wege, sie zu spielen, geht neben ihr ein Zettel auf: Kosten, der gedruckte Text und eine Taste von 1 bis 9; 0 blättert um. Eine Mana-Zeile geht sofort; jede andere wählst du erst an und bestätigst dann. Alternative Kosten fragt er hier, bevor ein Land getippt wird.
TourT10Title = Der Stapel
TourT10Body = Oben rechts siehst du den Stapel, jeden Eintrag als Karte: der nächste groß mit seinen Zielen, die übrigen kleiner darunter. Die Zeile oben sagt, auf wen der Tisch gerade wartet. Ein langer Text scrollt in seinem Kasten. Mit dem Knopf klappst du das Ganze ein.
TourT11Title = Antworten im Voraus
TourT11Body = Klick einen Eintrag an: Bis zur Markierung abarbeiten passt, bis er oben liegt, und hält davor. Für die Fähigkeit dieser Karte wählst du Immer passen, Immer fragen, Immer Ja oder Immer Nein; das gilt für jede Kopie, bleibt an deinem Konto, zurücksetzen unter Einstellungen › Spielablauf. Ziele und Kosten fragt er immer.
TourT12Title = F6 und F7
TourT12Body = F6 lässt den Stapel durchlaufen, F7 stellt die Fragen für diesen Zug ab. Beides endet von selbst, sobald etwas Neues passiert, und steht bis dahin in der Leiste; dieselbe Taste noch einmal beendet es auch. Dauerhaft automatisch läuft nur, was du unter Einstellungen › Spielablauf einschaltest; am Anfang ist nichts an.
TourT13Title = Fragen mit Auswahl
TourT13Body = Hat eine Frage mehrere Antworten, klappt aus der Leiste ein Fenster hoch: oben die Frage und das Bild der Karte, um die es geht (mit der Maus darüber siehst du es groß), unten die Antworten mit ihren Tasten. Was du wählen darfst, leuchtet auf dem Tisch. Das Fenster schließt nie von allein.
TourT14Title = Fenster einklappen
TourT14Body = Das Fenster verdeckt den Tisch. Mit X oder dem Knopf oben rechts klappst du es zu einem kleinen Streifen am rechten Rand ein; dann siehst du den ganzen Tisch und kannst jedes Ziel anklicken. Der Streifen oder X holt es zurück, die Frage bleibt. Klapp es jetzt ein und wieder auf.
TourT15Title = Andere Fragen
TourT15Body = Eine Farbe wählst du als Symbol, nicht als Wort. Eine Zahl tippst du ein oder stellst sie mit den Pfeiltasten; Rücktaste löscht. Bei einem Kreaturentyp stehen zuerst die Typen aus deinem Deck, dann alle nach Buchstaben; tippen filtert. Ein Kartenname kommt aus demselben Feld, über alle Karten.
TourT16Title = Die Phasen
TourT16Body = Am Rand jeder Matte stehen die zwölf Schritte eines Zugs, der aktuelle leuchtet. Grün: Hier fragt dich der Tisch. Rot: Hier passt er für dich. Am Anfang ist nichts rot. Ein Klick schaltet um, Shift+W und Shift+S wandern. Vorlagen: Einstellungen › Spielablauf. Deine Angriffs- und Blockschritte werden nie rot.
TourT17Title = Passen
TourT17Body = Leertaste passt. Tab läuft bis zur nächsten Phase, Shift+Tab bis zum nächsten Zug; jede Frage dazwischen bekommst du trotzdem. Hast du einen möglichen Angreifer, heißt Leertaste nicht „kein Angriff“; dafür ist O da. Am Gateway steht deine Uhr im Button: ab 60 Sekunden rot und hörbar; läuft sie ab, drückt sie für dich.
TourT18Title = Angreifen und blocken
TourT18Body = Ein Klick auf eine Kreatur schickt sie gegen den Gegner, der gerade im Fokus ist; noch ein Klick nimmt es zurück. Gibt es mehrere mögliche Gegner, wechselst du mit C und Shift+C. Angreifen, Blocken oder Nichts deklarieren schickt deine Antwort ab. Bei deinen Kampffragen fährt die Kamera von selbst zu dir nach Hause.
TourT19Title = Die Uhr in der Mitte
TourT19Body = In der Mitte des Tischs steht die Zugzahl, am Rand drumherum ein Edelstein je Spieler in dessen Farbe. Der lange helle Zeiger zeigt, wer am Zug ist, der kurze blaue, auf wen der Tisch wartet. Kommt der blaue Zeiger zu dir, pulst die Mitte und du hörst einen Ton; abstellen kannst du ihn im Menü unter Prioritätston.
TourT20Title = Namensschilder
TourT20Body = Jede Matte trägt ein Schild: Name, Leben, Handkarten, Bibliothek, Gift, Commander-Schaden; eine Krone zeigt den Monarchen, eine Marke die Haus-KI. Die Linie oben am Schild leuchtet hell, wenn der Spieler am Zug ist, und blau, wenn der Tisch auf ihn wartet. Unten links stehen alle Spieler noch einmal als kleine Chips.
TourT21Title = Die Anordnung
TourT21Body = Oben links wählst du, wie der Tisch aufgebaut ist: Ring, Aufrechter Ring, Drehteller, Bogenschiene, Raster, Rampenlicht, Drehteller+ oder Fokusring. P öffnet die Liste, Shift+P nimmt die nächste. Das Häkchen merkt sich deine Wahl für diese Spielerzahl. Im Duell gibt es nur eine. Drück jetzt Shift+P, dann noch einmal.
TourT22Title = Einen Spieler besuchen
TourT22Body = Klick unten links auf einen Chip oder drück F, und die Kamera schwenkt hinter diesen Spieler; sein Brett steht dann aufrecht und groß vor dir, auf dem Tisch bewegt sich nichts. Shift+F geht zum vorigen; H, Esc oder dein eigener Chip bringt dich zurück. Zielen geht auch von dort. Drück jetzt F, dann H.
TourT23Title = Dem Zug folgen
TourT23Body = Von selbst folgt die Kamera nur deinem Zug und deinen Kampffragen. Soll sie immer zum Spieler am Zug schwenken, schalte unter Einstellungen › Grafik Tisch folgt dem Zug ein. Sie schwenkt aber nie, solange eine Frage an dich offen ist oder dein Mauszeiger auf dem Tisch liegt.
TourT24Title = Auf dem Telefon
TourT24Body = Auf dem Telefon siehst du zu Hause deine Matte und die Tischmitte. Die anderen Spieler besuchst du über die Chips; die Anordnung findest du im Menü.
TourT25Title = Aufdecken
TourT25Body = Deckt ein anderer Spieler Karten auf, siehst du sie groß in einem Fenster: wer aufdeckt, die Karten, Schließen. Mit der Maus darüber wird jede noch größer. Du kannst es oben zu einem Streifen einklappen; Esc oder ein Klick schließt es. Es bleibt ein paar Sekunden stehen und verschwindet nicht, nur weil du passt.
TourT26Title = Das Protokoll
TourT26Body = L oder die Schriftrolle in der Leiste öffnet rechts das Protokoll: Uhrzeit, Spieler fett und jede Karte als Link, der die Vorschau öffnet, außer Karten, die du nicht sehen durftest. Es springt zur neuesten Zeile, bis du hochscrollst. Am Ende des Spiels steht es komplett im Ergebnisfenster. Drück jetzt L.
TourT27Title = Die Zonen
TourT27Body = G, die Zonen-Tür in der Leiste oder ein Klick auf einen Stapel am Mattenrand öffnet die Zonen: Friedhöfe, Exil und den Stapel. Das Suchfeld versteht dieselbe Schreibweise wie der Deckbauer. Fragt dich der Tisch nach einer Karte aus einem Stapel, geht er in der Mitte auf: Leertaste hakt an, Enter schickt ab.
TourT28Title = Alle Tasten
TourT28Body = ? zeigt dir alle Tasten, die am Tisch gerade gelten. F8 meldet ein Problem, Strg/Cmd mit +, − und 0 ändert die Schriftgröße. Jede Taste kannst du unter Einstellungen › Steuerung umbelegen; das bleibt an deinem Konto. Mit W, A, S, D und E spielst du ganz ohne Maus.
TourT29Title = Das Menü
TourT29Body = Das Menü unten rechts hat alles, was du im Spiel brauchst: Remis anbieten, Aufgeben, Anordnung, Prioritätston, Musik, Problem melden und Tour. Weitere Einstellungen gibt es am Tisch nicht; die warten in der Lobby.
TourT30Title = Aufgeben und Ende
TourT30Body = Aufgeben fragt noch einmal nach: Enter heißt Zurück; zum Aufgeben drückst du → und dann Enter, oder du klickst. Am Ende siehst du ein Fenster mit dem Ergebnis, dem Grund je Spieler und dem ganzen Protokoll. Am Gateway gibt es Nochmal spielen, offline Zurück zur Lobby; Esc bringt dich immer in die Lobby.
TourT31Title = Ein Problem melden
TourT31Body = Die Ecke oben rechts, F8 oder Problem melden im Menü öffnet ein Formular über dem Tisch. Wähle die Art, also Fehler, Vorschlag, Rückmeldung oder Sonstiges, und schreib, was los ist. Gesendet werden immer dein Text und die Version des Clients; alles andere nur, wenn du es anhakst. Öffne es jetzt.
TourT32Title = Anhänge
TourT32Body = Jede Zeile unter Anhänge ist eine Wahl, die dein Gerät sich merkt: System, der Tisch, wie du ihn siehst, dein Protokoll (Namen ersetzt), Einstellungen (ohne Schlüssel), ein Bild nur des Baylee-Fensters, nie des Desktops. Alles ist angehakt; nimm vor dem Senden heraus, was du behalten willst.
TourT33Title = Wohin es geht
TourT33Body = Dein Bericht geht an das Gateway, bei dem du angemeldet bist. Bist du nirgends angemeldet, geht er an den Feedback-Dienst dieser Version; gibt es den nicht, kannst du nur Kopieren. Mit Aufzeichnung fragt ein Fenster noch einmal nach. Am Gateway läuft deine Uhr derweil weiter; melde also lieber nach der Frage.
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
DiscoveryLive = Live-Aktualisierung verbunden
DiscoveryConnecting = Live-Aktualisierung verbindet …
DiscoveryRefresh = Tische aktualisieren
DiscoveryDetails = Gateway-Details
DiscoveryChoose = Finde deinen nächsten Tisch
DiscoveryHelp = Tritt einem Tisch bei und wähle dort dein Deck. Der Gastgeber legt Format und Spieltempo fest.
DiscoveryLoading = Tische werden aktualisiert
DiscoveryResults = { $count } Tische angezeigt
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
ShellKeyQuickSwitch = Gehe zu …
ShellKeyRefresh = Aktualisieren
ShellKeyTextLarger = Größerer Text
ShellKeyTextSmaller = Kleinerer Text
ShellKeyTextReset = Normale Textgröße
ShellKeyStart = Spiel starten
ShellKeySave = Deck speichern
ShellKeyExport = Exportieren
ShellOverlaySearch = Kürzel suchen
ShellOverlayEditKeys = Tasten ändern …
ShellClose = Schließen
ShellOverlayNoMatch = Kein Kürzel passt
ShellSeatedStrip = Am Tisch „{ $table }“ · { $count } von { $total } sitzen · { $ready } bereit
ShellPlayingStrip = Du spielst an „{ $table }“
ShellReturn = Zurück
ShellLeave = Verlassen
ShellTablesOnline = { $count } Tische · { $online } online
ShellTablesWaiting = { $count } warten
ShellReconnecting = Verbinde neu …
ShellRetryNow = Jetzt versuchen
ShellSessionEnded = Abgemeldet — deine Sitzung ist abgelaufen · Melde dich neu an
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
ShellStatsLine = { $online } online · { $waiting } warten · { $playing } spielen
ShellUpdateRequired = Update nötig — dieses Gateway läuft mit einer neueren Version
FrontClosedBeta = geschlossene Beta
FrontCompatible = kompatibel
FrontThisClient = dieser Client { $version }
FrontKeyHint = Dieses Gateway braucht einen Beta-Schlüssel — das Feld öffnet sich unter dem Knopf, den du wählst.
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
AboutCrates = Bevy und die anderen Rust-Crates dieses Builds: jede unter ihrer eigenen Lizenz (MIT, Apache-2.0 und ähnliche), von cargo-deny gegen eine Liste geprüft; NOTICE im Quellcode nennt den Rest.
AboutPrecons = Vorgefertigte Decklisten von MTGJSON (Zach Halpern): MIT-Lizenz.
SheetClose = Schließen
TermsTitle = Bevor du bei { $gateway } spielst
TermsVersion = Bedingungen dieses Gateways · Version { $version }
TermsUpdated = aktualisiert { $date }
TermsLoading = Die Bedingungen werden geladen …
TermsLoadFailed = Die Bedingungen konnten nicht geladen werden
TermsRetry = Erneut
TermsNotNow = Nicht jetzt
TermsAccept = Annehmen & weiter
TermsScrollToAccept = Bis zum Ende lesen, um anzunehmen
TermsSending = Wird gesendet …
TermsStoredNote = Lies bis zum Ende, um anzunehmen. Deine Zustimmung (Version und Zeit) wird mit deinem Konto auf diesem Gateway gespeichert; dieses Gerät merkt sie sich auch.
TermsGuestAsk = Ein Gast, der sich abmeldet, verliert seine Decks. Trotzdem abmelden?
TermsStay = Bleiben
TermsSignOut = Abmelden
TermsDecline = Ablehnen und Konto löschen
TermsLater = Später (abmelden)
TermsDeleteHere = Meine Daten auf diesem Gateway löschen
TermsStoredCentral = Lies bis zum Ende, um anzunehmen. Deine Zustimmung (Fassung, Text und Zeit) wird mit deinem Baylee-Konto bei Baylee Central gespeichert, damit ein anderes Gerät nicht erneut fragt.
TermsChatTitle = Chat
TermsChatNote = Wo ein Gateway Chat anbietet, sind Nachrichten an Freunde und an deinen Tisch Ende-zu-Ende-verschlüsselt: Dein Gerät verschlüsselt sie, nur die Geräte der Empfänger entschlüsseln sie, und das Gateway leitet sie weiter, ohne sie lesen zu können. Es sieht nur, wer wem wann wie viel schreibt. Meldest du eine Nachricht, schickt dein Gerät ihren Text an die Moderation dieses Gateways. Computergegner und Zuschauer nehmen am Chat nicht teil.
CentralTermsTitle = Nutzungsbedingungen von Baylee Central
CentralTermsVersion = Dein Baylee-Konto · Version { $version }
DeleteGatewayDataHint = Deine Decks, Einstellungen und Freunde auf diesem Gateway werden endgültig gelöscht. Dein Baylee-Konto bleibt.
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
ClockRapid = Schnellschach
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
RulesLife = { $amount } Leben
RulesMulligans =
    { $count ->
        [one] { $count } freier Mulligan
       *[other] { $count } freie Mulligans
    }
RulesFiveLands = fünf Länder im Spiel
RulesMixed = Decks jedes Formats
PlayHintJoin = auf einer Zeile: mitspielen
PlayPlayingAt = Du spielst an { $table }
PlayNoHost = Dieses Gateway hat gerade keinen Spielhost
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
RoomEmptySeat = — frei —
RoomWaitingFor = wartet, bis { $players } bereit ist
RoomNotStartable = noch ist nicht jeder Platz bereit
RoomMakeHost = Zum Gastgeber machen
RoomNextTeam = Nächstes Team
RoomLifeOverride = Eigene Lebenspunkte …
RoomStartingPosition = Startposition …
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
RoomNoModel = Auf diesem Rechner ist noch kein Sprachmodell eingerichtet. Lege unter Einstellungen › Sprachmodelle ein Profil an: Anbieter und Modell, die ein Platz spielt, und seinen Schlüssel, der im Schlüsselbund dieses Rechners bleibt.
RoomNoBridge = Dieser Installation fehlt die Sitzbrücke (baylee-seat) neben dem Spiel; installiere das neueste Release erneut.
RoomSetUpModel = Modell einrichten …
RoomChairTitle = Platz { $seat } · Sprachmodell
LeaveHostingQuestion = „{ $name }“ verlassen?
LeaveHostingHint = Du leitest diesen Tisch: Er geht an den, der am längsten sitzt, oder schließt, wenn sonst niemand sitzt.
RoomOpenSeats =
    { $count ->
        [one] { $count } Platz ist frei — ein Spieler, das Haus oder ein Modell nimmt ihn
       *[other] { $count } Plätze sind frei — Spieler, das Haus oder Modelle nehmen sie
    }
DecksSortSaved = Speicherdatum
DecksSortName = Name
DecksSortFormat = Format
DecksSortColours = Farben
FormatCommander = Commander
FormatFreeform = Frei
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
        [one] { $count } Karte spielt nicht
       *[other] { $count } Karten spielen nicht
    }
DecksBlockedOne = { $card } hier gesperrt
DecksBlockedMany = { $count } Karten hier gesperrt
CardBlocked = { $card } ist auf diesem Server gesperrt: { $reason }
CardBlockedUntil = { $card } ist auf diesem Server bis { $until } gesperrt: { $reason }
BuildBlocked = hier gesperrt
DecksSearchHint = Decks suchen …
PlaySortBy = { $order }
DecksSortBy = Nach { $order }
DecksNoMatch = Kein Deck passt zu „{ $query }“
DecksDuplicate = Duplizieren
DecksHistory = Verlauf …
DecksFavourite = Favorit
DecksUnfavourite = Kein Favorit mehr
DecksEmptyTitle = Dein Regal ist leer — beginne mit einem Hausdeck, baue eins oder importiere eine Liste.
DecksNewTileHint = oder eine Liste importieren — Strg/Cmd+V oder Datei ablegen
DecksNewTileTouch = oder eine Liste importieren ›
HouseAdd = Hinzufügen
HousePreview = Ansehen
ArtCredit = Illus. · { $artist }
HistoryTitle = Verlauf · { $deck }
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
HistoryPick = Wählen …
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
HistoryExport = Exportieren …
HistoryNeedsGateway = Der Verlauf braucht das Gateway
BuildUntitled = Neues Deck
BuildSave = Speichern
BuildNothingToSave = Nichts zu speichern
BuildUnsavedOne = Ungespeichert · 1 Änderung
BuildUnsavedMany = Ungespeichert · { $count } Änderungen
BuildSaving = Speichere …
BuildSavedNow = Gespeichert · gerade eben
BuildSavedAgo = Gespeichert · vor { $count } Min.
BuildSaved = Gespeichert
BuildNotSaved = Noch nicht gespeichert
BuildSaveFailed = Nicht gespeichert
BuildRename = Umbenennen
BuildEmptyDeck = Deck leeren …
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
BuildSyntaxMana = Manawert
BuildSyntaxPower = Stärke (auch tou, loy)
BuildSyntaxCost = Manakosten enthalten diese Symbole
BuildSyntaxCommander = kann ein Deck anführen
BuildSyntaxPartial = spielbar mit bekannter Lücke
BuildSyntaxNot = nicht
BuildSyntaxExact = genau dieser Name
BuildSyntaxOr = eines von beiden
BuildFilters = Filter
BuildRailType = Kartentyp
BuildRailMana = Manawert
BuildRailShow = Zeigen
BuildAlsoNarrowing = außerdem: { $chips }
BuildChoosePrinting = Druck wählen …
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
BuildByManaValue = Nach Manawert
BuildByColour = Nach Farbe
BuildCollapseAll = Alle zuklappen
BuildExpandAll = Alle aufklappen
BuildManaValue = Manawert { $mana_value }
BuildManaValueUp = Manawert { $mana_value }+
BuildMulticolour = Mehrfarbig
BuildShakyOne = 1 Karte ist nicht vollständig umgesetzt
BuildShakyMany = { $count } Karten sind nicht vollständig umgesetzt
BuildManaCurve = Manakurve
BuildCurveNote = Nichtland-Karten nach Manawert
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
BuildNoCatalog = Dieses Gateway hat keine Kartensuche — der Pool zeigt nur die eingebauten Karten
SectionGraphics = Grafik
SectionAudio = Audio
SectionDisplay = Anzeige & Oberfläche
SectionControls = Steuerung
SectionGameplay = Spielablauf
SectionAccount = Konto
SectionNetwork = Netzwerk & Gateway
SectionLanguageModels = Sprachmodelle
SectionUpdates = Updates
SectionPrivacy = Datenschutz & Daten
TagDevice = dieses Gerät
TagAccount = dein Konto
TagMachine = dieser Rechner
SettingsSearch = Einstellungen suchen …
SettingsNoMatch = Nichts gefunden
SettingsResetSection = Bereich zurücksetzen
SectionDisplayShort = Anzeige
SectionNetworkShort = Netzwerk
SectionModelsShort = Modelle
SectionPrivacyShort = Datenschutz
SettingsSignInToChange = Zum Ändern anmelden
SettingsAccountRule = Die Zeilen deines Kontos gelten auf jedem Gerät; eine Gerätezeile hebt sie nie an
SettingsSaveLine = Sofort gespeichert · markierte Zeilen gehen mit deinem Konto
SettingsSaveLineOffline = Auf diesem Gerät gespeichert · melde dich an, und die markierten Zeilen folgen deinem Konto
RowPreset = Voreinstellung
HelpPreset = Setzt die Gerätezeilen darunter; Eigene, sobald eine davon abweicht
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
AntiAliasingLocked = Auf Telefonen aus — Kachel-GPUs zahlen am meisten dafür
HelpVSync = Wartet auf den Bildschirm: kein Zerreißen, ein Bild Verzögerung · Strom ↓
HelpFrameLimit = Begrenzt die Bilder pro Sekunde · weniger ist kühler und leiser
HelpRestLimit = Bilder pro Sekunde am Tisch, wenn zwei Sekunden lang nichts geschieht · Strom ↓↓
HelpBackgroundLimit = Bilder pro Sekunde, während ein anderes Fenster vorn ist · Strom ↓↓
RowShowFrameRate = Bildrate anzeigen
HelpShowFrameRate = Ein kleiner Zähler in einer Ecke; ein Bericht trägt ihn mit
RowBackdrop = Hintergrund
HelpBackdrop = Das Gemälde hinter jedem Bildschirm; Schlicht liest sich am leichtesten und kostet am wenigsten
BackdropPainting = Gemälde
BackdropDimmed = Gedimmt
BackdropPlain = Schlicht
HelpAmbient = Treibendes Licht und Staub; nie über der Atmosphäre deines Kontos · GPU ↑
AmbientStill = Ruhig
AmbientSoft = Sanft
AmbientFull = Voll
HelpAtmosphere = Die Obergrenze für Umgebungsdetails auf jedem Gerät
RowHoldStill = Tisch ruhig halten
HelpHoldStill = Karten springen statt zu gleiten, überall; jede Regel bleibt
HelpSky = Das Licht hinter dem Tisch: folgt deiner Uhr oder bleibt bei Tag oder Nacht
HelpMaster = Jeder Klang, die Musik eingeschlossen
RowMusic = Musik
RowMusicTheme = Musik-Thema
HelpMusicTheme = Fünf Suiten in Bb-Dorisch und zehn weitere in ganz unterschiedlichen Stilen. Wechselnd wählt pro Spiel eine andere. Übergänge fließen in den nächsten musikalischen Puls
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
HelpMusicSamples = Beide Banken nutzen CC0-Aufnahmen mit 44,1 kHz. Studio bereitet sie hochwertig für 48 kHz auf; die Alternative spielt die Originalbank. Eigene Synthesizer laufen immer mit 48 kHz
MusicSamplesStudio = Studio
MusicSamplesOriginal = Original
MusicThemeRotating = Wechselnd
HelpMusic = Musik für Titelbildschirm, Lobby und Spiel
HelpEffects = Die Klänge des Tisches, unter den Tischklängen deines Kontos
HelpMuteUnfocused = Stille, während ein anderes Fenster vorn ist
RowTableSounds = Tischklänge
HelpTableSounds = Aus, halb oder voll auf jedem Gerät; Aus gewinnt gegen jeden Regler
HelpLanguage = Die Sprache der Oberfläche und, wo das Gateway sie hat, der Kartentexte
RowTextSize = Textgröße
HelpTextSize = Fünf Stufen für jeden Bildschirm außerhalb des Tisches · Strg/Cmd + = − 0
RowPreviewSize = Größe der Kartenvorschau
HelpPreviewSize = Wie groß eine Karte erscheint, auf die gezeigt wird
RowTextFace = Textansicht bevorzugen
HelpTextFace = Karten als Text statt als Druck zeichnen; die Zusatztaste tauscht, solange sie gehalten wird
RowShellKeys = Tastenkürzel
HelpShellKeys = Die Tasten jedes Bildschirms außerhalb des Tisches; eine belegte Taste wird abgelehnt, ein zweiter Druck nimmt sie
RowTableKeys = Tasten am Tisch
HelpTableKeys = Die Tasten, während ein Spiel offen ist
HelpAutomation = Was der Client für dich beantwortet; keine Regel ändert sich
RowAbilityAnswers = Antworten auf Fähigkeiten
HelpAbilityAnswers = Antworten, die der Client für dich weiter geben soll
RowHandle = Name
HelpHandle = Wie andere Spieler dich sehen
CopyHandle = Namen kopieren
RowSignOut = Abmelden
HelpSignOut = Beendet diese Sitzung hier und auf dem Gateway
HelpDeleteAccount = Das Konto, seine Decks und seine Einstellungen, für immer
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
ReportRefBattlefield = Schlachtfeld
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
HelpWhatIsKept = Das Gateway speichert dein Konto, deine Decks, Einstellungen und welche Bedingungen du angenommen hast. Dieser Client sendet von selbst nur Update-Prüfungen an GitHub (Desktop, solange geprüft wird), Absturzberichte nach deiner Zustimmung, während ein Offline-Spiel läuft eine anonyme „spielt noch“-Zählung (außer unten ausgeschaltet) und fragt im Browser Scryfall nach Kartenbildern
RowReportConsent = Was ein Bericht mitnehmen darf
HelpReportConsent = Hier angehakt, im Berichtsformular angehakt; ein Bericht zeigt genau, was gesendet wird
RowCrashReports = Absturzberichte
HelpCrashReports = Nach einem Absturz: Bericht senden, jedes Mal fragen oder nie
RowUsageCount = Anonyme Nutzungszahl senden
HelpUsageCount = Sagt beim Offline-Spielen einmal pro Minute, dass ein Spiel läuft: kein Konto, keine Geräte-ID, nichts über das Spiel
CrashSend = Senden
CrashAsk = Fragen
CrashNever = Nie
HelpReportProblem = Öffnet das Berichtsformular (F8)
HelpChatPrivacy = Wer dir flüstern darf, Freundschaftsanfragen, Chatverlauf, Töne und die Chat-Geräte dieses Kontos
ReportOpen = Problem melden
LanguageModelsElsewhere = Sprachmodelle werden in einem Desktop-Client eingerichtet
KeyFixed = Diese Taste ist fest und lässt sich nicht belegen
KeyHeldBy = Schon { $shortcut } — drücke die Taste noch einmal oder nimm sie
KeyTake = Nehmen
NoAbilityAnswers = Noch keine
SliderOff = aus
GroupReplay = Wiederholungen
ActReplayTurnBack = Wiederholung: einen Zug zurück
ActReplayTurnForward = Wiederholung: einen Zug weiter
ActReplayStart = Wiederholung: zum Anfang
ActReplayEnd = Wiederholung: zum Ende
ActReplaySlower = Wiederholung: langsamer
ActReplayFaster = Wiederholung: schneller
ActReplayPerspective = Wiederholung: Perspektive wechseln
ToastTitleInfo = Hinweis
ToastTitleAlert = Meldung
ToastTitleTable = Tisch
ToastTitleSocial = Kontakt & Chat
GatewayOnlinePlayers = Spieler online
GatewayInLobby = In der Lobby
GatewayInGames = Am Spielen
GatewayWaitingTables = Offene Tische
GatewayRunningGames = Laufende Spiele
GatewayServerVersion = Server-Version
ShellGatewayInlineStats = { $online } online ({ $lobby } Lobby · { $playing } spielen) · { $tables } Tische
SettingsQuickTitle = Schnelleinstellungen
SettingsAudioSection = Audio
SettingsDisplaySection = Anzeige & Bewegung
SettingsTextSize = Textgröße
