# Intensives Training gegen Sharp, 25.10.2026

**Abgeschlossen am 25.10.2026 um 12:42 MEZ: keine bestätigte Verbesserung.**
Alle acht angeforderten Trainingsrunden gegen `sharp` sind fertig. 1.248 Fälle
wurden gesammelt, davon 1.231 beendet und konvertiert; 17 Antwort-Caps erhielten
kein terminales Trainingslabel. 2.048 Policy-Updates wurden akzeptiert. Der auf
der Entwicklungssuite ausgewählte Stand aus Runde 2 erreichte mit neuen Seeds
in beiden Formaten eine niedrigere mittlere Utility als der Ausgangsstand.
Das Ergebnis begründet weder eine Spielstärke-Promotion noch die Aussage, das
Netz habe Sharp erreicht. Ergebnis und Abschlussstatus enthalten weiterhin
`promoted: false`.

Vorher wurde der [Critic-Pilot](trained-ai-v4-own-critic-2026-10-24.md) vollständig
beendet. Seine 247 vollständigen von 248 Fällen wurden ohne Replay-Ablehnung
konvertiert; der einzelne unvollständige Fall erhielt kein Remis-Label. Erst
nach dem tatsächlichen Prozessende begann dieser separate Policy-Lauf.

Experiment: `~/baylee-data/v4/experiments/sharp-intensive-20261025/`.
`plan.json`, eingefrorene Runtime-Hashes, Job-Manifeste, Status und Ergebnisse
liegen dort. Es gibt keine automatische Promotion.

## Verifizierter Abschluss

Diese Zahlen wurden aus `result.json`, `status.json`, allen acht
`collection-summary.json`, `dataset/meta.json`, `model/report.json`, den
`evaluation/*.json`, gepaarten `comparisons/*.json` und den Job-Manifesten
erneut gelesen. Alle **147 Job-Manifeste** stehen auf `complete`.

| Runde | Gesammelt | Beendet / konvertiert | Caps | Akzeptierte Updates | Entscheidungen in diesen Updates |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 156 | 156 | 0 | 256 | 33.157 |
| 2 | 156 | 154 | 2 | 256 | 33.178 |
| 3 | 156 | 153 | 3 | 256 | 33.233 |
| 4 | 156 | 156 | 0 | 256 | 33.212 |
| 5 | 156 | 156 | 0 | 256 | 33.204 |
| 6 | 156 | 155 | 1 | 256 | 33.189 |
| 7 | 156 | 151 | 5 | 256 | 33.204 |
| 8 | 156 | 150 | 6 | 256 | 33.199 |
| **Summe** | **1.248** | **1.231** | **17** | **2.048** | **265.576** |

In den vollständigen Spielen gab es **null Replay-Ablehnungen und null
unrepräsentierte Entscheidungen**. Die Konvertierung schrieb 16.392.755
Beobachtungs-/Entscheidungsframes aller Sitze. Für PPO wurden 1.294
Lerntrajektorien mit 3.542.779 chronologischen Frames und **510.064 echten
stochastischen Entscheidungen** vollständig auditiert. Die Audit-Anzahl ist
von den 265.576 Entscheidungsvorlagen in akzeptierten Updates zu unterscheiden.
Jede Runde stoppte bereits in Epoche 1 am Limit von 256 Updates; es wurden
nicht sämtliche auditierten Entscheidungen optimiert und nicht zwei volle
Epochen ausgeführt.

Alle acht Rundenausgaben sind geänderte Policy-Stände, keine unveränderten
Quellen nach Rollback. Kein aufgezeichneter Update-Versuch wurde abgelehnt.
Der maximale FP32-Logprob-Fehler betrug 0,0000467301 (Grenze 0,002).
Der abschließende Rollout-KL lag zwischen 0,0001887152 und 0,0005714601
(Grenze 0,02). Die Berichte bestätigen die bitidentischen gemeinsamen,
Gedächtnis- und Value-Tensoren. Das sind Implementierungsprüfungen, keine
Erfolgsmaße im Spiel.

Die Sammlung enthielt 205 neuronale Seitensiege, 16 echte Remis, 1.010
Niederlagen und 17 Caps. Die wechselnden Actoren und Trainingsfälle machen
diese Summen zu keinem Vorher-/Nachher-Vergleich. Die v4-Sammlung verzeichnet
**null verweigerte Antworten** und 844 explizite Haus-Fallbacks. Fallbacks
bleiben als solche markiert und sind keine stochastischen neuronalen PPO-Züge.

