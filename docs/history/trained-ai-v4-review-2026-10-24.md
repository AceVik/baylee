# Playernet v4: Ergebnisprüfung und nächste Trainingsstufe, 24. Oktober 2026

## Ergebnis des ersten Laufs auf dem erweiterten main

Basis: `88bc2b07f`, unveränderte Runtime unter
`/home/ace/baylee-data/v4/experiments/main-refresh-20261024/runtime-final`.
Die Modellbasis hat 54,83 Millionen Parameter (`d=640`, acht Schichten,
zehn Heads, H mit neun Slots, getrennter Arbeitszustand Z).
Der neue Arbeitskartenpool umfasst 10.058 Karten. Das versionierte
Deckmanifest enthält 1.958 Listen; Listenabdeckung ist keine Spielabdeckung.

Trainiert wurden 534.934 Frames aus 42 vollständigen Spielen, darunter
74.418 Policyentscheidungen: 72.534 SingleChoice, 1.458 Subset, 339 Blocker,
87 Zahlen. Der Lauf machte 2.103 Updates
mit B=8/U=32, Lernrate 1e-5 und K-Curriculum 1/2/4/8. Training einschließlich
Validierung dauerte 2.002 Sekunden, allokierter VRAM-Peak 8,33 GB, kein OOM.
Die feste Validierung umfasst 475.248 Frames aus 32 Partien.

| Kennzahl | Unveränderter Start | Nach erstem Paket |
| --- | ---: | ---: |
| Policy-Loss (damalige Lossversion 1) | 0,29722 | 0,28875 |
| Value-MSE | 0,15302 | 0,11765 |
| Gesamt-Top-1 | 90,207 % | 90,338 % |
| Nicht-Pass-Top-1 | 59,282 % | 59,028 % |
| Kalibrierfehler ECE | 0,16475 | 0,09118 |
| Siege gegen Sharp, Duell | 18/64 | 15/64 |
| Siege gegen Sharp, Vierer-Commander | 3/32 | 2/32 |

Der uniforme sitz-/teambezogene Value-Baseline-MSE beträgt 0,12195. Ein besserer
Value ist kein Beleg für bessere Züge. Nicht-Pass-Top-1 schließt freiwillige
Pass-Entscheidungen aus. Der aktuelle Konverter emittiert keine erzwungenen
Policyentscheidungen: Alle 72.534 SingleChoice-Zeilen hatten Alternativen.
Die 90-%-Gesamtzahl misst Lehrerübereinstimmung mit vielen Pass-Labels,
keine Zugpräzision wie bei einem erfahrenen Menschen.

Der gepaarte Duellvergleich ergibt 3 Verbesserungen, 6 Verschlechterungen und
55 Gleichstände auf 32 unabhängigen Tischen. Utility-Differenz −4,6875
Prozentpunkte, Table-Bootstrap-95-%-Intervall ungefähr [−15,625; +4,6875].
Commander: 2 Verbesserungen, 2 Verschlechterungen, 25 Gleichstände und drei
ungeklärte Cap-Paare; nur acht unabhängige Tische. Der Steady-Vergleich gewann
30/64 Duelle und 6/32 Commander-Spiele. Die Resultate rechtfertigen **keine
Promotion** und keine Behauptung einer Verbesserung gegenüber dem Startmodell.
Ein neuer kontrollierter Vergleich mit v3 ist in diesem Lauf nicht enthalten.

Quellen liegen neben der Runtime: `evaluation/{reference,bootstrap}-*.json`,
`evaluation/paired-bootstrap-*.json`, `jobs/train-bootstrap/stdout.log` und
`models/bootstrap/meta.json`. Die Auswertung ist eine wiederholt verwendete
Entwicklungssuite, kein unberührter finaler Test.

## Zusätzlicher Kontrolllauf während des Reviews

