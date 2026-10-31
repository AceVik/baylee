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
5. **Die Haus-KI ist Gegner, nie Lehrer** (Eigentümer, 31. Oktober). Sie
   spielt schwach; ihre Züge werden weder imitiert noch als Beschriftung
   eigener Partien genutzt. Lehrer sind LLM-Partien und das, was das Netz
   aus eigenen Partien lernt. `v6d` hat das gemessen bestätigt.

## 3. Laufende und nächste Experimente

| Name | Frage | Stand |
| --- | --- | --- |
| `v6ac-llm-bc`, Arm a | Verbessert LLM-Imitation R6 auf `dev1024`? R6 → 7.200 Updates (≈ 2 Epochen) auf 551 LLM-Partien, Loss5, ohne Ergebnisgewichtung, lr 1e-5 | **354/1024 Siege (34,6 %) gegen R6 325 (31,7 %)**; gepaart 114 besser / 85 schlechter |
| `v6c-llm-bc`, Arm c | Dasselbe mit Kandidatenrevision 3 (Abschnitt 4.1), exakt aus R6 übernommen | **302/1024 (29,5 %)**; gegen Arm a 61 besser / 113 schlechter. Bessere Imitation, schlechteres Spiel; 0 Haus-Fallbacks |
| `v6a2-llm-bc` | Arm a weiter, 2 Epochen auf 651 LLM-Partien (inkl. 100 Terra) | **339/1024 (33,1 %)**; bessere Imitation (50,4 % echte Spielzüge), 25 statt 43 Fallbacks, aber kein Spielgewinn gegen Arm a |
| `v6d-dagger`, d1 | Hilft DAgger mit Sharp als Lehrer? Arm a spielte 2.046 Trainingsdeck-Partien gegen Sharp (30,0 % Siege), Sharp beschriftete jede seiner Entscheidungen; 14.000 Updates, halbe Lanes Sharp-Beschriftung (Tischhälfte 0), halbe Lanes alle 651 LLM-Partien | **273/1024 (26,7 %)**, deutlich schlechter; LLM-Trefferquote sank 48,2 → 46,1 %. Richtung beendet (Entscheidung 5) |
| Value-Diagnose | Value-Güte pro Format und Spielphase | offen |

Offizielle gepaarte Vergleiche auf `dev1024` (`paired_eval4`, Tisch-Bootstrap,
512 unabhängige Tische, alle Auswertungen mit versiegeltem Manifest neu gespielt;
Wiederholungen trafen die Siegzahlen bis auf eine Partie genau):

| Vergleich | Differenz | 95-%-Intervall |
| --- | ---: | :---: |
| Arm a gegen R6 | **+2,8 pp** | [+0,1; +5,6] |
| a2 gegen R6 | +1,3 pp | [−1,4; +4,0] |
| a2 gegen Arm a | −1,5 pp | [−3,9; +0,8] |
| Arm c gegen R6 | −2,3 pp | [−5,0; +0,5] |
| Arm c gegen Arm a | −5,1 pp | [−7,7; −2,6] |
| d1 gegen Arm a | −7,9 pp | [−10,6; −5,2] |
| d1 gegen R6 | −5,1 pp | [−7,7; −2,4] |

Belege: `~/baylee-data/v6/train/v6a2-llm-bc/paired-*.json`,
`~/baylee-data/v6/train/v6d-dagger/paired-*.json`.

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

## 5. Lernen aus eigenen Partien (Plan, Stand 31. Oktober)

Ausgangspunkt: LLM-Partien sind knapp (rund 650, mehr folgen), die Haus-KI ist
nur Gegner (Entscheidung 5). Fortschritt über die LLM-Imitation hinaus muss
also aus eigenen Partien kommen. Die Literaturlage (Belege im
Recherchebericht vom 31. Oktober) ist eindeutig in einem Punkt: Messbare
Gewinne brauchen 10⁴–10⁵ Partien, nicht die 8–16 der bisherigen PPO-Versuche
(AlphaGo: 1,28 Mio. Partien vom BC-Start; Hearthstone/LoCM, DouZero,
Stratego in derselben Größenordnung oder darüber). Mit gebündelter
FP16-Inferenz (Abschnitt 6) wird das auf diesem PC erreichbar.

### 5.1 Schleife

| Teil | Festlegung |
| --- | --- |
| Akteur | `eval4 --inference-batch 32 --raw-policy --temperature 1`, FP16-Export des aktuellen Stands, Trainingsdecks |
| Vorteil | Jeder Deal wird G = 4-mal mit verschiedenen Policy-Seeds gespielt (gleicher Tisch, gleiche Mischung). Vorteil einer Partie = Ergebnis minus Mittel der drei anderen Wiederholungen (RLOO/GRPO-Prinzip), γ = 1, nur das Endergebnis. Das entfernt das Kartenglück des Deals, ohne einen Critic zu brauchen; der heutige Value-Kopf ist kaum besser als gleichverteilt |
| Gegner | 50 % aktueller Stand, 20 % frühere Stände, 20 % Haus-KIs (nur als Gegner), 10 % LLM-Anker (Arm a); neuer Stand kommt in den Pool, wenn er ihn zu ≥ 55 % schlägt |
| Lerner | PPO-Clip 0,2, ein bis zwei Durchgänge je Datensatz, Adam lr 1e-5–3e-5, Entropie 0,003–0,01; keine erzwungenen Entscheidungen im Verlust |
| Präzision | Akteur FP16 mit gespeicherter Verhaltens-Wahrscheinlichkeit μ. Lerner berechnet π_old zu Beginn jedes Datensatzes neu und gewichtet mit min(π_old/μ, 2) (gestutztes Importance Sampling; deckt FP16-Abweichung und Verzögerung ab). Überwacht wird \|log π_old − log μ\|; liegt mehr als 1 % außerhalb [0,8; 1,25], ist das rekurrente Gedächtnis gedriftet |
| Anker | Verlust = PPO + β·KL(π ‖ π_Arm a) + α·NLL(LLM-Entscheidungen). β fällt von 0,1 nach einem Potenzgesetz auf 0,01; α ≈ 0,5 auf 15 % der Lanes. Fällt die Trefferquote auf zurückgehaltenen LLM-Partien um mehr als 3 pp, steigt β |
| Bewertung | Gepaart auf denselben Deals; zwischendurch frische Deals mit sequenziellem Test (SPRT, H1 = +3 pp, α = β = 0,05); `dev1024` nur für Meilensteine; immer gegen Sharp, den letzten besten Stand und Arm a, damit reines Ausnutzen der Haus-KI auffällt |
| Umfang | 2.000–4.000 Partien je Iteration; ein Signal erst nach 10⁴–10⁵ Partien erwarten |

