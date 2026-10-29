# BayleeAI v5: Gegenprüfung mit Opus und Fable

Stand: 29. Oktober 2026. Nach erneuerter Claude-Anmeldung wurden
`claude-opus-5-5` und `claude-fable-5-1`, jeweils mit `xhigh`, tatsächlich erreicht.
Beide prüften denselben eingefrorenen Code-/Ergebnisstand unabhängig. Eine zweite
Runde enthielt die jeweils andere Bewertung, zusätzliche Codebelege und
Einwände von Codex. Alle vier Aufrufe endeten erfolgreich mit der angeforderten
Modellkennung. Die Modelle hatten keine Tools; ihre Bewertungen ersetzen keine
ausgeführten Tests. Die Belegmanifeste enthalten die Hashes der Eingaben.

## Entscheidung und nächste Reihenfolge

Der [Refactoring- und Trainingsplan](../proposals/trained-ai-v5-refactor-training.md)
bleibt maßgeblich: zunächst nachgewiesene Fehler und Datenverträge reparieren,
danach getrennte Lernversuche. Eine Vergrößerung des Netzes ist derzeit nicht
durch die Ergebnisse begründet. Ein Vorteil der v5-Architektur gegenüber v4
bleibt unbewiesen; diese Prüfung liefert keinen neuen Spielstärkevergleich.

1. Versionierten Kandidatenkontext fertigstellen. Die gemeinsame additive
   Verschiebung fällt bei kategorischem Softmax tatsächlich heraus. Das
   vorgeschlagene Gate kann diesen konkreten Fehler beheben; seine Lernwirkung
   und Spielstärke müssen erst gemessen werden. Eine kleine G-only-Probe ist
   optional und ersetzt kein Experiment mit gemeinsam trainiertem Trunk.
2. Zahlungs-/Cast-/Cancel-Folgen über alle 256 Referenzfälle untersuchen,
   einschließlich der sechs Izzet-Partien mit 55 Fallbacks. Die Fallback-Zahl
   begrenzt nicht die Zahl möglicher Fehlentscheidungen ohne Fallback.
3. Denkstufen mit versionierter Schedule und korrektem Resume-Vertrag trainieren.
   Latente Änderungen und tatsächliche Logit-/Aktionsänderungen getrennt messen.
4. Stochastischen v5-Head-PPO zunächst technisch absichern; vollständiges
   rekurrentes PPO und Lernen gegen verschiedene neuronale Gegner gesondert
   implementieren. Kleine Experimente bleiben durch den Nutzerauftrag autorisiert;
   die alte gestoppte Trainingsrezeptur wird nicht wieder gestartet.

## Bestätigte neue Korrektur: PPO-Logwahrscheinlichkeiten

Die echte Python-Funktion berechnete Subset-/Paarwahrscheinlichkeiten bisher über
`sigmoid(z).clamp(1e-7, 1-1e-7)` und anschließende Logarithmen. Rust verwendet
stabile Log-Sigmoid-Ausdrücke. Bei seltenen Aktionen mit großen Logits führt das
zu abweichenden Logwahrscheinlichkeiten und verschwindenden Gradienten.

Codex reproduzierte das auf der CPU mit beiden Heads, Temperatur 0,7 und 18
Fällen mit skalierten Logits zwischen −40 und +40. Maximale Abweichung:
**24,057615 Logprob-Einheiten**; zehn Fälle überschritten 0,001 und acht hatten
einen Nullgradienten trotz nichtverschwindender Referenzableitung. Bei +14 und
Nichtauswahl betrug die Abweichung bereits etwa 0,00353. Die Probe belegt einen
Numerikfehler, keine Ursache der bisherigen Spielstärkeverluste.

Die von GPT 6.1 Sol implementierte und von Codex geprüfte Korrektur besteht aus
zwei getrennten Commits:

- `7308f55df`: unveränderte Auslagerung nach `tools/trainer/policy_math4.py`;
  der öffentliche Import über `train_rl4` bleibt erhalten.
- `42633ebe6`: stabile Log-Sigmoid-Rechnung für Auswahl und Nichtauswahl sowie
  konsistente KL. Erzwungene/ungültige Einträge werden vor der Rechnung maskiert.

