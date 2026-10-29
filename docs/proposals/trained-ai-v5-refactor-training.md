# BayleeAI v5: Refactoring, Untersuchungen und nächstes Training

Stand: 29. Oktober 2026. Auftrag: altes Training stoppen, R0–R4 umsetzen,
die nicht auf PyTorch angewiesenen Trainingswerkzeuge durch Rust ersetzen und
anschließend begrenzte, aussagekräftige Lernversuche durchführen.
Arbeitsbasis: `codex/payment-context-v5-20261028`; die ursprünglich bei
`b9cf382ba` begonnenen Untersuchungen sind inzwischen in Implementierung übergegangen.

**Entscheidung:** Zuerst die nachgewiesenen Fehler in Aktionsbewertung und
Trainingsverträgen beheben. Danach kleine, getrennte Lernversuche mit vollständiger
Datenabdeckung, anschließend Lernen aus echten Spielergebnissen. Mehr Schichten,
ein weiteres Rater-Netz oder ein Weltmodell sind dafür zunächst nicht nötig.
Der Nutzen von v5 gegenüber v4 ist weiterhin nicht belastbar nachgewiesen.

Dieser Plan ersetzt die noch nicht gestarteten Folgeaktionen aus dem
[Review vom 29. Oktober](../history/trained-ai-v5-round10-review-2026-10-29.md),
insbesondere den unveränderten Start des vorbereiteten Full-Coverage-Paars.
Historische Manifeste bleiben erhalten. Maßgebliche Verträge stehen in
[trained-ai](../trained-ai.md), [Architektur](../architecture.md) und
[Lookahead-Design](lookahead-design.md).

**Aktueller Umsetzungsstand:** R0–R4 sind implementiert und in gezielten Tests
geprüft. Der neue Kandidatenkontext hat eine eigene Revision; Encoder8 bleibt
eine ausdrückliche Daten-/Modellentscheidung. Der gespeicherte Denkstufenvertrag
prüft Wiederaufnahme und getrennte vollständige K1/2/4/8-Validierung.
Head- und Full-PPO verlangen dieselbe native Datenzulassung. Full-PPO rekonstruiert
die aktuelle Policy-Historie und rollt bei abgelehnten Updates Gewichte,
Optimizer und RNG gemeinsam zurück. Der gemischte Collector führt getrennte
Policy-/Gedächtnis-/RNG-Zustände und Sitzrollen.

Ein echtes vollständiges, verlorenes Testspiel durchlief Collector → Encoder8-
Konverter → native Zulassung → Wahrscheinlichkeits-Replay: 919 Frames,
22 zugelassene Lernerentscheidungen, kein Fallback/Cap/Refusal, maximale
Logprob-Abweichung `2,38e-7`. Die Haus-KI-Aktionen sind keine PPO-Ziele.
Die Partie wurde nicht nach ihrem Ergebnis ausgewählt. Das ist ein technischer
Integrationstest mit kleinem Testnetz, kein Lern- oder Spielstärkennachweis.
Beleg: `candidate-context/native-01a18748-e70c-7010-b34a-cc750938be81/mixed-policy-receipt.txt`.

Rust übernimmt bereits Datenpacking, Caches, Zulassung, Importe, Auswertung,
begrenzte Jobs/Pipelines, Paketverwaltung und Trainingsüberwachung. Die installierte
allgemeine Überwachung benutzt die nativen Binaries; ihre bisherigen
Zustellquittungen bleiben unverändert. Auch Collector-Betriebssteuerung und
Quota-Wiederaufnahme sind nativ umgestellt. Alle drei lokalen Gateways verwenden
die Rust-Supervisoren. Eine verwaiste Collector-Partie wurde nach Prüfung von
Prozesszuordnung, Stop-Marker und fehlenden Spielern aufgeräumt; ihre unvollständige
Aufzeichnung bleibt ohne erfundenes Ergebnis vom Training ausgeschlossen.
Fünf alte Controller sind mit gesicherten Originalen maskiert.
Der erste E1-Vergleich und zwei vorab festgelegte Wiederholungen sind abgeschlossen:
der reparierte Kontext senkt in allen drei Paaren den Choice-NLL, verbessert aber
hauptsächlich Pass-Ziele; die Treffer auf Nicht-Pass-Ziele fallen in allen drei
Paaren. Das ist kein Spielstärkennachweis. Der
[Umsetzungs- und Forschungsbericht](../history/trained-ai-v5-rust-refactor-2026-10-29.md)
enthält Dosis, Prüfergebnisse und getrennte Fehlversuche.
Die Gates fanden außerdem eine Deployment-Reihenfolge, die mit dem neuen
Aufräumen unvereinbar war. Die Korrektur hält neue Partien zurück und lässt
laufende Spiele enden, bevor ihr Agent gestoppt wird. Default- und Feature-Gate
bestanden auf den im Bericht festgehaltenen Zwischenständen; das Default-Gate
auf `31e6a78c0` bestand alle 23.916 Tests. Die späteren Encoder9- und
PPO-Änderungen haben gezielte Prüfungen; vor einem Push sind beide vollständigen
Gates erneut erforderlich. Die mangels lokalem Scryfall-Cache ausgelassene
Katalogvalidierung und weitere Prüfgrenzen sind im Bericht ausgewiesen.
Der alte Dauerlauf bleibt gestoppt; kein neues Modell ist freigegeben.

**Nach dem E1-Spielvergleich:** Alle 48 vorab festgelegten Modellfälle sind
abgeschlossen. Deskriptiv erreicht R6 6/16 Siege, beide trainierten Kandidaten
4/16; Haus-Fallbacks treten 13/8/3-mal auf. Die ursprüngliche Zulassungsregel
markiert deshalb den betroffenen Viererblock für alle drei Modelle als
unbekannt: 36 gültige und 12 unbekannte Zeilen, keine gepaarte Aussage und keine
nachträgliche Auswahl nur sauberer Fälle. Der reparierte Kandidatenkontext
bleibt ein behobener Architekturfehler, bislang ohne nachgewiesenen
Spielstärkegewinn. Keine automatische Dosiserhöhung oder Freigabe.

Die konkrete Zahlungsdiagnose liefert eine nützlichere nächste Aufgabe:
Von 39 wiederholten R6-Abbruchentscheidungen hätte ein einzelnes angebotenes
`PassPriority` 21 Sprüche erfolgreich bezahlt und auf den Stack gelegt; bei
18 hätte die Engine den unbezahlten Vorgang rückabgewickelt. Die Stellen sind
deterministisch nachgespielt. Das ist weder ein Optimalitätslabel noch ein
Nachweis, dass Encoder8 sie löst. Diese Validierungspartien bleiben Diagnose-
und Regressionstestmaterial, keine Trainingsbeispiele.

Der Stand und die nächste Reihenfolge lauten:

1. **Umgesetzt (`2ca3238a4`):** Zahlungsabschlüsse mit sichtbarem `casting`,
   `announced_cast` oder `owed` behalten ihr Lerngewicht. Die alte Idle-Regel
   konnte diese bei leerem Stack außerhalb des eigenen Zugs sogar ausschließen.
   Neue Konvertierungen deklarieren `idle_pass_weighting_revision=2`; alte Daten
   und Caches werden nicht umetikettiert. Echte Engine-Regressionstests decken
   Encoder7/8 und beide Zugkontexte ab.
2. **Umgesetzt (`a414cf194`):** Loss5 gewichtet kategorische Entscheidungen
   explizit symmetrisch (`categorical_weighting="symmetric_v1"`). Exaktes Resume
   verweigert alte Loss5-Verträge ohne eindeutigen Marker; frische Initialisierung
   aus deren Gewichten bleibt möglich. Loss4 bleibt unverändert. E1 benutzte
   Loss4; dieser zweite Fehler erklärt seinen Ausgang ausdrücklich nicht.
3. **E2-Vorprobe abgeschlossen:** Beide Modelle spielten dieselbe vollständige
   Historie mit 811 Beobachtungen nach; die ersten 32 echten Entscheidungen
   wurden bei gleichem kausalem H mit K1/2/4/8 verglichen. Alle 31 kategorialen
   Fälle und der eine Subset-Fall zeigen veränderte Wahrscheinlichkeiten. Die
   Denkstufen wirken, ihr Nutzen ist weiterhin unbewiesen. Keine tatsächlichen
   Subset-/Paar-Aktionen dekodiert, keine Optimizer-Updates. Der eigentliche
   Vergleich von festem K und trainiertem gemischtem K steht noch aus.