### 5.2 Was dafür gebaut wird

1. Sammelvertrag `selfplay_pg_v1`: `convert4` und die Zulassung nehmen
   FP16-Akteure mit gespeichertem μ und der Gruppenzuordnung (Deal,
   Wiederholung) an. Die bestehende FP32-PPO-Zulassung bleibt unverändert.
2. Lerner in PyTorch: Gruppenvorteile, PPO mit gestutztem IS, KL zum
   eingefrorenen Anker (FP16, ≈ 150 MB), LLM-BC-Lanes, Drift-Messung.
3. Schleifensteuerung in Rust (`train_ops`): sammeln → konvertieren → lernen
   → FP16 exportieren. Der Akteur der nächsten Iteration läuft parallel zum
   Lerner (eine Iteration Verzögerung, durch das IS-Gewicht abgedeckt).
4. Sequenzieller gepaarter Test in `paired_eval4`.

### 5.3 LLM-Kontingent besser nutzen (Vorschlag an den Eigentümer)

Statt ganzer Partien kann ein LLM kritische Stellungen aus den eigenen
Partien des Netzes beschriften (DAgger mit LLM-Lehrer): nur Entscheidungen
mit hoher Unsicherheit des Netzes, einige Dutzend pro Partie. Das streckt
ein Wochenkontingent grob zehnfach. Außerdem kann Self-Play an Stellungen
aus LLM-Partien abzweigen, damit das Netz die Lagen starker Spieler übt.

### 5.4 Später

- Ein Critic, der im Training verdeckte Information sieht (PerfectDou,
  Suphx), ist nachweislich wirksam, berührt aber die Regel, dass verdeckte
  Information nicht darstellbar ist (`docs/architecture.md`). Nur mit
  eigener Entscheidung und einem Zaun, der ihn aus jedem ausgelieferten Pfad
  hält.
- Suche mit gezogenen verdeckten Zuständen (Determinisierung) und Destillation
  der Suchziele, sobald ein Critic brauchbar ist.

## 6. Rechenressourcen (gemessen am 31. Oktober)

Akteur: `eval4` mit gebündelter Inferenz und FP16-Export (Arm a, gegen Sharp,
Temperatur 1, ohne Haus-Duplikat), Lerner: `train5` Batch 8 × 32 mit den
Durchsatz-Optionen aus Abschnitt 3.

| Messung | Ergebnis |
| --- | --- |
| Akteur, 16/32/64/96 Threads | 1,94 / 2,99 / 3,68 / 3,73 Partien/s; ab 64 Threads gesättigt (≈ 13.000 Partien/h, 18 Zeilen je Aufruf, 1,3 GB VRAM) |
| Lerner, Batch 8/16/32 | 2,06 / ≈ 1,1 Updates/s / Speicher voll; 530 / 560 Frames/s, 4,2 / 10,4 GB. Größere Batches bringen kaum Durchsatz |
| Beides gleichzeitig | Lerner −37 %, Akteur −40 %, GPU 98 % belegt, CPU 82 % frei; zusammen ≈ 1,2× der Einzelleistung |

Folgerungen:

1. Der Akteur ist nicht mehr der Engpass: Er erzeugt rund zehnmal mehr
   Frames, als der Lerner verarbeitet (≈ 1.000 Partien/h bei einmaliger
   Nutzung). Akteur und Lerner laufen gleichzeitig; der Akteur braucht dann
   nur einen Bruchteil der Zeit.
2. Der Hebel liegt im Lerner. Nur 12,6 % der Frames sind Entscheidungen; der
   Rest sind passive Gedächtnis-Updates mit vollem Trunk in Vorwärts- und
   Rückwärtsrichtung. Für RL wird der Lerner deshalb entscheidungszentriert:
   Der Akteur speichert den rekurrenten Zustand (FP32) an Fensteranfängen,
   der Lerner rechnet nur kurze Fenster (Aufwärmphase plus Entscheidung,
   R2D2-Prinzip) statt ganzer Partien. Erwartung: ein Mehrfaches an
   Entscheidungen pro Sekunde; das Gedächtnis lernt weiter in den
   BC-Phasen mit voller Rückpropagation.
3. Die CPU ist fast ungenutzt (≈ 80–90 % frei). Spielsimulation,
   Haus-Gegner und Konvertierung haben Spielraum; mehr Akteur-Threads
   kosten nichts.
4. FP16 statt BF16 im Lerner bringt auf der 4070 Ti keinen Durchsatz; es
   verkleinert aber die Abweichung zum FP16-Akteur und wird beim RL-Lerner
   zusammen mit dem gestutzten IS-Gewicht genutzt, sofern die
   Verlustskalierung stabil bleibt.