`concept-review-20261024` trainierte vom gleichen Startmodell aus auf 52
weiteren geprüften Spielen (554.500 Frames), mit unverändertem alten Code.
2.178 Updates einschließlich Validierung dauerten 2.049,7 Sekunden;
allokierter Peak 8,12 GB, kein OOM. Policy-Loss 0,29349, Value-MSE 0,11444,
Nicht-Pass-Top-1 59,446 %, ECE 0,07077.

| K=1 gegen Sharp, gleiche Bedingungen | Start | Erster Lauf | Kontrolllauf |
| --- | ---: | ---: | ---: |
| Duell | 18/64 | 15/64 | 23/64 |
| Commander | 3/32 | 2/32 | 1/32 |

Duell: 8 verbesserte, 3 schlechtere, 53 gleiche Fälle;
Table-Bootstrap-95-%-Intervall der Differenz [−3,125; +17,1875] Prozentpunkte.
Commander: 1 besser, 4 schlechter, 24 gleich, 3 ungeklärte Cap-Paare;
Intervall [−34,375; +25] Prozentpunkte. Gegenüber dem Startmodell steigen
Commander-Fallbacks von 24 auf 57 und Stalls von 11 auf 24. Beide Modelle
haben dort null zurückgewiesene Antworten; die neuralen Entscheidungen können
trotzdem in Schleifen führen. **Keine Promotion**: Ein positiver Duellpunktwert
kompensiert die Commander-Schwäche nicht. Die Haus-Baseline bleibt 30/64 bzw.
6/32. Das unterstreicht den Bedarf an getrennten Formatmetriken und künftigem
Training auf vom Schüler selbst erzeugten Fehlersituationen.

Die vollständigen Resultate stehen unter
`/home/ace/baylee-data/v4/experiments/concept-review-20261024/evaluation/`.

## Gefundene Fehler und methodische Korrekturen

1. **Kopierausnahmen verloren ihre strukturelle Fähigkeitsreferenz.**
   Machine God's Effigy behält seine Manafähigkeit als kopierbare Fähigkeit.
   Nach dem Kopieren von Goblin Striker oder Thrun, Breaker of Silence ist der
   Laufzeitindex dieser Fähigkeit kein gedruckter Index der kopierten Karte.
   Der Konverter wies deshalb zwei abgeschlossene Spiele korrekt ab. Die
   öffentliche Projektion enthält nun die Herkunft zusammengesetzter Fähigkeiten,
   einschließlich der konkreten verschachtelten `CopyMod::GrantAbility`-Klausel.
   Der Kartenkatalog stellt diese Klausel als eigene Wurzel bereit; Kandidaten
   und Stack-Fähigkeiten verwenden die exakte Referenz. Verdeckte Objekte geben
   keine Herkunft preis. Alte Wire-Nachrichten lesen das optionale Feld leer;
   der veränderte Katalogdigest erfordert neue Features und einen neuen Export.
   Beide Originalpartien wurden erneut vollständig samt Zustands-Hashes
   verarbeitet: 12.535 Frames, null Abweisungen, null unkodierte Entscheidungen.

2. **Detonate konnte eine Lehrerspiel-Endlosschleife erzeugen.**
   Die Haus-KI maximierte X nach Mana, obwohl der Ziel-Mana-Wert exakt X sein
   muss. Das Spiel rollte die unmögliche Zielwahl zurück und versuchte sie
   wieder. Planung und Zahlenwahl berücksichtigen jetzt sichtbare passende
   Mana-Werte sowie grundlegende Zielausschlüsse. Gleiche Decks, Profile,
   Run-ID und Seed 1400020: vorher Caps bei 6.000 und 12.000 Antworten, danach
   reguläres Ende nach 580 Antworten in Zug 25. Dies ist eine Haus-KI-Korrektur,
   keine Änderung der Spielregeln und kein allgemeiner Ersatz für den
   Legalitätsprüfer. Die alten Cap-Daten bleiben unverändert und ungelabelt.