4. **Sammlung und native Zulassung abgeschlossen:** Acht vorab festgelegte
   Trainingsdeck-Partien gegen Sharp/Steady sind vollständig und ohne
   Cap/Fallback/Refusal/Stall beendet. Alle acht bleiben im Datensatz:
   13.655 Sitz-Frames, acht Lernerhistorien und 1.575 echte Lernerentscheidungen,
   keine Validierungsdaten oder nachträglichen Ausschlüsse. Die zwei Siege und
   sechs Niederlagen sind keine belastbare Stärkeaussage. Vollständige
   Wahrscheinlichkeits-Replays und CPU/CUDA-Gradientenproben sind bestanden.
   CUDA B8/U32/Board-Chunk4 reduziert die gemessene Probe auf 30,84 Sekunden
   gegenüber 175,35 Sekunden bei CUDA B1; die Gradienten stimmen innerhalb
   der vorab festgelegten Toleranz überein. Diese Ausführung ist für genau
   einen Head-PPO-Updateversuch mit allen 1.575 Entscheidungen ausgewählt.
   Dieser ist nach repariertem Prozessstart erfolgreich abgeschlossen:
   ein akzeptierter Schritt, 24 geänderte Policy-Tensoren, 542 unveränderte
   Tensoren, vollständige Abdeckung und finale KL `3,88775e-6` bei Limit `0,02`.
   Das belegt den technischen Lernweg, noch keine bessere Spielstärke.
   Ein zusätzlicher reparierter Referenzfehler (`09a30de36`) betrifft beide
   PPO-Pfade: Nach bestandenem Replay bleiben die tatsächlich aufgezeichneten
   Logprobs Nenner und KL-Referenz, statt sie durch numerisch leicht abweichende
   Replaywerte zu ersetzen. 26 gezielte CPU-Tests bestehen.
   Kein nachträgliches Austauschen schwieriger Partien, kein Senken der
   Zulassungsschwelle, keine Lockerung des KL-Limits nur für einen akzeptierten
   Update. Der spätere gemischte E3-Vergleich bleibt eine eigene Untersuchung.
5. **E3-Vorprobe abgeschlossen, größere Sammlung zurückgestellt:** Vier vorab
   festgelegte Fälle gegen einen historischen Netzstand und in Selfplay sind
   beendet. In beiden historischen Fällen greift der Wiederholungsschutz ein:
   Fireball mit erzwungenem X=0 und mehreren unbezahlbaren Zielen wird dreimal angekündigt
   und automatisch zurückgesetzt. Das deterministische Replay bestätigt die
   Schleife; die zwei sauberen Selfplay-Fälle werden nicht nachträglich als
   Trainingssatz herausgepickt. Keine der vier Partien ist zugelassen, die
   übrigen 124 sind nicht gestartet. Zusatzkosten, Quellkarte und Mana sind in
   den Eingaben vorhanden. Acht kontrollierte Ankündigungszweige bestätigen
   die Kostenrechnung und einen separaten Beobachtungsfehler: X=0 und X=1
   liefern bei der Zielwahl identische E7/E8-Eingaben, einschließlich der
   vollständigen zehn Beobachtungen davor. Die eigene Zahlenwahl fließt nicht
   in H zurück. Das beweist die X-Lücke, nicht ihre Ursächlichkeit für die
   tatsächlichen X=0-Schleifen. Der native Beobachtungsteil ist in `1ee48685b`
   mit 57 gezielten Tests umgesetzt: separate autorisierte Ankündigungsansichten
   unterscheiden unbekanntes X und gewählte Null; alte Ansichten bleiben gleich.
   Encoder9 bindet diese Information inzwischen separat durch native Sammlung,
   Replay, versionierte Hashes und 82 statt 80 globale Eingänge an. Bestehende
   E7/E8-Artefakte behalten ihren Vertrag. Der explizite Gewichtsübergang beginnt
   mit zwei Nullspalten; ein kleiner PyTorch-Lerntest bestätigt deren Lernbarkeit
   bei trainierbarer Projektion. Das ist noch kein Spielstärkennachweis.
   Zusätzlich wird geprüft, wie Fehlentscheidungen
   lernbar bleiben, ohne Haus-Fallback-Ergebnisse der reinen Policy zuzuschreiben.
   Kein Lockern des Guards oder nachträgliches Ändern alter Datenverträge.
6. **Einzelschritt-Transfertest abgeschlossen:** Alle vier E3-Vorprobenfälle
   sind mit dem aktualisierten Netz und unveränderten Gegnern nachgespielt.
   Alle Ergebnisse, Fallbacks und Fireball-Rückabwicklungen bleiben gleich.
   Ein Fall hat eine echte Änderung von vier auf drei Ziele, beide Varianten
   bleiben unbezahlbar. Die anderen drei Aktionsspuren bleiben identisch.
   Der Schritt wirkt auf die Policy, behebt den Fehler aber bislang nicht.
   Ein Artefaktcheck stoppte den ersten Vergleich nach zwei Spielen; nur die
   beiden ungestarteten Fälle wurden anschließend ausgeführt. Kein erneuter
   Lauf bereits gespielter Fälle und keine nachträgliche Auswahl.
7. **Full-PPO-Vorprobe abgeschlossen:** Vom ursprünglichen R6/E7 aus wurden
   acht von acht begrenzten Updates akzeptiert, mit frischem Optimizer und
   unveränderten Daten-/Architektur-/Belohnungsverträgen. 311 von 566 Tensoren
   änderten sich, einschließlich Trunk, relationalem Modul und Gedächtnis.
   Finale KL über die gesamte Sammlung: `4,00522484e-5`. Die Updates selbst
   umfassten 131 Lernerentscheidungen in 512 Frames; die vollständigen 1.575
   Entscheidungen wurden vor und während des Lernens geprüft. Keine volle
   Epoche und noch kein Spielstärkennachweis. Laufzeit 988,33 Sekunden,
   maximal 2,51 GiB reservierter VRAM, keine Allocator-Retries. Der nächste
   getrennte Schritt ist ein Vergleich auf 16 neuen Entwicklungsfällen je
   Modell mit gleichen Seeds und vertauschten Sitzen. Betriebsstand,
   Fehlversuche und Übernahme sind im [Trainingshandbuch](../training-runbook.md)
   und der [aktuellen Übergabe](../history/training-handover-2026-10-29.md) verlinkt.

Die folgenden Befunde dokumentieren die Ausgangslage und ihre Reparaturen;
historische Aussagen „noch nicht implementiert“ sind durch diesen Stand ersetzt.

**Früher R0-Meilenstein:**
Head-PPO veröffentlicht jetzt über `source_identity` auch Architektur-, Familien-
und Katalogmetadaten. 20 bestehende Tests und zwei neue Regressionstests bestehen;
der neue Roundtrip umfasst 16 Modell-/Update-Ausgänge einschließlich alter v5r1-
Konfigurationen und unveränderter Rückfallmodelle. Ein tatsächlich veröffentlichtes
synthetisches v5r2-Testmodell besteht CPU-ONNX-Parität bei K0/1/2/4/8
(15 Frames, maximale Abweichung `5,96e-7`). Datenadmission ist in diesen
Regressionstests eine Fixture; die spätere echte Gameplay-/Encoder8-Integration
steht oben. Belege: `refactor-investigations-20261029/r0-publication-01a18655-087f-700d-8a63-bd73090097d9/`.
Das verspätete Terminal-Ereignis `training:49efb950e5b82472:terminal` wurde gegen
den tatsächlichen Besitzer-Stopp geprüft und dedupliziert, ohne Neustart.

Die vorbereitende R1-Modultrennung ist ebenfalls abgeschlossen (`e4e3318fa`):
`PlayerNet4` liegt bei unveränderten öffentlichen Imports in `model4/player.py`.
2.730 Tensorvergleiche einschließlich 1.918 Gradienten sind bitgleich über
v4/v5r1/v5r2 und K0/1/2/4/8; neun vorhandene Modelltests bestehen.
Darauf aufbauend wurde die versionierte Kontextberechnung umgesetzt. Belege
der vorausgehenden verhaltensneutralen Trennung:
`refactor-investigations-20261029/r1-move-01a18665-1952-75d2-a1dd-93f000faed08/`.

