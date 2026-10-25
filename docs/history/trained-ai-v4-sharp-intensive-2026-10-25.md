# Intensives Training gegen Sharp, 25.10.2026

Der Nutzer verlangt ausdrücklich intensives Training gegen die Haus-KI `sharp`.
Der aktive [Critic-Pilot](trained-ai-v4-own-critic-2026-10-24.md) wird vollständig
beendet. Dessen Sammlung ist fertig: 247 von 248 Fällen waren beendet; alle 247
wurden ohne Replay-Ablehnung konvertiert. Die eine unvollständige Partie erhält
kein terminales Remis-Label. Ein anschließender Controller startet frische
Spiel- und Policy-Trainingsrunden, sobald der Critic-Prozess nach Export und
Paritätsprüfung tatsächlich beendet ist.

Experiment: `~/baylee-data/v4/experiments/sharp-intensive-20261025/`.
`plan.json`, eingefrorene Runtime-Hashes, Job-Manifeste, Status und Ergebnisse
liegen dort. Es gibt keine automatische Promotion.

## Trainingsumfang

Geplant sind **acht Runden mit jeweils 156 eigenen Partien**, insgesamt
**1.248 Trainingsfälle**, ausschließlich gegen feste `sharp`-Hausgegner.
Pro Runde: 96 Duelle, 32 Vierer-Commander, 8 Teamfälle (2v2), 6 Dreier-FFA,
6 Sechser-FFA und 8 Achter-FFA. Der fokale Sitz und gegebenenfalls seine
Teamkollegen spielen neuronal. Decks kommen aus dem Trainingssplit des bestehenden
Manifests; jede Runde verwendet neue Seeds und den tatsächlich zuvor akzeptierten
Actor. Temperatur 0,6 und K=1 bleiben fest. Das sind eigene Spiele gegen Sharp,
keine bloße Wiederholung von Sharp-Lehrerzügen.

Die erste neue Sammlung hat Vorrang vor den erneuten Vergleichsspielen. Der
unveränderte Ausgangs-Actor und v3 sind eingefroren und können nach dem ersten
Policy-Training mit denselben Vergleichsbedingungen bewertet werden.

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

Nach jeder Runde folgen 128 Duelle und 64 Commander-Fälle mit festen
Validierungsdecks, Seeds und Sitzrotationen gegen Sharp. Der Steady-Hausvergleich
wird mit identischen Startbedingungen dupliziert. Gepaarte Berichte vergleichen
den Kandidaten mit dem Ausgangsmodell, der vorigen Runde und v3. Unsicherheit
wird nach ganzen Tischen geclustert; Caps bleiben unbekannte Ausgänge.

Die wiederholt betrachtete Suite ist ein Entwicklungssignal. Ein Kandidat wird
vorab nach dem Mittel seiner Duell-/Commander-Utility gewählt, einschließlich
des unveränderten Ausgangsmodells. Nur für diese konservative Rangfolge zählen
unbekannte Caps als untere Utility-Grenze null. Anschließend erhält der gewählte
Kandidat einen weiteren Vergleich gegen den Ausgangsstand mit **neuen Seeds**.
Die Deckfamilien bleiben Entwicklungsfamilien: Das ist kein unangetasteter
Familien-Endtest und keine automatische Freigabe als stärkstes Modell.

## Implementierungsprüfung

14 CPU-Tests prüfen unter anderem vollständige Erinnerung, echte versus
erzwungene Entscheidungen, rohe bedingte Masken, spätere fehlerhafte Metadaten,
Gradientennormierung über Microbatches, Lernen eines bekannten Signals, unveränderte
Critic-/Gedächtnisausgaben, identische Logprobs bei gruppierter Board-Berechnung,
Nullgradienten und vollständigen Rollback einschließlich Adam-Zählern. Ein
unabhängiger Agent prüfte PPO-Semantik und Controller. Vor dem Dauerlauf folgt
zusätzlich ein begrenzter Schritt des echten großen Modells auf acht vollständigen,
kurzen Sharp-Duellen; das ist eine Implementierungsprobe, kein Spielstärketest.

Diese Probe ist inzwischen bestanden: 138,6 Sekunden auf der CPU, 1,57 GiB
Peak-PSS, 3.265 chronologische Lernframes und 659 auditierte Entscheidungen.
Maximale Logprob-Abweichung 0,0000058413. Ein Update mit 129 Entscheidungen
wurde akzeptiert; dessen KL beträgt 0,0000055835 und der anschließende Audit
über alle 659 Entscheidungen 0,0000051767. Alle gemeinsamen, Gedächtnis- und
Value-Tensoren blieben bitidentisch. Der Probekandidat wird nicht als Quelle
des Dauerlaufs verwendet.

Der Controller und sein Abschluss-/Fehler-Wecker sind gestartet. Anfangsphase
ist `waiting-for-critic`; die GPU gehört weiter dem gesunden Critic-Lauf.
Nach dessen tatsächlichem Ende folgen Sammlung und Policy-Training ohne
weitere Benutzerfreigabe. Die Python-Runtime ist bei `da10b2133` eingefroren,
die Rust-Binaries wurden bei `6afb09c7d` gebaut und separat gehasht. Es wurde
kein laufender Versuch umgeschrieben und kein Push ohne lokale Gates ausgeführt.

## Übergang nach dem Critic-Abschluss

Der Critic-Lauf ist abgeschlossen; der bereits gestartete Controller hat ohne
weiteren Start die erste Sharp-Sammlung begonnen. Die 96 Duell- und 32
Commander-Fälle der ersten Runde sind fertig; die weiteren Formate folgen.
Alle acht geplanten Runden bleiben unverändert; ein zusätzlicher Controller
wurde nicht gestartet.

Der [Critic-Review](trained-ai-v4-own-critic-2026-10-24.md#ergebnis-und-review-am-25102026)
findet einen besseren Gesamt-MSE, aber keine belastbare Verbesserung an
Entscheidungsstellen. Der neue Head bleibt deshalb ausschließlich eingefrorene
Zustandsbaseline für vollständige Monte-Carlo-Returns. Die echten Spielausgänge
liefern das Lernziel. Es gibt weiterhin keine Value-basierte Suche oder
automatische Promotion. Die anschließenden gepaarten Spielvergleiche müssen
zeigen, ob die Policy-Updates tatsächlich helfen.
