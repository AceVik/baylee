<p align="center">
  <a href="README.md">English</a> · <b>Deutsch</b>
</p>

<p align="center">
  <img src="docs/images/readme/hero.webp" alt="Baylees Eingang: ein mondbeschienener Wintergarten mit der Gateway-Liste" width="100%">
</p>

<h1 align="center">Baylee</h1>

<p align="center">
  Eine Regel-Engine und ein 3D-Spieltisch für Magic: The Gathering – mit Freunden oder gegen die Haus-KI.<br>
  Kostenlos, quelloffen (AGPL-3.0), inoffizieller Fan-Content.
</p>

<p align="center">
  <a href="https://github.com/AceVik/baylee/actions/workflows/ci.yml"><img alt="ci" src="https://github.com/AceVik/baylee/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://github.com/AceVik/baylee/releases"><img alt="neueste Vorabversion" src="https://img.shields.io/github/v/release/AceVik/baylee?include_prereleases&label=pre-release"></a>
  <a href="LICENSE"><img alt="Lizenz: AGPL-3.0-only" src="https://img.shields.io/badge/license-AGPL--3.0--only-blue"></a>
</p>

> [!NOTE]
> **Baylee ist in der geschlossenen Beta.** Aktuell ist
> [v0.1.0-beta.2](https://github.com/AceVik/baylee/releases/tag/v0.1.0-beta.2), eine Vorabversion.
> Das öffentliche Gateway **[CLOSED BETA] Baylee Sanctuary** unter `https://baylee.acevik.de`
> nimmt neue Spieler nur mit einem Beta-Schlüssel auf. Ohne Schlüssel kannst
> du offline gegen die Haus-KI spielen.

## Inhalt

- [Was Baylee ist](#was-baylee-ist)
- [Bilder](#bilder)
- [Installation](#installation)
  - [Windows](#windows)
  - [macOS](#macos)
  - [Linux](#linux)
  - [Handy, Tablet und Browser](#handy-tablet-und-browser)
  - [Download prüfen](#download-prüfen)
  - [Aktualisieren](#aktualisieren)
- [Zugang: Beta-Schlüssel](#zugang-beta-schlüssel)
- [Spielen](#spielen)
- [Fehler melden](#fehler-melden)
- [Ein eigenes Gateway betreiben](#ein-eigenes-gateway-betreiben)
- [Aus dem Quellcode bauen](#aus-dem-quellcode-bauen)
- [Wie alles zusammenhängt](#wie-alles-zusammenhängt)
- [Mitmachen](#mitmachen)
- [Lizenz und Rechtliches](#lizenz-und-rechtliches)

## Was Baylee ist

Baylee ist eine in Rust geschriebene Plattform für Magic: The Gathering. Sie
besteht aus zwei Teilen.

**Eine Regel-Engine, die das Spiel durchsetzt.** Die Engine wendet die
Comprehensive Rules an: Priorität und Stapel, das Ebenensystem,
Ersatzeffekte, ausgelöste Fähigkeiten, zustandsbasierte Aktionen und den
Kampf. Was erlaubt ist, entscheidet nie der Client. Bei jedem Schritt zählt
die Engine auf, welche Möglichkeiten ein Spieler hat, und der Client bietet
genau diese an. Die Engine ist **deterministisch**: Sie nutzt einen
Zufallsgenerator mit festem Startwert, liest keine Uhr und hängt nirgends von
der Reihenfolge einer Hash-Map ab. Dieselbe Partie mit denselben Antworten
endet deshalb immer gleich. Jede gehostete Partie wird Zug für Zug
aufgezeichnet, und die Aufzeichnung lässt sich bis zum selben Stand
nachspielen.

**Ein 3D-Tisch**, gebaut mit [Bevy](https://bevyengine.org/). Er zeigt
Schlachtfeld, Hände, Stapel und Friedhöfe und bietet eine Lobby für Konten,
Decks und Räume.

Was Baylee ausmacht:

- **Verdeckte Information bleibt verdeckt, und zwar durch die Bauweise.** Der
  Client eines Spielers bekommt nur, was dieser sehen darf: Bibliotheken und
  fremde Hände kommen als Anzahl an, und eine verdeckte Karte trägt keine
  Identität, solange der Spieler kein Recht darauf hat. Ein veränderter
  Client kann nicht schummeln, denn die Daten erreichen ihn gar nicht.
- **Zwei bis acht Plätze.** Duelle, Jeder-gegen-jeden und Teamtische. Der
  Gastgeber teilt die Plätze in Seiten ein, und die Seiten müssen nicht gleich
  groß sein (3 gegen 2 geht). Teams behalten eigene Züge und eigene
  Lebenspunkte; das ist also kein Two-Headed Giant. Teammitglieder können
  einander ihre Hand zeigen, wenn sie wollen.
- **Commander und Freeform.** Ein Deck mit Kommandeur spielt Commander, jedes
  andere Deck spielt Freeform. Bannlisten und Formatprüfungen gibt es noch
  nicht.
- **Eine Haus-KI in fünf Stufen**: novice, casual, steady, sharp und expert
  (in der deutschen Oberfläche Anfänger, Locker, Solide, Scharf und Experte;
  [docs/house-ai.md](docs/house-ai.md)). Sie spielt aus derselben Sicht, die
  ein Mensch bekommt.
- **Ein Prozess pro Partie.** Jede Partie läuft in einem eigenen
  Engine-Prozess, den ein separater Agent startet. Das Gateway, das Konten und
  Räume verwaltet, führt selbst keine Regeln aus.
- **Kartentext in 19 Sprachen**, wenn das Gateway den Kartenkatalog hat. Die
  Oberfläche gibt es auf Deutsch und Englisch.

Der Kartenpool wächst Stapel für Stapel; diese Version kennt rund 2.700
Karten. Der Deckbau kann sich auf spielbare Karten beschränken und warnt,
wenn ein Deck Karten enthält, die noch nicht vollständig umgesetzt sind und
deshalb nicht wie gedruckt spielen.

## Bilder

Die Bilder vom Tisch zeigen Baylees **Textansicht**. Sie zeichnet jede Karte
aus ihrem Regeltext statt aus dem gedruckten Kartenbild (warum, steht unter
[Lizenz und Rechtliches](#lizenz-und-rechtliches)). Im Spiel kommen die
Kartenbilder von Scryfall, und `T` schaltet zwischen den beiden Ansichten um.
Aufgenommen wurden die Bilder mit Version 0.1.0-beta.2 an einem lokalen
Test-Gateway mit erfundenen Spielern; das Bild ganz oben und das Anmeldefenster zeigen das echte
Gateway Baylee Sanctuary.

<table>
  <tr>
    <td width="50%"><img src="docs/images/readme/front-door.webp" alt="Anmeldefenster des Beta-Gateways"><br><sub><b>Anmelden.</b> Das Beta-Gateway verlangt von neuen Spielern einen Schlüssel; wer schon ein Konto hat, meldet sich mit Benutzername und Passwort an. Unten stehen der Fan-Content-Hinweis und der Link zum Quellcode.</sub></td>
    <td width="50%"><img src="docs/images/readme/lobby.webp" alt="Lobby mit einem Deck und offenen Tischen"><br><sub><b>Die Lobby.</b> Links deine Decks, rechts die offenen Tische, dazu <i>Gegen das Haus</i> für eine schnelle Partie gegen die KI.</sub></td>
  </tr>
  <tr>
    <td width="50%"><img src="docs/images/readme/room.webp" alt="Ein Tisch wird eingerichtet: Plätze, KI-Stufen, Lebenspunkte"><br><sub><b>Einen Tisch einrichten.</b> Der Gastgeber legt Plätze, Startlebenspunkte und Mulligans fest und gibt jedem KI-Platz eine Stufe von novice bis expert.</sub></td>
    <td width="50%"><img src="docs/images/readme/deck-builder.webp" alt="Deckbau mit Statistik und Kartensuche"><br><sub><b>Der Deckbau.</b> Kommandeur, Hauptdeck und Sideboard, Manakurve und Länderstatistik, dazu eine Suche über den ganzen Pool. (Die Bildfelder sind leer, weil die Kartenbilder für diese Aufnahmen abgeschaltet waren.)</sub></td>
  </tr>
  <tr>
    <td width="50%"><img src="docs/images/readme/table-duel.webp" alt="Ein laufendes Duell, Kartenvorschau in der Textansicht"><br><sub><b>Ein Duell</b> gegen die Haus-KI. Mit der Maus über einer Karte erscheint sie groß, hier in der Textansicht.</sub></td>
    <td width="50%"><img src="docs/images/readme/game-log.webp" alt="Das Spielprotokoll neben dem Tisch"><br><sub><b>Das Spielprotokoll</b> (<kbd>L</kbd>) liest sich wie ein Chat; jeder Kartenname öffnet die Vorschau der Karte.</sub></td>
  </tr>
  <tr>
    <td width="50%"><img src="docs/images/readme/table-ring.webp" alt="Ein Tisch mit vier Plätzen"><br><sub><b>Vier Plätze.</b> Größere Tische sitzen im Kreis um den Filz.</sub></td>
    <td width="50%"><img src="docs/images/readme/report.webp" alt="Meldeformular mit freiwilligen Kästchen"><br><sub><b>Einen Fehler melden</b> (<kbd>F8</kbd>). Jede zusätzliche Angabe hat ein eigenes Kästchen, und alle sind anfangs leer.</sub></td>
  </tr>
</table>

## Installation

Lade das Archiv für dein System von der
**[Release-Seite](https://github.com/AceVik/baylee/releases)** herunter. Dort
stehen auch die Vorabversionen. Jedes Archiv enthält das Spiel, seinen Ordner
`assets`, `LICENSE`, `NOTICE` und eine `README.txt` mit diesen Schritten für
den ersten Start.

| System | Datei |
| --- | --- |
| Windows, Intel/AMD 64 Bit | `baylee-client-<version>-x86_64-pc-windows-msvc.zip` |
| Windows auf ARM | `baylee-client-<version>-aarch64-pc-windows-msvc.zip` |
| macOS, Apple Silicon (M1 und neuer) | `baylee-client-<version>-aarch64-apple-darwin.zip` |
| Linux, x86-64 | `baylee-client-<version>-x86_64-unknown-linux-gnu.tar.gz` |
| Linux, ARM64 | `baylee-client-<version>-aarch64-unknown-linux-gnu.tar.gz` |

Die Programme sind **nicht signiert**, weil eine Signatur Geld kostet. Windows
und macOS warnen deshalb beim ersten Start. Wie du an der Warnung vorbeikommst,
steht in den Schritten unten.

### Windows

1. Entpacke die ZIP-Datei in einen beliebigen Ordner, etwa
   `Dokumente\Baylee`. Der Ordner `assets` muss neben `baylee-client.exe`
   liegen bleiben.
2. Starte `baylee-client.exe`.
3. Meldet Microsoft Defender SmartScreen „Der Computer wurde durch Windows
   geschützt“, klicke auf **Weitere Informationen** und dann auf
   **Trotzdem ausführen**.

Die x86-64-Version braucht eine CPU mit x86-64-v2 (SSE4.2, POPCNT); ohne
eine solche startet auch Windows 11 24H2 nicht.

### macOS

Unterstützt werden vorerst nur Macs mit **Apple Silicon** (M1 und neuer); eine
Intel-Version gibt es nicht. Voraussetzung ist macOS 11 oder neuer.

1. Entpacke die ZIP-Datei und lege `Baylee.app` ab, wo du willst, zum
   Beispiel in `Programme`.
2. Öffne die App. Beim ersten Mal lehnt macOS sie ab, weil sie nicht
   notarisiert ist.
3. Öffne **Systemeinstellungen → Datenschutz & Sicherheit**, scrolle zum
   Hinweis zu Baylee und klicke auf **Dennoch öffnen**.

Oder im Terminal:

```bash
xattr -dr com.apple.quarantine /pfad/zu/Baylee.app
```

### Linux

```bash
tar -xzf baylee-client-<version>-x86_64-unknown-linux-gnu.tar.gz
cd baylee-client-<version>-x86_64-unknown-linux-gnu
./baylee-client
```

Du brauchst einen Vulkan-Treiber und die Laufzeitbibliotheken für ALSA, udev,
X11/Wayland und xkbcommon. Unter Debian oder Ubuntu sind das
`libasound2 libudev1 libxkbcommon-x11-0 libwayland-client0`. Gebaut wird auf
Ubuntu 22.04; das Programm läuft auf jeder Distribution mit glibc 2.35 oder
neuer. Wie unter Windows braucht die x86-64-Version eine CPU mit x86-64-v2.

### Handy, Tablet und Browser

**Mobile Versionen zum Herunterladen gibt es noch nicht.**

- **Android:** Der Client lässt sich bauen und läuft auf einem echten Handy
  (`scripts/mobile/android-build.sh`). Die Lobby ist aber noch nicht per
  Touch bedienbar, deshalb kannst du dich auf dem Handy nicht anmelden.
- **iOS:** Der Client läuft nur im Simulator
  (`scripts/mobile/ios-sim-run.sh`).
- **Browser:** Der Client lässt sich auch für WebAssembly bauen und spielt
  gegen ein Gateway. Er braucht WebGPU, das Browser nur auf HTTPS-Seiten
  freigeben. Das Projekt veröffentlicht keine gehostete Web-Version.

Was genau geht und was nicht, steht in [docs/mobile.md](docs/mobile.md).

### Download prüfen

Neben jedem Archiv liegt auf der Release-Seite eine `.sha256`-Datei. Lade
beide in denselben Ordner und führe aus:

```bash
# Linux
sha256sum -c baylee-client-<version>-<target>.tar.gz.sha256
# macOS
shasum -a 256 -c baylee-client-<version>-<target>.zip.sha256
```

Unter Windows führst du in der PowerShell
`certutil -hashfile baylee-client-<version>-<target>.zip SHA256` aus und
vergleichst das Ergebnis mit der Zahl in der `.sha256`-Datei.

### Aktualisieren

Eine automatische Aktualisierung gibt es noch nicht. Lade das neue Archiv
herunter und ersetze den alten Ordner (oder `Baylee.app`) durch den neuen.

Deine Einstellungen bleiben dabei erhalten, weil sie nicht neben dem Programm
liegen. Der Client speichert sie in `baylee/` unter `$XDG_CONFIG_HOME`, oder
unter `~/.config/baylee/`, wenn diese Variable nicht gesetzt ist. Das gilt
auch auf macOS. Dort liegen:

- `client-settings.json`: Sprache, Gateway-Liste, Einwilligungen für
  Meldungen und die Sitzung eines Gastes;
- `preferences.json`: Tastenbelegung und stehende Antworten;
- `offline-decks.json`: offline gebaute Decks.

Kartenbilder werden getrennt davon zwischengespeichert, in
`~/Library/Caches/baylee` (macOS), `%LOCALAPPDATA%\baylee` (Windows) oder
`~/.cache/baylee` (Linux). Dein Konto und die online gebauten Decks liegen auf
dem Gateway, nicht auf deinem Rechner. Wenn du dich anmeldest, lädt der
Client deine Tastenbelegung und Vorlieben vom Gateway.

> [!WARNING]
> **Windows:** Der Pfad für die Einstellungen kommt aus der Umgebungsvariable
> `HOME` (oder `XDG_CONFIG_HOME`), die Windows meist nicht setzt. Prüfe, ob
> deine Einstellungen einen Neustart überleben, bevor du dich darauf
> verlässt. Dein Konto und deine Online-Decks betrifft das nicht, denn die
> liegen auf dem Gateway.

Jede Version zeigt ihre Versionsnummer und ihren Commit in der Lobby. Eine
neue Minor-Version (0.1 → 0.2) kann mit einem älteren Gateway unter Umständen
nicht mehr sprechen. Aktualisiere also, sobald eine neue Version erscheint.

## Zugang: Beta-Schlüssel

Das Gateway Baylee Sanctuary ist eine geschlossene Beta. **Ein neues Konto und
ein neuer Gast brauchen jeweils einen Schlüssel** der Form
`BAYLEE-XXXX-XXXX-XXXX-XXXX`. Die Schlüssel vergibt der Projektinhaber
([@AceVik](https://github.com/AceVik)); ein öffentliches Anmeldeformular gibt
es nicht.

Mit Schlüssel:

1. Starte Baylee. Im Eingang steht **Baylee Sanctuary** schon in der Liste;
   wähle es aus.
2. Wähle **Registrieren**. Such dir einen *Benutzernamen* (3–24 Buchstaben,
   Ziffern, `_ - .`), einen *Anzeigenamen* und ein Passwort aus und füge den
   Schlüssel ein. Groß- und Kleinschreibung, Leerzeichen und Bindestriche im
   Schlüssel spielen keine Rolle.
3. Oder wähle **Als Gast spielen**: Ein Anzeigename und ein Schlüssel reichen. Gäste
   können Decks bauen und spielen, aber keine Hüllen oder Spielmatten
   hochladen. Ein Gast wird gelöscht, wenn er sich abmeldet oder 30 Tage lang
   nicht genutzt wurde.

Die Registrierung **fragt nach keiner E-Mail-Adresse**: Ein Konto besteht aus
Benutzername und Passwort. Der Benutzername bleibt
privat: Andere sehen nur deinen Anzeigenamen mit einem kurzen Kürzel, etwa
`Alice#af03`. Mit einem Konto meldest du dich künftig mit Benutzername und
Passwort an und brauchst keinen Schlüssel mehr. Du kannst das Konto jederzeit
im Client löschen (`DELETE /account`).

## Spielen

**Offline**, ganz ohne Gateway: Wähle im Eingang *Offline spielen*. Du
bekommst dieselbe Lobby mit deinen Offline-Decks, dem Deckbau und einem Tisch
mit Gegnern der Haus-KI. *Gegen das Haus* startet sofort ein Duell mit den
beiden mitgelieferten Decks.

**Online**, angemeldet an einem Gateway:

- **Decks.** Bau ein Deck im Deckbau: Durchsuche den Pool (auf Wunsch nur
  vollständig spielbare Karten), stelle Hauptdeck, Sideboard und Kommandeur
  zusammen, behalte die Manakurve im Blick und wähle für jede Karte eine
  Druckversion. Gespeichert wird ein Deck als eine Textzeile pro Karte, etwa
  `1 Lightning Bolt (M11) 149` ([docs/deck-format.md](docs/deck-format.md)).
  Das Gateway führt für jedes Deck eine Versionsgeschichte. Registrierte
  Konten können einem Deck eine eigene Hülle und Spielmatte geben.
- **Räume.** Eröffne einen Raum mit 2 bis 8 Plätzen, auf Wunsch mit Namen und
  Passwort. Als Gastgeber legst du fest, wer wo sitzt: ein anderer Spieler
  oder die Haus-KI in einer ihrer fünf Stufen. Du bestimmst auch die Seiten,
  die Startlebenspunkte, freie Mulligans, bleibende Karten, die schon zu Beginn
  im Spiel liegen, und die Bedenkzeit
  (`casual`, `standard`, `blitz` oder `untimed`). Jeder Spieler wählt sein
  eigenes Deck und meldet sich *bereit*, dann startet der Gastgeber die
  Partie. Nach dem Ende kann der Tisch gleich noch einmal spielen.
- **Am Tisch.** Die Engine fragt, du antwortest. Du kannst mit der Maus oder
  komplett mit der Tastatur spielen:

| Taste | Wirkung |
| --- | --- |
| <kbd>Leertaste</kbd> | Bestätigen / Priorität abgeben |
| <kbd>Enter</kbd> | Die Karte unter dem Cursor spielen, sonst abgeben |
| <kbd>W</kbd> <kbd>A</kbd> <kbd>S</kbd> <kbd>D</kbd>, <kbd>E</kbd> | Karten-Cursor bewegen; die Karte darunter spielen oder wählen |
| <kbd>Esc</kbd> | Abbrechen (Schritt für Schritt) |
| <kbd>Tab</kbd> / <kbd>⇧ Tab</kbd> | Zur nächsten Phase / zum nächsten Zug vorspulen (Entscheidungen bleiben deine) |
| <kbd>F6</kbd> / <kbd>F7</kbd> | Stapel verrechnen lassen / in diesem Zug nichts mehr |
| <kbd>K</kbd> / <kbd>B</kbd>, <kbd>Y</kbd> / <kbd>N</kbd> | Mulligan: behalten / Karte nach unten legen; ja / nein |
| <kbd>T</kbd>, <kbd>Cmd</kbd>/<kbd>Alt</kbd> halten | Kartentext statt Bild (umschalten / nur solange gehalten) |
| <kbd>G</kbd>, <kbd>L</kbd> | Zonenansicht (Friedhöfe, Exil, Stapel), Spielprotokoll |
| <kbd>F8</kbd> | Einen Fehler melden |

Jede Taste lässt sich in den Einstellungen neu belegen, und ein angemeldetes
Konto speichert seine Belegung auf dem Gateway. Die vollständige Liste, auch
für Kampf und Fähigkeitenblatt, steht in
[docs/keyboard-map.md](docs/keyboard-map.md).

## Fehler melden

Drück <kbd>F8</kbd> am Tisch oder in der Lobby, oder nimm den Melde-Knopf im
Menü. Die Meldung geht an das Gateway, an dem du angemeldet bist. Sie enthält
immer die Art der Meldung, deinen Text, die Client-Version und, am Tisch, die
Kennung der Partie. Das Gateway legt seine eigene Aufzeichnung dieser Partie
bei; darin sind die Plätze nummeriert, und kein Spieler wird genannt.

Alles Weitere ist **freiwillig, mit je einem eigenen Kästchen**, und jedes
Kästchen ist anfangs leer: System und Hardware, der Tisch aus deiner Sicht,
dein Spielprotokoll (mit den Mitspielern als „Player A“, „Player B“, …),
deine Einstellungen und ein Bildschirmfoto. Sitzungsschlüssel werden nie
gesendet. Nach einem Absturz fragt der nächste Start einmal, ob
Absturzberichte gesendet werden dürfen. Bist du an keinem Gateway angemeldet,
wird nichts gesendet.

Was Client, Gateway und Meldedienst speichern, steht vollständig in
[docs/privacy.md](docs/privacy.md).

## Ein eigenes Gateway betreiben

Ein Gateway ist der Server, an dem sich Spieler anmelden. Eine Partie läuft
über drei Arten von Prozessen:

- **`baylee-gateway`** verwaltet Konten, Decks und Räume und vermittelt
  zwischen Spielern und Partien. Es führt keine Regeln aus und braucht
  PostgreSQL.
- **`baylee-agent`** startet auf Anfrage des Gateways für jede Partie einen
  Engine-Prozess. Auch er kennt keine Regeln.
- **`baylee-engine-server`** ist eine Partie. Er verbindet sich von sich aus
  mit dem Gateway.

Dieser Abschnitt ist eine Anleitung. Verbindlich sind
[docs/protocol.md](docs/protocol.md) und der Abschnitt *Environment* in
[CLAUDE.md](CLAUDE.md#environment).

### 1. Datenbank

```bash
docker compose up -d          # PostgreSQL 18 aus compose.yaml
export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee
```

Ändere das Passwort für alles, was von außen erreichbar ist. Das Gateway legt
seine Tabellen selbst an und migriert sie.

### 2. Bauen und starten

```bash
cargo build --release -p baylee-gateway -p baylee-agent -p baylee-engine-server

export BAYLEE_AGENT_TOKEN=$(openssl rand -hex 32)   # gemeinsam für Gateway und Agent
RUST_LOG=info BAYLEE_GATEWAY_NAME="Mein Tisch" ./target/release/baylee-gateway   # lauscht auf 0.0.0.0:28766
RUST_LOG=info ./target/release/baylee-agent        # findet baylee-engine-server neben sich
```

Ohne `RUST_LOG=info` protokollieren die Server nichts. Ist kein Agent
verbunden, kann das Gateway zwar Räume anzeigen, aber keine Partie starten
(`503`).

Laufen Gateway und Agent auf derselben Maschine, können sie statt über TCP
über einen **Unix-Socket** sprechen. Setze dazu
`BAYLEE_UNIX_SOCKET=/run/baylee/gateway.sock` beim Gateway und
`BAYLEE_GATEWAY=unix:/run/baylee/gateway.sock` beim Agenten. Siehe
[docs/protocol.md § On the same machine](docs/protocol.md#on-the-same-machine-the-unix-socket).

Stell `baylee-engine-server` nie allein ins Netz: Ohne die Argumente des
Gateways ist er ein Entwicklungswerkzeug ohne jede Anmeldung.

### 3. Wichtige Einstellungen

| Variable | Wirkung |
| --- | --- |
| `DATABASE_URL` | Pflicht. |
| `PORT` | Port, auf dem das Gateway lauscht (Standard `28766`). |
| `BAYLEE_AGENT_TOKEN` | Das Geheimnis des Agenten. Ohne es kann sich kein Agent verbinden. |
| `BAYLEE_GATEWAY_NAME` | Der Name, den Clients für das Gateway anzeigen. |
| `BAYLEE_REGISTRATION` | Nicht gesetzt: offene Registrierung. `invite`: geschlossene Beta mit Schlüsseln. `off`: keine Registrierung. |
| `BAYLEE_GUESTS=off` | Keine Gastkonten. `BAYLEE_GUEST_CAP` begrenzt die Zahl aktiver Gäste (Standard 1000). |
| `BAYLEE_TRUSTED_PROXIES` | Adresse deines Reverse-Proxys, damit die Anfragebegrenzung die echte IP des Clients sieht. |
| `BAYLEE_ART_PATH` / `BAYLEE_DECK_IMAGE_PATH` | Wo Kartenbilder zwischengespeichert und hochgeladene Hüllen/Matten abgelegt werden (`off` schaltet sie ab). |
| `BAYLEE_ENGINE_URL` | Setzen, wenn der Agent auf einer anderen Maschine läuft. |
| `BAYLEE_SOURCE_URL` | Wo der Quellcode deiner Version liegt, falls du ihn geändert hast (AGPL §13). |
| `BAYLEE_FEEDBACK_URL`, `…_TOKEN`, `…_KEY` | Optional: <kbd>F8</kbd>-Meldungen an einen Meldedienst weiterleiten. |

Das Gateway beantwortet `GET /source` ohne Anmeldung mit Lizenz, Version und
Commit. Die AGPL verlangt, dass du den Nutzern deines Servers den Quellcode
anbietest, und diese Route erledigt das für dich.

### 4. Beta-Schlüssel

Mit `BAYLEE_REGISTRATION=invite` erzeugst du Schlüssel auf dem Server:

```bash
baylee-gateway invite create --uses 1 --expires 30d --note "Max"
baylee-gateway invite list
baylee-gateway invite revoke <id>
```

Der Befehl braucht nur `DATABASE_URL`; das Gateway muss dafür nicht laufen.
Jeder Schlüssel wird einmal angezeigt, gespeichert wird nur sein Hash. Auf
einem Server, der mit `scripts/server/baylee-deploy` eingerichtet wurde, liest
`sudo baylee-invite …` die Einstellungen des Gateways für dich.

### 5. Kartentext (optional)

Auch ohne Katalog laufen Partien: Die Karten zeigen ihren englischen
Oracle-Text, der in die Programme eingebaut ist. Für durchsuchbaren
Kartentext in 19 Sprachen lädst du Scryfalls Massendaten in dieselbe
Datenbank:

```bash
RUST_LOG=baylee_catalog=info cargo run --release -p baylee-catalog -- ingest                 # alle Sprachen, ~390 MB Download
RUST_LOG=baylee_catalog=info cargo run --release -p baylee-catalog -- ingest --english-only  # ~80 MB
```

### 6. HTTPS davor

Das Gateway spricht einfaches HTTP und WebSocket, also gehört ein TLS-Proxy
davor. Das Repository bringt keine Proxy-Konfiguration mit; dieses minimale
Beispiel für [Caddy](https://caddyserver.com/) ist ein Ausgangspunkt:

```caddyfile
baylee.example.org {
    reverse_proxy 127.0.0.1:28766
}
```

Caddy besorgt ein Zertifikat, reicht WebSockets durch und setzt
`X-Forwarded-For`. Setze dann `BAYLEE_TRUSTED_PROXIES=127.0.0.1` beim
Gateway. Das Gateway authentifiziert mit Bearer-Tokens und setzt keine
Cookies, weitere CORS- oder Cookie-Einstellungen sind also nicht nötig.

### 7. Fehlermeldungen (optional)

`baylee-feedback` ist ein eigener Dienst. Er sammelt die Meldungen, die
Gateways weiterleiten, und hat eine kleine Weboberfläche zum Lesen. Die
Einrichtung beschreibt
[docs/feedback.md § Running it](docs/feedback.md#running-it). Ohne ihn lehnt
das Gateway Meldungen mit „reports are not configured“ ab.

## Aus dem Quellcode bauen

Du brauchst **Rust stable, 1.95 oder neuer** (`rust-toolchain.toml` legt den
stabilen Kanal mit `rustfmt`, `clippy` und dem Ziel `wasm32-unknown-unknown`
fest). Unter Linux brauchst du außerdem
`libasound2-dev libudev-dev libwayland-dev`.

```bash
git clone https://github.com/AceVik/baylee.git
cd baylee
cargo run --release -p baylee-client          # das Spiel; offline braucht es nichts weiter
```

- **Browser-Version:** `trunk serve index.html --release` in
  `crates/baylee-client/`. Immer mit `--release` bauen: Ein Debug-Wasm ist
  rund 350 MB groß.
- **Android / iOS-Simulator:** `scripts/mobile/android-build.sh`,
  `scripts/mobile/ios-sim-run.sh` ([docs/mobile.md](docs/mobile.md)).
- **Ein lokaler Tisch mit mehreren Plätzen:** Gateway und Agent starten (siehe
  oben), dann `cargo run -p xtask -- dev-table --seats 4 --ai sharp --play`.

Für Mitwirkende die Prüfungen, die auch die CI ausführt:

```bash
cargo fmt --all
./scripts/gate-rules.sh                        # schnell: die Regel-Crates, ohne Client
DATABASE_URL=… ./scripts/gate.sh               # die volle Prüfung: fmt, clippy, nextest, validate
DATABASE_URL=… ./scripts/gate-features.sh      # die Builds mit Nicht-Standard-Features
```

## Wie alles zusammenhängt

```mermaid
flowchart LR
  cardtext[baylee-cardtext] --> core[baylee-core]
  core --> engine[baylee-engine]
  engine --> gamehost[baylee-gamehost]
  view[baylee-view] --> gamehost
  ai[baylee-ai] --> gamehost
  gamehost --> engineserver[baylee-engine-server]
  core --> clientcore[baylee-client-core]
  view --> clientcore
  protocol[baylee-protocol] --> clientcore
  clientcore --> client[baylee-client<br/>Bevy]
  protocol --> gateway[baylee-gateway]
  protocol --> agent[baylee-agent]
  db[baylee-db] --> gateway
  catalog[baylee-catalog] --> gateway
```

Jeder Pfeil nimmt eine Fähigkeit weg. Die **Engine** ist synchron und rein:
kein I/O, kein async, keine Uhr. **gamehost** baut aus ihr die Sicht jedes
Platzes, und diese Sicht verbirgt, was der Platz nicht sehen darf.
**client-core** ist die Logik des Clients ohne Grafik und enthält die meisten
Client-Tests. **Gateway** und **Agent** binden überhaupt keine Regeln ein.
Karten sind Rust-Code in einer kleinen DSL, eine Datei pro Karte unter
`crates/baylee-cards/src/cards/`.

Zum Weiterlesen (die Dokumentation ist englisch):

| Thema | Dokument |
| --- | --- |
| Gesamtplan | [docs/architecture.md](docs/architecture.md) |
| Engine-Interna (Ebenen, Ereignisse, Cleanup, Schleifen) | [docs/engine-internals.md](docs/engine-internals.md) |
| Protokoll, Gateway, Räume, Schlüssel | [docs/protocol.md](docs/protocol.md) |
| Der Client | [docs/client.md](docs/client.md) |
| Karten schreiben | [docs/card-dsl.md](docs/card-dsl.md), [docs/card-identity.md](docs/card-identity.md) |
| Haus-KI | [docs/house-ai.md](docs/house-ai.md) |
| Datenschutz | [docs/privacy.md](docs/privacy.md) |
| Rechtliches | [docs/legal.md](docs/legal.md) |
| Veröffentlichen | [docs/releasing.md](docs/releasing.md) |

## Mitmachen

Issues und Pull Requests sind auf
[GitHub](https://github.com/AceVik/baylee) willkommen. Lies vorher
[AGENTS.md](AGENTS.md) und [CLAUDE.md](CLAUDE.md): Das sind die
Arbeitsregeln des Projekts, für Menschen und Coding-Agenten gleichermaßen. Ein
paar Punkte daraus:

- Lass vor dem Push die Prüfungen laufen. Die CI baut zusätzlich für Windows,
  macOS, wasm32 und die kleinste unterstützte Rust-Version.
- Jede Karte, die du hinzufügst oder reparierst, bekommt einen Test, der sie
  spielt, in `crates/baylee-engine/src/engine/card_tests/`.
- Die Engine bleibt deterministisch: kein `std::time`, kein Zufall außer dem
  Generator mit festem Startwert, keine Hash-Map-Iteration in heißen Pfaden.
- Nichts, was eine Karte, ein Symbol, eine Schrift oder einen Klang zeigt,
  wird gemergt, ohne es gegen [docs/legal.md](docs/legal.md) zu prüfen.

## Lizenz und Rechtliches

Baylees Quellcode steht unter der
**[GNU Affero General Public License v3.0 only](LICENSE)**. Wer ein
verändertes Gateway für andere betreibt, muss ihnen dessen Quellcode
anbieten; die Route `GET /source` des Gateways erledigt das. Das Projekt ist
kostenlos und nicht kommerziell, und keine Funktion wird je verkauft.

**Fan-Content.** Der Hinweis, den die Fan Content Policy verlangt, steht hier
im englischen Wortlaut, so wie ihn auch der Client zeigt:

> Baylee is unofficial Fan Content permitted under the
> [Fan Content Policy](https://company.wizards.com/en/legal/fancontentpolicy).
> Not approved/endorsed by Wizards. Portions of the materials used are
> property of Wizards of the Coast. ©Wizards of the Coast LLC.

Baylee steht in keiner Verbindung zu Wizards of the Coast, und dieses
Repository enthält kein Material von Wizards of the Coast.

**Scryfall.** Kartendaten und -bilder stellt
[Scryfall](https://scryfall.com) bereit. Der Client lädt die Kartenbilder
beim Spielen von Scryfall; sie gehören nicht zu diesem Repository und werden
nicht weiterverbreitet. Deshalb zeigen die Bilder oben Baylees eigene
Textansicht: Sie zeichnet eine Karte aus Name und Regeltext in unserem
eigenen Layout, ohne Rahmengrafik und ohne das Bild einer gedruckten Karte
([docs/legal.md](docs/legal.md), Abschnitte 2 und 3). Baylee steht in keiner
Verbindung zu Scryfall.

**Fremdes Material.** [NOTICE](NOTICE) führt die mitgelieferten Schriften auf
(Alegreya Sans, Faustina, Font Awesome Free und die Mana-Schrift, alle unter
SIL OFL 1.1); die CC0-Instrumentenaufnahmen, mit denen die Musik gespielt
wird, behandelt [docs/legal.md](docs/legal.md) §5. NOTICE nennt außerdem das
externe, GPL-lizenzierte Korpus von Kartenskripten, das die Codegenerierung
aus einer lokalen Kopie des Entwicklers lesen darf. Keine Datei dieses Korpus
wird je in dieses Repository kopiert oder mit einem Build ausgeliefert.