## 1. Betriebsstand und gesicherte Arbeit

- `baylee-continuous-v5-training.service` ist gestoppt; Controller und
  GPU-Trainingsprozess sind nachweislich beendet. Automatischer Neustart der
  bisherigen Trainingsrezeptur ist deaktiviert.
- Runde 11 war in `llm-bc-mix`, geplant waren 7.374 Updates. Gesichert ist der
  vollständige Wiederaufnahmezustand bei **Schritt 1.800**, einschließlich
  Optimizer, RNG und Gedächtnis. Das ist eine **unvollständige Runde 11**.
  Eventuelle Updates nach diesem Checkpoint sind nicht als gesichert behauptet.
- Der Checkpoint wurde vor dem Stop per Hardlink gesichert, anschließend geladen
  und geprüft: alle 566 Gewichtstensoren endlich; SHA-256
  `ee976bc3e03b783c112a20da6314b5f324dc7527f628e2e8f2a9d51beb2c655c`.
- Der CPU-Referenzvergleich ist mit allen 256 Fällen und 128 Tisch-Clustern
  abgeschlossen und geprüft: R6 gewinnt 74, Steady 125 Partien, jeweils gegen
  Sharp. Sechs Netz-Partien enthalten insgesamt 55 ausgewiesene Fallbacks.
  Details und Einschränkungen stehen im
  [Referenzbericht](../history/trained-ai-v5-reference-2026-10-29.md).
  Es wurden keine Gewichte geändert. Das Nutzer-Gateway auf Port 28766 bleibt
  verfügbar; es wurde kein neues Modell bereitgestellt.
- Die drei CPU-Untersuchungen und der begrenzte GPU-Denkstufentest sind beendet.
  Dieser GPU-Test machte **null Optimizer-Updates**. Spätere GPU-Arbeit folgt
  ausschließlich den unten beschriebenen begrenzten Experimenten.

Lokale Belege liegen unter
`/home/ace/baylee-data/v5/ops-codex-20261028/refactor-investigations-20261029/`:
`stop-complete.json`, `checkpoint-audit.json`, `preserved-round11-last.pt` sowie
die nachfolgenden Untersuchungsverzeichnisse. Die Betriebsplanung in
`ops-codex-20261028/next-actions.json` verweist auf diesen Plan. Ein verspätetes
Monitor-Ereignis zum alten Training ist kein Auftrag, es erneut zu starten.

## 2. Durchgeführte Untersuchungen

### 2.1 Aktionskontext: echter Fehler, Reparatur isoliert erprobt

Der bisherige Kandidaten-Head berechnet sinngemäß
`logit_i = w · (h_i + A(pool(h))) + b`. Der zweite Term ist für alle Kandidaten
gleich und fällt bei kategorischem Softmax heraus. Er kann weder die Rangfolge
noch die Auswahlwahrscheinlichkeiten ändern. Andere Eingaben des Heads wirken
weiterhin; das Netz ist deshalb nicht insgesamt kontextblind.

Eine begrenzte CPU-Probe am tatsächlichen Deferred-Scorer bestätigt:

- Zero-init einer zusätzlichen multiplikativen Kontextanbindung erhält die
  bisherigen Logits exakt.
- Die neue Anbindung erhält einen kategorischen Gradienten; die alte gemeinsame
  Verschiebung erhält nur numerisches Rauschen.
- Ändert man nur einen dritten Kandidaten, kann die Reparatur das relative
  Urteil über die unveränderten ersten beiden Kandidaten beeinflussen.
- Maskierung, Permutation und ein Menü mit 1.025 Kandidaten bestehen die Probe.

Der alte Kontext bleibt erforderlich: Bei Subset-Sigmoid/BCE wirkt die gemeinsame
Verschiebung auf die Einschlusswahrscheinlichkeit. Paar- und Zahlen-Head benutzen
diese Operation nicht. Belege: `candidate-context/{report.txt,probe.py,results.json}`.
Diese ursprüngliche Probe war kein Nachweis besserer Spielstärke. Die spätere
versionierte Implementierung besteht Ausführungs-/Exporttests einschließlich
K0/1/2/4/8; ein Spielstärkennachweis steht weiterhin aus.

### 2.2 Denkstufen: ursprüngliche Vertragslücke, inzwischen behoben

Die echte v5-CPU-Ausführung wurde sequentiell, deferred und kompakt verglichen:
K1/2/4/8 sowie die bestehende gemischte Folge. Gültige Ausgaben, maskierter Loss
und alle Parametergradienten stimmen innerhalb kleiner FP32-Abweichungen überein
(maximal etwa `6e-8` bei Ausgaben, `2.4e-7` bei Gradienten).

Das persistente H ist bei allen positiven K identisch. Passive K0-Beobachtungen
aktualisieren H; der Entscheidungsscratchpad bei K>0 überschreibt H nicht.
Mehr Denken bedeutet hier zusätzliche Verarbeitung derselben Entscheidung,
keine zusätzliche simulierte Partie und keine längere Beobachtungshistorie.

`--thinking-curriculum` war eine periodische Folge
`(1,2,4,8)[(step+t)%4]`, keine fortschreitende Schwierigkeitskurve. Das Flag fehlte
im Resume-Vertrag. Validierung und Checkpoint-Auswahl prüften nur das konfigurierte
Standard-K. R2 ersetzt diese impliziten Annahmen durch den gespeicherten Vertrag
und getrennte vollständige Validierung je K. Belege der Ausgangsdiagnose:
`depth-contract/{report.md,probe.py,results.json}`.

### 2.3 v5-PPO: ursprüngliche Veröffentlichungs- und Integrationsfehler

Der Head-PPO-Trainer konnte v5 laden, ließ beim Schreiben jedoch `architecture`
und `model_family` weg. Die ursprüngliche Lade-/Update-/Speicherprobe reproduzierte für
beide v5-Relationsrevisionen ein anschließend nicht mehr ladbares Modell.
Die Wiederverwendung der bereits existierenden `source_identity`-Funktion löst
diesen isolierten Fehler. Die sieben bestehenden Policy-Training-Tests bestehen;
sie deckten diesen v5-Roundtrip bisher nicht ab.

Darüber hinaus war der vollständige rekurrente PPO-Trainer v4-spezifisch.
Encoder8 wurde im gemeinsamen Kalibrierungsloader noch vor dem Modelldispatch
abgewiesen. R4 behebt diese Integrationslücken mit ausdrücklich geprüften
Modell-/Datenverträgen. Der bestehende Live-Seat spielt mit Temperatur null und liefert damit keine
für PPO geeigneten stochastischen Verhaltenswahrscheinlichkeiten. Das sind
getrennte Integrationsaufgaben, kein Grund, Versionsprüfungen einfach zu lockern.
Der neue stochastische Sammelpfad läuft deshalb zuerst über den Offline-Collector.
Belege: `v5-rl/{report.md,reproduce.py,results.json}`.

### 2.4 GPU-Diagnose: zusätzliches K hilft dem bestehenden Modell hier nicht

Fixiertes R6-Modell, dieselben fünf vollständigen LLM-Entwicklungsspiele,
7.608 Beobachtungen, 1.030 echte Einzelentscheidungen, davon 716 mit aktivem
Lehrerziel; FP32, B8/U32, unabhängig zurückgesetzte Historien pro K:

| K | Trefferquote Einzelentscheidungen | Trefferquote aktive Lehrerziele | NLL Einzelentscheidungen |
| --- | ---: | ---: | ---: |
| 1 | 52,33 % | 47,21 % | 1,26435 |
| 2 | 52,23 % | 47,07 % | 1,26730 |
| 4 | 52,23 % | 47,07 % | 1,27039 |
| 8 | 52,23 % | 47,07 % | 1,27165 |

Die Unterschiede sind klein. Das ist Lehrerübereinstimmung auf bereits
benutzten Entwicklungsdaten, weder Siegquote noch frischer Holdout. Insbesondere
belegt es keine Überlegenheit von K1. Es liefert **keinen Anlass, K8 ohne
passendes Training als stärkere Schwierigkeit auszuliefern**.