## Entwicklungsauswahl und Bestätigung

Die Entwicklungssuite verwendete je Actor K=1, greedy, 128 Duellfälle
(64 unabhängige Tische) und 64 Commander-Fälle (16 Tische) gegen dieselben
Sharp-Gegner, Decks, Seeds und fokalen Sitze. Steady ist ein separat gespielter,
duplizierter Haus-Actor gegen Sharp, kein direktes Duell gegen das Netz.

| Actor auf Entwicklungssuite | Duell-Utility | Commander-Utility | Gleiches Formatmittel |
| --- | ---: | ---: | ---: |
| Ausgangsstand | 0,218750 | 0,082031 | 0,150391 |
| v3 | 0,250000 | 0,101563* | 0,175781* |
| Runde 1 | 0,210938 | 0,035156 | 0,123047 |
| **Runde 2, ausgewählt** | **0,218750** | **0,136719** | **0,177734** |
| Runde 3 | 0,218750 | 0,066406 | 0,142578 |
| Runde 4 | 0,210938 | 0,082031 | 0,146484 |
| Runde 5 | 0,179688 | 0,066406 | 0,123047 |
| Runde 6 | 0,195313 | 0,074219 | 0,134766 |
| Runde 7 | 0,187500 | 0,062500 | 0,125000 |
| Runde 8, letzter Stand | 0,210938 | 0,109375 | 0,160156 |
| Steady-Hausduplikat | 0,468750 | 0,246094 | 0,357422 |

\* v3 hatte einen Commander-Cap. Der skalare Evaluator-Mittelwert enthält
dessen rechnerischen Halbwert; das gepaarte Urteil behandelt den Ausgang als
unbekannt. Zusätzlich hatte v3 im Duell eine verweigerte Band-Antwort
(Fall 42, Seed 3200022) mit Haus-Fallback. Die v4-Vergleichsläufe hatten keine
verweigerte Antwort. Verglichen wird die tatsächlich ausgeführte archivierte
v3-Policy samt Fallback, keine vollständig fehlerfreie v3-Strategie.

Runde 2 gewann die wiederholt betrachtete Entwicklungsrangfolge und wurde
vor Öffnen der Bestätigungsseeds in `confirmation-selection.json` festgelegt
(Checkpoint-SHA-256
`a4848212bdd178e2e593c9b4fb76ec58790a20abcd4d3d6b80c4a7f560e2ac8d`).
Gegen den Ausgang war sie in Entwicklungsduellen einmal besser/einmal
schlechter/126-mal gleich, im Commander sechsmal besser/einmal schlechter/
57-mal gleich. Die Commander-Differenz +0,0546875 stammt aber aus nur
16 Tischen und aus der Auswahl unter acht Runden. Gegen v3 waren Duelle
13-mal besser/17-mal schlechter/98-mal gleich; Commander 10-mal besser/
6-mal schlechter/47-mal gleich und einmal unbekannt. Das belegt keine
allgemeine Verbesserung. Auch Runde 2 blieb unter beiden Steady-Hausmitteln.

Die Bestätigung verwendete `first_seed=5200001` statt `3200001`, erneut
128 Duell- und 64 Commander-Fälle. **Die Deckfamilien blieben dieselben
Entwicklungsfamilien; dies ist kein Familien-Holdout.**

| Neue Seeds | Runde 2: Siege/Remis/Niederlagen | Ausgang: Siege/Remis/Niederlagen | Utility Runde 2 / Ausgang | Gepaarte Differenz |
| --- | --- | --- | --- | ---: |
| Duell | 32 / 0 / 96 | 34 / 0 / 94 | 0,250000 / 0,265625 | −0,015625 |
| Commander | 1 / 2 / 61 | 3 / 0 / 61 | 0,023438 / 0,046875 | −0,0234375 |

Beide Formate hatten null Caps und null verweigerte Antworten. Gepaarte
Duellfälle: **0 besser, 126 gleich, 2 schlechter**. Commander: **2 besser,
60 gleich, 2 schlechter**; unterschiedliche Side-Utility der veränderten
Ausgänge ergibt trotzdem die negative mittlere Differenz. Das gleich
gewichtete Formatmittel fiel von 0,156250 auf 0,136719 (−0,01953125).

