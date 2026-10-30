<p align="center">
  <a href="README.md">English</a> · <b>Deutsch</b>
</p>

<p align="center">
  <img src="docs/images/readme/hero.webp" alt="Baylees Eingang: die Anmeldung an einem Gateway in einem mondbeschienenen Garten mit Laternen und Glyzinien, der sich dahinter sacht bewegt" width="100%">
</p>

<h1 align="center">Baylee</h1>

<p align="center">
  Ein kostenloser 3D-Spieltisch für Magic: The Gathering – mit Freunden, gegen die Haus-KI oder gegen ein Sprachmodell,<br>
  mit einer Regel-Engine, die die Regeln kennt, damit niemand darüber streiten muss.<br>
  Quelloffen (AGPL-3.0), inoffizieller Fan-Content.
</p>

<p align="center">
  <a href="https://baylee.app/?lang=de"><b>Website</b></a> ·
  <a href="https://github.com/AceVik/baylee/releases"><b>Download</b></a> ·
  <a href="https://discord.gg/4pP3uAwgY"><b>Discord</b></a> ·
  <a href="https://github.com/AceVik/baylee-central"><b>Accounts (Baylee Central)</b></a>
</p>

<p align="center">
  <a href="https://github.com/AceVik/baylee/actions/workflows/ci.yml"><img alt="ci" src="https://github.com/AceVik/baylee/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://github.com/AceVik/baylee/releases"><img alt="neueste Vorabversion" src="https://img.shields.io/github/v/release/AceVik/baylee?include_prereleases&label=pre-release"></a>
  <a href="LICENSE"><img alt="Lizenz: AGPL-3.0-only" src="https://img.shields.io/badge/license-AGPL--3.0--only-blue"></a>
</p>