3. **Defensive Loss-Absicherung; kein Fehler im geprüften Paket.**
   Die ursprüngliche Reviewhypothese einer Verdünnung durch erzwungene
   SingleChoice-Zeilen trifft auf den aktuellen Datenproduzenten **nicht zu**:
   `convert4/replay.rs` schließt sie bereits vor dem Emittieren aus. Das
   vollständige Daten-Audit bestätigt null solche Trainingszeilen. Lossversion 2
   schützt den Trainer zusätzlich bei anderen Datenproduzenten, indem nur
   Zeilen mit mindestens zwei Kandidaten in die SingleChoice-Normierung kommen.
   Daraus ist hier kein Lerngewinn zu erwarten. Alle Beobachtungen bleiben im
   Gedächtnisstream und im Value-/Hilfstraining. Neue Kennzahlen
   `val_choice_nll`, `val_choice_top1_acc`, `val_choices` zählen tatsächliche
   SingleChoice-Wahlfragen, einschließlich freiwilligem Passen. Sie sind
   ungewichtet und bewerten nicht die Subset-/Blocker-/Zahlenköpfe.
   Versionsübergreifende Vergleiche benutzen denselben Validator. Das Audit
   liegt in `concept-fixed-20261024/initial-decision-audit.json`.

4. **Checkpoint-Auswahl vermischte Policy und Value.**
   Der Standard verwendet nun Policy-Loss statt `Policy + 2×Value`.
   `--validate-initial` bezieht das unangetastete Startmodell ein;
   `last.pt` erhält den tatsächlichen Trainingszustand unabhängig vom besten
   Snapshot. Verlustversion und Auswahlverfahren stehen im Checkpoint;
   inkompatibles exaktes Resume wird abgewiesen. Promotion bleibt eine
   getrennte Entscheidung anhand gepaarter Spiele und Taktiktests.
   Im ersten Paket wurden Policy und Value beide besser; hier hat das alte
   Kriterium also keine schlechtere Policy durch einen besseren Value bevorzugt.
   Die Änderung verhindert diesen Konflikt in künftigen Auswahlen, erklärt
   aber ebenfalls nicht das schwache bisherige Spielergebnis.

5. **Ein fehlerhaftes Folgepaket darf unabhängiges Training nicht vernichten.**
   Der alte Controller endete nach dem ersten Vergleich wegen der beiden
   Konverterabweisungen. Die bereits verifizierten Spiele und Checkpoints
   blieben erhalten. Der begrenzte Kontrolllauf `concept-review-20261024`
   verwendet 52 gesondert geprüfte, bislang nicht trainierte Spiele mit
   554.500 Frames, eingefrorenem altem Code und unverändertem Startmodell.
   Er lief unabhängig von der Korrektur; die obigen Vergleiche begründen keine
   Übernahme seiner Gewichte.

Reproduktionen und Logs:
`/home/ace/baylee-data/v4/experiments/concept-fixed-20261024/{copy-replay,detonate-repro,jobs}`.

## Konzeptentscheidung

Die H/Z-Trennung und das strukturierte Kartenfundament bleiben bestehen.
Ein kleines erstes Paket rechtfertigt keine Parameterverdoppelung. Jetzt sind
breitere, korrekt abgebildete Erfahrungen und echte Entscheidungen die
höher priorisierten Engpässe. BC-Loss und Modellgröße sind keine Ersatzmaße
für Spielstärke; längeres Rechnen benötigt nachweisbar nützliche Revisionsziele.

K=1/2/4/8 bleiben erhalten. Vor produktiver 15–45-Sekunden-Suche sind K=1/K=4
auf gleichen Fällen zu vergleichen: Welche Fehler werden korrigiert, welche
neu erzeugt, wie viel Latenz entsteht? Der Snapshot-Observer belegt weder
vollständige interne Ereignisabdeckung noch Reconnect-Korrektheit. Das bleibt
ein eigener Vertrag mit Tests an der `PlayerView`-Grenze.