Die nach Tischen gebootstrappten 95-%-Intervalle der Differenz sind Duell
[−0,0390625; 0] und Commander [−0,07421875; 0,01171875]. Die Berichte warnen
vor scheinbar engen Bootstrap-Intervallen bei nur 16 Commander-Tischen; die
konservativen begrenzten Tischintervalle sind viel breiter. **Keine bestätigte
Verbesserung** ist die angemessene Interpretation, nicht eine statistisch
gesicherte allgemeine Verschlechterung.

Es gab keine neue v3-Bestätigung auf diesen Seeds. Steady erreichte dort
0,484375 Duell- und 0,222656 Commander-Utility; Runde 2 lag erneut darunter.
Neue Seeds reparieren weder wiederverwendete Deckfamilien noch die bekannten
Informationslücken des encoder-6 Actors.

## Datenqualität und nächste Konsequenz

Die 17 abgebrochenen Trainingsfälle verteilen sich auf Commander 1/256,
Teams 1/64, Sechser-FFA 6/48 und Achter-FFA 9/64. Duelle (768) und Dreier-FFA
(48) hatten keine Caps. Besonders die 12,5 % in Sechser- und 14,1 % in
Achter-FFA verdienen Diagnose an den erhaltenen Aufzeichnungen; der
controllerweite Runden-Grenzwert von 10 % konnte diese formatbezogene Häufung
übersehen. Sie wurde nicht durch erfundene Remis-Labels kaschiert.

Die letzten beiden Runden hatten fünf beziehungsweise sechs Caps; das ist
kein kontrollierter Beweis einer lernverursachten Regression. Weiter bestehen
Stall-/Haus-Fallbacks in Vergleichen, besonders Commander: Bestätigung Runde 2
153 Stallereignisse und 345 Haus-Fallbacks, Ausgang 162 und 358. Ein Stallzähler
zählt keine verschiedenen abgebrochenen Spiele.

Es wurde kein Replay-/Konvertierungsfehler gefunden, der einen Neustart oder
nachträgliche Änderung eingefrorener Daten rechtfertigt. Nötig sind gezielte
Diagnose der Caps/Fallbacks und die versionierte Behebung der im
[Live-Informationsaudit](trained-ai-v4-live-information-audit-2026-10-25.md)
belegten Actor-Lücken, bevor mehr gleichartiges Heads-only-Training als Lösung
ausgegeben wird. Diese Berichtskorrektur startet kein Training und nimmt keine
automatische Promotion vor.

Nach der Abschlussauswertung wurde eine Ursache konkret reproduziert:
Acht der 17 Caps enthalten eine lange Haus-KI-Schleife mit Kessig Wolf Run
(Aktivieren, X=0, Ziel wählen, unbezahltes `{R}{G}`-Fenster passen). Die
Engine nimmt die unvollständige Aktivierung zurück; der gleiche unveränderte
Zustand verleitete die Haus-KI zum erneuten Versuch. Ein Session-Test beweist
die Rückkehr zum identischen State-Hash. Die neue Vorprüfung in
`baylee-ai/src/activate/payment.rs` prüft den finanzierbaren Mindestpreis,
ohne die Aktivierungsquelle doppelt zum Tappen einzurechnen. Ein zweiter
Session-Test erhält die tatsächlich bezahlbare X=0-Aktivierung. Damit ist
ein Fehler der Trainingsgegner behoben, keine Verbesserung der neuronalen
Gewichte gemessen. Die alten vollständigen Vergleichsreihen wurden mit diesem
geänderten Gegner nicht wiederholt; alle eingefrorenen Artefakte bleiben erhalten.

## Trainingsprotokoll

Geplant und ausgeführt wurden **acht Runden mit jeweils 156 eigenen Partien**, insgesamt
**1.248 Trainingsfälle**, ausschließlich gegen feste `sharp`-Hausgegner.
Pro Runde: 96 Duelle, 32 Vierer-Commander, 8 Teamfälle (2v2), 6 Dreier-FFA,
6 Sechser-FFA und 8 Achter-FFA. Der fokale Sitz und gegebenenfalls seine
Teamkollegen spielen neuronal. Decks kommen aus dem Trainingssplit des bestehenden
Manifests; jede Runde verwendet neue Seeds und den tatsächlich zuvor akzeptierten
Actor. Temperatur 0,6 und K=1 bleiben fest. Das sind eigene Spiele gegen Sharp,
keine bloße Wiederholung von Sharp-Lehrerzügen.

