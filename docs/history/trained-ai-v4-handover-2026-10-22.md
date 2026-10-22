# Übergabe an Gemini 4 Argon: Playernet v4 implementieren und trainieren

**Datum:** 22. Oktober 2026.
**Repository:** `/home/ace/Projects/baylee`.
**Geprüfte Basis:** `main` / `HEAD` @ `30779bc920147a2a2acc713ebcb784badceced4a`.
**Arbeitsgrundlage:** [überarbeitetes Architektur- und Trainingsproposal](../proposals/trained-ai-v4.md).

Diese Datei ist der vollständige Arbeitsauftrag samt Kontext und Übergabe.
Du brauchst die vorherige Chatunterhaltung nicht. Lies zusätzlich das Proposal
vollständig: Es enthält die detaillierten Codebefunde, Rechnungen,
Forschungsquellen und Schnittstellenentwürfe. Die Übergabe ersetzt diese
Belege nicht. Weder v4 noch seine Trainingspipeline sind bereits implementiert.

## 1. Auftrag und Erfolgskriterium

Übernimm die Implementierung und den Trainingsneustart. Arbeite in überprüfbaren
Etappen bis zu einem nachweislich starken, dauerhaft beobachtenden neuronalen
Spielagenten. Bleibe nicht bei einer weiteren allgemeinen Architekturberatung
stehen. Prüfe Annahmen am Code und passe den Plan begründet an, wenn neue
Ergebnisse dafür sprechen. Routineentscheidungen innerhalb dieses Auftrags
kannst du selbst treffen.

Die wichtigste Anforderung des Nutzers lautet:

> Das neuronale Netz soll mindestens so gut und präzise spielen wie ein guter
> erfahrener Spieler.

Das umfasst mehr als hohe Haus-KI-Winrate: präzise Entscheidungen, gelernte
mehrzügige Pläne, korrekten Umgang mit Interaktion und verdeckter Information,
robuste Ausführung und Generalisierung auf neue Decks und Kartenkombinationen.
Ein Profilname `expert`, niedriger Trainingsloss oder hohe Lehrerübereinstimmung
beweisen das nicht. Dieses Niveau muss evaluiert werden; weder Parameterzahl
noch ein Paper garantieren es.

Das System ist langfristig für zwei Experten gedacht:

1. **Jetzt:** universelle Spiel-KI für unterstützte Karten/Decks, 1v1, 2v2,
   3–4er-FFA einschließlich Commander und Tische bis acht Spieler.
2. **Später:** Deckbuilder, Synergiebewertung, Unterstützung bei Regelvalidierung
   und Commander-Bracket-/Power-Level-Einschätzung auf gemeinsamem Kartenfundament.

Format-/Regellegalität bleibt deterministischer Code. Ein Netz darf dazu
Vorschläge und Einschätzungen liefern. Commander-Brackets sind kontextabhängige,
versionierte Kategorien; aus einer Deckliste allein entsteht kein universell
exakter Stärkewert. Die Anforderungen des zweiten Experten sind im Encoder
vorzubereiten, ohne dessen Produkt jetzt vollständig bauen zu müssen.

**Vom Nutzer ausdrücklich gewünscht:** größere Modelle als ursprünglich
gerechnet, Training von Grund auf, moderne geeignete Architektur, eigenes
neuronales Nachdenken in mehreren Stufen, kritische Neubewertung eigener
Entscheidungen und verlässlicher Umgang mit Enginebugs während der Sammlung.
Es besteht keine Pflicht, alte Gewichte oder Datensätze weiterzuverwenden.

## 2. Vor Beginn lesen und Arbeitsstand schützen

- [AGENTS.md](../../AGENTS.md) und zusätzliche Anweisungen im berührten Bereich.
- [Proposal](../proposals/trained-ai-v4.md), besonders §3 und §5–9.
- [Trained AI](../trained-ai.md), [House AI](../house-ai.md),
  [Lookahead-Design](../proposals/lookahead-design.md).
- [Architektur](../architecture.md), [Protokoll](../protocol.md),
  [DSL](../card-dsl.md) und bei Enginearbeit die einschlägigen normativen
  Engine-/Kosten-/Pending-Dokumente aus AGENTS.md.
- [Befehle](../dev/commands.md), [Konventionen](../dev/conventions.md),
  [Umgebung](../dev/environment.md), [Legal Guardrails](../legal.md).
- [Verification-Hooks](../verification-hooks.md),
  [Feedback](../feedback.md), [bisheriger Trainer-Handover](../../tools/trainer/HANDOVER.md).

Prüfe `git status`, Branch und aktuellen Commit. Zum Zeitpunkt dieser Übergabe
war das überarbeitete Proposal **untracked**. Diese Datei ist ebenfalls neu.
Beide erhalten, auch bei Worktree-/Branchwechseln. Keine fremden Änderungen
überschreiben. Bei inzwischen verändertem Code Befunde neu abgleichen; alte
Zeilennummern sind keine Autorität.

Der vorherige Auftrag hat nur Dokumentation geändert. Es wurden Quellpfade,
historische JSON-Berichte und aktuelle Primärquellen geprüft sowie kleine
CPU-Proben ausgeführt. Kein neuer Trainingslauf, keine v4-Implementierung,
keine Backendinstallation und kein Benchmark auf `acenb` wurden durchgeführt.
Keine vollständigen Rust-/Release-Gates sind für diesen Review ausgeführt.
Die Freigabetabellen des Proposals sind noch zu erfüllende Anforderungen.