Jeder Durchlauf dauerte einschließlich Modell-/Datenladen ungefähr 16 Sekunden,
bei rund 1,48 GiB reserviertem GPU-Speicher und ohne Allocator-Retry. Das ist
Inferenz auf einem kleinen Datensatz, kein Training mit Backward/Optimizer und
kein Live-Latenzbenchmark. Daraus folgt keine K8-Trainingskapazität auf 12 GB.
Artefakte: `gpu-depth-01a18644-c0e0-7578-a990-58ca8ea53bc5/`, insbesondere
`plan.json`, `result.json`, `k1.json` bis `k8.json` und die Job-Manifeste.

### 2.5 Gegenprüfung durch Opus und Fable

Nach erneuerter Anmeldung haben `claude-opus-5-5` und `claude-fable-5-1` den
aktuellen Stand unabhängig geprüft und danach gegenseitige Rückmeldungen sowie
Codekorrekturen erhalten. Beide Runden sind abgeschlossen. Der
[Konsultationsbericht](../history/trained-ai-v5-opus-fable-review-2026-10-29.md)
trennt bestätigte Befunde von zurückgenommenen Schlussfolgerungen.

Ein zusätzlich reproduzierter Fehler betraf die PPO-Wahrscheinlichkeitsrechnung:
Python klemmte bei Subset-/Paaraktionen zunächst Sigmoid-Wahrscheinlichkeiten und
nahm anschließend deren Logarithmus; Rust protokolliert stabile Log-Sigmoid-
Ausdrücke. Eine CPU-Probe der ursprünglichen Python-Funktion liefert bei skalierten Logits
bis ±40 Abweichungen bis 24,06 Logprob-Einheiten und in acht von 18 Fällen einen
Nullgradienten trotz nichtverschwindender Referenzableitung. Schon bei +14 und
Nichtauswahl liegt die Abweichung bei etwa 0,00353, oberhalb des bestehenden
0,001-Auditlimits. Das ist kein belegter Auslöser der bisherigen Trainingsergebnisse.

Die Korrektur ist umgesetzt: unveränderte Auslagerung in `policy_math4.py`
(`7308f55df`), danach stabile Log-Sigmoid-Rechnung einschließlich komplementärem
Ereignis, KL und Maskierung (`42633ebe6`). Sieben neue Regressionstests und
38 weitere vorhandene Checks bestehen. Die 18 ursprünglichen Proben haben jetzt
keine Abweichung zur stabilen Referenz und keine irrtümlichen Nullgradienten.
Auditgrenzen, Samplingversionen und Provenienz bleiben unverändert. Die echte
Rust-/ONNX-/PPO-Datenkette vor E3 bleibt gesondert zu prüfen.

## 3. Refactoring in prüfbaren Schritten

### R0 — Verträge, Veröffentlichung und Modulgrenzen

1. Große betroffene Modelldateien zuerst in einem reinen Verschiebe-Commit
   zerlegen. Bestehende Importpfade, Parameternamen, RNG-Reihenfolge und Verhalten
   erhalten; danach getrennte Verhaltensänderungen.
2. Gemeinsame, strikte Modellidentität für Laden, Training, Veröffentlichung,
   Export und Rust-Zulassung verwenden. Architektur-, Encoder-, Relations- und
   spätere Kandidatenrevision getrennt prüfen. Unbekannte Kombinationen ablehnen.
3. Head-PPO-Veröffentlichung über vorhandene Identitätslogik reparieren.
   Encoder8-Unterstützung bewusst über `encoder_contract` integrieren;
   Encoder7-Daten niemals durch Umetikettieren zu Encoder8 machen.
4. Tests für Laden → Update/abgelehntes Update → Speichern → Laden → ONNX-Export
   ergänzen, einschließlich Null akzeptierter Updates, alter v4/v5-Checkpoints
   und jeder tatsächlich unterstützten Encoder-Kombination.

Abnahme: archivierte Modelle laden unverändert, unbekannte Identitäten werden
abgewiesen, neue Veröffentlichungen sind tatsächlich verwendbar. Kein Eingriff
in laufende Gateway- oder Referenzprozesse.

### R1 — Kandidatenkontext wirksam machen

Erste Reparatur:
`h_neu_i = h_i + A(pool(h)) + h_i * tanh(G(pool(h)))`.
`G` startet vollständig bei null. Bei d=640 kommen 819.840 Parameter hinzu;
eine quadratische Kandidaten-Self-Attention ist dafür nicht nötig.

Status: Modultrennung, versionierte Kontextänderung, expliziter Gewichtsimport
und gezielte Ausführungs-/Exporttests sind umgesetzt. Die folgenden Kriterien
bleiben die Verträge; der begrenzte E1-Kopfvergleich und sein Spielvergleich
sind abgeschlossen, ein Vorteil eines umfangreicheren Trainings ist noch offen.

Vor einem vollen E1 ist eine begrenzte G-only- oder Kopftraining-Probe auf einem
kleinen, vorab festgelegten Satz vollständiger Partien sinnvoll. Bei G-only
bleiben alle bisherigen Gewichte eingefroren. Die Probe prüft Optimierbarkeit
und Verteilungsänderung; ihr Scheitern beweist weder Unbrauchbarkeit des Gates
noch Aussichtslosigkeit eines gemeinsamen Trunk-/Kopftrainings. Erfolgreiches
Fitten beweist keine Spielstärke. Kontroll- und Reparaturarm erhalten identische
Lanezuordnung und Samplerseeds; ihre reine Zahlenidentität wird protokolliert.

- Eigene `candidate_context_revision`; vorhandene Modelle bleiben Revision 1.
  Erste neue Kombination: v5/Relationsrevision 2/Kandidatenrevision 2 mit
  eigener Architekturkennung. Die Relationsrevision bleibt dabei unverändert.
- Ein gemeinsamer Scoring-Helper für Referenz- und Deferred-Pfad. Pooling über
  das gesamte gültige Menü, auch wenn die Kandidaten in 512er-Blöcken berechnet
  werden. Padding oder Nachbarpartien dürfen nicht in dieses Pooling gelangen.
- Expliziter Gewichtsimport in einen neuen Versuch: alle alten Tensoren exakt
  kopieren, nur G neu; frischer Optimizer und neue Datenlanes. Kein implizites
  Upgrade eines alten Resume-Checkpoints. Die Kontrollgruppe erhält dieselbe
  Neustartpolitik, damit nicht zusätzlich der Optimizer verglichen wird.
- Neue Parameter in beide Optimizer-Auswahlen aufnehmen: `freeze_trunk` und
  `policy_training4.POLICY_PREFIXES`. Freeze-/Rollback-Prüfung mitführen.
- Der alte Subset-Kontext bleibt. Nach Training darf die neue Anbindung auch
  Subset-Ausgaben beeinflussen; deren Kalibrierung wird deshalb mitgemessen.

Abnahme: zunächst identische alte Ausgaben, anschließend nachweisbare
kandidatenabhängige Wirkung und Gradienten; vollständige Loss-/Gradientenparität
der Ausführungspfade; Masken, leere gültige Menüs mit Sentinel, große Menüs,
Paar-/Zahlenentscheidungen und K0/1/2/4/8; PyTorch/CPU-ORT/native Parität.

### R2 — Denkstufen reproduzierbar trainieren und bewerten

Eine gemeinsame Schedule-Funktion mit gespeichertem Vertrag einführen:
Version, Modus, erlaubte positive K, Phasenregel, Auswahl-K und berichtete K.
Zunächst `fixed` und explizites `legacy_frame_cycle` unterstützen. K0 bleibt
allein der Beobachtungsmodus. Ein zufälliger Schedule wäre eine weitere
versionierte Variante mit eigenem gespeicherten RNG.

Resume prüft den ganzen Schedule und Validierungsvertrag vor Optimizer/H-Laden.
Bei alten Checkpoints lässt sich ein fehlendes Flag nur aus dem gesicherten
Aufruf/Manifest herleiten; sonst ist es eine neue Fortsetzung, keine exakte
Wiederaufnahme. Pro K separate vollständige Validierungshistorien durchführen.
Checkpoint-Auswahl bleibt zunächst vorab festgelegt bei K2; die anderen K sind
benannte Diagnosen. Tatsächliche Entscheidungen und Loss-Gewichte je K zählen.

