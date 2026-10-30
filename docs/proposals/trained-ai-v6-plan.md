# BayleeAI: Arbeitsplan ab 30. Oktober 2026 (Phase v6)

Verantwortlich: Claude-Session `baylee-f9` (Opus 5.5), Übernahme von Codex am
30. Oktober. Landungen auf `main` laufen über die PM-Session. „v6“ bezeichnet
die Arbeitsphase und den Datenbereich `~/baylee-data/v6/`; das Netz ist bis auf
Weiteres BayleeAI v5 (Relationsrevision 2, Encoder 7). Eine neue
Modellversion entsteht erst mit einer Architekturänderung (Abschnitt 4).

## 1. Ausgangslage in fünf Zahlen

| Messung | Wert |
| --- | --- |
| R6 gegen Sharp, `dev1024` (1.024 gepaarte Fälle) | 31,7 % Siege |
| Haus-KI Steady gegen Sharp, dieselben Fälle | 49,8 % |
| Abstand | −18,1 pp, 0 Caps, 62 Haus-Fallbacks |
| Zugelassene LLM-Partien (Argon/Flash, 1v1) | 551, 1,04 Mio. Frames |
| Value-Kopf R6 (Val-MSE gegen Gleichverteilung) | 0,152 gegen 0,157 |

## 2. Entscheidungen

1. **LLM-Partien sind die wertvollsten Lehrerdaten.** LLMs spielen
   deutlich stärker als die Haus-KI. Ihre Züge werden aus beiden Sitzen und
   unabhängig vom Ergebnis imitiert (`--outcome-weighting-off`).
2. **Daten überleben den Build.** `convert4` prüft LLM-Aufzeichnungen
   anderer Builds über den build-stabilen `seen`-Digest nach jeder Eingabe
   (`replay_checks` in `meta.json`). Neuronale Sidecars und PPO-Daten bleiben
   bei exaktem Engine-Hash.
3. **Messen mit genug Fällen.** Entwicklung: `dev1024` (Seeds ab
   `930301000000000000`, Policy-Seed `63001`, 29 Test-Decks, K2, greedy, raw).
   Freigabe: dieselbe Konfiguration mit Seeds ab `940301000000000000`, nur
   für Freigabeentscheidungen. Bei rund 28 % diskordanten Paaren erkennt
   `dev1024` etwa 6–7 pp; 64 Fälle erkennen erst rund 20 pp.
4. **Ein Lauf ist eine `train_ops pipeline`:** Training, Export, Evaluation
   und gepaarter Vergleich in einem Manifest, aus einem eingefrorenen
   Git-Worktree (`runtime/src-<commit>`) und eingefrorenen Binaries
   (`runtime/<id>/`). Keine zusätzlichen Siegel pro Datei.

## 3. Laufende und nächste Experimente

| Name | Frage | Stand |
| --- | --- | --- |
| `v6a-llm-bc` | Verbessert LLM-Imitation R6 auf `dev1024`? R6 → 3 Epochen auf allen LLM-Partien, Loss5, ohne Ergebnisgewichtung, lr 1e-5 | läuft seit 30.10. |
| `v6b` | Wie v6a plus Haus-Lehrerdaten als Anker (halbe Lanes) gegen Vergessen | nach v6a |
| Terra-Ergänzung | Neue Terra-Partien der laufenden Queue in den Korpus | nach Queue-Ende |
| Value-Diagnose | Value-Güte pro Format und Spielphase | parallel, CPU |

## 4. Architektur (nach den BC-Läufen, je eine Änderung, gleiche Rechenzeit)

1. Kandidaten als Tokens: Optionen sehen einander und das Board; die
   Denkschritte sehen die Optionen. Heute schaut der Denkblock nur auf das
   Board, daher bringen K1/2/4/8 nichts.
2. Schrittweise Auswahl für Angreifer, Blocker und Ziele statt unabhängiger
   Einzelwahrscheinlichkeiten.
3. Relationsmodul durch einen Kantentyp-Bias in jeder Attention-Schicht
   ersetzen oder streichen.
4. Trainingsdurchsatz: 82 % der Frames sind passive Gedächtnis-Updates mit
   vollem Trunk. Variante mit billigerem Gedächtnis gegen die heutige.
5. Danach ein kleinerer Schüler (5–15 Mio. Parameter) für Laptop/NPU und als
   Grundlage für RL mit mindestens 1.000–2.000 Spielen pro Iteration.