Arbeite in sinnvollen Meilensteinen und halte den tatsächlich implementierten
Stand in der Dokumentation fest. Vor Push/Landing gelten alle drei lokalen
Gates: `cargo dev gate`, `cargo dev gate features`, `cargo dev gate wasm`.
Benötigtes `DATABASE_URL` über die vorgesehene Umgebung beziehen, nicht als
Credential in Befehle oder Dateien schreiben. `cargo dev land` führt Gates
und Landing aus; ein Gate ist keine eigenständige Veröffentlichungsanweisung.
Gezielte Tests während der Arbeit ersetzen die erforderlichen Gates nicht.

Engine/Core bleiben synchron und deterministisch. Keine Uhren oder
unabhängige Zufallsquellen in Regeln; keine Iteration über ungeordnete Maps
als Spielsemantik. Diagnose-IDs sind UUID v7. Keine Corpusdateien kopieren,
keinen vollständigen Codegen ohne Corpus, generierte Dateien nicht von Hand
ändern. Kartenfixes folgen Card-Author-/Review-Vorgaben. Starte für Tests nur
benötigte Prozesse und beende die von dir gestarteten Clients samt Helfern
anschließend nachweislich; fremde Prozesse bleiben unberührt.

## 3. Hardware und Laufzeitvertrag

| Einsatz | Hardware / Vorgabe |
| --- | --- |
| Training | Ryzen 9 3900X, 24 Threads, 64 GB RAM, RTX 4070 Ti mit 12 GB VRAM |
| Später live, `acenb` | Ryzen 7 PRO 7840U, 32 GB RAM, Radeon 780M, Phoenix XDNA NPU, Ubuntu 26.04 vorgesehen |
| Klare Entscheidung | vollständige Antwort möglichst unter 50 ms |
| Schwierige Entscheidung | 15–45 s erlaubt, äußerste Frist 60 s oder frühere Tischfrist |

**CPU-ONNX ist der verbindliche Deploymentpfad.** Phoenix-NPU und iGPU sind
optionale Backendexperimente. Die im Proposal belegte AMD-Linux-Unterstützung
beginnt bei Strix Point; ein Phoenix-Kerneltreiber beweist keinen passenden
ONNX-Compilerpfad. Nicht auf NPU-Unterstützung warten und keine künstliche
Architekturgrenze aus einer ungeprüften NPU-Annahme ableiten.

Speicher und Latenz messen: vollständiger Featurebau, Beobachtungsupdate,
Boardencoder, Denkblock, Kandidatendekodierung, Synchronisation und Export.
Berichte p50/p95/p99, kalten Start, kleine/große Bretter und 1/4/8 gleichzeitig
aktive KI-Sitze. BF16/VNNI-Instruktionen allein sind kein Laufzeitnachweis.
Hardwarezugang prüfen; wenn `acenb` nicht verfügbar ist, lokalen Fortschritt
fortsetzen und dessen Messungen ausdrücklich offenlassen.

## 4. Verifizierter Ausgangspunkt und priorisierte Befunde

Die Details und Belege stehen in Proposal §3. Die folgenden Punkte sind eine
Arbeitsliste, keine bereits erledigten Reparaturen. Repariere sie im neuen
oder weiterverwendeten gemeinsamen Pfad; eine Komplettmodernisierung aller
ungenutzten Legacy-Pfade ist keine Voraussetzung.

### P0: Daten, Fairness und RL vor großen Läufen

1. Collector verwendet standardmäßig Temperatur 1.0, `train_rl.py` 0.7;
   `rl_loop.sh` reicht die Temperatur nicht durch. Verhalten und Update
   müssen dieselbe dokumentierte Verteilung verwenden.
2. GAE verwendet für negative Profil-IDs über `train3.evaluate` Profil 5,
   Actor/Live-Pfad dagegen 4. Auch der Value hängt am Profilkontext.
3. Tatsächliche Behavior-Logprobs und Herkunft `net/house/forced/search`
   fehlen. Haus-Fallbacks dürfen keine On-policy-Netzaktionen vortäuschen.
4. Rekonstruierte kanonische Mehrfachpick-Reihenfolge ist nicht notwendig
   die wirklich gesampelte Reihenfolge. Präfixe und bedingte Logprobs direkt
   aufzeichnen; RL-Ziel auf Pick- oder vollständiger Aktionsebene definieren.
5. Unseen-Faltung im Export und ID-Dropout im Training verändern die
   Verteilung. Alle stochastischen Trainingsmodi im Policyvergleich prüfen.
   Bei unveränderten Gewichten muss die Ratio-Identität bis zur begründeten
   numerischen Toleranz stimmen. Kein Lossfilter kaschiert diesen Vertrag.
6. Multiplayer-Drawtargets und GAE verwenden unterschiedliche Utilities.
   Eine konsistente Seitenutility festlegen; FFA nicht wie ein Nullsummenduell
   durch Vorzeichenwechsel behandeln. Terminal, Timeout, Fehler und reguläre
   TBPTT-Grenzen getrennt führen.
7. `convert3` enthält Entscheidungsbilder, keinen vollständigen
   Beobachtungsstrom pro Sitz. `--every 1` allein repariert das nicht.
8. Hauslehrer ohne privilegiertes Scouting erzeugen. Spielseed, Hosthash
   und wirkliche verborgene Karten dürfen Actor/Suche nicht beeinflussen.
   Im Liga-Netzpfad bleiben reale Seedableitung und Snapshot-Loop-Guard zu prüfen.