Vor E2 Schrittänderungen im latenten Zustand **und** in Logits/Entscheidungen
messen. Kleine latente Änderungen allein belegen keinen fehlenden Spieleffekt.
Dass alle Endpunkte dasselbe Lehrerziel erhalten, schließt erlernte iterative
Verbesserung nicht aus; schrittspezifische Ziele sind keine vorab notwendige
Erweiterung. Die ungeübten höheren Schritt-Embeddings des K2-Modells sind kein
fairer Ersatz für einen trainierten Multi-K-Kandidaten.

Abnahme: ununterbrochener Lauf versus Save/Resume des nächsten Updates inklusive
H/Optimizer/RNG; widersprüchliche Verträge werden abgewiesen; Änderungen an
zukünftigen Frames beeinflussen keine früheren Ausgaben; CUDA-/Exportparität.

### R3 — Encoder8 und Spielerwissen gesondert abschließen

Die vorhandenen Zahlungs-/Aktionskontext-Arbeiten nicht mit R1 vermischen.
View-Projektion, deterministischer Replay, Features, Konvertierung, Modellloader
und Export müssen denselben Encodervertrag verwenden. Der Live-Seat darf
Encoder8 erst nach nachgewiesener Wire-Fähigkeit annehmen.

Abnahme an sichtbaren Spielsituationen: Zahlungsabsicht und verbleibende Kosten,
eigene Hand/Deckwissen, öffentliche Zonen, Marken, Instants/Stack und Kampfschritte
bleiben vollständig im erlaubten View abgebildet. Private gegnerische Karten
und Bibliotheksreihenfolgen sind keine Eingaben. Bestehende Daten müssen sauber
neu konvertiert werden können; Änderungen am Encoder sind ein eigenes Experiment.

Der vollständige R6-Referenzlauf liefert konkrete Regressionen für diese Arbeit:
alle 55 Fallbacks in sechs Partien mit `adapted/DDJ/izzet`, davon 53 unmittelbar
nach `CancelCast`. Replays für reproduzierbare Zahlungs-/Abbruchszenarien nutzen
und strukturierte Fallback-Gründe ergänzen. `stalls = 0` darf nicht als Beleg
für ausschließlich neuronale Entscheidungen gelten; der interne Schutz greift
früher als der äußere Stallzähler. Die genaue Ursache jeder einzelnen Auslösung
ist bislang nicht protokolliert. Belege und Grenzen stehen im Referenzbericht.

Diese Diagnose vor größeren Lernversuchen beginnen: Abbrüche über alle 256
Referenzfälle zählen, statt nur die sechs Guard-Fälle zu betrachten; betroffene
Cast-/X-/Ziel-/Zahlungsfolgen deterministisch nachspielen. Engine-Konformität,
angebotene Möglichkeiten und deren sichtbare Kodierung getrennt vergleichen.
Eine angebotene X-Grenze ist laut Encoder8-Vertrag keine Zusicherung, dass jeder
Wert mit den vorhandenen Ressourcen bezahlt werden kann.

Widersprüchliche Lehrerlabels bei gleicher aktueller Kodierung sind lediglich
ein Suchsignal für fehlende Information. Historie beziehungsweise H, Lehrerquelle,
Stochastik und nachweislich gleichwertige Aktionen mitberücksichtigen. Erst eine
nachweislich notwendige, im Beobachtungs-/Historienvertrag fehlende Unterscheidung
oder ein kontrolliertes Gegenbeispiel belegt einen Repräsentationsfehler.
Gleiche aggregierte Trefferraten sind ebenfalls kein Beleg für unveränderte Züge:
zustandsweise richtig→falsch, falsch→richtig und falsch→anders falsch ausweisen.
Same-View-Abweichungen zur Haus-KI dienen der Diagnose, nicht als Wahrheitslabel.

### R4 — v5 aus eigenen Spielergebnissen lernen lassen

Zuerst den vorhandenen **Head-PPO-Pfad** vollständig nutzbar machen. Offline
`eval4` kann unter seinem geprüften Raw-Sampler stochastische Daten liefern;
für den ersten Pilot muss deshalb nicht das Nutzer-Gateway umgebaut werden.
K, FP32-Sampling, Temperatur und vollständige Aktionsmasken werden festgeschrieben.

**Collector-Voraussetzung, inzwischen umgesetzt:** `eval4` unterscheidet
Evaluation von `--collection-purpose on-policy-training`. Neben `--against`
sind `--opponent-model` und `--opponent-role historical|self-play` verfügbar.
Der erste zulässige PPO-Sammelpfad ist 1v1 ohne Teams, Raw/FP32, K2, Temperatur
0,7. Er besitzt getrennte Policy-, Memory- und RNG-Zustände je Sitz sowie
rollengebundene Modellhashes und Lerner-/Gegnerprovenienz.
Aktionen einer anderen neuronalen Policy dürfen nicht allein wegen `source=Net`
als Lerneraktionen in PPO gelangen. Ein erster kleiner Haus-KI-Smoke kann vorher
die vorhandene Kette prüfen; er erfüllt nicht das gemischte E3-Experiment.

Der gemeinsame Dateneingang verlangt echte ausgeführte Aktionen, zertifizierte
Policy-Provenienz, Modellhash, Verhaltens-Logprob und vollständige chronologische
Lernerhistorien. Erzwungene, skalare oder anderweitig nicht unterstützte Aktionen
werden explizit von der PPO-Likelihood ausgeschlossen; die Beobachtungshistorie
bleibt erhalten. Alte greedy Daten und Terra-Aktionen bekommen keine erfundenen
neuronalen Wahrscheinlichkeiten. Vollständiges Wahrscheinlichkeits-Replay gegen
die eingefrorene Behavior-Policy wird nach der Sammlung vor dem ersten Update
geprüft. Die aktualisierte Policy darf davon abweichen; ihre Abweichung wird
pro Update und abschließend über die ganze Sammlung begrenzt.

Subset-/Paarentscheidungen verwenden kanonisch geordnete, bedingte Bernoulli-
Schritte. Sie müssen nicht gemeinsam unabhängig sein. Die freie Maske folgt
den bereits ausgeführten Entscheidungen und Constraints; Gewichte dürfen deren
Besuchsreihenfolge nicht ändern. Python-/Rust-Logprob-Parität einschließlich
ausgewählter und nicht ausgewählter Extremwerte vor E3 prüfen. Die Numerik aus
Abschnitt 2.5 ist korrigiert; Provenienz- und Samplingversionen alter Daten wurden
nicht umgeschrieben. Der Raw-Modus und die erlaubte Actor-Rolle müssen explizit geprüft
sein; taktische Haus-KI-Antworten besitzen keine neuronale Behavior-Likelihood.

Der **Full-Model-Pfad** ist eine getrennte Aufgabe. Eingefrorene Board-Caches,
`no_grad` und gespeichertes Verhaltens-H aus dem Head-Pilot können nicht einfach
bei freigegebenem Trunk weiterverwendet werden. Dafür nötig:

- v5-fähiger Loader, Export und identische strikte Datenzulassung;
- erneute Encoder-/Graph-/Memory-Berechnung für die aktuelle Policy;
- deklarierte Rekonstruktion/Burn-in am Segmentanfang und differenzierbares H
  innerhalb des Unrolls; U32 beschränkt Gradientenkredit, nicht die Lebenszeit von H;
- vollständige Rückwärtsberechnung der Episode-Ziele oder ausdrücklich definierte
  Bootstrap-Ziele; Minibatch-Grenzen sind kein künstliches Spielende;
- feste Behavior-Policy pro Sammelrunde, eigene Memories pro Sitz, KL-Grenzen
  einschließlich Rollback von Gewichten und Optimizer.

Abnahme: nichtverschwindende Gradienten in Graph, Trunk und Memory; richtige
Reset-/Terminal-/Team-Semantik; Wahrscheinlichkeits- und Resume-Parität; gemessener
Backward-Speicherbedarf. Ein Head-PPO-Erfolg beweist kein Lernen des Denkblocks.

## 4. Lernversuche und Reihenfolge