Sieben neue gezielte Regressionstests plus 38 vorhandene Checks bestehen. Die
Tests umfassen Temperaturen, extreme endliche Logits bis ±1.000, Ableitungen,
KL, Maskierung, kategoriale Kompatibilität und ausgeschlossene skalare Aktionen.
In den 18 ursprünglichen Proben sind die Abweichung zur stabilen Referenz und
die Zahl falscher Nullgradienten jetzt beide null. Das sind synthetische
CPU-Tests und vorhandene Python-Vertragstests, noch kein echter Gameplay-PPO-Lauf.
Freie Logits müssen weiterhin endlich sein. Alte Trainingsnumerik bei gesättigten
Logits wird absichtlich korrigiert; Samplingversionen, Datenprovenienz und die
bestehenden Auditgrenzen wurden nicht geändert.

## Schlussfolgerungen, die die Gegenprüfung korrigiert hat

- Gleiche aggregierte Trefferzahlen bedeuten nicht dieselben Entscheidungen.
  Dafür braucht es Übergänge pro Zustand: richtig→falsch, falsch→richtig und
  falsch→anders falsch.
- Widersprüchliche Labels bei gleicher aktueller Kodierung beweisen noch keinen
  Informationsverlust: Historie/H, Lehrerquelle, Stochastik und gleichwertige
  Aktionen können Unterschiede erklären. Ein negativer linearer H-Probe beweist
  nicht, dass das Gedächtnis keine relevante Information enthält.
- Dass Denkstufen dasselbe Endziel erhalten, verhindert iterative Verbesserung
  nicht. Die flache Kurve eines bei K2 trainierten Modells beweist nicht, dass
  gezieltes Multi-K-Training nutzlos wäre.
- Eine kleine KL garantiert keine unveränderten Argmax-Züge oder Spielergebnisse.
  Ein Pilot mit eingefrorenem Trunk kann Regressionen zeigen, aber weder
  vollständiges rekurrentes Lernen noch dessen Speicherbedarf belegen.
- Caps bleiben im Hauptbericht ungelöst. Sie werden nicht nachträglich zu
  Niederlagen oder Remis umgedeutet. Fallzahlen und Unsicherheit berücksichtigen
  vollständige Tisch-Cluster.
- Subset-/Paaraktionen sind bedingte Schritte in kanonischer Reihenfolge; eine
  pauschale Annahme gemeinsam unabhängiger Bernoullis ist falsch. Der Raw-Modus
  ist bereits gegen taktische Haus-KI-Likelihood geschützt. Die in einer Antwort
  vermutete taktische PPO-Provenienzlücke wurde im Code nicht bestätigt.
- Encoder8 ändert nicht pauschal die behaupteten Kandidatenspalten 0/2/3.
  Geänderte Entity-/Kontextsemantik verlangt trotzdem eine eigene Encoderidentität.
  Vorhandene Shadow-Teacher-Versuche sind bereits DAgger-ähnlich und müssen in
  der Bewertung berücksichtigt werden.

## Noch fehlende Integration

Der aktuelle `eval4` akzeptiert ein Netz und ein Hausprofil über `--against`.
Fokal-/Teamsitze verwenden dieses Netz, andere Sitze die Haus-KI. Der geplante
Mix mit historischen Netzen und Selbstspiel ist dort noch nicht implementiert.
Er verlangt getrennte Policies, Memories und RNGs je Sitz sowie Modellhashes
und explizite Lernerrollen. `source=Net` allein darf später keine gegnerischen
Aktionen in die PPO-Lernerhistorie aufnehmen. Ein Haus-KI-Smoke kann die heutige
Kette prüfen, ersetzt aber nicht das geplante gemischte Experiment.

R1 ist bisher nur als Modultrennung und isolierte Kontextprobe vorbereitet.
R2-Schedule, vollständiges v5-PPO und Encoder8-Integration bleiben offen. Kein
Checkpoint wurde durch die Beratung trainiert oder zum Live-Modell befördert.
Das alte Training ist inaktiv; das Nutzer-Gateway auf Port 28766 läuft weiter.

## Lokale Belege

Konsultationen, eingefrorene Eingaben, Receipts und ursprüngliche Numerikprobe:
`/home/ace/baylee-data/v5/advice/opus-fable-refactor-review-20261029-01a1871f-ac25-77b8-9f63-0fb636b49717/`.

Korrekturprüfung und erneute Numerikprobe:
`/home/ace/baylee-data/v5/advice/ppo-logprob-stability-20261029-01a1872c-deeb-76b2-8411-5f08c93aaf19/verification.json`.

Der [Referenzbericht](trained-ai-v5-reference-2026-10-29.md) dokumentiert die
256 Partien. Der [frühere Review](trained-ai-v5-round10-review-2026-10-29.md)
bleibt als historischer Stand erhalten; sein damaliges Auth-Hindernis ist behoben.