9. `rl_loop.sh` übernimmt nach der Arena den Kandidaten unabhängig vom
   Ergebnis. Akzeptierten Champion, eingefrorenen Vergleichsgegner und
   abgelehnte Kandidaten getrennt halten.
10. Crashaufzeichnung muss die auslösende Aktion auch dann erhalten, wenn
    der Engineaufruf nicht zurückkehrt. Supervisor und Vorabaufzeichnung
    kommen vor großen Sammlungen.

### Repräsentation und Abdeckung

- Hohe Status-/Offered-Bits fallen weg. v3 hat allerdings zusätzlich ein
  numerisches `picked`; nicht behaupten, jede Pickinformation fehle.
- `mode_kind` hat keinen Aufrufer: nur die Tabellenbreite zu vergrößern
  stellt die fehlende Modussemantik nicht her. Modusindizes kollabieren
  außerdem ab 31.
- Objektpointer werden Existenzbits; relative Sitze 3–7 kollabieren in
  Vier-Sitz-Relationen. Gerichtete Mehrfachrelationen erhalten.
- Stackfähigkeiten, kopierte/gewährte Regeln und aktive Faces brauchen
  eigene Bedeutung unabhängig von der physischen Karten-ID.
- Die Hashkanäle des DSL-Walkers werden doppelt transformiert; der
  Debug-String-Walker verliert Struktur, Reihenfolge und Werte.
- View-/Pendingsemantik, Zahlungen, Ziele, Commander-/Teamrelationen und
  dynamische Berechtigungen vollständig inventarisieren.
- Stille Token-/Kandidatenbegrenzungen und Pile-Kompression dürfen keine
  legalen individuellen Objekte oder Zuordnungen verschwinden lassen.
- Coverage-Matrix je Pending-/Aktionsfamilie aufbauen. Fallbacks explizit
  benennen, zählen und als verbleibende neuronale Abdeckungsgrenze behandeln.
- Exportmetadaten für View, DSL, Kartenmanifest, Features, Aktionen,
  Gedächtnis und Modell verpflichtend validieren.

### Inventar nicht mit Trainingsabdeckung verwechseln

Das Review zählte 50 spielbare Precon-TSV-Zeilen und 15 Eval-Decks. Elf der
Precon-Zeilen enthalten nur 26–33 Karten; Deckformat/Hauptdeck/Sideboard prüfen,
nicht blind alle Listen als reguläre Constructed-Decks behandeln.
9.343 registrierte Definitionen, davon 9.265 mit `Coverage::Implemented`,
sind eine Quellzählung, kein frisch bestandener Testlauf. Der wirkliche
Trainingspool folgt `Working::scan`, Testreferenzen und versionierten Manifesten.

Ein Smart-Deckgenerator existiert bereits. Liga startet standardmäßig nur
mit `allytifact,victory`, `GEN=0`; kein `--decks house`-Alias. Vorhandene
Deckreader nutzen. Arena kennt Eval-Decks, Liga/Arena sind aber Duellpfade;
`selfplay` unterstützt schon zwei bis acht Sitze. Eine laufende Viererpartie
beweist noch keine funktionierende Multiplayer-RL-/Evaluationspipeline.

## 5. Zielarchitektur und ihre Zustandsgrenzen

Zielprofil für frisches Training: `d=512`, acht Layer, acht Heads,
SwiGLU-FFN 1536, Pre-RMSNorm, relationale Mehrfachkanten, struktureller
Card-/Ability-Encoder, autoregressiver Kandidatendecoder und Seitenwerte.
QK-Norm konfigurierbar prüfen. Der dichte Trunk allein hat ungefähr 27,26 Mio.
Parameter; Gesamtspeicher und Gesamtparameterzahl müssen gemessen werden.
`384/8` ist ein Kapazitätsvergleich, `256/4` ein kurzer Integrationsmaßstab.
Das kleinere Netz ist nicht automatisch das Endmodell.

Vier Funktionen mit getrennten Zuständen:

| Funktion | Vertrag |
| --- | --- |
| `SeatKnowledge` | Exakte erlaubte Beobachtungen und explizite Unsicherheit; kein heimliches Verfolgen verdeckter Karten durch Host-IDs. |
| Matchgedächtnis `H` | Gelerntes Update bei jedem sitzautorisierten Ereignis. Neun Slots, zunächst `9×512` gegen `9×384`; eigene stabile Sitzordnung. |
| Denkzustand `Z` | Innerhalb einer Entscheidung wiederholte Verfeinerung; vorgeschlagen `16×512` mit zwei geteilten Self-/Cross-Attention-Layern. Hypothesen verändern keine beobachteten Fakten. |
| Optionaler `Intent` | Gelernte bedingte Absicht über Entscheidungen hinweg; eigener versionierter Ein-/Ausgang, Annahme nur zur gültigen tatsächlich ausgeführten Aktion. Gegen H/Z ohne Intent testen. |

`H` ist klein; sein Trainingsgraph kann teuer sein. Neun Slots garantieren
weder exakte Erinnerung noch kalibrierte Beliefs. Ein kleiner Eventbuffer
ist eine Ablation. Ein kleiner Observer verarbeitet jeden erlaubten Übergang;
der große Board-Trunk läuft bei Entscheidungen und relevanten Änderungen.
Ständiges Beobachten bedeutet nicht ständiges Ausführen des größten Netzes.

