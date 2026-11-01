# Eigene Partien und isolierter Critic-Pilot, 24.10.2026

Der [Tiefenpilot](trained-ai-v4-depth-pilot-2026-10-24.md) zeigt bisher keinen
belastbaren Spielstärkegewinn durch zwölf statt acht Layer. Sein Bewertungskopf
ist schlechter kalibriert als ein einfacher Gleichverteilungsprior. Die ersten
1.000 Updates sahen nur 33 Spiele; wenige lange verlorene Trajektorien dominierten
die Frames. Deshalb folgt zunächst ein isolierter Versuch mit eigenen Partien
und unveränderter Policy. Ein besserer Value-Head allein spielt noch keinen
besseren Zug; er bereitet verlässlichere spätere Policy-Updates vor.

## Datenerzeugung

`~/baylee-data/v4/experiments/critic-own-games-20261024/` sammelt den eingefrorenen,
nicht promovierten 12-Layer-Checkpoint des Tiefenpiloten. Seine Auswahl ist keine
Behauptung, dass er acht Layer oder v3 übertroffen hat.

- 248 Fälle: 152 Training und 96 Validierung, getrennte Deck-Splits und Seeds.
- Duelle, 4er-Commander, 2v2 und FFA mit 3/6/8 Sitzen; jeweils `steady` und
  `sharp` als feste Hausgegner, Temperatur 0,6 und K=1.
- Der fokale Sitz und gegebenenfalls seine Teamkollegen spielen neuronal.
  Andere Sitze liefern keine als neuronal ausgegebenen Verhaltensziele.
- Eingefrorene Engine-/Collector-/Converter-Binaries und Modellgraphen behalten
  ihre Hashes. Replays prüfen Zustands-Hashes vor dem Veröffentlichen der Shards.
- Caps und Fehler bleiben Befunde, keine erfundenen Remis. Die Auswertung muss
  den Anteil ausgeschlossener Spiele nennen: Kalibrierung auf beendeten Spielen
  allein belegt keine unverzerrte Erfolgswahrscheinlichkeit aller Spiele.
- Die Validierungsfamilien sind Entwicklungsdaten, kein unangetasteter Endtest.

Die Sammlung fand unzulässige Banding-Teilmengen: Die angebotene Frage beschreibt
Anzahlgrenzen, aber die generische neuronale Auswahl kennt die zusätzliche
Mitgliedschaftsbedingung nicht. Ein neuer expliziter House-Fallback lehnt optionale
Bands ab und behauptet keine neuronale Verhaltenswahrscheinlichkeit. Der
Engine-Regressionstest demonstriert erst die Ablehnung der alten Auswahl und dann
die Annahme des Fallbacks. Die laufende Sammlung behält ihren eingefrorenen
Collector; der Fix gilt erst bei der nächsten Runtime-Version.

## Kalibrierung und Prüfung

`~/baylee-data/v4/experiments/critic-head-20261024/` ist der getrennte Folgelauf.
Sein Controller wartet auf vollständige Konvertierung und das Ende des exakt
identifizierten Collectors, bevor er GPU-Arbeit beginnt. Er prüft alle 24
Sammlungsjobs, Modell-/Graph-/Eingabe-Hashes und den erfolgreichen Converter.

Zuerst folgt ein Verhaltenswahrscheinlichkeits-Smoketest auf zwei vollständigen
Trainingsshards. Dieser begrenzte Audit ersetzt nicht den vollständigen
On-Policy-Audit vor einem späteren PPO-Lauf. Danach extrahiert
`calibrate_value4.py` chronologische FP32-Features mit B=8, U=16 und Board-Chunk=4.
Jede Beobachtung aktualisiert das Gedächtnis; gecacht werden erste/terminale
Zustände und jeder 64. Cursor. Es werden keine künftigen Informationen in
frühere Features eingespeist. Nur das terminale Trainingslabel kommt aus dem
bekannten Spielausgang.

