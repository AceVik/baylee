# BayleeAI v5: vollständige R6-Referenz, 29. Oktober 2026

Der Referenzlauf ist abgeschlossen und geprüft. R6 gewinnt auf diesem Satz
bekannter Decks deutlich seltener gegen Sharp als die Haus-KI Steady.
Das ist eine Ausgangsmessung für das Refactoring, kein Vergleich eines neuen
Modells mit R6 und kein Nachweis für oder gegen die v5-Erweiterung gegenüber v4.

## Ergebnis und Vergleichsbedingungen

| Spieler gegen Sharp | Siege | Niederlagen | Remis | Siegquote |
| --- | ---: | ---: | ---: | ---: |
| R6 final, einschließlich ausgewiesener Fallbacks | 74 | 182 | 0 | 28,91 % |
| Steady | 125 | 131 | 0 | 48,83 % |

Beide spielen getrennt gegen Sharp auf denselben Ausgangslagen. Es sind keine
direkten R6-gegen-Steady-Partien. Von 256 gepaarten Fällen ist R6 in 11 besser,
in 62 schlechter und in 183 gleich. Die mittlere Utility-Differenz beträgt
−19,92 Prozentpunkte. Ein Bootstrap über die 128 vollständigen Tisch-Cluster
(10.000 Stichproben, RNG-Seed 33029) ergibt ein 95-%-Perzentilintervall von
−25,00 bis −14,84 Prozentpunkten. Dieses Intervall beschreibt diesen
Entwicklungssatz; es ist keine Aussage über alle Karten oder Formate.

Fixiert waren 29 zugelassene bekannte Decks, 128 Tisch-Cluster mit jeweils beiden
Fokussitzen, erster Spielseed `813301729012345000`, Policy-Seed `47001`, K2 und
Raw-Policy mit Temperatur 0. Modell: R6 final, v5 Relationsrevision 2, Encoder7;
Checkpoint-SHA-256:
`dac99a0c2bfa0612728e7909b56d75a6ec0b50cbb7538316cd8604c391f42827`.
Der eingefrorene Evaluator stammt aus
`518148add05b8fa5164d004472ddb6a8168ad1ad`. Der Lauf dauerte ungefähr zwei Stunden.

Alle 64 Chunks mit je vier Fällen sind vollständig. Die Prüfung verifiziert
727 eindeutige Datei-Hashes einschließlich Eingaben, Jobs, Ausgaben und
Replay-Artefakten. Fälle 0–255 sind genau einmal vorhanden; beide Sitze jedes
Tisch-Clusters besitzen denselben Seed und dieselben Decks. Alle Jobs endeten
erfolgreich. Kein Fall wurde wegen Niederlage oder ungünstiger Baseline entfernt.
Es gibt keine Caps, erfassten Stalls oder abgewiesenen Aktionen.

## Fallbacks und Zahlungsfolgen

Sechs Netz-Partien enthalten zusammen 55 Haus-KI-Fallbacks. Alle sechs betreffen
das fokale Deck `adapted/DDJ/izzet`; alle 55 ausgeführten Fallback-Aktionen sind
`PassPriority`. In 53 Fällen folgt das unmittelbar auf eine fokale
`CancelCast`-Aktion, in zwei Fällen auf `PassPriority`. Die fokalen Replay-Eingaben
wurden vollständig und positionsgenau mit den Sidecar-Einträgen abgeglichen.

| Fall | Tisch | Fokalsitz | Fallbacks | R6-Sieg | Steady-Sieg |
| --- | ---: | ---: | ---: | ---: | ---: |
| 19 | 9 | 1 | 4 | 1 | 0 |
| 77 | 38 | 1 | 5 | 1 | 1 |
| 135 | 67 | 1 | 4 | 0 | 0 |
| 164 | 82 | 0 | 24 | 0 | 0 |
| 193 | 96 | 1 | 13 | 0 | 0 |
| 222 | 111 | 0 | 5 | 0 | 0 |

Die wiederholten Zaubern-/Abbrechen-Folgen passen zum internen View-Schleifenschutz:
Im eingefrorenen Code liefert er bereits bei `visits >= 4` eine House-Aktion;
der äußere Evaluator zählt Stalls erst bei `visits > 4`. Deshalb bedeutet
`stalls = 0` nicht, dass alle Aktionen vom Netz stammen. Die genaue ausgelöste
Fallback-Verzweigung fehlt im Protokoll; ihre Zuordnung für jeden Einzelfall
bleibt eine Schlussfolgerung, kein bewiesener Trace. Ein Engine-Absturz oder
ein nicht unterstützter Banding-Fall ist hier nicht belegt.

Folgeauftrag für R3: reproduzierbare Izzet-Zahlungs-/Abbruchszenarien aufnehmen,
Kosten- und Aktionskontext überprüfen und Fallback-Gründe strukturiert erfassen.
Den Schutz nicht lediglich abschalten. Alle sechs Fälle bleiben im primären
Vergleich; die Siegquote ist ausdrücklich keine Messung ausschließlich
neuronaler Aktionen.

## Verwendung und nächste Schritte

Die Referenz bleibt Entwicklungsdiagnostik bekannter Deckfamilien. Neue Seeds
machen daraus keinen unbekannten Karten-/Deck-Holdout. Wiederverwendung setzt
identische Engine-, View-, Modell- und Sampler-Verträge voraus; insbesondere
ein Engine-/Encoderwechsel erfordert gegebenenfalls eine neue Referenz.

Die Replays sind **nicht zum Training zugelassen**. Das historische native
Manifestfeld `training_data=true` ändert den festgelegten Evaluationszweck nicht.
Die greedy Temperatur-0-Aktionen sind auch keine stochastischen PPO-Daten.
Es erfolgt keine Modellfreigabe oder Änderung des Nutzer-Gateways.

Der [Refactoring- und Trainingsplan](../proposals/trained-ai-v5-refactor-training.md)
bleibt maßgeblich. Die alte Trainingsschleife bleibt auf Nutzerwunsch gestoppt.
Nach der R0-Veröffentlichungskorrektur ist jetzt die vorbereitende R1-Modultrennung
committed (`e4e3318fa`): `PlayerNet4` und die Checkpoint-Upgrade-Funktion liegen
in `model4/player.py`, die bisherigen öffentlichen Imports bleiben erhalten.
Die Funktionskörper sind unverändert. 2.730 Tensorvergleiche, darunter 1.918
Gradientenvergleiche, sind für v4/v5r1/v5r2 und K0/1/2/4/8 bitgleich;
zusätzlich bestehen neun vorhandene Modelltests. Das prüft die Verschiebung,
noch nicht die geplante Kontextreparatur oder deren Spielstärke.

## Lokale Belege

Run root:
`/home/ace/baylee-data/v5/ops-codex-20261028/full-coverage-reference-20261029/run-01a185f4-9d19-7270-a9b3-7c8d6dbbb312/`
mit `summary.json`, `derived-r6-final.json`, Jobs und Chunk-Replays.

Audit root:
`/home/ace/baylee-data/v5/ops-codex-20261028/refactor-investigations-20261029/`
mit `audit-reference.py`, `reference-audit.json`, `reference-fallback-audit.json`
und `r1-move-01a18665-1952-75d2-a1dd-93f000faed08/{results,verification}.json`.
Monitor-Ereignis: `cpu_review:9bb1fec221cfe055:terminal`.