Definiere einen gemeinsamen Beobachtungsvertrag für Training und Livebetrieb.
Anzeige-Logs und Snapshot-Diffs allein reichen nicht: zwischen Snapshots
kurz sichtbare Karten können sonst fehlen. Ereignisberechtigung gilt zum
Ereigniszeitpunkt. Eigene Startliste ist erlaubt, vollständiges `GamePreset`
oder fremde private Pendingoptionen nicht.

Cursor, Watermark, Duplicate-/Gap-Erkennung, Reset, Kontrollwechsel,
Eliminierung und Reconnect sind Teil des Schemas. Vor einer Entscheidung
müssen die zugehörigen Beobachtungen verarbeitet sein. Wiederholtes `decide`
darf dasselbe Ereignis nicht erneut ins Gedächtnis schreiben. Pondering und
Suchäste benutzen getrennte Zustände. Acht KI-Sitze teilen allenfalls statische
Kartenfeatures, nicht private Beobachtungen, H, Z oder Beliefs.

Rekurrentes Training verwendet zusammenhängende Sitzstreams, Burn-in,
Lossmasken und TBPTT. Forced-Aktionen und fremde Züge sind trotz fehlendem
Policy-Loss relevante Beobachtungen. Veraltete gespeicherte H-Zustände und
aktuelle Modellgewichte dürfen nicht unbemerkt eine andere Policy erzeugen.
Langzeitwissen über deutlich mehr als eine Chunklänge testen.

## 6. Das Netz soll selbst planen und seine Entscheidung prüfen

Der Zielpfad hat drei Stufen:

```text
erlaubte Beobachtungen -> schneller vollständiger Policyvorschlag
                      -> trainierte neuronale Verfeinerung / Gegenprüfung
                      -> optional validiertes Vorausspielen
```

Der Boardkontext wird innerhalb einer Entscheidung wiederverwendet.
Ein geteilter Denkblock überarbeitet `Z`; Readouts liefern Policy, Value
und später ein trainiertes Stoppsignal. Zunächst feste, protokollierte
`K=1/2/4/8` trainieren und auswerten. Größere Tiefen nur nach Nutzenbeleg.
Gemischte Tiefen und Supervision früher/später Ausgänge sollen einen starken
Sofortpfad erhalten. Bloßes Wiederholen eines nicht dafür trainierten Netzes
ist kein Nachdenken.

Curricula stellen Probleme, keine handgeschriebenen Gewinnpläne:
mehrzügige Lösungen, verführerische Fehler, gegnerische Antworten, Ressourcen-
und Timingkonflikte. Mehrere gültige Linien akzeptieren. Gegenprüfung lernt
an tatsächlichen Outcomes, überprüfbaren Szenarien und gültigen Simulationen.
Nicht denselben Critic mehrfach abstimmen lassen und dies unabhängige
Selbstkritik nennen.

Messen: korrigierte Erstfehler, durch Revision verursachte Fehler, Qualität
pro Rechentiefe, Laufzeit und vollständige Planausführung. Selbstsicherheit,
niedrige Entropie oder Konvergenz allein beweisen keine Richtigkeit.
Planannahmen bei neuer gegnerischer Aktion erneut bewerten.

Denktiefe/Budget gehören zur Behavior-Provenienz. Bei stochastischem Halting
oder Branching müssen deren Wahrscheinlichkeiten zum definierten RL-Ziel
passen; alternativ separat lernen und einfrieren. Bei PPO nicht heimlich mit
einer anderen Denktiefe als beim Sampling rekonstruieren. ONNX exportiert
einzelne Observer-/Denk-/Decoder-Schritte; die Hostschleife kontrolliert sie.

## 7. Value-Netz: Rolle, Altbefunde und neue Prüfung

Das separate alte `ValueNet` schätzte den späteren Sieg eines Haus-KI-Sitzes
bei Haus-KI-Fortsetzung. Es war ein Stellungswert `V`, kein allgemeiner
Zugrichter `Q(observation, action)`.

Die beim Review gelesenen lokalen Berichte liegen unter
`/home/ace/baylee-data/models/`:

| Bericht | Historischer Befund |
| --- | --- |
| `value-v1/report.json` | Überkonfident: Brier 0,2084, Logloss 0,7225, ECE 0,1239. |
| `value-v2/report.json` | Brier 0,1601 gegenüber 0,2090 einer einfachen Baseline; ECE 0,0060. Nur zwei Decks und ungefähr 200 Karten. |
| `value-v2/eval-d004-mirror.json` | Brier 0,1783 gegenüber 0,2492 einer Sitz-/Zugphasenbaseline. Frühe Züge deutlich schwächer als der Gesamtdurchschnitt. |
| `v3-a/report.json` | Gemeinsamer Policy-/Value-Head: Brier 0,1574. Die v3-GAE verwendet den eigenen Value, nicht das alte separate `value-v2`. |

Das sind gelesene historische Ergebnisse, keine neue Messung auf aktueller
Engine. Held-out-Daten dienten teilweise schon der Checkpointauswahl;
Positionen desselben Spiels sind korreliert. Globale Kalibrierung verdeckt
Gruppenunterschiede. Daraus keine allgemeine menschliche Spielstärke ableiten.

Ein zusätzlicher Legacy-Befund: `train_policy.py` trainiert nur Policy-
Cross-Entropy, exportiert aber über `export_onnx.py` auch einen nicht auf
Outcomes trainierten Value-Kopf. Dieser ist nicht das trainierte `value-v2`
und nicht der v3-Value. Den Existenznachweis eines Tensors niemals als
Nachweis eines ausgebildeten Critic behandeln.