R0/R1 und die Vorbereitung von R2/R4 können unabhängig bearbeitet werden.
GPU-Läufe besitzen exklusiv `gpu-work.lock`. Der alte Dauercontroller bleibt
aus; Folgeversuche bekommen neue UUIDv7-Verzeichnisse und neue Manifeste.
Es ist keine erneute Freigabe des Nutzers nötig, um nach bestandenen Kriterien
den nächsten begrenzten Versuch auszuführen. Die folgenden Budgets sind
Planungsentscheidungen und keine bereits erprobten optimalen Hyperparameter.

| Versuch | Fragestellung | Vorbedingung und Budget | Entscheidung danach |
| --- | --- | --- | --- |
| E0 | Funktionieren Verträge und Ausführung? | Die abgeschlossenen Diagnosen plus R0–R2-Integrationstests, kein Langzeittraining | Fehler beheben, bevor Lernkurven interpretiert werden |
| E1 | Hilft der reparierte Aktionskontext? | Alte/neue Kandidatenrevision, gleicher R6-Ausgangspunkt, K2, gleiche vollständige Daten und Loss-Gewichte; je ein kompletter festgeschriebener Datendurchlauf | Nach Köpfen/Quellen prüfen, dann gepaarte Spiele; keine Übernahme nur nach NLL |
| E2 | Lernt ein bereits trainiertes Modell verschiedene Denkbudgets sinnvoll? | Feste K2-Kontrolle versus kurze K1/2/4/8-Finishing-Phase, gleiche Initialisierung und Quellenabdeckung; zunächst ein vorab versiegelter vollständiger Pilot-Datensatz | Alle K auswerten, Mehrkosten messen; nur bei Nutzen vergrößern |
| E3 | Ist outcome-basiertes v5-Lernen technisch tragfähig? | R0/R4 Head-Pfad; 64 vollständige 1v1-Tische je zwei Sitzbelegungen = 128 Partien, höchstens 32 Update-Versuche und ein Durchlauf durch die zugelassenen Entscheidungen | Datenintegrität, KL, Verhalten und Spiele vergleichen; keine Aussage über Full-Model-Lernen |
| E4 | Verbessert RL auch Repräsentation und Denken? | Separater Full-Model-Pfad bestanden; dieselben Grenzen von 128 Partien, höchstens 32 Update-Versuchen/einem Durchlauf, feste K2 | Erst bei stabiler Verbesserung nächste Sammelrunde; keine Endlosschleife |
| E5 | Helfen neue Relationsziele bzw. Encoder8/9? | Je ein eigenständiges gepaartes Experiment; keine gleichzeitige Änderung von Encoder, Kandidatenhead und K | Gewinn einem konkreten Mechanismus zuordnen |

E3-Vorbereitung muss nicht auf den Ausgang sämtlicher BC-/Denkversuche warten.
Bei identischen gemeinsamen Verträgen darf der begrenzte technische RL-Pilot
früher laufen. Full-Model-RL beginnt erst nach seinen eigenen Abnahmekriterien.

### Datenabdeckung und faire Kontrollen

Die bisherigen prefixbegrenzten LLM-Updates sind als Ursache für ungleichmäßige
Abdeckung belegt; in Runde 10 wurden nur 147 der 546 LLM-Spiele erreicht.
Ein kleineres Loss auf diesem Präfix begründet keine breite Verbesserung.
Der neue Sampler führt vollständige chronologische Partien und weist je Quelle,
Deck, Partie, Kopf und Entscheidungsart die tatsächliche Nutzung aus.

Der bereits versiegelte Full-Coverage-Versuch mit 8.443 Updates je Arm bleibt als
Vorbereitung erhalten, wird aber nicht unverändert gestartet. Gleiche Updates
oder Frames bedeuten nicht gleiche echte Policy-Labels. E1 verwendet in beiden
Armen **dieselben Daten**, um zunächst nur R1 zu messen. Ein späterer Vergleich
von LLM- gegen Haus-KI-Daten muss dessen andere Labeldosis ausdrücklich ausweisen.

E2 verwendet denselben Pilot-Datensatz in beiden Armen, vorab nach Spielidentität
und ohne Auswahl nach Siegen zusammengestellt. Beide erhalten alle Pilotspiele;
der gemischte Arm beaufsichtigt den gewählten Denk-Endpunkt. Messgrößen sind
zusätzlich Rechenschritte, tatsächliche Labels je K, Zeit und VRAM. Gleiche
Updates bedeuten bei unterschiedlichen K keine gleiche Rechenleistung.

### RL-Pilot und Gegnerpool

Für E3 zunächst 32 Tische gegen Hausprofile (Sharp/Steady), 16 gegen eine
eingefrorene historische Policy und 16 gegen den eingefrorenen Lerner selbst;
jeweils zwei Sitzbelegungen. Die Tabelle, Seeds und Samplingverträge werden vor
dem Start fixiert. Für E3 und E4 gilt jeweils zusätzlich: Sammlung höchstens
sechs Stunden, Updates höchstens 30 Minuten; die zuerst erreichte Grenze beendet
die Phase und erhält deren Artefakte. Reicht das Budget nicht für den
festgelegten vollständigen Pilot, wird er als unvollständig ausgewiesen und
vor Gewichtsupdates geprüft. Selbstspiel-Winrate gegen die eigene identische
Policy ist kein Fortschrittsmaß. Deck-/Startglück bleibt im Lernsignal verrauscht; deswegen
viele Partien und gepaarte externe Vergleiche, keine Etikettierung jedes
Verliererzugs als falsch.

Terra ergänzt anschließend einen begrenzten eigenen Gegner- und Lehrerstrang.
Das erfordert den stochastischen neuralen Live-Collector mit geprüftem
Aktions-/Logprob-Replay. Ohne ihn sind neue Terra-Partien BC-Daten, kein
neuronales PPO. Bestehende Tarif-/Quota-Grenzen bleiben wirksam. Terra-Korrekturen
für den Lerner erhalten nur dessen View und autorisierte Historie.

Ergebnis-Utility für den ersten 1v1-Pilot: Sieg 1, Niederlage 0, echtes Remis 0,5.
Der Critic schätzt diese Utility als Baseline; er entscheidet nicht, was objektiv
ein guter Zug ist. Abbrüche, Caps, Fallbacks und Enginefehler sind keine Remis
und keine Erfolge. Ihr Anteil und die betroffenen Decks bleiben sichtbar; für
den ersten strikten Pilot werden solche Partien von Policy-Updates ausgeschlossen.
Nicht unbegrenzt neue Versuche nachfordern: höchstens 160 Sammelversuche für
128 vollständige Partien, danach untersuchen statt die Daten still zu ersetzen.
Technische Wiederholungen behalten Seed und Sitzbelegung; unvollständige
Tischpaare werden nicht durch nachträglich gewählte leichtere Tische ersetzt.

Die E3-Vorprobe zeigt zugleich die Grenze dieser Zulassung: Wenn Fehler der
Policy den Schutzmechanismus auslösen, zensiert eine Sammlung ausschließlich
sauberer Spiele genau diese Zustände. Der aktuelle vollständige Outcome-Vertrag
darf deshalb nicht unverändert zum Dauertraining werden. Ein späterer eigener
Vertrag kann eine ausdrücklich definierte Trainingsstrafe beim Abbruch oder
korrekt abgeschnittene Trajektorien mit geprüftem Bootstrap unterstützen.
Beides ist noch nicht implementiert; ein Helfer für GAE allein reicht dafür
nicht. Die echte MTG-Niederlage, ein technischer Abbruch und eine zusätzliche
Trainingsstrafe bleiben unterschiedliche Daten. Bereits aufgezeichnete
Haus-Fallback-Spiele werden nicht rückwirkend zu Raw-Policy-Daten umetikettiert.

Der nächste begrenzte Forschungsweg prüft einen separaten Vertrag
`failure_terminal_research_v1`: Bei einer replay-bestätigten wiederholten
automatischen Ankündigungsrücknahme des Lerners endet die Forschungs-Episode
vor der ersten Haus-Ersatzaktion. Das echte MTG-Ergebnis bleibt unbekannt;
eine explizite Trainingsutility von 0 mit `bootstrap=false` ist eine definierte
Strafe, keine nachträglich behauptete Niederlage. Ein Fehler des Gegners,
ein Zeitlimit oder ein Enginefehler erhält keinen erfundenen Lernersieg.
Bewusstes legales Abbrechen ist ebenfalls nicht automatisch ein Fehler.