Der kopierte `seat_value_head` lernt höchstens 20 Epochen mit Adam, LR=1e-4,
Batch=256 und Seed 31431. Alle anderen Gewichte bleiben bitidentisch. Jede
neuronal besuchte Sitztrajektorie bekommt dasselbe Gesamtgewicht, unabhängig
von ihrer Länge; innerhalb der Trajektorie sind die Stichproben gleich gewichtet.
Es gibt kein künstliches Gewinner-/Verlierer-Balancing. Der unveränderte Head
ist Epoche 0 und bleibt erhalten, falls kein Kandidat seinen Validierungs-MSE
verbessert. Berichte trennen Sitzanzahl, Team-/Commander-Flags, Entscheidungs-
und passive Zustände sowie Spielphasen (der Encoder clippt Turn>=20).

Die Messwerte beschreiben diese Stichproben, keine vollständige Frame-Auswertung.
Die Baselines sind Quell-Head, gleichverteilte aktive Sitze mit Teamaggregation
und allein aus Trainingsdaten geschätzte Sitzanzahl-Priors. Die Entscheidung
über weitere Policy-Arbeit verlangt auch brauchbare Kalibrierung an echten
Entscheidungen; ein gutes Gesamtmittel der passiven Zustände genügt nicht.
Export prüft danach ONNX-Parität für K=0/1/2/4/8. Es erfolgt keine automatische
Promotion und kein blind angeschlossener PPO-Lauf.

## Validierung der Implementierung

Acht fokussierte CPU-Tests prüfen unter anderem den tatsächlichen Modell-Readout
mit fehlenden/ausgeschiedenen/Team-Sitzen, vollständige chronologische Erinnerung,
nachträgliche Trajektorienberechtigung, erste Nicht-Null-Cursor, Lernfähigkeit bei
bekanntem synthetischen Signal, identische Policy-/Gedächtnisausgaben, die Wahl
des unveränderten Heads bei Regression, Cache-Korruption, Provenienz und atomare
Veröffentlichung. Ein unabhängiger Agent prüfte die Kernlogik. Der Banding-Fix
bestand alle 68 Tests der Trainingsbibliothek mit ONNX-Feature.

Der lokale Wecker überwacht Abschluss und Fehler des Folgelaufs. Status und
Ergebnis liegen in dessen `status.json` und `result.json`; geplante Schritte
sind dadurch nicht als bereits erzielte Trainingsresultate zu lesen.

## Anschließend

Zuerst Ergebnisse, ausgeschlossene Spiele, Fallzahlen und Decision-Kalibrierung
prüfen. Bei brauchbarem Critic folgen kleine Policy-Updates mit ausreichend
echten Entscheidungen pro Update und vollständigem Verhaltenstest. Die
KL-Grenze bleibt aktiv. Ein neuer gepaarter Vergleich gegen eingefrorene
Hausgegner, v3 und den bisherigen v4-Stand entscheidet über Spielstärkefortschritt.
Historische neuronale Gegner und echtes Selbstspiel erfordern weiterhin eine
Erweiterung des Collectors; sie sind noch kein Bestandteil dieses Piloten.

## Ergebnis und Review am 25.10.2026

Die Sammlung und die Kalibrierung sind abgeschlossen. 247 von 248 Spielen wurden
beendet und ohne Replay-Ablehnung konvertiert; ein Sechser-FFA der Validierung
gegen Sharp erreichte das Limit und bleibt ausgeschlossen. Das Training nutzte
10.311 gecachte Zustände aus 168 Lerntrajektorien. Die Validierung umfasst
6.273 Zustände aus 103 Lerntrajektorien in 95 Spielen. Zusätzliche Trajektorien
stammen von neuronalen Teamkollegen. Die Validierung enthält nur acht gewinnende
Lerntrajektorien, 85 verlorene und zehn mit anteiliger Remis-Utility; entsprechend
unsicher bleiben Aussagen über seltene positive Ausgänge.

Epoche 20 wurde anhand des vorab festgelegten Gesamt-MSE ausgewählt. Alle Zahlen
unten gewichten zunächst jede Trajektorie gleich und dann deren Stichproben;
die Teilgruppen haben jeweils ihre eigene Normalisierung.

