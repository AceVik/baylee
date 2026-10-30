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
| v5-Netz gegen Gemini Argon (Arena, 70 Partien) | 6 Siege, Argon 64 |
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
| `v6ac-llm-bc`, Arm a | Verbessert LLM-Imitation R6 auf `dev1024`? R6 → 7.200 Updates (≈ 2 Epochen) auf 551 LLM-Partien, Loss5, ohne Ergebnisgewichtung, lr 1e-5 | **354/1024 Siege (34,6 %) gegen R6 325 (31,7 %)**; gepaart 114 besser / 85 schlechter |
| `v6c-llm-bc`, Arm c | Dasselbe mit Kandidatenrevision 3 (Abschnitt 4.1), exakt aus R6 übernommen | **302/1024 (29,5 %)**; gegen Arm a 61 besser / 113 schlechter. Bessere Imitation, schlechteres Spiel; 0 Haus-Fallbacks |
| `v6a2-llm-bc` | Arm a weiter, 2 Epochen auf 651 LLM-Partien (inkl. 100 Terra) | läuft seit 31.10.; wertet R6/a/c/a2 mit versiegeltem Manifest neu aus und vergleicht gepaart |
| `v6b` | Haus-Lehrerdaten als Anker (halbe Lanes) gegen Vergessen | offen |
| Value-Diagnose | Value-Güte pro Format und Spielphase | offen |

Lehren aus den ersten Läufen: Imitationsmetriken auf LLM-Validierung
sagen die Spielstärke nicht voraus (Arm c gewinnt jede Imitationsmetrik
außer echten Spielzügen und verliert im Spiel). Einzelne
Validierungspunkte der Trefferquote echter Spielzüge schwanken um ±2 pp.
Wiederholte Evaluierungen derselben Fälle streuen um etwa eine Partie
von 1.024.

### Trainingsdurchsatz

Das Training war durch die CPU beim Starten vieler kleiner GPU-Aufrufe
begrenzt (GPU 26–52 % ausgelastet). Bei 150 echten Updates ohne Änderung der
Validierung: Standard-CUDA-Kernel (`--nondeterministic-kernels`) 1,2×,
dazu vier Frames pro Trunk-Aufruf (`--board-chunk 4 --board-token-budget 8264`)
2,6×. Der Readout-Pfad `--decision-chunk 4` bündelt zusätzlich über die Zeit,
checkpointet die Kandidatenbewertung und hielt den kritischen Batch von Arm c
bei 5,96 statt 11,43 GiB; seitdem ≈ 1,5 Updates/s statt 0,45.

## 4. Architektur (nach den BC-Läufen, je eine Änderung, gleiche Rechenzeit)

1. Kandidaten als Tokens. **Umgesetzt als Revision 3** (`model5/candidate_decoder.py`):
   Jede Option schaut per Cross-Attention auf die Denk-Slots und alle
   Entities, bevor sie bewertet wird. Optionen sehen einander bewusst nicht,
   damit große Menüs exakt in Scheiben bewertet werden können. Offen: Denkschritte, die
   die Optionen sehen. Heute schaut der Denkblock nur auf das Board, daher
   bringen K1/2/4/8 nichts.
2. Schrittweise Auswahl für Angreifer, Blocker und Ziele statt unabhängiger
   Einzelwahrscheinlichkeiten.
3. Relationsmodul durch einen Kantentyp-Bias in jeder Attention-Schicht
   ersetzen oder streichen.
4. Trainingsdurchsatz: 82 % der Frames sind passive Gedächtnis-Updates mit
   vollem Trunk. Variante mit billigerem Gedächtnis gegen die heutige.
5. Danach ein kleinerer Schüler (5–15 Mio. Parameter) für Laptop/NPU und als
   Grundlage für RL mit mindestens 1.000–2.000 Spielen pro Iteration.