Für v4 drei Aufgaben unterscheiden: öffentlicher Stellungswert für Live-
Entscheidungen, Trainingsbaseline für Advantages und optional aktionsabhängige
Alternativen-/Planbewertung. Gemeinsamer öffentlicher Trunk ist ein sinnvoller
Start; vollständige Netztrennung nur bei belegtem Nutzen. Privilegierter
Trainingscritic darf Actor-History plus private Trainingsfeatures lesen,
aber kein Vorwärtspfad dieser Features darf Actor/Live-Suche erreichen.

Ein Zug mit 10 % Siegchance kann der beste verfügbare sein; 90 % können eine
verschenkte sichere Gewinnlinie bedeuten. Deshalb neben Kalibrierung auch
Alternativenrangfolge und tatsächlichen Nutzen für Policy/Suche messen.
Ziele aus echten gesunden Spielausgängen, definierten Returns und validierten
Szenarien erzeugen. Policy-/Gegnerwechsel verändern den Wert. Keine dauerhaft
eingefrorene alte Hausbewertung als Wahrheit vorgeben; eigene ungeprüfte
Vorhersagen nicht als bewiesene neue Labels behandeln.

## 8. Sichere System-2-Suche und Pondering

Der Lookahead-Vertrag bleibt verbindlich: keine Kopie der echten Engine,
kein Checkpoint/Journal, echter Spielseed oder fremdes Deck vom Host.
Hypothetische Welten aus erlaubter History und konsistenten Beliefs sind
erlaubt. Bei gleichen erlaubten Eingaben, gleichem unabhängigen Policyseed
und festem Arbeitsbudget muss die Entscheidung reproduzierbar bleiben.

`SimulationPosition` und der sichere Resume-Konstruktor existieren am
Reviewstand noch nicht. Beginne nur mit vollständig belegbaren Formen aus
dem Lookahead-Proposal. Leerer Stack plus Priority genügen allein nicht:
Continuations, Trigger, Loyalty, Pass- und Loop-History müssen stimmen.
Fehlen Fakten, lehne diese Rekonstruktion ab. Der vorhandene Kampfsearch
ist eine Näherung, kein vollständiger Regel- oder Lethal-Oracle.

Vergleiche vollständige legale Rootaktionen über konsistente Teilchen und
vergleichbare Zukunftsstreams. Simulierte Gegner erhalten nur ihre jeweilige
simulierte Sicht. Eine Hand pro Teilchen zu kennen darf nicht zu allwissenden
zukünftigen Root-/Gegneraktionen führen; Strategy Fusion ausdrücklich testen.
FFA-/Teamutility korrekt führen, keine pauschale Koalition aller Gegner.

Zuerst einfache nachvollziehbare Kandidatenverteilung/Sequential Halving;
Gumbel und gelernte Dynamik später kontrolliert vergleichen. Ein Weltmodell
braucht Stochastik, Beobachtungen, Legalität, neue Objektidentitäten und
Mehrschritt-Konsistenz. Ein Next-Value-Head erfüllt das nicht.

Sichere früh eine vollständige legale Referenzaktion. Plane Sucharbeit in
reproduzierbaren Arbeitseinheiten. Ein äußerer Watchdog liefert bei Fristbruch
die definierte Referenzaktion; nicht den vom zufälligen Threadfortschritt
abhängigen halbfertigen Gewinner. Neue Beobachtungen/Decision-IDs invalidieren
veraltete Ergebnisse. Pondering darf geplante Arbeit vorziehen, nicht still
die Qualität davon abhängig machen, wie lange der Gegner wartet.

## 9. Kartenfundament für beide Experten

Ersetze Debug-String-Hashing durch einen versionierten typisierten DSL-Walker.
Erhalte variable Fähigkeitslisten, alle Faces, Farbidentität, Commander- und
Partnerrollen, volle Kosten einschließlich Hybrid/Phyrexian/Schnee/X,
Aktivierungszonen, Timing, Bedingungen, Limits, geordnete Effekte, Ziele,
Amount-Ausdrücke, Ressourcenflüsse, Dauer sowie Copy-/Token-/Grant-/Replacement-
Semantik. Laufzeitregeln ergänzen statische Registryeinträge.

Karten-ID, unbekannte ID, unbekannte Regelstruktur, Padding und echte leere
Fähigkeit sind verschiedene Zustände. Deckrepräsentation als Multimenge mit
Kopienzahlen und Rollen, nicht nur als Set oder ungewichteter Mittelwert.
Synergien hängen auch von Kosten, Bedingungen und Reihenfolgen ab.

Statische Vektoren dürfen gecacht werden; beim gemeinsamen Training müssen
Gradienten und Cacheinvalidierung korrekt bleiben. Kein veralteter Cache
nach Encoderupdates. Dekodierte Kandidaten verweisen auf echte Objekte und
Fähigkeiten; individuelle Zuordnungen und mehrere Beziehungen bleiben erhalten.
Das gemeinsame Fundament vorbereitet halten, die Spiel-KI bleibt der Fokus.

## 10. Trainingsplan mit messbaren Übergängen

Die Zahlen sind Startbudgets, keine Zusage für Trainingsdauer oder Spielstärke.
Mehr Training ist erlaubt, wenn die Daten stimmen und Fortschritt messbar ist.
Nicht einfach die alten Loops mit größerem Modell starten.