> [!NOTE]
> **Baylee ist in der geschlossenen Beta.** Diese Seite beschreibt Version
> **0.1.0-beta.8b**, eine Vorabversion für **Windows-, macOS- und
> Linux-Rechner**. Auf dem öffentlichen Server **Baylee Sanctuary** unter
> `https://gateway.baylee.app` spielst du mit einem Baylee-Account, und den
> legst du auf [baylee.app](https://baylee.app/register) mit einem
> Beta-Schlüssel an. Ohne Schlüssel kannst du trotzdem offline gegen die
> Haus-KI spielen, Decks bauen oder für deine Runde einen eigenen Server
> betreiben.

## Inhalt

- [Was Baylee ist](#was-baylee-ist)
- [Neu in beta.8](#neu-in-beta8)
- [Bilder](#bilder)
- [Installation](#installation)
- [Dein erstes Spiel](#dein-erstes-spiel)
- [Mit Freunden spielen](#mit-freunden-spielen)
- [Freunde und Chat](#freunde-und-chat)
- [Replays](#replays)
- [Gegen ein Sprachmodell spielen](#gegen-ein-sprachmodell-spielen)
- [Fehler melden](#fehler-melden)
- [Wo die Karten stehen](#wo-die-karten-stehen)
- [Ein eigenes Gateway betreiben](#ein-eigenes-gateway-betreiben)
- [Aus dem Quellcode bauen](#aus-dem-quellcode-bauen)
- [Wie alles zusammenhängt](#wie-alles-zusammenhängt)
- [Mitmachen](#mitmachen)
- [Lizenz und Rechtliches](#lizenz-und-rechtliches)

## Was Baylee ist

Baylee ist ein Programm, mit dem du Magic: The Gathering am Rechner spielst.
Du startest es, wählst ein Deck und setzt dich an einen Tisch: allein gegen
den Computer oder mit Freunden über das Internet.

**Das Spiel kennt die Regeln.** Baylee hat eine Regel-Engine, die den
Comprehensive Rules von Magic folgt: Priorität und Stapel, Auslöser,
Ersatzeffekte, Ebenen, Kampf und die vielen kleinen Prüfungen dazwischen. In
jedem Moment sagt sie, was du tun darfst, und der Tisch bietet genau das an.
Niemand muss etwas nachschlagen oder diskutieren. Du spielst mit der Maus
oder ganz mit der Tastatur.

**Es ist von Grund auf fair.** Dein Rechner bekommt nur, was du sehen darfst.
Die Hände deiner Gegner und die Bibliotheken kommen als Anzahl an, nie als
Karten. Ein verändertes Programm kann also nicht spicken: Die Information ist
schlicht nicht da.

Was du damit machen kannst:

- **Offline spielen** gegen die Haus-KI, in fünf Stufen von *Anfänger* bis
  *Experte*, ohne Account und ohne Internet.
- **Online spielen** auf einem *Gateway* (einem Baylee-Server): 2 bis 8
  Spieler an einem Tisch, jeder gegen jeden oder in Teams (auch ungleiche wie
  3 gegen 2), mit oder ohne KI-Spieler auf den freien Plätzen. Ein Deck mit
  Commander spielt Commander, jedes andere Deck spielt frei.
- **Einmal anmelden** mit einem Baylee-Account: Er bringt deine Gateways und
  Einstellungen mit und sichert deine Chat-Schlüssel verschlüsselt, sodass
  ein anderer Rechner dort weitermacht, wo der letzte aufgehört hat.
- **Freunde hinzufügen und mit ihnen chatten**, Ende-zu-Ende-verschlüsselt,
  oder mit allen in der Lobby reden.
- Beim laufenden Spiel eines Freundes **zuschauen** und **eigene Spiele als
  Replay noch einmal ansehen**.
- **Ein Sprachmodell an den Tisch setzen**, mit deinem eigenen Schlüssel
  oder eines, das der Betreiber des Servers anbietet.
- **Decks bauen** mit einer Suche über alle Karten des Pools, Manakurve und
  Länderstatistik, und sie in gängigen Textformaten importieren und
  exportieren.
- **Kartentexte in 19 Sprachen lesen**, wenn das Gateway den Kartenkatalog
  hat. Das Programm selbst spricht Deutsch und Englisch.

Baylee ist kostenlos und bleibt es: keine Bezahlschranke, keine Werbung,
nichts zu kaufen. Es ist ein nicht kommerzielles Fanprojekt und steht in
keiner Verbindung zu Wizards of the Coast.

## Neu in beta.8

Was sich seit beta.7 geändert hat, in 0.1.0-beta.8a und in dieser Version,
0.1.0-beta.8b:

- **Baylee-Accounts.** Die Anmeldung zieht zu **Baylee Central** um, dem
  Account-Dienst des Projekts. Auf Baylee Sanctuary meldest du dich mit
  einem Baylee-Account an, den du auf der Website anlegst,
  [baylee.app](https://baylee.app/register); der Client holt danach deine
  Gateway-Liste und deine Einstellungen zurück und bringt dich zu dem
  Gateway, das du wählst ([Dein erstes Spiel](#dein-erstes-spiel)).
- **Erst die Bedingungen von Central.** Vor dem ersten Gateway zeigt der
  Client die Nutzungsbedingungen von Baylee Central. Deine Zustimmung bleibt
  bei deinem Account, ein anderer Rechner fragt also nicht noch einmal.
- **Eine Sitzung hält, solange du spielst.** Sie verlängert sich selbst,
  solange Baylee läuft, bis zu 12 Stunden nach der Anmeldung mit deinem
  Passwort. In beta.8a endete sie nach 30 Minuten und jede Partie darauf
  mit ihr.
- **Deine Chat-Schlüssel in einem Tresor.** Auf deinem Rechner schließt dein
  Account-Passwort die Chat-Schlüssel ab (*Chat entsperren*), und mit einem
  Baylee-Account werden sie und dein Chatverlauf verschlüsselt gesichert,
  sodass ein neuer Rechner sie zurückbekommt
  ([Einzelheiten](#freunde-und-chat)).
- **Ein Lobby-Chat.** Der Reiter *Lobby* im Chatfenster spricht mit allen
  auf dem Gateway. Er ist öffentlich und nicht verschlüsselt, anders als
  Flüstern, Gruppen und Tischchat.
- **Die Update-Karte.** Ein Update zeigt seinen Download jetzt, während er
  läuft (Anteil, Megabyte, Geschwindigkeit und Restzeit), danach die Prüfung
  seiner Signatur, und bietet **Jetzt neu starten** an: Baylee startet in
  der neuen Version neu und macht dort weiter, wo du warst, auch mitten in
  einer Partie. *Später* installiert es beim Beenden
  ([Aktualisieren](#aktualisieren)).
- **Eine neue Kopfleiste in der Lobby**, mit dem Gateway, deinen Freunden,
  der Glocke für Benachrichtigungen und deinem Account; die Hinweise der
  Lobby stehen oben links.
- **Zurück nach einem Abbruch.** Einen Tisch, zu dem die Verbindung
  abgerissen ist, wählt der Client so lange neu an, bis das
  Wiederverbindungsfenster des Tisches vorbei ist (drei Minuten, wenn der
  Tisch nichts anderes sagt), und sagt dann, ob deine Anmeldung abgelaufen
  ist oder der Tisch nicht zu erreichen war.
- **Kosten im Bezahlfenster.** Kumulative Versorgung (Ice Age) und
  Angriffssteuern bezahlst du jetzt auch dort, mit vorgemerkten Ländern,
  bevor irgendetwas getappt wird.
- **Behoben nach euren Berichten.** Unter den Problemen, die Spieler mit
  <kbd>F8</kbd> geschickt haben: Ein Sprachmodell-Platz, der kurz abgewiesen
  wurde, wählt sich neu ein, statt zu gehen; die Hinweise der Lobby stehen
  nicht mehr über einer Partie; ein Duell, das das Haus für einen
  gegangenen Spieler zu Ende spielt, endet mit einem Ergebnis; ein
  überladener Cyclonic Rift lässt die Permanents eines Teamkameraden in
  Ruhe; eine Kreatur auf einem Abenteuer lässt sich aus dem Exil wirken;
  und Züge mit Tausenden von Auslösern laufen um ein Vielfaches schneller.
- **6.000 Karten mehr spielen genau wie gedruckt** (3.202 in beta.7, jetzt
  9.265): The Dark und Fallen Empires sind vollständig, und Ice Age ist zur
  Hälfte fertig ([Wo die Karten stehen](#wo-die-karten-stehen)).

Diese Version ist nur für Desktop-Rechner. Die Browser-Version und die
Handy-Apps gehören nicht zu beta.8b ([docs/mobile.md](docs/mobile.md) sagt,
wo sie stehen).

## Bilder

Die Bilder sind kurze Aufnahmen des laufenden Spiels, mit den Kartenbildern
von Scryfall, so wie das Spiel sie zeichnet. Die Aufnahmen am Tisch, das
Replay und der Deckbauer entstanden mit 0.1.0-beta.8b an einem lokalen
Test-Gateway, mit ausgedachten Spielern und Decks aus Magics eigenen Sets;
Eingang, Lobby, Tischeinrichtung und Update-Karte zeigen keine Karten und
stammen aus den Aufnahmen kurz vor beta.8b (die Oberfläche ist hier
englisch; sie spricht auch Deutsch).

<table>
  <tr>
    <td width="50%"><img src="docs/images/readme/lobby.webp" alt="Die Lobby: Ein Flüstern und eine Tischeinladung einer Freundin kommen an, dann öffnet sich der private Chat"><br><sub><b>Die Lobby, mit Freunden und Chat.</b> Eine Freundin flüstert dir zu, dass an ihrem Tisch ein Platz frei ist, ihre Einladung mit <i>Join</i> folgt; die Freundesleiste zeigt, wer online ist und an welchem Tisch, und das Flüstern ist Ende-zu-Ende-verschlüsselt.</sub></td>
    <td width="50%"><img src="docs/images/readme/table.webp" alt="Ein Duell: Ein Land wird gespielt, Raging Goblin und Storm Seeker gehen auf den Stapel, die Haus-KI wird als Ziel gewählt, und das Protokoll füllt sich"><br><sub><b>Am Tisch.</b> Ein Land, dann geht Raging Goblin auf den Stapel, und Storm Seeker folgt als Antwort; sein Ziel wählst du in einem Blatt, das die Karte zeigt, und das Protokoll neben dem Tisch schreibt jeden Zug mit, jede Karte ein Link.</sub></td>
  </tr>
  <tr>
    <td width="50%"><img src="docs/images/readme/combat.webp" alt="Kampf: sechs Angreifer, drei davon von zwei Mauern und einem Engel geblockt, und die geblockten gehen auf den Friedhof"><br><sub><b>Kampf.</b> Sechs Kreaturen greifen an, drei werden geblockt; die Linien zeigen, wer gegen wen kämpft. Wall of Spears schlägt zuerst zu, neun Schaden kommen durch, und die drei geblockten Kreaturen gehen auf den Friedhof.</sub></td>
    <td width="50%"><img src="docs/images/readme/room.webp" alt="Ein Raum wird eingerichtet: Drei freie Plätze gehen an die Haus-KI in drei Stufen"><br><sub><b>Einen Tisch einrichten.</b> Der Gastgeber gibt drei freie Plätze der Haus-KI, jeden in einer eigenen Stufe; einen Platz kann auch ein anderer Spieler oder ein Sprachmodell nehmen.</sub></td>
  </tr>
  <tr>
    <td width="50%"><img src="docs/images/readme/payment.webp" alt="Das Bezahlfenster: Die kumulative Versorgung von zwei wird mit zwei vorgemerkten Ländern bezahlt"><br><sub><b>Das Bezahlfenster.</b> Die kumulative Versorgung von Cold Snap verlangt zwei Mana: Angeklickte Länder werden vorgemerkt, getappt wird erst mit <i>Pay</i>, und <i>Untap all</i> nimmt alles zurück.</sub></td>
    <td width="50%"><img src="docs/images/readme/replay.webp" alt="Ein Replay läuft mit doppelter Geschwindigkeit, dann öffnet sich das Menü Watch from"><br><sub><b>Replays.</b> Eine gespeicherte Partie gegen das Haus läuft mit 2×, mit Stapel, Angriff und Blocks; <i>Your seat</i> wählt, aus wessen Sicht du zusiehst: deiner, der eines anderen Spielers, des Tisches oder aller Hände.</sub></td>
  </tr>
  <tr>
    <td width="50%"><img src="docs/images/readme/deck-builder.webp" alt="Der Deckbauer: Die Suche angel serra engt den Pool auf drei Karten ein, Serra Angel wird groß gezeigt und kommt ins Deck"><br><sub><b>Der Deckbauer.</b> Die Suche <i>angel</i> lässt von einem Pool aus 9.343 Karten 92 spielbare übrig, und <i>serra</i> dahinter engt sie auf drei ein; zeigst du auf Serra Angel, siehst du sie groß, und <i>+</i> legt sie ins Deck, nach Typ gruppiert.</sub></td>
    <td width="50%"><img src="docs/images/readme/update.webp" alt="Die Update-Karte: ein Download mit Anteil, Geschwindigkeit und Restzeit, dann Restart to update"><br><sub><b>Aktualisieren.</b> Die Update-Karte zeigt den Download, während er läuft, dann Prüfung und Entpacken, und bietet <i>Restart to update</i> an (hier ein simulierter Download von beta.8b).</sub></td>
  </tr>
  <tr>
    <td colspan="2"><img src="docs/images/readme/keyword-borders.webp" alt="Sechs Kreaturen auf dem Spielfeld, jede mit eigenem bewegtem Rand"><br><sub><b>Auf einen Blick.</b> Von links: indestructible (ein goldener Rand mit einem Glanzlicht), shroud (violetter Nebel), hexproof (eine blaue Linie mit fließenden Schlieren), defender mit hexproof (Steinblöcke und die blaue Linie), defender mit shroud (Stein in violettem Nebel) und defender (Steinblöcke).</sub></td>
  </tr>
</table>

<p align="center"><sub>Kartenbilder: Scryfall · © Wizards of the Coast</sub></p>

## Installation

Lade Baylee für dein System von der
**[Release-Seite](https://github.com/AceVik/baylee/releases)** herunter (dort
stehen auch die Vorabversionen) oder von der
[Website](https://baylee.app/download?lang=de), die die passende Datei
für dich auswählt. Für jedes System gibt es einen **Installer**, der einfachste
Weg, und ein **Archiv** mit demselben Spiel zum Entpacken an beliebiger
Stelle. Danach aktualisiert Baylee sich selbst
([Aktualisieren](#aktualisieren)).

| System | Installer | Archiv |
| --- | --- | --- |
| Windows, Intel/AMD 64 Bit | `Baylee-Setup-<version>-x64.exe` | `baylee-client-<version>-x86_64-pc-windows-msvc.zip` |
| Windows auf ARM | `Baylee-Setup-<version>-arm64.exe` | `baylee-client-<version>-aarch64-pc-windows-msvc.zip` |
| macOS, Apple Silicon (M1 und neuer) | `Baylee-<version>-aarch64.dmg` | `baylee-client-<version>-aarch64-apple-darwin.zip` |
| macOS, Intel | `Baylee-<version>-x86_64.dmg` | `baylee-client-<version>-x86_64-apple-darwin.zip` |
| Linux, x86-64 | `Baylee-<version>-x86_64.AppImage` oder `baylee_<version>_amd64.deb` | `baylee-client-<version>-x86_64-unknown-linux-gnu.tar.gz` |
| Linux, ARM64 | `Baylee-<version>-aarch64.AppImage` oder `baylee_<version>_arm64.deb` | `baylee-client-<version>-aarch64-unknown-linux-gnu.tar.gz` |

Jedes Archiv enthält das Spiel, seinen Ordner `assets`, `LICENSE`, `NOTICE`
und eine `README.txt` mit diesen Schritten für den ersten Start.

Die Programme sind **nicht signiert**, weil eine Signatur Geld kostet.
Windows und macOS warnen deshalb beim ersten Start. Wie du an der Warnung
vorbeikommst, steht in den Schritten unten.

### Windows

1. Starte `Baylee-Setup-<version>-x64.exe` (bzw. `-arm64` unter Windows auf
   ARM).
2. Meldet Microsoft Defender SmartScreen „Der Computer wurde durch Windows
   geschützt“, klicke auf **Weitere Informationen** und dann auf
   **Trotzdem ausführen**.
3. Das Setup installiert Baylee nur für deinen Benutzer, in
   `%LOCALAPPDATA%\Programs\Baylee`, und fragt nach keinen
   Administratorrechten. Es legt Baylee ins Startmenü und, wenn du das
   Häkchen setzt, auf den Desktop.

Entfernen kannst du es unter **Einstellungen → Apps**. Heruntergeladene
Updates und zwischengespeicherte Kartenbilder bleiben in
`%LOCALAPPDATA%\baylee`; lösche den Ordner mit, um alles zu entfernen.

Ohne Setup: Entpacke die ZIP-Datei in einen beliebigen Ordner, etwa
`Dokumente\Baylee`, lass den Ordner `assets` neben `baylee-client.exe` liegen
und starte `baylee-client.exe`.

Die x86-64-Version braucht eine CPU mit x86-64-v2 (SSE4.2, POPCNT); ohne
eine solche startet auch Windows 11 24H2 nicht.

### macOS

Macs mit Apple Silicon (M1 und neuer) nehmen die `aarch64`-Version,
Intel-Macs die `x86_64`-Version (Apple-Menü → Über diesen Mac nennt Chip
oder Prozessor). Voraussetzung ist macOS 11 oder neuer.

1. Öffne `Baylee-<version>-aarch64.dmg` (Intel: `-x86_64.dmg`) und ziehe
   **Baylee** auf **Programme** (Applications). Wirf das Image danach aus.
   Mit der ZIP-Datei entpackst du sie und ziehst `Baylee.app` ebenso nach
   „Programme“.
2. Öffne Baylee aus „Programme“. Beim ersten Mal lehnt macOS die App ab,
   weil sie nicht notarisiert ist.
3. Öffne **Systemeinstellungen → Datenschutz & Sicherheit**, scrolle zum
   Hinweis zu Baylee und klicke auf **Dennoch öffnen**.

Oder im Terminal:

```bash
xattr -dr com.apple.quarantine /Applications/Baylee.app
```

Startest du Baylee direkt aus dem Download (aus dem Image oder dem
entpackten Ordner in „Downloads“), führt macOS es aus einer versteckten,
schreibgeschützten Kopie aus. Baylee aktualisiert sich trotzdem, und die
Einstellungen bieten **In meinen Programme-Ordner verschieben** an: Baylee
kopiert sich nach `~/Applications` (ohne Passwort), prüft die Kopie, startet
sie und schließt sich. Beim ersten Start fragt die Kopie, ob die alte in den
Papierkorb soll; gelöscht wird nichts, ohne dass du es sagst.

### Linux

Das **AppImage** ist eine einzige Datei, die du direkt startest:

```bash
chmod +x Baylee-<version>-x86_64.AppImage
./Baylee-<version>-x86_64.AppImage
```

Das **.deb** (Debian, Ubuntu und Verwandte) installiert Baylee nach
`/opt/baylee`, mit Menüeintrag und dem Befehl `baylee`:

```bash
sudo apt install ./baylee_<version>_amd64.deb
```

Oder entpacke das Archiv:

```bash
tar -xzf baylee-client-<version>-x86_64-unknown-linux-gnu.tar.gz
cd baylee-client-<version>-x86_64-unknown-linux-gnu
./baylee-client
```

Du brauchst einen Vulkan-Treiber und die Laufzeitbibliotheken für ALSA, udev,
X11/Wayland und xkbcommon; das `.deb` nennt sie, `apt` installiert sie also
mit, und das AppImage nutzt wie das Archiv die des Systems. Unter Debian oder
Ubuntu sind das
`libasound2 libudev1 libxkbcommon-x11-0 libwayland-client0 libvulkan1`.
Gebaut wird auf Ubuntu 22.04; das Programm läuft auf jeder Distribution mit
glibc 2.35 oder neuer. Wie unter Windows braucht die x86-64-Version eine CPU
mit x86-64-v2.

### Download prüfen

Neben jedem Installer und jedem Archiv liegt auf der Release-Seite eine
`.sha256`-Datei, und die Release-Notizen nennen die Prüfsummen der Installer
auch. Lade die Datei und ihre `.sha256` in denselben Ordner und führe aus:

```bash
# Linux
sha256sum -c Baylee-<version>-x86_64.AppImage.sha256
# macOS (ein Intel-Mac: Baylee-<version>-x86_64.dmg.sha256)
shasum -a 256 -c Baylee-<version>-aarch64.dmg.sha256
```

Unter Windows führst du in der PowerShell
`certutil -hashfile Baylee-Setup-<version>-x64.exe SHA256` aus und
vergleichst das Ergebnis mit der Zahl in der `.sha256`-Datei.

### Aktualisieren

Baylee aktualisiert sich selbst. Es fragt GitHub beim Start und alle sechs
Stunden nach einer neueren Version, lädt sie herunter, prüft ihre Signatur
und installiert sie beim Beenden. Eine Karte unten links in der Lobby zeigt
den Download, während er läuft (Anteil, Megabyte, Geschwindigkeit und
Restzeit), danach Prüfung und Entpacken; ist das Update bereit, bietet sie
**Jetzt neu starten** an, das in der neuen Version neu startet und dort
weitermacht, wo du warst, auch mitten in einer Partie, oder **Später**, das
es beim Beenden installiert. Am Tisch erscheint die Karte erst, wenn das
Update bereit ist, nie mitten in einer Frage, und das Menü des Tisches sagt,
wie weit der Download ist. Der nächste Start sagt einmal „Aktualisiert auf
X“. Ein Update, dessen Signatur nicht stimmt, wird nie installiert. In
den Einstellungen gibt es zwei Schalter, „Automatisch aktualisieren“ und
„Automatisch nach Updates suchen“ (aus: Baylee fragt GitHub gar nichts, bis
du „Nach Updates suchen“ drückst); `docs/privacy.md` sagt, was GitHub dabei
sieht.

Updates liegen in deinem eigenen Zustandsordner (`~/.local/state/baylee`,
unter Windows `%LOCALAPPDATA%\baylee`) und werden nie in den Ordner des
Programms geschrieben. Darum aktualisieren sich die Windows-Installation,
eine Mac-App, egal wo sie liegt, und ein AppImage in einem eigenen Ordner
selbst. Gehört der Ordner des Programms jemand anderem (etwa `/opt/baylee`
aus dem `.deb`), fragt Baylee nie nach Administratorrechten: Es verlinkt die
Release-Seite, und du aktualisierst von Hand, indem du das neue Paket über
das alte installierst. Versionen bis 0.1.0-beta.5 aktualisierten sich auf
dem Mac nicht, solange macOS sie aus der schreibgeschützten Kopie startete:
Verschiebe so eine `Baylee.app` einmal von Hand nach „Programme“.

**Wo deine Sachen liegen.** Deine Einstellungen bleiben beim Ersetzen des
Programms erhalten, weil sie nicht daneben liegen: unter Windows in
`%APPDATA%\Baylee`, unter Linux und macOS in `~/.config/baylee/`
(`$XDG_CONFIG_HOME/baylee`, wenn du diese Variable setzt). Dort findest du

- `client-settings.json`: Sprache, deine Gateway-Liste, Einwilligungen für
  Berichte und die Sitzung eines Gastes;
- `preferences.json`: Tastenbelegung und stehende Antworten;
- `offline-decks.json`: offline gebaute Decks;
- `update.json`: die beiden Update-Schalter;
- `records/`: deine letzten 20 Spiele gegen das Haus, für Replays
  (`records/archive/` hält die archivierten);
- `vault/`: für jeden Account und jedes Gateway deine Chat-Schlüssel,
  abgeschlossen mit deinem Account-Passwort, und deinen Chatverlauf,
  verschlüsselt;
- `llm-seat.json` und `llm-spend.json`, sobald du ein Sprachmodell
  einrichtest.

Kartenbilder werden getrennt davon zwischengespeichert, in
`~/Library/Caches/baylee` (macOS), `%LOCALAPPDATA%\baylee` (Windows) oder
`~/.cache/baylee` (Linux). Ein Baylee-Account liegt bei Baylee Central, mit
deiner Gateway-Liste, deinen Einstellungen und einer Sicherung deiner
Chat-Schlüssel und deines Verlaufs, alles schon auf deinem Rechner
verschlüsselt, bevor es ihn verlässt; deine Online-Decks, deine Freunde und
deine Replays liegen auf dem Gateway. Beides findest du auf jedem Rechner
wieder, an dem du dich anmeldest.

Jede Version zeigt ihre Versionsnummer und ihren Commit in der Lobby. Eine
neue Version kann mit einem älteren Gateway unter Umständen nicht mehr
sprechen. Aktualisiere also, sobald eine neue Version erscheint.

## Dein erstes Spiel

Wenn Baylee startet, stehst du an seinem **Eingang**: einer Liste von
Gateways (Servern) und dem Weg, offline zu spielen. Beim ersten Mal führt
dich eine kurze Tour herum, und <kbd>?</kbd> zeigt auf jedem Bildschirm jede
Taste, die gerade wirkt.

**Offline, sofort.** Wähle **Offline spielen**. Du bekommst die Lobby mit
deinen Offline-Decks, den Deckbauer und **Gegen das Haus**, womit sofort ein
Spiel gegen die Haus-KI beginnt. Der kleine Pfeil am Knopf wählt, wie stark
die KI spielt.

**Online.** Wähle ein Gateway aus der Liste (**Baylee Sanctuary** steht
schon drin, weitere fügst du über ihre Adresse hinzu) und melde dich an.

**Auf Baylee Sanctuary** meldest du dich mit einem **Baylee-Account** an,
der bei Baylee Central liegt, dem Account-Dienst des Projekts. Du legst ihn
auf der Website an, unter [baylee.app/register](https://baylee.app/register?lang=de)
(die Anmeldung im Client verlinkt dorthin): ein *Benutzername* (3 bis 24
Zeichen: Buchstaben a–z, Ziffern und dazwischen `.`, `_` oder `-`) und ein
Passwort. Eine E-Mail-Adresse wird nicht verlangt. Den Benutzernamen siehst
nur du; auf einem Gateway sehen andere einen Anzeigenamen und ein kurzes
Kennzeichen, etwa `Alice#af03`. Die Anmeldung holt deine Gateway-Liste und
deine Einstellungen zurück und bringt dich zu dem Gateway, das du wählst;
die Sitzung verlängert sich selbst, solange Baylee läuft, bis zu 12
Stunden, danach meldest du dich neu an.

**Ein Account aus der Zeit vor beta.8** auf Baylee Sanctuary braucht keinen
neuen: Wenn du dich zum ersten Mal im Client damit anmeldest, mit demselben
Benutzernamen und Passwort, zieht er zu Baylee Central um, und deine Decks
und Einstellungen kommen mit. Unterwegs stimmst du den Nutzungsbedingungen
von Central zu.

Vor dem ersten Gateway zeigt der Client die **Nutzungsbedingungen von
Baylee Central**; deine Zustimmung bleibt beim Account, ein anderer Rechner
fragt also nicht noch einmal. Gelöscht wird ein Baylee-Account vorerst auf
Anfrage, über den Kontakt im Impressum der Website.

**Auf einem Gateway, das seine Spieler selbst anmeldet**, etwa einem, das
du oder ein Freund betreibt, fragt **Registrieren** nach Benutzername,
Anzeigename und Passwort, und **Als Gast spielen**, wo das Gateway es
anbietet, nur nach einem Anzeigenamen. Ein Gast kann Decks bauen und
spielen, aber keine Hüllen oder Spielmatten hochladen, und wird gelöscht,
wenn er sich abmeldet oder 30 Tage lang nicht benutzt wurde. Einen Account
auf so einem Gateway kannst du jederzeit im Client löschen; alles, was
dazugehört, geht mit.

Ein Gateway kann eigene **Nutzungsbedingungen** haben: Du siehst sie, wenn
du dich dort zum ersten Mal anmeldest, und wieder, wenn sie sich ändern.
Nennt der Betreiber ein Impressum, verlinkt der Client es unter *Über* am
Eingang, bei den Bedingungen und unter *Einstellungen → Netzwerk*.

### Zugang: Beta-Schlüssel

Baylee Sanctuary ist eine geschlossene Beta: **Ein neuer Baylee-Account
braucht einen Schlüssel** der Form `CENTRAL-XXXX-XXXX-XXXX-XXXX`, den du im
Anmeldeformular der Website eingibst. Schlüssel vergibt der Projektinhaber
([@AceVik](https://github.com/AceVik)). Mit einem Account meldest du dich
danach mit Benutzername und Passwort an und brauchst keinen Schlüssel mehr.
Ein Gateway, das jemand für seine eigene Runde betreibt, kann ebenfalls
eine geschlossene Beta sein, mit eigenen Schlüsseln
(`BAYLEE-XXXX-XXXX-XXXX-XXXX`, [unten](#4-beta-schlüssel)).

## Mit Freunden spielen

**Decks.** Im Deckbauer durchsuchst du den Pool (auf Wunsch nur Karten, die
genau wie gedruckt spielen), stellst Hauptdeck, Sideboard und Commander
zusammen, behältst die Manakurve im Blick und wählst für jede Karte einen
Druck. Ein Deck lässt sich als Text importieren und exportieren (Baylees
eigenes Format, JSON, YAML und Moxfields Textexport;
[docs/deck-format.md](docs/deck-format.md)). Das Gateway hebt jede Version
eines Online-Decks auf, und ein registrierter Account kann einem Deck eine
eigene Hülle und Spielmatte geben. Eine Karte, die der Betreiber des
Gateways gesperrt hat, ist markiert, mit dem Grund.

**Räume.** In der Lobby eröffnest du einen Tisch mit 2 bis 8 Plätzen und,
wenn du willst, einem Namen und einem Passwort. Als Gastgeber bestimmst du,
wer wo sitzt: ein anderer Spieler, die Haus-KI in einer ihrer fünf Stufen
oder ein Sprachmodell. Du legst auch die Teams fest, das Startleben, freie
Mulligans und die Uhr (`casual`, `standard`, `blitz` oder `untimed`), und ob
andere zuschauen dürfen. Jeder wählt sein Deck und sagt *Bereit*, und der
Gastgeber startet. Ist das Spiel vorbei, kann der Tisch noch einmal spielen.

**Am Tisch.** Das Spiel fragt, und du antwortest: Was du spielen kannst,
leuchtet, und die Leiste unten sagt, was gefragt ist. Im Team könnt ihr
einander eure Hände zeigen. Die wichtigsten Tasten:

| Taste | Bewirkt |
| --- | --- |
| <kbd>Leertaste</kbd> | Bestätigen / Priorität abgeben / bezahlen |
| <kbd>Enter</kbd> | Mit der Karte unter dem Cursor handeln, sonst passen |
| <kbd>W</kbd> <kbd>A</kbd> <kbd>S</kbd> <kbd>D</kbd>, <kbd>E</kbd> | Karten-Cursor bewegen; die Karte darunter spielen oder wählen |
| <kbd>Esc</kbd> | Abbrechen (Schritt für Schritt), eine aufgedeckte Karte wegklicken |
| <kbd>Tab</kbd> / <kbd>⇧ Tab</kbd> | Vorspulen bis zur nächsten Phase / zum nächsten Zug (Entscheidungen bleiben deine) |
| <kbd>F6</kbd> / <kbd>F7</kbd> | Stapel abhandeln lassen / in diesem Zug nichts mehr |
| <kbd>K</kbd> / <kbd>B</kbd>, <kbd>Y</kbd> / <kbd>N</kbd> | Mulligan: behalten / Karte unter die Bibliothek; ja / nein |
| <kbd>T</kbd>, <kbd>Cmd</kbd>/<kbd>Alt</kbd> halten | Kartentext statt Bild (umschalten / nur solange gedrückt) |
| <kbd>Shift</kbd> über einer Vorschau | Karte auf Rückseite oder andere Hälfte drehen |
| <kbd>G</kbd>, <kbd>L</kbd> | Zonen-Browser (Friedhöfe, Exil, Stapel), Spielprotokoll |
| <kbd>F8</kbd> | Fehler melden |
| <kbd>?</kbd> | Alle Tasten, die gerade wirken |

Jede Taste lässt sich in den Einstellungen neu belegen, und ein angemeldeter
Account speichert die Belegung mit dem Account. Die vollständige Liste steht
in [docs/keyboard-map.md](docs/keyboard-map.md).

**Zuschauen.** Lässt ein Tisch es zu, kann jeder, der am Gateway angemeldet
ist, in der Lobby *Zuschauen* drücken und das Spiel aus der öffentlichen
Sicht verfolgen: keine Hände, keine verdeckten Karten. Die Spieler sehen,
wie viele zuschauen.

**Bricht deine Verbindung ab**, wartet dein Platz auf dich: Der Client
verbindet sich von selbst neu und versucht es, bis das Fenster des Tisches
vorbei ist (drei Minuten, wenn der Tisch nichts anderes festlegt), und die
anderen sehen einen Countdown. Bist du nicht rechtzeitig zurück, spielt die
Haus-KI deinen Platz, bis du wieder da bist.

## Freunde und Chat

Auf einem Gateway mit Chat (Baylee Sanctuary hat ihn) zeigt die
**Freundesleiste** rechts in der Lobby deine Freunde und ob sie online sind,
spielen oder zuschauen. Einen Freund fügst du mit Name und Kennzeichen hinzu
(`Alice#af03`) oder aus den Spielern, mit denen du zuletzt gespielt hast.
Aus der Zeile eines Freundes heraus kannst du ihm zuflüstern, an seinem
Tisch zuschauen oder ihn an deinen einladen. Gruppen sind Chats für mehrere
Freunde. Am Tisch hat jeder, der dort sitzt, einen Tischchat, und jedes Team
seinen eigenen. Der Reiter *Lobby* ist ein Chat für alle auf dem Gateway.

**Was verschlüsselt ist.** Flüstern, Gruppen-, Tisch- und Teamnachrichten
sind **Ende-zu-Ende-verschlüsselt**: Sie werden auf dem Rechner des
Absenders für die Rechner der Personen im Gespräch verschlossen, und nur
diese können sie öffnen. Der Lobby-Chat ist es nicht: Er ist öffentlich,
das Gateway reicht ihn als Klartext weiter und behält seine letzten 50
Zeilen nur im Speicher, bis es neu startet. Ehrlich gesagt heißt die
Verschlüsselung:

- **Der Server kann deine Nachrichten nicht lesen.** Er leitet sie weiter
  und bewahrt sie verschlüsselt auf, bis sie zugestellt sind, höchstens 14
  Tage (der Betreiber kann 1 bis 30 wählen; bei einem Gast 1 Tag).
- **Der Server sieht aber**, wer wem wann schreibt, wie lang eine Nachricht
  ist, wer in einer Gruppe ist und wie sie heißt, wer mit wem befreundet ist
  und wer online ist und an welchem Tisch. Darüber führt er kein Protokoll.
- **Deine Geräte sind die Schwachstelle.** Eine Nachricht lässt sich auf
  jedem Gerät lesen, auf dem die Personen im Gespräch Baylee nutzen – solange
  diese Geräte wirklich ihre sind. Um sicherzugehen, vergleicht ihr die
  **Sicherheitsnummern** (die Schloss-Zeile oben im Chat öffnet sie). Wer
  deinen Rechner benutzen kann, solange der Chat entsperrt ist, kann den
  darauf gespeicherten Verlauf lesen.
- **Deine Schlüssel schließt dein Passwort ab.** Auf deinem Rechner liegen
  Chat-Schlüssel und Verlauf in einem Tresor, den dein Account-Passwort
  öffnet (*Chat entsperren*; *Chat sperren* schließt ihn wieder). Mit einem
  Baylee-Account werden sie zusätzlich gesichert, schon auf deinem Rechner
  verschlüsselt, sodass ein neuer Rechner sie zurückbekommt. Die Schlüssel
  eines Gastes liegen nur im Speicher.
- **Der Tischchat** geht nur an die Personen, die am Tisch sitzen.
  Zuschauer, die Haus-KI und Sprachmodelle bekommen nie Chat, und Chat ist
  nie Teil einer Spielaufzeichnung oder eines Replays.
- **Meldest du eine Nachricht**, schickst du genau diese eine Nachricht an
  die Moderation des Gateways, nichts sonst aus dem Gespräch.

Das Chatfenster sitzt in der Lobby unten; am Tisch ist es dasselbe Fenster
wie das Spielprotokoll. Seine Einstellungen (*Chat & Privatsphäre*
in der Freundesleiste) regeln, wer dir zuflüstern darf, wie lange dein
Verlauf bleibt, die Töne und die Liste deiner Geräte. Was das Gateway genau
speichert, steht in [docs/privacy.md](docs/privacy.md).

## Replays

Der Reiter **Replays** in der Lobby listet jedes Spiel, das du dir noch
einmal ansehen kannst:

- **Spiele gegen das Haus**, auf deinem Rechner gespeichert (die letzten 20
  und alle, die du archivierst).
- **Online-Spiele**, an denen du gesessen hast, vom Gateway, auf jedem
  Rechner, an dem du dich anmeldest.

Ein Replay hat unten am Tisch eigene Bedienelemente: eine Zeitleiste
mit einem Feld pro Zug, Abspielen und Anhalten, einen Moment oder einen Zug
vor und zurück, Anfang und Ende und Geschwindigkeiten von ½× bis 8× (die
Tasten: <kbd>Leertaste</kbd>, <kbd>←</kbd> <kbd>→</kbd>, <kbd>⇧ ←</kbd>
<kbd>⇧ →</kbd>, <kbd>Pos1</kbd> <kbd>Ende</kbd>, <kbd>-</kbd> <kbd>=</kbd>).
Der Platz-Schalter wählt, **wessen Sicht** du siehst: deinen eigenen Platz,
den öffentlichen Tisch, alle Hände zugleich oder den Platz eines anderen
Spielers.

**Wer deine verdeckten Karten sieht.** Für jedes Online-Spiel entscheidest
du, ob die anderen Spieler in seinen Replays deine Hand und deine
Bibliothek sehen dürfen. Bei Spielen ab beta.7 ist die Freigabe **an**,
solange du sie nicht abschaltest (der Bildschirm am Spielende zeigt das
Häkchen, und das Menü des Spiels im Reiter Replays ändert es
jederzeit). Bei Spielen vor beta.7 ist sie aus, weil damals niemand gefragt
wurde. Die Haus-KI und Sprachmodelle geben immer frei. Ein Platz, dessen
Spieler Nein gesagt hat, trägt ein Schloss, und seine Hand bleibt verdeckt.

**Gelöscht wird nichts.** *Archivieren* nimmt ein Spiel aus deiner Liste
(und *Zurückholen* bringt es zurück); die Aufzeichnung bleibt auf dem
Gateway, weil Spielaufzeichnungen die Haus-KI trainieren und beim
Ausbalancieren der Decks helfen ([docs/privacy.md](docs/privacy.md) sagt,
was eine Aufzeichnung enthält: keinen Chat, und keine Namen mehr, sobald ein
Account gelöscht ist). Ein Replay spielt die heutige Version der Engine
noch einmal; wo sich das nicht gegen das prüfen lässt, was der Tisch damals
zeigte, markiert es die Zeitleiste.

## Gegen ein Sprachmodell spielen

Einen Platz kann auch ein Sprachmodell spielen, etwa eines hinter der API
von Anthropic oder einer OpenAI-kompatiblen API, oder ein lokales Modell in
LM Studio. Es spielt aus genau dem, was dieser Platz sehen würde, über
dieselben Fragen, die auch ein Mensch beantwortet. Der Name eines Spielers
wird nie an ein Modell geschickt: Die Plätze heißen `P1`, `P2`, … und der
eigene Platz des Modells „du“. Der Raum zeigt, welches Modell auf welchem
Platz sitzt, bevor jemand *Bereit* sagt.

Es gibt zwei Wege.

### Ein Modell, das der Server anbietet

Manche Gateways bieten **Gateway-Modelle** an, die der Betreiber des Servers
betreibt und bezahlt. Der Betreiber legt für jedes Modell fest, welche
Spieler es an den Tisch setzen dürfen. Darfst du, bietet ein freier Platz in
deinem Raum **Gateway-Modell** an: Such eines aus, und es setzt sich. Auf
deinem Rechner brauchst du dafür nichts.

### Dein eigenes Modell, mit deinem eigenen Schlüssel

Das geht im Desktop-Client, auf jedem Gateway, und kostet, was dein Anbieter
berechnet.

1. **Ein Profil anlegen.** Öffne **Einstellungen → Sprachmodelle** und wähle
   eine Vorlage: Anthropic, OpenAI, DeepSeek, LM Studio auf diesem Rechner
   oder ein Agenten-Kommandozeilenprogramm, bei dem du angemeldet bist (zum
   Beispiel Claude Code). Ein Profil sagt, welcher Anbieter und welches
   Modell spielen und wie gründlich das Modell nachdenkt.
2. **Einen Schlüssel geben.** Füge den Schlüssel ins Schlüsselfeld ein und
   drücke *Behalten*. Er landet im Schlüsselbund deines Systems
   (macOS-Schlüsselbund, Windows-Anmeldeinformationsverwaltung, der Secret
   Service unter Linux), nie in einer Datei, und das Fenster zeigt ihn nie
   wieder an. Ein lokales Modell in LM Studio braucht keinen Schlüssel.
3. **Grenzen setzen.** Tages- und Monatsgrenzen in Dollar oder Tokens, dazu
   ein Budget pro Spiel. Jedes Spiel reserviert sein Budget, bevor es sich
   setzt, und rechnet danach ab; das Fenster zeigt, was heute und in diesem
   Monat ausgegeben wurde.
4. **Es an den Tisch setzen.** In einem Raum, den du leitest, drückst du auf
   einem freien Platz **Sprachmodell**, wählst Profil, Modell und Deck und
   wartest, bis der Platz *Bereit* zeigt: Vorher prüft die Brücke, ob das
   Modell antwortet.

Wie es hineinkommt: Dein Client holt sich beim Gateway ein kurzlebiges
**Platz-Ticket** (Chair Ticket) für diesen Platz und gibt es an die
Sitz-Brücke weiter (`baylee-seat`, die mit dem Client ausgeliefert wird).
Das Modell sitzt dann als *dein* Vertreter auf dem Platz, mit einem Namen
`LLM-…` und ohne eigenen Account; bist du mit einem Account angemeldet, klappt
das also auch auf einem Gateway der geschlossenen Beta. Scheitert das
Modell, läuft ihm die Zeit davon oder antwortet es etwas Unzulässiges,
beantwortet das Haus diese eine Frage für es.

**Für Bastler** läuft dieselbe Brücke auch im Terminal. Starte ein lokales
Gateway und einen Agenten ([Ein eigenes Gateway betreiben](#ein-eigenes-gateway-betreiben))
und dann:

```bash
cargo run -p xtask -- dev-table --bridge profile:<name> --play   # du auf Platz 0, das Modell auf Platz 1
cargo run -p xtask -- dev-table --bridge house --play             # das Haus über dieselbe Brücke, zur Probe
```

Die Brücke schreibt, was sie gefragt und was das Modell geantwortet hat,
nach `target/seat-transcripts/`. Die Einstellungsdatei, die Grenzen, die
Anbieter und die Kommandozeilenprogramme beschreibt
[docs/llm-seat.md](docs/llm-seat.md) (englisch).

## Fehler melden

Drücke <kbd>F8</kbd> am Tisch oder in der Lobby, oder nimm den Melde-Knopf im
Menü. Ein Bericht geht an das Gateway, an dem du angemeldet bist. Er
enthält immer seine Art, deinen Text, die Client-Version und am Tisch die
Kennung des Spiels.

Alles Weitere hat **je ein Häkchen**, und jedes Häkchen ist zu Beginn
gesetzt: Nimm weg, was du lieber für dich behältst, bevor du *Senden*
drückst. Die Häkchen sind System und Hardware, der Tisch, wie du ihn siehst,
dein Spielprotokoll (mit Spielern umbenannt in „Spieler A“, „Spieler B“, …),
deine Einstellungen, ein Bild nur des Baylee-Fensters und am Tisch die
ganze Aufzeichnung des Spiels (jeder Zug und die Karten jedes Platzes, keine
Namen). „Zeigen, was gesendet wird“ zeigt jedes Byte. Sitzungsschlüssel und
Chat werden nie gesendet. Nach einem Absturz fragt der nächste Start
einmal, ob Absturzberichte gesendet werden dürfen. Bist du an keinem Gateway
angemeldet, geht der Bericht direkt an den Feedback-Dienst, den der Build
nennt; nennt er keinen, wird nichts gesendet.

[docs/privacy.md](docs/privacy.md) listet alles, was Client, Gateway und
Feedback-Dienst speichern. Eine Karte, die falsch spielt, ist der
nützlichste Bericht überhaupt; du kannst auch
[ein Issue auf GitHub eröffnen](https://github.com/AceVik/baylee/issues).

## Wo die Karten stehen

Jede Karte in Baylee ist Code: eine Datei pro Karte, in einer kleinen
Regelsprache geschrieben und in einem Test gespielt. Es gibt keinen
Interpreter, der Kartentext ungefähr errät. Deshalb wächst der Pool Stapel
für Stapel statt auf einmal.

Gezählt am Pool, mit dem beta.8b erscheint (der Kartenstand vom 21. Oktober
2026):

| | Karten |
| --- | --- |
| Spielen **genau wie gedruckt** (`Implemented`) | **9.265** |
| Spielen, aber **noch nicht jeder Satz** (`Partial`, der Deckbauer warnt) | 33 |
| Im Pool, **noch ohne Regeln** (der Deckbauer warnt) | 45 |
| Absichtlich **nie gebaut** (Ante-, Geschicklichkeits- und Subgame-Karten) | 14 |
| **Alle Magic-Karten**, auf die wir hinarbeiten (jede Karte einmal, eine pro Oracle-Text) | 33.694 |

Heute spielt also mehr als **jede vierte Karte** von ganz Magic. Der
Deckbauer kann nur spielbare Karten zeigen und warnt, wenn ein Deck Karten
enthält, die nicht wie gedruckt spielen.

**Sets.** Karten entstehen Set für Set in der Reihenfolge, in der Magic sie
gedruckt hat; jede Karte zählt in dem Set, in dem sie zuerst erschien:

| Set | Karten | Wie gedruckt | Teilweise | Noch ohne Regeln | Noch nicht im Pool | Nie gebaut |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Limited Edition Alpha (und die zwei Karten, die Beta dazubrachte) | 292 | 288 | 0 | 0 | 0 | 4 |
| Arabian Nights | 77 | 75 | 0 | 0 | 0 | 2 |
| Antiquities | 85 | 84 | 0 | 0 | 0 | 1 |
| Legends | 310 | 307 | 0 | 0 | 0 | 3 |
| The Dark | 119 | 119 | 0 | 0 | 0 | 0 |
| Fallen Empires | 102 | 102 | 0 | 0 | 0 | 0 |
| Ice Age | 346 | 187 | 33 | 1 | 124 | 1 |

**Alpha bis Fallen Empires sind vollständig**: Jede spielbare Karte spielt
genau wie gedruckt (die wenigen fehlenden sind Ante-, Geschicklichkeits-
und Subgame-Karten, darunter Chaos Orb). Ice Age entsteht gerade, samt
kumulativer Versorgung, und die Sets danach folgen in der
Reihenfolge, in der Magic sie gedruckt hat. Karten aus späteren Sets, die
schon spielen, kamen mit früheren Stapeln dazu.

**Bekannte Lücken.** Noch keine Bannlisten und keine Prüfung der
Format-Legalität; die besonderen Objekte der Varianten-Formate (Planes,
Schemes, Vanguards, Conspiracies, Dungeons) sind nicht gebaut. Die
[Website](https://baylee.app/cards?lang=de) zeigt jedes Set und jede
Karte mit ihrem Stand und bei einer teilweise umgesetzten Karte, was ihr
noch fehlt; [docs/mechanics-roadmap.md](docs/mechanics-roadmap.md) §E8 ist
der Plan, und [data/card-pool.txt](data/card-pool.txt) die Liste der Karten
im Pool.

## Ein eigenes Gateway betreiben

Ein Gateway ist der Server, an dem sich Spieler anmelden. Du kannst eines für
deine eigene Runde betreiben: alles, was das öffentliche kann, auf deinem
eigenen Rechner, nur dass deines seine Spieler selbst anmeldet, mit eigenen
Accounts, wo Baylee Sanctuary Baylee Central nutzt. Ein Spiel läuft in drei
Arten von Prozessen:

- **`baylee-gateway`** verwaltet Accounts, Decks, Räume, Freunde, Chat und
  Replays. Es führt selbst keine Regeln aus und braucht PostgreSQL.
- **`baylee-agent`** startet pro Spiel einen Engine-Prozess, wenn das
  Gateway darum bittet.
- **`baylee-engine-server`** ist ein Spiel. Es verbindet sich zurück zum
  Gateway.

Dieser Abschnitt ist eine Anleitung. Maßgeblich sind
[docs/protocol.md](docs/protocol.md) und der Abschnitt *Environment* in
[docs/dev/environment.md](docs/dev/environment.md).

### 1. Datenbank

```bash
docker compose up -d          # PostgreSQL 18 aus compose.yaml
export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee
```

Ändere das Passwort für alles, was von außen erreichbar ist. Seine Tabellen
legt das Gateway selbst an und migriert sie.

### 2. Bauen und starten

```bash
cargo build --release -p baylee-gateway -p baylee-agent -p baylee-engine-server

export BAYLEE_AGENT_TOKEN=$(openssl rand -hex 32)   # gemeinsam für Gateway und Agent
RUST_LOG=info BAYLEE_GATEWAY_NAME="Mein Tisch" ./target/release/baylee-gateway   # lauscht auf 0.0.0.0:28766
RUST_LOG=info ./target/release/baylee-agent        # findet baylee-engine-server neben sich
```

Ohne `RUST_LOG=info` protokollieren die Server nichts. Ohne verbundenen
Agenten kann das Gateway Räume anzeigen, aber kein Spiel starten (`503`).

Laufen Gateway und Agent auf demselben Rechner, können sie statt über TCP
über einen **Unix-Socket** sprechen: `BAYLEE_UNIX_SOCKET=/run/baylee/gateway.sock`
für das Gateway und `BAYLEE_GATEWAY=unix:/run/baylee/gateway.sock` für den
Agenten. Siehe
[docs/protocol.md § On the same machine](docs/protocol.md#on-the-same-machine-the-unix-socket).

Stelle `baylee-engine-server` nie allein ins Netz: Ohne die Argumente des
Gateways ist es ein Entwicklungswerkzeug ohne Anmeldung.

### 3. Wichtige Einstellungen

| Variable | Wirkung |
| --- | --- |
| `DATABASE_URL` | Pflicht. |
| `PORT` | Port, auf dem es lauscht (Standard `28766`). |
| `BAYLEE_AGENT_TOKEN` | Das Geheimnis des Agenten. Ohne es kann sich kein Agent verbinden. |
| `BAYLEE_GATEWAY_NAME` | Der Name, den Clients für das Gateway zeigen. |
| `BAYLEE_REGISTRATION` | Nicht gesetzt: offene Anmeldung. `invite`: geschlossene Beta, Schlüssel nötig. `off`: keine Anmeldung. |
| `BAYLEE_GUESTS=off` | Keine Gast-Accounts. `BAYLEE_GUEST_CAP` begrenzt die gleichzeitigen Gäste (Standard 1000). |
| `BAYLEE_TRUSTED_PROXIES` | Adresse deines Reverse Proxys, damit die Ratenbegrenzung die echte Client-IP sieht. |
| `BAYLEE_ART_PATH` / `BAYLEE_DECK_IMAGE_PATH` | Wo Kartenbilder zwischengespeichert und hochgeladene Hüllen/Matten abgelegt werden (`off` schaltet sie ab). |
| `BAYLEE_SOCIAL` / `BAYLEE_SOCIAL_KEEP_DAYS` | Freunde und Chat (`off` schaltet sie ab); wie viele Tage eine nicht zugestellte Nachricht bleibt (1–30, Standard 14). |
| `BAYLEE_REPLAY_JOBS` | Wie viele Replays gleichzeitig berechnet werden (Standard 4; `0` schaltet Replays ab). |
| `BAYLEE_TERMS_PATH` | Deine Nutzungsbedingungen: eine Markdown-Datei oder ein Ordner mit `terms.<lang>.md`-Dateien, darunter `terms.en.md`. Spieler stimmen ihnen bei der Anmeldung und nach jeder Änderung zu. |
| `BAYLEE_IMPRESSUM_URL` | Ein Link auf dein Impressum, den die Clients zeigen. |
| `BAYLEE_ENGINE_URL` | Setzen, wenn der Agent auf einem anderen Rechner läuft. |
| `BAYLEE_SOURCE_URL` | Wo der Quellcode deines Builds liegt, falls du ihn verändert hast (AGPL §13). |
| `BAYLEE_FEEDBACK_URL`, `…_TOKEN`, `…_KEY` | Optional: <kbd>F8</kbd>-Berichte an einen Feedback-Dienst weiterleiten. |

Das Gateway beantwortet `GET /source` mit Lizenz, Version und Commit, ohne
Anmeldung. Die AGPL verlangt, dass du den Nutzern deines Servers den
Quellcode anbietest, und diese Route erledigt das für dich.

### 4. Beta-Schlüssel

Mit `BAYLEE_REGISTRATION=invite` erzeugst du Schlüssel auf dem Server:

```bash
baylee-gateway invite create --uses 1 --expires 30d --note "Max"
baylee-gateway invite list
baylee-gateway invite revoke <id>
```

Der Befehl braucht nur `DATABASE_URL`; das Gateway muss dafür nicht laufen.
Jeder Schlüssel wird einmal ausgegeben, gespeichert wird nur sein Hash. Auf
einem mit `baylee-deploy` (Crate baylee-ops) eingerichteten Server liest der
Wrapper `sudo baylee-invite …` die Einstellungen des Gateways für dich. Die
Admin-Konsole unten kann dasselbe im Browser.

### 5. Kartentext (optional)

Ohne Katalog laufen Spiele trotzdem: Die Karten zeigen ihren englischen
Oracle-Text, der in die Programme eingebaut ist. Für durchsuchbaren
Kartentext in 19 Sprachen lädst du Scryfalls Massendaten in dieselbe
Datenbank:

```bash
RUST_LOG=baylee_catalog=info cargo run --release -p baylee-catalog -- ingest                 # alle Sprachen, ~390 MB Download
RUST_LOG=baylee_catalog=info cargo run --release -p baylee-catalog -- ingest --english-only  # ~80 MB
```

### 6. HTTPS davor

Das Gateway spricht einfaches HTTP und WebSocket; setze also einen
TLS-Proxy davor. Dieses minimale [Caddy](https://caddyserver.com/)-Beispiel
ist ein Anfang (`scripts/server/` enthält die des öffentlichen Servers):

```caddyfile
baylee.example.org {
    reverse_proxy 127.0.0.1:28766
}
```

Caddy besorgt ein Zertifikat, reicht WebSockets durch und setzt
`X-Forwarded-For`. Setze dann `BAYLEE_TRUSTED_PROXIES=127.0.0.1` beim
Gateway. Das Gateway authentifiziert mit Bearer-Tokens und setzt keine
Cookies; weitere CORS- oder Cookie-Einstellungen braucht es nicht.

### 7. Fehlerberichte und Admin-Konsole (optional)

`baylee-feedback` ist ein eigener Dienst, der die Berichte sammelt, die
Gateways weiterleiten, mit einer Weboberfläche zum Lesen. Dieselbe
Oberfläche ist die **Admin-Konsole** für ein Gateway, für Admins, die du mit
`baylee-feedback admin add` anlegst: eine Übersicht mit der Serverlast, wer
online ist und an welchem Tisch, Accounts (Passwort zurücksetzen, sperren,
löschen), ein laufendes Spiel beenden, eine Karte für eine Zeit oder auf
Dauer sperren (die Spieler erfahren den Grund), Beta-Schlüssel und die
Gateway-Modelle (welcher Sitz-Agent welches Modell betreibt, ihre Schlüssel
und welche Spieler welches Modell setzen dürfen). Die Seite des Gateways
dafür lauscht nur auf dem eigenen Rechner (`BAYLEE_ADMIN_TOKEN`,
`BAYLEE_ADMIN_BIND`). Die Einrichtung beschreibt
[docs/feedback.md § Running it](docs/feedback.md#running-it); ohne den
Dienst lehnt das Gateway Berichte mit „reports are not configured“ ab.

### 8. Gateway-Modelle (optional)

Um deinen Spielern Modelle anzubieten, betreibst du neben dem Gateway einen
**Sitz-Agenten** (`baylee-seathost`), mit `BAYLEE_SEATHOST_TOKEN` auf beiden
Seiten. Seine Modelle, ihre Schlüssel und ihre Ausgabengrenzen liegen auf
dem Rechner des Sitz-Agenten; die Admin-Konsole richtet sie ein und
entscheidet, wer welches Modell setzen darf. Einzelheiten stehen in
[docs/llm-seat.md § A hosted seat](docs/llm-seat.md#a-hosted-seat) und in
[docs/protocol.md](docs/protocol.md) §„Hosted language-model seats“.

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

- **Ein lokaler Tisch mit Plätzen:** Starte Gateway und Agent (oben), dann
  `cargo run -p xtask -- dev-table --seats 4 --ai sharp --play`.
- **Browser- und Handy-Versionen** lassen sich weiter bauen
  (`trunk serve index.html --release` in `crates/baylee-client/`;
  `cargo dev android`, `cargo dev ios-sim`), gehören aber nicht zu diesem Release
  ([docs/mobile.md](docs/mobile.md)).

Für Mitwirkende die Prüfungen, die auch die CI ausführt:

```bash
cargo fmt --all
cargo dev gate rules                        # schnell: Regel-Crates ohne Client
DATABASE_URL=… cargo dev gate               # das volle Gate: fmt, clippy, nextest, validate
DATABASE_URL=… cargo dev gate features      # die Builds mit Nicht-Standard-Features
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
  seal[baylee-seal] --> clientcore
  clientcore --> client[baylee-client<br/>Bevy]
  clientcore --> seat[baylee-seat]
  protocol --> gateway[baylee-gateway]
  protocol --> agent[baylee-agent]
  db[baylee-db] --> gateway
  catalog[baylee-catalog] --> gateway
  seal --> gateway
```

Jeder Pfeil nimmt eine Fähigkeit weg. Die **Engine** ist synchron und
deterministisch: keine Ein-/Ausgabe, keine Uhr, ein Zufallsgenerator mit
festem Startwert. Dasselbe Spiel mit denselben Antworten endet also immer
gleich, und das macht Aufzeichnungen und Replays erst möglich.
**gamehost** baut daraus die Sicht jedes Platzes, und diese Sicht verbirgt,
was der Platz nicht sehen darf. **client-core** ist die Logik des Clients
ohne Darstellung und enthält die meisten Client-Tests; der 3D-Tisch ist mit
[Bevy](https://bevyengine.org/) gebaut. **seal** ist die Verschlüsselung des
Chats. **Gateway** und **Agent** enthalten überhaupt keine Regeln. Karten
sind Rust-Code in einer kleinen DSL, eine Datei pro Karte unter
`crates/baylee-cards/src/cards/`.

Zum Weiterlesen (englisch):

| Thema | Dokument |
| --- | --- |
| Gesamtplan | [docs/architecture.md](docs/architecture.md) |
| Interna der Engine (Ebenen, Ereignisse, Cleanup, Schleifen) | [docs/engine-internals.md](docs/engine-internals.md) |
| Protokoll, Gateway, Räume, Chat, Replays, Schlüssel | [docs/protocol.md](docs/protocol.md) |
| Der Client | [docs/client.md](docs/client.md) |
| Karten schreiben | [docs/card-dsl.md](docs/card-dsl.md), [docs/card-identity.md](docs/card-identity.md) |
| Haus-KI | [docs/house-ai.md](docs/house-ai.md) |
| Sprachmodell-Plätze | [docs/llm-seat.md](docs/llm-seat.md) |
| Datenschutz | [docs/privacy.md](docs/privacy.md) |
| Rechtliches | [docs/legal.md](docs/legal.md) |
| Releases | [docs/releasing.md](docs/releasing.md) |

## Mitmachen

Issues und Pull Requests sind auf [GitHub](https://github.com/AceVik/baylee)
willkommen, und auf [Discord](https://discord.gg/4pP3uAwgY) kannst du reden
und Gegner finden. Bevor du mit Code anfängst, lies [AGENTS.md](AGENTS.md):
Es ist die Karte der Arbeitsregeln des Projekts, für Menschen und
Coding-Agenten gleichermaßen, und nennt für jeden Bereich sein Dokument. Ein
paar Punkte:

- Lass das Gate laufen, bevor du pushst. Die CI baut außerdem für Windows,
  macOS, wasm32 und die minimale Rust-Version.
- Jede Karte, die du hinzufügst oder reparierst, bekommt einen Test, der sie
  spielt, in `crates/baylee-engine/src/engine/card_tests/`.
- Die Engine bleibt deterministisch: kein `std::time`, kein Zufall außer dem
  Generator mit festem Startwert, keine Iteration über Hash-Maps in heißen
  Pfaden.
- Nichts, was eine Karte, ein Symbol, eine Schrift oder einen Ton zeigt, wird
  gemergt, ohne es gegen [docs/legal.md](docs/legal.md) zu prüfen.

## Lizenz und Rechtliches

Baylees Quellcode steht unter der
**[GNU Affero General Public License v3.0 only](LICENSE)**. Wer ein
verändertes Gateway für andere betreibt, muss ihnen dessen Quellcode
anbieten; die Route `GET /source` des Gateways erledigt das. Das Projekt ist
kostenlos und nicht kommerziell, und keine Funktion wird je verkauft.

**Zusätzliche Bedingungen (AGPL Abschnitt 7).** [NOTICE](NOTICE) fügt drei
Bedingungen hinzu, die die Lizenz erlaubt: die Hinweise behalten, eine
veränderte Fassung als verändert kennzeichnen und nicht als das offizielle
Baylee ausgeben, und keine Rechte an den Namen „Baylee“ und „Baylee
Central“ oder am Logo.

**Accounts.** Die Anmeldung läuft über Baylee Central, den Account-Dienst des
Projekts ([AceVik/baylee-central](https://github.com/AceVik/baylee-central)).
Sein Quellcode ist öffentlich, damit jeder prüfen kann, wie er mit Daten
umgeht; der Servercode ist source-available, die Client-Bibliotheken, die
Baylee einbindet (`central-api`, `central-client`, `central-crypto`), stehen
wie Baylee unter der AGPL. Die Nutzungsbedingungen von Central lassen nur
die offiziellen Clients zu. Standard-Nutzungsbedingungen und eine Vorlage
für Datenschutzhinweise für alle, die ein Gateway betreiben, liegen in
[docs/gateway-legal](docs/gateway-legal/README.md).

**Fan-Content.** Baylee ist ein inoffizieller Fan-Inhalt im Rahmen der
[Richtlinie für Fan-Inhalte](https://company.wizards.com/de/legal/fancontentpolicy).
Nicht von Wizards gefördert/gesponsert. Bestandteile des enthaltenen
Materials sind Eigentum von Wizards of the Coast. ©Wizards of the Coast LLC.
Baylee ist nicht mit Wizards of the Coast verbunden. Der englische Wortlaut,
den auch der Client zeigt:

> Baylee is unofficial Fan Content permitted under the
> [Fan Content Policy](https://company.wizards.com/en/legal/fancontentpolicy).
> Not approved/endorsed by Wizards. Portions of the materials used are
> property of Wizards of the Coast. ©Wizards of the Coast LLC.

Magic: The Gathering sowie Kartennamen, Setnamen, Kartenbilder und
Manasymbole sind Eigentum von Wizards of the Coast LLC. Das Spiel liefert
kein Material von Wizards of the Coast mit; die Aufnahmen in dieser README
zeigen Kartenbilder © Wizards of the Coast (siehe Scryfall, unten).

**Scryfall.** Kartendaten und -bilder bereitgestellt von
[Scryfall](https://scryfall.com). Der Client lädt die Kartenbilder beim
Spielen von Scryfall oder aus dem Zwischenspeicher seines Gateways; das Spiel
liefert keine mit. Die Aufnahmen oben zeigen sie so, wie das Spiel sie
zeichnet, nur verkleinert, und keine Universes-Beyond-Karte darunter.
Diese Kartenbilder sind © Wizards of the Coast, bereitgestellt von Scryfall,
und fallen nicht unter die Lizenz dieses Repositorys
([docs/images/readme/README.md](docs/images/readme/README.md);
[docs/legal.md](docs/legal.md), Abschnitt 3). Baylee steht in keiner
Verbindung zu Scryfall.

**Fremdes Material.** [NOTICE](NOTICE) führt die mitgelieferten Schriften auf
(Alegreya Sans, Faustina, Font Awesome Free und die Mana-Schrift, alle unter
SIL OFL 1.1); die CC0-Instrumentenaufnahmen, mit denen die Musik gespielt
wird, behandelt [docs/legal.md](docs/legal.md) §5. NOTICE nennt außerdem das
externe, GPL-lizenzierte Korpus von Kartenskripten, das die Codegenerierung
aus einer lokalen Kopie des Entwicklers lesen darf. Keine Datei dieses Korpus
wird je in dieses Repository kopiert oder mit einem Build ausgeliefert.