| Validierungsgruppe | Quell-Head MSE | Neuer Head MSE | Prior aus Trainings-Sitzanzahl |
| --- | ---: | ---: | ---: |
| Alle Zustände, 6.273 Stichproben | 0,065756 | 0,053246 | 0,069596 |
| Entscheidungszustände, 733 Stichproben | 0,060437 | 0,064344 | 0,073615 |
| Passive Zustände, 5.540 Stichproben | 0,068344 | 0,052507 | 0,069596 |

Der Gesamtfehler sinkt um 19,0 %. An Entscheidungsstellen steigt der gemessene
Fehler dagegen um 6,5 %; deren ECE verschlechtert sich von 0,065919 auf 0,090678.
Das ist kein nachgewiesen besserer Zugbewerter. Insbesondere bei Team- und
Commander-Zuständen schlägt der neue Head den einfachen Trainingsprior nicht
durchgehend. Die MSE-Zahlen des früheren Tiefenpiloten stammen aus anderen
Lehrer-/Validierungsdaten und dürfen nicht direkt mit diesen Zahlen verglichen
werden.

Ein unabhängiger CPU-Audit wiederholte die Head-Vorhersagen und ermittelte
gepaarte Intervalle mit 20.000 Bootstrap-Stichproben ganzer Spiele. Dabei bleiben
Teamtrajektorien zusammen. Für die MSE-Differenz neuer minus alter Head ergeben
sich insgesamt −0,012510 mit 95-%-Intervall [−0,024421; −0,001580], an
Entscheidungsstellen +0,003908 mit [−0,007609; +0,013739]. Die beobachtete
Decision-Regression ist damit statistisch nicht eindeutig. Auch ein Vorsprung
gegenüber dem Trainingsprior ist nicht abgesichert. Diese Intervalle sind
Diagnostik auf den bereits zur Epochenwahl genutzten Entwicklungsdaten; sie
berücksichtigen den Auswahlvorteil nicht und ersetzen keinen unabhängigen Test.
Die kleinere Gruppe mit neuronaler Entscheidungsprovenienz zeigt dieselbe
Tendenz; der Feature-Cache enthält jedoch keine vollständigen Auswahlmasken,
um dort erzwungene Entscheidungen erneut strukturell auszuschließen.

Nur `seat_value_head.*` wurde verändert. Policy, gemeinsamer Encoder und
Gedächtnis blieben bitidentisch. Der Export prüfte 79 Frames für K=0/1/2/4/8;
die maximale ONNX-Abweichung beträgt 0,00001812. Kalibrierung einschließlich
chronologischer Feature-Extraktion dauerte rund 4 Stunden 21 Minuten. Es gab
keine Promotion und keinen durch dieses Value-only-Training nachgewiesenen
Spielstärkegewinn. Die vollständigen Befunde stehen lokal in `result.json`,
`model/report.json` und `review-statistics.json` des Critic-Experiments.

Der inzwischen ausdrücklich beauftragte
[Sharp-Lauf](trained-ai-v4-sharp-intensive-2026-10-25.md) läuft weiter. Er nutzt
vollständige terminale Monte-Carlo-Returns; der eingefrorene Critic ist lediglich
eine aktionsunabhängige Zustandsbaseline. Seine begrenzte Güte kann die Varianz
der Updates erhöhen, ersetzt aber keine echten Ergebnisse durch geschätzte
TD-Ziele. Die gemischte Kalibrierung blockiert diesen kontrollierten Versuch
daher nicht. Sie rechtfertigt weiterhin weder Value-basierte Suche noch eine
Behauptung über die Qualität einzelner Züge. Quelle und Versuchsplan werden
während der laufenden Runde nicht nachträglich ausgetauscht.

Für den nächsten Critic-Versuch muss die Auswahlmetrik gezielt die echten
Lernentscheidungen abbilden, zusätzlich zum Gesamt-MSE. Entscheidungsmasken
sollen im Cache eine erneute Prüfung erzwungener Entscheidungen erlauben.
Ein vergrößerter, vorab festgelegter Datensatz und eine separate Bestätigung
müssen klären, ob eine solche Auswahl den Quell-Head und den Trainingsprior
übertrifft. Weitere Epochen auf derselben kleinen Validierung lösen diese
Unsicherheit nicht.