| Phase | Arbeit | Nachweis vor Ausweitung |
| --- | --- | --- |
| T0: Integrität | P0s, `features4`-/History-/Aktionsverträge, Faultpipeline, Coverage-Matrix. | Golden-Fälle, Behavior-Parität, Anti-Cheat, absichtlich ausgelöste Crash-/Hangfälle bestehen. |
| T1: Hardwarepilot | 256/4, 384/8, 512/8 mit Denkblock; B=2/4/8, U=16/32, K=1/2/4/8; reale Tokenverteilungen. | Vollständige Optimizersteps passen mit Reserve in 12 GB; CPU-Pfad, Overflow und Fristen gemessen. |
| T2: Datenpilot | 200–500 gesunde vollständige Matches über Tischgrößen; etwa 100–300 variierte Taktikfixtures. | Herkunft, Sitzstreams, Cursor, Manifest, Coverage, Bytes pro Beobachtung und Repros korrekt. |
| T3: Encoder und Imitation | Frische Gewichte; DSL-Hilfsziele, zuerst 100.000–300.000 gültige Entscheidungen, danach zunächst 2–5 Mio.; zugehörige Beobachtungen erhalten. | Dev-Generalisation, funktionierendes Gedächtnis und trainierter Outcome-Value, keine stille ID-Memorierung oder NaNs. |
| T4: Neuronales Denken | Gemischte K, mehrschrittige Aufgaben, Gegenaktionen und Revisionslernen; optional Intent-Ablation. | Mehr Rechentiefe verbessert schwierige Devfälle; der Sofortpfad bleibt stark, Revision schadet nicht häufiger als sie hilft. |
| T5: Recurrent PPO | Zunächst 25.000–100.000 Learnerentscheidungen je Iteration, 1–3 Optimierungsepochen, zehn Pilotiterationen. | Korrekte Ratios/Returns, stabile Updates, belegter Fortschritt gegenüber BC und historischen Gegnern. |
| T6: Suche und Distillation | Enge sichere Rekonstruktion, Kandidaten-/Teilchenvergleich, Pondering, getrennte Suchdaten. | Zusätzliche Suche verbessert Qualität bei gleichem Gerätebudget und fairem Informationszugriff. |
| T7: Zusätze | HL-Gauss, Muon, SPO, Diff-Attention V2, MoE, gelernte Dynamik einzeln vergleichen. | Gewinn gegenüber passender Referenz bei vergleichbaren Daten-/Rechenbudgets und vollständigem Export. |

Architekturversuche dürfen beginnen, sobald die jeweils nötigen Integritäts-
und Messgrundlagen stehen; diese Tabelle verlangt nicht, erst menschliches
Spitzenniveau zu erreichen, bevor eine bessere Attention getestet wird.
Gelernter Denkblock und strukturelle Relationen gehören bereits zum Zielnetz.

Startpunkte: AdamW; BC-Lernraten `1e-4/3e-4`, RL `1e-5/3e-5`, Gradientenclip
1.0, PPO-Clip 0.1/0.2, `gamma=1`, Lambda 0.9/0.95. Temperatur zunächst 1.0.
Das sind Suchpunkte, keine pauschal optimalen Einstellungen.

`B` zählt Sitzstreams, `U` Beobachtungen, `K` Denkiterationen. Nicht alle
Beobachtungen tragen Policy-Loss. Kleine Microbatches, BF16 und Accumulation
zunächst auf 128–256 gültige Policypositionen prüfen. Peak-VRAM einschließlich
Optimizer, Referenzmodell, Critic, Burn-in, Kartenencoder und Denkgraph messen.
Nicht mit einem einzelnen Inferenzforward den Trainingsspeicher schätzen.

Als Anfangsmischung nach Entscheidungsdaten: 30 % geeignete Precons,
20 % Haus-/eigene Trainingsdecks, 50 % Smart-Generator. Taktikszenarien
zusätzlich separat ausweisen, z. B. 10–20 % der Updates. Tischmix zunächst
50 % Duelle, 25 % 3–4er-FFA, 15 % 2v2 und 10 % 5–8er-Tische. Anhand echter
Coverage und Simulationszeit anpassen. Scouting bleibt aus.

Liga zunächst ungefähr 50 % faire Hausgegner, 25 % aktuelle Selfplay-Policy,
25 % akzeptierte historische Snapshots; später nach Lernerfolg umgewichten.
Replay alter Daten als BC-/Hilfsloss oder korrektes Off-policy-Verfahren,
nicht als beliebige Zumischung in vermeintlich reines PPO. Suchaktionen
werden nicht als direkt gesampelte Netzpolicy ausgegeben.

Train/Dev/Test vor großer Sammlung festschreiben. Alle Perspektiven eines
Spiels bleiben im selben Split. Die 15 bekannten Eval-Decks sind eine
Vergleichssuite, kein unberührter Test mehr nach wiederholtem Tuning.
Zusätzliche gesperrte Decks/Seeds/Szenariofamilien und Generalisationstests
anlegen; Varianten derselben Fixture nicht über Splits verteilen.
„DSL bekannt, Gameplay ungesehen“ von „Struktur ungesehen“ unterscheiden.

CPU-Worker z. B. 6/12/18 vergleichen, Thread-Oversubscription vermeiden.
Sammlung und Training zunächst phasenweise auf der einen GPU betreiben.
Nicht blind die 96 Collector-Threads des alten Skripts übernehmen.
Der vorhandene GPU-Schalter ist `GPU=1` mit passenden `onnx-cuda`-Builds
beziehungsweise `BAYLEE_EP=cuda`, nicht ein erfundenes `ORT_CUDA=1`.