Die erste neue Sammlung hatte Vorrang vor den erneuten Vergleichsspielen.
Ausgangs-Actor und v3 wurden nach dem ersten Policy-Training mit denselben
Vergleichsbedingungen bewertet.

Der Collector enthält den neuen optionalen Band-Fallback. Training und Vergleich
verwenden dieselbe eingefrorene Engine/Collector-Version. Raw Records, fertige
Trainingsshards, Regelkataloge, Paritätsdaten, Modelle und ausführbarer Code werden
inhaltlich gehasht. Fehler bleiben mit Seed, Decks, Aufzeichnung und Provenienz
erhalten. Replays müssen gelingen; bei Ablehnungen oder mehr als 10 % unvollständigen
Fällen einer Runde wird zur Prüfung angehalten und der Wecker ausgelöst.

## Zunächst gezieltes Lernen der Zugauswahl

Der 12-Layer-Actor behält zunächst sein gemeinsames Karten-/Tischverständnis,
Gedächtnis, Denken und den kalibrierten Bewertungskopf unverändert. Trainiert
werden `cand_proj`, `cand_scorer`, `pair_blk_proj`, `pair_atk_proj` und
`pair_scorer`. Scalar-/Zahlantworten bleiben ebenfalls unverändert.
Diese Phase trainiert ausdrücklich **nicht sämtliche Gewichte**. Sie prüft,
ob ein ausreichend großes, korrektes eigenes Spielsignal die Zugwahl verbessert,
bevor weitere gemeinsame Gewichte freigegeben werden.

Der alte PPO-Lauf konnte auch ohne echte Policy-Entscheidungen durch Value-/Anchor-
Loss und AdamW-Decay gemeinsame Gewichte ändern. Sein KL-Test konnte erst im
nächsten Batch reagieren. Der neue separate Einstieg `train_policy4.py` behebt
diese Ursachen, ohne die bisherige CLI still umzudeuten:

- Vollständiger FP32-Audit der Verhaltenswahrscheinlichkeiten auf allen echten
  neuronalen Entscheidungen der Trainingspartien; maximale Logprob-Abweichung
  0,002 vor dem ersten Update. Fremde/erfundene bedingte Masken werden abgelehnt.
- Alle Beobachtungen jeder Lerntrajektorie bleiben chronologisch vorhanden.
  Reine Haus-Trajektorien werden nicht unnötig neuronal nachgerechnet.
  Segment-Eingangsgedächtnis und Ziele liegen in temporären Dateien, nicht in
  einem wachsenden Python-Objektgraphen.
- Monte-Carlo-Returns aus dem echten terminalen Side-Outcome, `gamma=lambda=1`.
  Der eingefrorene Critic dient nur als Zustandsbaseline zur Varianzreduktion.
  Er erklärt nicht einen einzelnen Zug für gut oder schlecht. Bei weiteren
  Policy-Änderungen wird seine Kalibrierung erneut geprüft werden müssen.
- Mindestens 128 echte, nicht erzwungene Entscheidungen pro Optimizer-Update;
  gemeinsamer Nenner über alle Microbatches. Untergroße Endgruppen werden
  ausgewiesen und verworfen, nie als Value-only-Schritte ausgeführt.
- AdamW ohne Weight Decay, initial LR=2e-5, höchstens zwei Epochen bzw. 256
  akzeptierte Updates pro Runde, Clip-Epsilon 0,2, KL-Grenze 0,02. Kein
  zusätzlicher Value-/BC-Loss. Nullgradienten bewegen auch keine Adam-Momente.
- Nach jedem Update wird dessen gesamte Entscheidungsgruppe geprüft; bei zu
  großer Änderung werden Gewichte und Adam-Zustand zurückgesetzt. Abschließend
  prüft ein weiterer Audit alle geeigneten Rollout-Entscheidungen. Verletzt
  dieser die Grenze, wird der unveränderte Ausgangsstand mit null akzeptierten
  Updates veröffentlicht, klar von einem trainierten Kandidaten unterschieden.