Eine konkrete noch offene Knowledge-Grenze: `features4/knowledge.rs` behandelt
gegnerische `looking_at`-Karten derzeit mit `origin_zone=1`, obwohl das Wire-Feld
auch Bibliothekssuchen enthält. `features4/state.rs` zählt diese Herkunft als
bekannte Handkarten; historische Erinnerung und gesicherter aktueller Ort sind
damit nicht sauber getrennt. `push_remembered_card` überträgt außerdem das
gespeicherte Sichtungsalter nicht. Das greift auf keine verborgene Enginewelt zu,
ist aber eine unzuverlässige Wissensrepräsentation. Vor der geplanten exakten
Suchrekonstruktion braucht sie eindeutige Entitlement-/Zoneninformation,
Unsicherheit und Invalidationstests für Shuffle und verdeckte Zonenwechsel.
Der eingefrorene BC-Lauf ist kein Nachweis, dass dieser Vertrag bereits erfüllt
ist; eine Änderung dieser Features erfordert einen eigenen Replay-/Exportstand.

Weiterer Lernhorizont: U=32 meint Beobachtungen. An einem großen Tisch können
dazwischen nur wenige eigene Entscheidungen liegen. H läuft zwar weiter,
aber K-Denkschleifen verlängern die zeitliche Gradientenstrecke nicht. Nach
diesem Versuch deshalb U=64/128/Burn-in bei angepasstem B mit verzögerten
Gedächtnisaufgaben vergleichen, ohne Zwischenbeobachtungen zu verwerfen.
Das tatsächliche Z hat derzeit neun Slots und einen wiederverwendeten
Denkblock; die alten Angaben zu 16 Slots und zwei Layern waren Entwurfswerte.