Shards, Datenkompression und ein Speicherlimit vorsehen. Laufzeitplanung aus
gültigen Entscheidungen pro Sekunde und Sekunden pro Optimizerstep ableiten;
Evaluation, Konvertierung und Fehlerreplays separat rechnen. Messungen aus
dem tatsächlichen U/K/Board-Mix verwenden. Mehr Beobachtungsdaten können die
Plattenkosten gegenüber v3 deutlich erhöhen.

Jeder Lauf speichert Config, Versions-/Manifest-/Splithashes, Seeds,
Behaviorpolicy, Optimizer/Scheduler/RNG, Data-Cursor und Checkpointlinie.
Resume muss konsistent sein; keine beschädigten oder inkompatiblen Shards
stillschweigend einlesen. Große Modelle/Records bleiben außerhalb von Git.

## 11. Enginefehler erkennen, sichern und reparieren

Aufzeichnung und Reproduktion sind vor produktiver Massensammlung nötig:

1. Setup/Seed/Versionsmanifest vor der Engineerzeugung beim Supervisor sichern.
2. Jede Aktion an der zentralen Apply-Grenze vor dem kritischen Aufruf als
   `attempt` protokollieren, einschließlich automatischer Pump-/Hausantworten.
3. Danach Ergebnis und Fortschritt ergänzen. Bestehendes Recordformat erhalten;
   der Diagnosekanal ergänzt das beim Crash möglicherweise unvollständige Replay.
4. Spielworker pro aktivem Spiel isolieren, Speicher/Arbeitsbudget begrenzen,
   Exit/Signal/Heartbeat beobachten. `catch_unwind` deckt Abort, OOM und Hangs
   nicht ab. Inferenzfehler von Engine-Regelfehlern unterscheiden.
5. Fault-Bundle mit UUID v7, Signatur, Commit, Manifesten, Seeds, Recordpräfix,
   auslösendem Attempt und bereinigtem Stacktrace sichern. Privilegierte
   Diagnosedaten niemals in Actorfeatures übernehmen.
6. Mit denselben gespeicherten Aktionen reproduzieren, deduplizieren und nur
   durch weiterhin legal abspielbare Präfixe/Fixtures minimieren.
7. Fehlerhafte Spiele aus Policy- und Value-Daten ausschließen, Fehlerquote
   dennoch berichten. Bei Integritäts-/Sichtbarkeitsfehlern betroffene Sammlung
   stoppen; begrenzte Retries verhindern Endlosschleifen.

Automatisierbare Reparaturjobs können lokal ein isoliertes Checkout verwenden:
Repro, fehlschlagender Regressionstest, Prüfung der Regel, kleinster korrekter
Patch, fokussierte Tests und nötige Gates. Keine Fehlerunterdrückung, still
deaktivierte Fähigkeit oder geänderter Reward als vermeintlicher Fix.
Kartenreviewregeln bleiben gültig. Ein von einem Agenten vorgeschlagener Patch
ist nicht allein dadurch regelkorrekt.

Enginewechsel an **Sammlungsepochengrenzen**, nicht mitten in laufenden
Partien. Betroffene Daten/Modelle über ihre Abstammung identifizieren;
divergierende Daten neu erzeugen, bei erheblicher Kontamination vom letzten
sauberen Checkpoint fortsetzen. Neue Engine plus alte Aktionen erzeugen
nicht automatisch gültige neue PPO-Behavior-Daten.

Auch stille Fehler prüfen: abgewiesene angebotene Aktionen, Zustandsmutation
nach Ablehnung, Replayabweichungen, unplausible Ressourcen-/Siegstatistiken,
Loops und gezieltes Ausnutzen von Enginefehlern durch die Policy.
Verification-Hooks gezielt einsetzen; nicht voraussetzen, dass Testbinary-
Hooks auch im normalen Liga-Binary laufen. Externe Fehlermeldungen/Uploads
sind keine Voraussetzung; zunächst lokale reproduzierbare Artefakte.

## 12. Evaluation, Promotion und das menschliche Qualitätsziel

Automatische Evaluationssuite: ganze Spiele, Sitzrotation, gepaarte Seeds,
unterschiedliche Decks/Gegner und getrennte Duell-/FFA-/Teamberichte.
Konfidenzintervalle auf Spielebene beziehungsweise passend gepaarten Clustern,
nicht Tausende korrelierte Positionen als unabhängige Stichproben zählen.
Abbrüche und Fallbacks nicht aus dem Ergebnis verschwinden lassen.

Neben Sieg-/Drawutility messen: Pending-/Karten-/Abilityabdeckung, legale
Ausführung, Fehler/Timeouts, Valuekalibrierung nach Phase/Format, neue Karten-
und Combo-Generalisation, Gedächtnis, CPU-Latenzen und Qualitätskurven über
Denktiefe/Suchbudget. Noch ungeprüfte seltene Fähigkeiten bleiben sichtbar.

Für „wie ein erfahrener Spieler“ zwei unabhängige Nachweise vorbereiten:

- Gesperrte schwierige Situationen: Mulligans, Sequencing/Mana, Stack/Timing,
  Kampf, Ressourcentausch, mehrzügige Combos und Gegenaktionen, Threat
  Assessment und Entscheidungen unter erlaubter Unsicherheit. Mehrere gute
  Linien akzeptieren; keine perfekte Kenntnis verdeckter Testkarten verlangen.