Frische Runden verhindern, dass beliebig lange auf veralteten Verhaltensdaten
optimiert wird. Ohne akzeptierte Updates wird die Lernrate halbiert; nach drei
solchen Runden endet der Controller mit einem prüfpflichtigen Befund. Auch ein
technisch akzeptiertes PPO-Update ist noch kein Spielstärkenachweis.

## Vergleiche und Abschluss

Nach jeder Runde folgten 128 Duelle und 64 Commander-Fälle mit festen
Validierungsdecks, Seeds und Sitzrotationen gegen Sharp. Der Steady-Hausvergleich
wurde mit identischen Startbedingungen dupliziert. Gepaarte Berichte vergleichen
den Kandidaten mit dem Ausgangsmodell, der vorigen Runde und v3. Unsicherheit
wird nach ganzen Tischen geclustert; Caps bleiben unbekannte Ausgänge.

Die wiederholt betrachtete Suite ist ein Entwicklungssignal. Ein Kandidat wurde
vorab nach dem Mittel seiner Duell-/Commander-Utility gewählt, einschließlich
des unveränderten Ausgangsmodells. Nur für diese konservative Rangfolge zählen
unbekannte Caps als untere Utility-Grenze null. Anschließend erhielt der gewählte
Kandidat einen weiteren Vergleich gegen den Ausgangsstand mit **neuen Seeds**.
Die Deckfamilien bleiben Entwicklungsfamilien: Das ist kein unangetasteter
Familien-Endtest und keine automatische Freigabe als stärkstes Modell.

## Implementierungsprüfung

14 CPU-Tests prüfen unter anderem vollständige Erinnerung, echte versus
erzwungene Entscheidungen, rohe bedingte Masken, spätere fehlerhafte Metadaten,
Gradientennormierung über Microbatches, Lernen eines bekannten Signals, unveränderte
Critic-/Gedächtnisausgaben, identische Logprobs bei gruppierter Board-Berechnung,
Nullgradienten und vollständigen Rollback einschließlich Adam-Zählern. Ein
unabhängiger Agent prüfte PPO-Semantik und Controller. Vor dem Dauerlauf folgte
zusätzlich ein begrenzter Schritt des echten großen Modells auf acht vollständigen,
kurzen Sharp-Duellen; das ist eine Implementierungsprobe, kein Spielstärketest.

Diese Probe ist inzwischen bestanden: 138,6 Sekunden auf der CPU, 1,57 GiB
Peak-PSS, 3.265 chronologische Lernframes und 659 auditierte Entscheidungen.
Maximale Logprob-Abweichung 0,0000058413. Ein Update mit 129 Entscheidungen
wurde akzeptiert; dessen KL beträgt 0,0000055835 und der anschließende Audit
über alle 659 Entscheidungen 0,0000051767. Alle gemeinsamen, Gedächtnis- und
Value-Tensoren blieben bitidentisch. Der Probekandidat wird nicht als Quelle
des Dauerlaufs verwendet.

Der Controller und sein Abschluss-/Fehler-Wecker starteten zunächst in der
Phase `waiting-for-critic`; die GPU blieb bis zum tatsächlichen Ende beim
Critic-Lauf. Danach folgten Sammlung und Policy-Training ohne einen zweiten
Controllerstart. Die Python-Runtime ist bei `da10b2133` eingefroren,
die Rust-Binaries wurden bei `6afb09c7d` gebaut und separat gehasht. Es wurde
kein laufender Versuch umgeschrieben und kein Push ohne lokale Gates ausgeführt.

## Übergang nach dem Critic-Abschluss

Nach dem Critic-Abschluss begann der bereits wartende Controller mit der
ersten Sharp-Sammlung und führte anschließend alle acht Runden und die
Bestätigung aus. Ein zusätzlicher Controller wurde nicht gestartet.

Der [Critic-Review](trained-ai-v4-own-critic-2026-10-24.md#ergebnis-und-review-am-25102026)
findet einen besseren Gesamt-MSE, aber keine belastbare Verbesserung an
Entscheidungsstellen. Der neue Head bleibt deshalb ausschließlich eingefrorene
Zustandsbaseline für vollständige Monte-Carlo-Returns. Die echten Spielausgänge
liefern das Lernziel. Es gibt weiterhin keine Value-basierte Suche oder
automatische Promotion. Die nun abgeschlossenen gepaarten Spielvergleiche
bestätigen keine Verbesserung durch diese Policy-Updates.