Nach der breiteren BC-Stufe folgen Lernzustände, die das Netz selbst besucht:
view-beschränkte Lehrerlabels auf diesen Zuständen, komplette Gedächtnisverläufe,
getrennte Lehrer-/Verhaltensherkunft und unverfälschte Outcome-Labels.
Diese DAgger-artige Richtung adressiert die Verteilungsverschiebung durch
eigene Fehler; sie ersetzt keine Evaluation. Siehe
[Ross et al.](https://arxiv.org/abs/1011.0686).
Neuere Arbeiten zu rekursivem Rechnen wie
[TRM](https://arxiv.org/abs/2510.04871),
[Looped Actor](https://arxiv.org/abs/2609.37432) und
[ELR](https://arxiv.org/abs/2610.12582) motivieren Experimente, belegen aber
keine universelle Magic-Spielstärke oder Phoenix-NPU-Tauglichkeit.

## Nächster Lauf

Experiment: `/home/ace/baylee-data/v4/experiments/concept-fixed-20261024`.
Runtime eingefroren auf `8bb208a49`, Controller-PID beim Start 427933.
Die GPU hat nach Abschluss des Kontrollvergleichs diesen Lauf übernommen.
Das erste fertige Paket enthält 59 Partien/506.954 Frames, das zweite
61 Partien/520.184 Frames. Alle 32 festen Validierungspartien sind neu
hashgeprüft: 475.248 Frames, null Abweisungen oder unkodierte Entscheidungen.
Ihr CPU-Cache wurde parallel zum vorherigen GPU-Spielvergleich vorbereitet.
Der komplette Bestand ist nun ebenfalls fertig: **240 Trainingspartien,
3.753.658 Frames, 531.888 Haus-Policyentscheidungen**, null abgewiesene Spiele,
null unkodierte Entscheidungen. Weitere 146.459 erzwungene Entscheidungen
werden als solche gezählt, nicht als Policylabels emittiert. Das Replay
dauerte 1.247,5 Sekunden parallel zum GPU-Kontrolllauf. Die auf zwei Pakete
begrenzte CPU-Vorbereitung läuft unabhängig vom GPU-Verbrauch weiter.
Nach der vollständigen Ausgangsvalidierung sind die ersten 25 GPU-Lernupdates
nachgewiesen, ohne OOM oder Allocator-Retry. Die neue Wahlmetrik erfasst
68.498 SingleChoice-Fragen der Validierung (90,207 % Lehrerübereinstimmung);
sie ist keine Spielstärkemessung. Der Policy-Loss des Startmodells bleibt
mit 0,2972155 praktisch identisch zur alten Messung, passend zum Audit:
die defensive Behandlung erzwungener Policyzeilen ändert diesen Datenbestand
nicht. Ab jetzt bleiben die Laufartefakte die Quelle für neue Ergebnisse.

1. Getesteten Quellstand, Binaries, Pythonmodule und Katalog einfrieren.
   Startgewichte vom bisherigen Referenzmodell übernehmen, nicht vom im
   kleinen Vergleich schlechteren Kandidaten. Strukturelle Operatoren nach
   Namen übertragen, unbekannte Operatoren abweisen; Adam/H beginnen frisch.
2. Die 239 abgeschlossenen Folgepartien mit korrigierten Features neu
   verarbeiten und den reparierten Detonate-Deal ergänzen. Die gleichen
   32 Validierungspartien ebenfalls neu verarbeiten. Nur vollständig
   hashgeprüfte Spiele mit null unkodierten Entscheidungen zulassen.
3. Ganze Partien in gemischten Paketen von ungefähr 500.000 Frames trainieren.
   CPU-Replay/Cacheaufbau und GPU-Training laufen parallel, höchstens zwei
   Pakete voraus. Pro Paket eine Epoche, B=8/U=32, LR=1e-5, K-Curriculum,
   BF16/Activation-Checkpointing, feste Validierung mit B=8.
   Jeder Paketstart wird mitvalidiert; die Folge übernimmt den tatsächlichen
   Optimiererstand aus `last.pt`, Gedächtnis startet für neue Partien frisch.
4. Nach dem ersten Paket sowie am Ende Kandidat gegen Startmodell vergleichen:
   gleiche neue Engine/Haus-KI, Decks, Seeds und Sitze. Auch die Referenz wird
   nach der Haus-KI-Korrektur neu gespielt. K=1 zuerst; separate K=4-Gegenprobe.
   Caps bleiben unbekannt, Unsicherheit wird über unabhängige Tische geschätzt.
5. Kein automatisches Ersetzen des produktiven Modells. Verluste, echte
   Wahlgenauigkeit, Ausführungsfehler, Kalibrierung und gepaarte Ergebnisse
   zusammen beurteilen. Nach wiederholter Auswahl ist eine neue gesperrte
   Testsuite erforderlich.

## Fortsetzung nach dem Ausschalten des Rechners

Am 24. Oktober wurde der Rechner laut Besitzer versehentlich über den
physischen Einschaltknopf heruntergefahren. Die Wiederaufnahme verwendet
dieselbe eingefrorene Runtime und dieselben Trainingsparameter.
Alle 90 Runtime-Dateien wurden gegen `sources.json` geprüft; die sieben
fertig vorbereiteten Pakete enthalten zusammen weiterhin 240 Spiele.
Die ersten fünf Pakete sind vollständig trainiert und bleiben erhalten.

Paket `package-0005` hatte 2.369 Updates erreicht und wurde während seiner
Abschlussvalidierung unterbrochen. Sein vollständig lesbarer Checkpoint
enthält Schritt/Batch 2.300, endliche Modellgewichte und das Gedächtnis H
mit Form `[8, 9, 640]`. Nur die 69 noch nicht gespeicherten Updates und die
Validierung werden wiederholt. `--resume` übernimmt Modell, Adam, RNG,
Samplerposition und H; es ist kein neuer Trainingsstart mit frischem Adam.
Die Fingerprints von Trainings- und Validierungsdaten werden beim Laden
erneut abgeglichen.

`resume_after_shutdown.py` im Experimentverzeichnis setzt den Ablauf fort
(Controller-PID beim Neustart: 5404). Der ursprüngliche Job und seine Logs
bleiben unverändert. `recovery-after-shutdown/` bewahrt beide ursprünglichen
Checkpoints sowie ein Manifest mit Prüfsummen und dem Neustartgrund.
Der neue Versuch protokolliert nach
`jobs/resume-train-package-0005/`; der Trainer bestätigt
`resumed epoch=1 batch=2300 step=2300` auf CUDA. Die ersten 25 weiteren
Updates sind nachgewiesen (Schritt 2.325, endlicher Loss, allokierter
VRAM-Peak 8,69 GB, null Allocator-Retries).
Anschließend folgen automatisch Paket `package-0006`, die unveränderte
Policy-Loss-Auswahl sowie die geplanten gepaarten K=1-/K=4- und
Fresh-Seed-Vergleiche. Es gibt weiterhin keine automatische Promotion.

Ein zweiter Neustart unterbrach später `eval-final-duels`. Zu diesem Zeitpunkt
waren **alle sieben Trainingspakete vollständig abgeschlossen**; weitere
Lernupdates gingen nicht verloren. Die bereits gespeicherte Auswahl nach
Policy-Loss bleibt Paket `package-0000` (0,28689335). Weitere Pakete lieferten
nach diesem Kriterium keinen besseren Snapshot; Spielstärke ist damit noch
nicht abschließend bewertet.
`resume_comparisons_after_shutdown.py` setzt die geplante Auswertung fort
(Controller-PID beim Start: 3695). Der unterbrochene Duellvergleich bekommt
einen eigenen Versuch unter
`jobs/eval-final-duels-resume-after-second-shutdown/`. Vollständige ältere
Vergleiche und alle Checkpoints bleiben erhalten. Runtime- und
Evaluationseingaben wurden erneut anhand ihrer Prüfsummen kontrolliert;
der neue GPU-Auswertungsprozess ist aktiv. Auf ausdrücklichen Wunsch des
Besitzers ignoriert logind nun dauerhaft die An/Aus-Taste, siehe
[lokale Einstellung](../workstation-monitor.md#local-power-button-setting-24-october-2026).

## Abgeschlossener Lauf und aktueller Vergleich mit v3

Alle sieben Pakete wurden abgeschlossen: 240 Partien, 3.753.658 Frames,
14.888 gespeicherte Lernupdates über die Pakete. Zusätzlich wurden nach dem
ersten Ausschalten 69 verlorene Updates wiederholt. Die Auswertung der
unveränderten Validierung wählte weiterhin den Snapshot nach Paket 0 mit
Policy-Loss 0,28689335; der Start lag bei 0,29721552. Die sechs Folgepakete
lieferten keinen besseren ausgewählten Policy-Snapshot. Das ausgewertete
Modell enthält deshalb nicht sämtliche späteren Lernupdates.

Die größere Vergleichssuite verwendet Seedbasis 1800001: 128 Duellfälle auf
64 unabhängigen Tischen und 64 Commander-Fälle auf 16 Tischen. Jeder Spieler
bekommt dieselben Decks, Startbedingungen und Sitzrotationen gegen Sharp.
Nach Abschluss wurde das archivierte v3 `rl-23` mit exakt derselben
eingefrorenen Engine und demselben Deckmanifest nachgemessen. Der gepaarte
Prüfer bestätigt identische Fälle und reproduzierte Haus-Baseline-Ergebnisse.
Es sind Vergleiche gegen gleiche Gegner, keine direkten v3-v4-Partien.

| Policy am untersuchten Sitz | Siege Duell / 128 | Siege Commander / 64 |
| --- | ---: | ---: |
| Haus-KI Steady | 60 (46,9 %) | 15 (23,4 %) |
| Archiviertes v3 rl-23, Expert-Profil | 42 (32,8 %) | 6 (9,4 %) |
| v4 vor diesem Trainingslauf | 36 (28,1 %) | 3 (4,7 %) |
| Ausgewähltes neues v4, K=1 | 30 (23,4 %) | 3 (4,7 %) |

Das neue v4 hat im Duell gegenüber seinem Start 3 bessere und 9 schlechtere
Ergebnisse; gegenüber v3 sind es 12 bessere und 24 schlechtere. Das
Table-Bootstrap-95-%-Intervall der Utility-Differenz zu v3 ist [−18,75; 0]
Prozentpunkte. Commander gegenüber v3: 4 bessere, 6 schlechtere, 52 gleiche
und 2 ungeklärte Cap-Paare; Intervall [−16,41; +7,42] Prozentpunkte. Die zwei
Caps von v4 sind keine Remis. Commander hat nur 16 unabhängige Tische;
die Siegpunktwerte sind kein Beweis einer stabilen allgemeinen Rangfolge.
Die Seeds sind neu, die Deckfamilien gehören zur bestehenden Validierung.

Ausführungsqualität bleibt getrennt sichtbar: v4 benötigte 21/113
Haus-Fallbacks (Duell/Commander), v3 103/221. Zurückgewiesene Aktionen:
v4 1/0, v3 5/0. Erfasste Stalls: v4 2/37, v3 2/56. Beim v3-Duell protokolliert
der Evaluator ungültige Banding-Auswahlen; diese bleiben als Befund erhalten.
Der v3-Vergleich bewertet damit das archivierte Modell mit seinen bestehenden
Haus-Fallbacks, nicht eine rein neuronale Policy ohne Hilfe.

K=4 brachte auf der separaten Suite mit Seedbasis 1300001 keine besseren
Endergebnisse als K=1: beide 15/64 Duellsiege und 0/32 Commander-Siege.
Alle 64 Duellpaare und 29 auswertbaren Commander-Paare haben dieselbe Utility;
drei Commander-Paare bleiben wegen Caps ungeklärt. Das schließt andere Züge
nicht aus, belegt aber keinen Nutzen der zusätzlichen Denkschritte.

**Keine Promotion.** Besserer Imitations-Loss hat hier keinen Spielstärkezuwachs
gebracht. Die nächste Lernstufe sollte konkrete Fehlentscheidungen und vom
Netz selbst erzeugte Zustände untersuchen, geeignete view-beschränkte
Lehrer-/Suchziele ergänzen und Fortschritt zunächst an Taktikfällen prüfen.
Größere Modelle oder weitere unveränderte BC-Pakete sind durch diesen Lauf
nicht als Lösung belegt.

Artefakte: `concept-fixed-20261024/evaluation/fresh-{candidate,reference,v3}-*.json`,
`paired-fresh-v4-vs-v3-*.json`, `paired-fresh-*.json` und
`paired-k4-vs-k1-*.json`. `compare_v3.py` sowie die zugehörigen Jobmanifeste
halten Modell-, Engine- und Deckprüfsummen fest; beide v3-Auswertungen und
ihre gepaarten Prüfungen sind vollständig abgeschlossen.

## Verifikation

Betroffene Rust-Librarytests (`baylee-ai`, `baylee-gamehost`, `baylee-view`,
`baylee-train`) einschließlich Kopierprovenienz, verdeckter Karten,
Copy-of-copy, exakter DSL-Klausel und X-Zielwahl bestanden. Die Wire-Shape-
Prüfung berücksichtigt das kompatible optionale Herkunftsfeld.
698 Rust-Tests bestanden, zwei bestehende Tests waren ignoriert. Clippy für
alle Targets der vier betroffenen Crates einschließlich ONNX-CUDA ist sauber.
Der CPU-Python-Testlauf umfasste 52 Tests: 49 bestanden, drei hardwareabhängige
Tests wurden ausgelassen. Referenz-/Geräte-Lossgleichheit, erzwungene Zeilen, Resume und
Paketintegrität sind abgedeckt. Der Live-Kontrolllauf verwendet die GPU.
Die vollständigen drei Repository-Landegates sind für diesen neuen Stand
noch nicht ausgeführt; ohne sie erfolgt kein Push.