- Faire Vergleichspartien gegen mehrere formatkundige erfahrene Menschen,
  mit vergleichbarer Deckkenntnis, festen Zeitregeln und rotierten Decks/Sitzen.
  Ein Pilot von etwa 20–50 Partien findet Schwächen und plant den eigentlichen
  Test; er garantiert keinen statistischen Nachweis. Menschen nicht erneut
  dieselbe inzwischen bekannte verdeckte Seedwelt vorsetzen.

Menschliche Mitwirkung kann vorbereitet, aber nicht erfunden werden. Wenn sie
noch fehlt, automatisierte Arbeit fortsetzen und diesen Nachweis offenlassen.
Keine Menschen ohne Auftrag kontaktieren. Testreplays nicht anschließend
unbemerkt als Trainingsdaten verwenden.

Ein Punktwert von 50 % im Duell beweist keine Ebenbürtigkeit. Unsicherheit,
Deck-/Sitzkontrolle und gegebenenfalls eine vorab benannte Nichtunterlegenheits-
marge berücksichtigen; FFA hat andere Referenzwerte. Nicht nach einem günstigen
Zwischenstand stoppen. Eine Gesamtwinrate darf häufige vermeidbare Patzer nicht
verdecken. Menschliche Top-1-Übereinstimmung allein ist ebenfalls unzureichend.

Promotion nur bei vorher festgelegten Qualitäts-/Coveragegrenzen. Kandidat
und Champion getrennt speichern, inklusive Rückkehrmöglichkeit. Bei unklarer
Arena keine automatische Übernahme. Drei Dev-Prüfpunkte ohne Fortschritt lösen
Diagnose aus; Vertragsbruch, NaNs oder Sichtbarkeitsverletzung stoppen den
betroffenen Trainingspfad sofort. Abschluss-Testsplit nicht laufend zum Tuning
verwenden. Fehlende Evidenz als fehlend berichten.

## 13. Forschung richtig einordnen

Das Proposal enthält 29 externe Quellenverweise einschließlich aktueller
Originalarbeiten und Herstellerdokumentation. Besonders relevant sind
[Looped Actor](https://arxiv.org/abs/2609.37432),
[Elastic Latent Recurrence](https://arxiv.org/abs/2610.12582),
[Thinker](https://arxiv.org/abs/2307.14993) und
[Asymmetric Actor-Critic](https://arxiv.org/abs/2105.11674).
Sie begründen Experimente, keine Zusicherung für Acht-Spieler-Magic.

Diff-Attention V1/V2 nicht vermischen; FlashAttention-Kompatibilität ist kein
ONNX-/NPU-Leistungsnachweis. Soft MoE mit Expert-Slots ist nicht ein globaler
Mix aus FFNs, die trotzdem alle je Token laufen. HL-Gauss-Verteilungen sind
nicht automatisch epistemische Unsicherheit. SPO braucht ebenso korrekte
Behaviordaten wie PPO. Muon betrifft geeignete 2D-Hidden-Weights; Embeddings,
Köpfe und andere Parameter separat behandeln. Keine dieser Methoden repariert
fehlerhafte Datenherkunft oder fehlende Spielsemantik.

ReBeL-Garantien für Zweispieler-Nullsummenspiele nicht auf Commander übertragen.
Latente Suche und Gumbel lösen weder Informationsgrenzen noch falsche Beliefs.
Versionsabhängige Backend-/Bibliotheksannahmen bei der Implementierung anhand
aktueller Primärquellen erneut prüfen. Änderungen am Rezept mit Hypothese,
Kosten und Ergebnis dokumentieren, nicht mit Neuheit allein begründen.

## 14. Erster konkreter Lieferumfang und weitere Arbeitsweise

Beginne mit einem kurzen Abgleich von Arbeitsstand, P0s und berührten Modulen.
Implementiere dann T0–T2 als durchgängigen Pfad: autorisierte Sitzstreams,
verlustarme Aktionen, sichere Aufzeichnung, kleiner Sammler, ein frisches
rekurrentes Testnetz, ein nachvollziehbarer Trainingsschritt, ONNX-Export
und CPU-Replay. Ein kleines Netz ist hierfür zulässig. Danach vermesse das
größere Zielprofil und setze T3–T7 nach den jeweiligen Nachweisen fort.

Liefere pro Meilenstein:

1. Tatsächliche Änderungen und aktualisierte normative Dokumentation.
2. Ausgeführte Tests mit Ergebnissen sowie explizit noch offene Nachweise.
3. Reproduzierbare Befehle/Configs und Artefaktpfade ohne Secrets.
4. Gemessene Parameter-, Speicher-, Laufzeit-, Coverage- und Qualitätszahlen.
5. Verbleibende Grenzen und den nächsten durch die Ergebnisse begründeten Schritt.

Bei der abschließenden Gegenprüfung dieser Übergabe wurden besonders die
Trennung von Matchgedächtnis und Denkzustand, der alte untrainierte Legacy-
Value, die Testsplit-Nutzung, stabile mehrschrittige Absichten und der Abstand
zwischen einem erfolgreichen Pilot und menschlicher Spielstärke berücksichtigt.
Diese Punkte nicht beim Vereinfachen der ersten Implementierung verlieren.

Das Ziel ist weder das größte noch das methodenreichste Netz, sondern ein
nachweislich präziser, lernender und fairer Spieler. Wenn der nächste Schritt
autorisiert und konkret ist, führe ihn aus. Wenn eine Annahme scheitert,
diagnostiziere und verbessere sie; bezeichne einen erfolgreichen Smoke-Test
nicht als abgeschlossenes Training oder belegte universelle Spielstärke.
