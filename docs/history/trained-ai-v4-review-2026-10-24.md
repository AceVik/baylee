# Playernet v4: Ergebnisprüfung und nächste Trainingsstufe, 24. Oktober 2026

## Ergebnis des ersten Laufs auf dem erweiterten main

Basis: `88bc2b07f`, unveränderte Runtime unter
`/home/ace/baylee-data/v4/experiments/main-refresh-20261024/runtime-final`.
Die Modellbasis hat 54,83 Millionen Parameter (`d=640`, acht Schichten,
zehn Heads, H mit neun Slots, getrennter Arbeitszustand Z).
Der neue Arbeitskartenpool umfasst 10.058 Karten. Das versionierte
Deckmanifest enthält 1.958 Listen; Listenabdeckung ist keine Spielabdeckung.

Trainiert wurden 534.934 Frames aus 42 vollständigen Spielen, 2.103 Updates
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
Value ist kein Beleg für bessere Züge. Nicht-Pass-Top-1 enthält auch Antworten
ohne Alternative; die 90-%-Gesamtzahl misst ebenfalls keine Zugpräzision wie
bei einem erfahrenen Menschen.

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

## Aufgedeckte Ursachen und Korrekturen

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

3. **Erzwungene SingleChoice-Antworten verdünnten den Policy-Loss.**
   Ihr NLL ist null, ihr Gewicht stand aber im Nenner und schwächte echte
   Entscheidungen ab. Lossversion 2 nimmt nur Zeilen mit mindestens zwei
   Kandidaten in diese Policy-Normierung. Alle Beobachtungen bleiben im
   Gedächtnisstream und im Value-/Hilfstraining. Neue Kennzahlen
   `val_choice_nll`, `val_choice_top1_acc`, `val_choices` zählen tatsächliche
   Wahlfragen, einschließlich freiwilligem Passen. Sie sind ungewichtet.
   Alte und neue Losszahlen sind nicht unmittelbar vergleichbar.

4. **Checkpoint-Auswahl vermischte Policy und Value.**
   Der Standard verwendet nun Policy-Loss statt `Policy + 2×Value`.
   `--validate-initial` bezieht das unangetastete Startmodell ein;
   `last.pt` erhält den tatsächlichen Trainingszustand unabhängig vom besten
   Snapshot. Verlustversion und Auswahlverfahren stehen im Checkpoint;
   inkompatibles exaktes Resume wird abgewiesen. Promotion bleibt eine
   getrennte Entscheidung anhand gepaarter Spiele und Taktiktests.

5. **Ein fehlerhaftes Folgepaket darf unabhängiges Training nicht vernichten.**
   Der alte Controller endete nach dem ersten Vergleich wegen der beiden
   Konverterabweisungen. Die bereits verifizierten Spiele und Checkpoints
   blieben erhalten. Der begrenzte Kontrolllauf `concept-review-20261024`
   verwendet 52 gesondert geprüfte, bislang nicht trainierte Spiele mit
   554.500 Frames, eingefrorenem altem Code und unverändertem Startmodell.
   Er läuft unabhängig von der Korrektur und wird nicht automatisch übernommen.

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

## Verifikation

Betroffene Rust-Librarytests (`baylee-ai`, `baylee-gamehost`, `baylee-view`,
`baylee-train`) einschließlich Kopierprovenienz, verdeckter Karten,
Copy-of-copy, exakter DSL-Klausel und X-Zielwahl bestanden. Die Wire-Shape-
Prüfung berücksichtigt das kompatible optionale Herkunftsfeld.
52 Python-Tests bestanden mit drei hardwareabhängigen Auslassungen beim
CPU-Testlauf; Referenz-/Geräte-Lossgleichheit, erzwungene Zeilen, Resume und
Paketintegrität sind abgedeckt. Der Live-Kontrolllauf verwendet die GPU.
Die vollständigen drei Repository-Landegates sind für diesen neuen Stand
noch nicht ausgeführt; ohne sie erfolgt kein Push.