Die vier bekannten Fälle 64/65/96/97 wurden mit allen kanonischen Inputs und
Sitzrollen offline geprüft: Fall 64 betrifft den Gegner, Fall 65 den Lerner;
96/97 bleiben Kontrollen ohne Recovery. Die Grenze vor der ersten Ersatzaktion
ist anhand der Record-Bytes nachweisbar; Record-Inputnummer und
Evaluator-Entscheidungsnummer sind dabei getrennte Zähler. Der Lernerpräfix
hat nur 127 stochastische Net-Samples und erfüllt allein nicht das geplante
Minimum von 128. Diese bekannten Entwicklungsfälle belegen nur die
Fehlergrenze, keine neue Trainingszulassung. Vor neuen Versuchen brauchen Collector,
Konverter, Targetschema, Cache, Zulassung und Resume einen gemeinsamen
ausdrücklichen Vertrag; die bisherige Zulassung muss diese Daten weiter
abweisen. Eine solche Strafe verteilt Verantwortung über die ganze Historie
und kann übermäßiges Passen fördern. Deshalb müssen zusätzlich Ankündigungen,
Zielmengen, Pass-/Abbruchhäufigkeit, Schleifen und vollständige Spielergebnisse
gemessen werden. Lokale Hilfsziele und Bootstrap bleiben getrennte Alternativen,
damit nicht mehrere Änderungen gleichzeitig den Versuch erklären könnten.
Die neue X-Beobachtung wird nicht gleichzeitig mit diesem Reward-Vertrag
als vermeintlich belegte Lösung eingeführt.

Der erste RL-Pilot bleibt bei FP32, K2 und dem vorhandenen geprüften Raw-Sampler.
Die Temperatur wird ausdrücklich auf **0,7** gesetzt (der unterstützte
stochastische Pfad verlangt mehr als 0,35). Ausgangswerte des vorhandenen
Head-Trainers sind Lernrate `2e-5` und Ziel-KL `0,02`; diese und die vollständigen
Rollback-Regeln werden im Manifest festgeschrieben. Eine kleine Sammel-/Replay-
Probe geht dem Pilot voraus; das vollständige Replay folgt nach Sammlung vor
den Updates. Für E4 werden diese Werte erneut am Full-Model-Pfad geprüft, nicht
als dort bereits bewährt bezeichnet. Keine Hochstufung von
Temperatur oder KL-Grenze allein, um einen fehlgeschlagenen Versuch fortzusetzen.
BC-Anker werden separat gekennzeichnet und verändern die Policy ebenfalls:
auch ihr Endergebnis muss die gemeinsame Abweichungs- und Spielprüfung bestehen.

### Relationslernen und längerfristige Architekturfragen

E5 beginnt mit kleinen, verifizierbaren Aufgaben aus erlaubtem Wissen, etwa
Kostenabdeckung durch eine vorgeschlagene Zahlungsfolge oder Kampfkonsequenzen
unter explizit festgelegten Bedingungen. Ein Ziel soll Konsequenzen von
Beziehungen lehren, nicht bereits eingegebene Stärke/Marken nur kopieren.
Mehrere nachweislich gleichwertige Zahlungsreihenfolgen sind eine gültige
Antwortmenge; diese Äquivalenz gilt nicht pauschal für strategische Züge.

Die kontrollierte X-Diagnose erfordert zunächst einen eigenen Beobachtungsvertrag:
autorisierter Ankündigungszustand statt einer aus alten Aktionen geratenen
Zahl. E7/E8 verwenden 80 globale Merkmale und 128 Merkmale je Entität; die
vorhandenen Plätze sind belegt. Der getrennte Encoder9 hängt zwei globale
Merkmale an: X vorhanden und X-Wert. Ungewählt und gewählte Null bleiben
unterscheidbar. Dazu gehören Sammlung, Replay, Gedächtnis-Hash,
Dataset-/Cache-Identität, Modellform und ONNX-Parität; vorhandene E7/E8-Daten
bleiben unverändert. Die zwei zusätzlichen Projektionsspalten beginnen bei einer
ausdrücklichen Gewichtsübernahme mit Null und benötigen danach einen trainierbaren
Eingangspfad. Der bisherige Head-PPO-Versuch friert diesen ein und kann solche
Nullspalten nicht lernen. Für einen eigentlichen Nutzenvergleich braucht es deshalb
einen getrennten Lernversuch mit aktueller vollständiger Gedächtnisrekonstruktion.
Ein E7→E9-Übergang umfasst außerdem die E8-Kontextsemantik; er ist keine reine
X-Ablation. Ein solcher Vergleich muss mit derselben E8-Basis und ansonsten
identischem Training kontrolliert werden. Weder diese
Erweiterung noch zusätzliche Zielkosten-Eingaben werden als bereits nachgewiesene
Lösung der beobachteten Fireball-Schleife bezeichnet.
Auch E8→E9 garantiert trotz Nullspalten keine identische rekurrente Ausführung:
Eine X-Änderung kann einen zusätzlichen passiven Beobachtungsschritt erzeugen.
Der Vergleich isoliert daher den neuen X-Beobachtungsvertrag einschließlich
seines Takts, nicht nur zwei Zahlen bei ansonsten identischer Historie.

Verdeckte Zustände dürfen nicht für kontrafaktische Engine-Klone verwendet
werden. Exakte Labels kommen aus vollständig bestimmten synthetischen Aufgaben
oder einer geprüften Rekonstruktion aus View und autorisierter Historie.
Unbekannte Reaktionen erfordern eine deklarierte Bedingung, eine Verteilung
oder ein maskiertes Ziel. Trainings-Readouts können später entfallen.

Slot-Mixing im Gedächtnis, längeres U, weitere Graph-Runden, Stabilitätsregularisierung
und ein Weltmodell bleiben getrennte Hypothesen. K8 allein implementiert nicht
die gewünschte System-2-Suche über 15–45 Sekunden. Dafür wären später lernbare
Aktionsfolgen, Beliefs und eine sichere Suche nötig; das ist kein Blocker für
das jetzt geplante Lernen aus Ergebnissen.

## 5. Bewertung und Entscheidung über ein neues Live-Modell

Die wiederholt verwendeten 64 Fälle bleiben Diagnostik. Auch die abgeschlossene
256-Fälle-Referenz ist ein Entwicklungsvergleich bekannter Deckfamilien.
Sie ist nur wiederverwendbar, wenn Modell-/Engine-/Sampler-/View-Verträge des
betreffenden Vergleichs unverändert bleiben; sonst muss die Referenz neu laufen.
Zwischenergebnisse bestimmen keine nachträgliche Auswahl günstiger Testfälle.

Für jeden aussichtsreichen Kandidaten vorab neue, nicht zum Training oder Tuning
verwendete Spielseeds reservieren, vollständig nach Sitz/Deck paaren und den
ganzen Satz auswerten. Karten-/Deck-Generalisation benötigt zusätzlich eine
nachweislich getrennte Modell- und Datenlinie: neue Seeds allein heilen frühere
Exposition nicht. Die bisherigen Hausdeckdaten dürfen weiter als bekannte
Trainingsfamilien genutzt werden, werden aber entsprechend bezeichnet.

Berichten: Siege/Utility, gepaarte besser/schlechter-Fälle, Unsicherheit auf
Tisch-Clusterebene, Caps/Fallbacks, technische Fehler und Kopf-/Quellenmetriken.
Pass-/aktive Ziele, Subsets, Kampfpaarungen und Zahlen getrennt ausweisen.
Manatappen ohne Nutzung, Reaktionsfenster/Counterspells, Mulligan und Remis
bekommen feste Szenariotests. Eine bessere Lehrer-Trefferquote ist kein Ersatz
für bessere Partien.

Ein kleiner durchschnittlicher KL-Schätzer garantiert weder unveränderte
Greedy-Aktionen noch kleine Auswirkungen auf Spielergebnisse. E3 bekommt daher
Aktionsvergleich und begrenztes gepaartes Gameplay als Regressionsdiagnostik;
sein Budget ist keine vorab behauptete Teststärke. Kein Anheben der KL-Grenze,
um einen leichter messbaren Effekt zu erzwingen. Caps bleiben ungelöst und
werden nicht nachträglich als echte Niederlagen oder Remis etikettiert.

v4 gegen v5 braucht einen gemeinsamen, auditierten Ausgangspunkt, dieselben
Daten und ein offengelegtes Update-/Rechenbudget. Eine Graph-Abschaltung an einem
bereits trainierten v5 misst Abhängigkeit von diesem Pfad, keine faire v4-Alternative.
Zunächst die nutzbare Lernpipeline reparieren; danach diesen Architekturvergleich
erneut aufsetzen. Jede zugesagte Schwierigkeit muss Stärke **und** gemessene
Live-Latenz bei ihrem K erfüllen. ONNX auf CPU ist der verbindliche Laufzeitpfad;
NPU-Kompatibilität ist ein separater späterer Test.

## 6. Ressourcen, Fehlerbehandlung und nächster Arbeitsschritt

Ein GPU-Besitzer zur Zeit. CPU-Datenvorbereitung darf parallel laufen, wird aber
nach RAM- und GPU-Versorgung gemessen begrenzt. Zwölf physische Kerne sind kein
Grund, unkontrolliert 24 Prozesse mit eigenem großen Kartensatz zu starten.
Gateway und Eingabe bleiben bedienbar. Erst reale Transfer-/Compute-Zeiten
messen, dann Prefetch oder Batchgrößen ändern; GPU-Auslastung allein ist kein
Lernfortschritt.

Vor einem neuen Trainingsarm repräsentative große Boards/Menüs einschließlich
Backward und Optimizer auf 12 GB prüfen. B8/U32 und bewährte Chunking-/Checkpoint-
Verfahren sind Ausgangspunkte, keine Kapazitätsgarantie für K8 oder Encoder8.
Verschiedene Präzision oder Batchformen werden im Vergleich als eigene
Ausführungsänderung erfasst. Die kleine Inferenzmessung aus Abschnitt 2.4 darf
nicht als Trainingsspeicherprognose dienen.

Jeder Lauf schreibt atomare Zustände, Manifest-/Modellhashes, PID samt Boot- und
Startidentität, begrenzte Lauf-/Versuchszahlen und eindeutige Abschlussartefakte.
Monitor-Ereignisse werden dedupliziert; laufende Controller vor Neustart geprüft.
Enginefehler sichern Seed, Replay, Version und Fehlerbild, erhalten einen
Regressionstest und werden vor erneuter Zulassung repariert. Zugehörige Engine-
und Seat-Prozesse werden auch bei Fehlern beendet und ihr Ende geprüft.
Fehlgeschlagene Artefakte bleiben erhalten; kein blinder automatischer Retry.

**Nächster Schritt:** Der Encoder9-Anschluss einschließlich nativem Loader und
vollständigem kompilierten Kartenkatalog ist geprüft. Ein kleiner synthetischer
Lerntest kann die neuen Eingangsspalten lernen; das ist noch kein erfolgreicher
Spielversuch. Als Nächstes den Lernvertrag für fehlgeschlagene Ankündigungen
klären und einen getrennten, begrenzten Versuch festlegen. Vor größerem Full-PPO die aktuelle
Gedächtnisrekonstruktion und ihren Ressourcenbedarf prüfen. Die vollständige
KL-Prüfung trägt H jetzt während ihres Durchlaufs unter festen aktuellen
Gewichten weiter; die differenzierbaren Trainingssegmente behalten ihren
gesonderten aktuellen Burn-in. Das vermeidet wiederholte Präfixberechnung,
ändert aber weder Datenabdeckung noch KL-Grenze. Eine R6-Vorprobe mit 256 Frames
und 60 Entscheidungen war damit 4,21-mal schneller (15,10 → 3,59 s), bei exakt
gleichen Aktionslogprobs. Das ist ausdrücklich nur ein unvollständiges
Inferenzpräfix, kein vollständiger Lernlauf oder Backward-Speichernachweis.

Der Export-/Vier-Fälle-Transfertest des einzelnen Updates ist abgeschlossen,
ohne verbesserte Ergebnisse oder behobene Schleife. Die alte Referenz bleibt
versiegelt. Daraus folgt kein unabhängiger Stärkevergleich und keine Freigabe.
Die übrige gemischte Sammlung bleibt bis zur Klärung des Lernsignals für
fehlgeschlagene Ankündigungen zurückgestellt; mehr vom selben sauberen
Datensatz allein beantwortet diese Frage nicht.
Vollständige CPU/CUDA-Replays und der einzelne Head-PPO-Schritt
über alle 1.575 Entscheidungen sind bestanden, mit unverändertem KL-Limit und
vollständiger Tensor-/Dosisprüfung. Die Rust-Guard-Reparatur `97ef8e5ad` bestand
den tatsächlichen direkten Venv-Aufruf samt Torch-/Datenloader-Vorprobe und
anschließender Veröffentlichung des isolierten Kandidaten. Der gesicherte
erste Startfehler lag vor dem Torch-Import und führte zu keinem Update.
E1-Spielvergleich und Zahlungsdiagnose sind abgeschlossen;
weitere BC-Updates allein wegen eines besseren aggregierten NLL sind nicht
begründet. Der abgeschlossene,
versiegelte Pilot enthält 32 vollständige Trainingspartien, 66.739 Frames und 64
Sitzhistorien; fünf getrennte, bereits bekannte Entwicklungspartien sind
Validierung, kein finaler Holdout. Beide Arme starteten aus demselben d640-
R6-Modell mit zwölf Schichten, frischem Adam, K2 und einem vollständigen
Datendurchlauf. Nur die Köpfe wurden trainiert; dies isoliert die Reparatur
und prüft kein Full-Model-Lernen. E0 hat die echte vollständige Datenkette bestanden;
der vollständige E3-Gegnerpool bleibt ein eigener Versuch. Ein FP32-Backward-
Test mit d640, acht Schichten, zehn Köpfen, B8/U32 und K2 benötigte 2,67 GiB
reservierten VRAM; er verwendete einen synthetischen Katalog und ist ausdrücklich
keine Kapazitätsgarantie für echte große Boards/Kataloge. Vor dem Lernpilot erfolgte
deshalb eine Messung mit seinen tatsächlichen Daten. Diese neue Messung bestand
für beide Kandidatenrevisionen: echte Backwards, rund 0,93 GB maximal belegter
VRAM, keine Optimizer-Updates und keine angelegten Adam-Momente. Das ist keine
Kapazitätsaussage für vollständiges Modelltraining. Zwei Startfehler wurden
gesichert und getrennt repariert: fehlende Git-Identität im eingefrorenen
Trainer sowie ein Procfs-Rennen beim Ende eines kurzlebigen Hilfsprozesses.
Es wurde kein abgeschlossener Trainings-Checkpoint aus diesen Fehlversuchen
übernommen. Keine unveränderte Fortsetzung
der alten BC-Runden und keine Live-Freigabe ohne Spielvergleich. Lokale Gates
gemäß AGENTS.md vor Push/Landung.

## 7. Forschungsgrundlage und Grenzen der Übertragung

Die ausführliche Quellenprüfung und Codezuordnung steht im verlinkten
[Review](../history/trained-ai-v5-round10-review-2026-10-29.md#expanded-research-training-the-relational-and-recurrent-components).
Für die Reihenfolge relevant sind
[Adaptive-Depth Is a Finishing Stage](https://arxiv.org/abs/2610.14252),
[Recurrent-Depth VLA](https://arxiv.org/abs/2602.07845) und
[STARS](https://arxiv.org/abs/2605.26733) für trainierte statt bloß erhöhte Denktiefe;
[CLRS](https://proceedings.mlr.press/v162/velickovic22a.html) für strukturierte
Zusatzaufgaben; [DAgger](https://proceedings.mlr.press/v15/ross11a.html) für
Lehrerziele auf vom Lerner tatsächlich besuchten Zuständen; sowie
[DeepSeek-R1](https://arxiv.org/abs/2501.12948),
[SPIRAL](https://arxiv.org/abs/2506.24119) und
[AlphaStar](https://deepmind.google/blog/alphastar-grandmaster-level-in-starcraft-ii-using-multi-agent-reinforcement-learning/)
für Ergebnisfeedback und unterschiedliche Gegner. Diese Arbeiten motivieren
die Versuche. Sie beweisen weder die Ursache unserer bisherigen Rückschritte
noch eine garantierte Übertragbarkeit auf Magic, Commander oder dieses
Hardwarebudget.
