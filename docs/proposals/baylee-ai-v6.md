# BayleeAI v6: Konzept und Plan

Stand 6. November 2026, zweite Fassung nach einer unabhängigen Prüfung. Löst
Abschnitt 4 und 5 von `trained-ai-v6-plan.md` ab; dessen Messungen gelten weiter.
„v6“ meint hier die Modellversion `model_version = 6`, nicht die Arbeitsphase.

Vorgabe des Eigentümers: Alles kommt in **eine** neue Modellversion. BayleeAI v6
lernt gegen verschiedene Gegner mit verschiedenen Spielstärken.

## 1. Was wir wissen

Alle Zahlen: Siege von 1024, Seed B der Zielseiten-Spalte, `dev` / `fresh`, gegen
`sharp`, wenn nicht anders genannt.

| Befund | Zahl |
| --- | --- |
| Netz vor der Spalte | 328 / 341 |
| **Vergleichsnetz B-refresh, roh** | **366 / 379** |
| **B-refresh mit Taktik-Schicht** | **497 / 506** |
| Teile der Taktik-Schicht (dev) | Priorität 423, Teilmengen 385, Einzelwahl 375, Paare 370, Zahlen 364 |
| Summe der Teile gegen das Ganze | +87 gegen +131 Siege, 44 entstehen erst im Zusammenspiel |
| einzelne Regeln (dev) | cast 389 (+2,2 pp), react 377, tap 374, spend 373, Mulligan 371; nur cast mit Intervall über null |
| Suche zur Laufzeit (s1, älterer Build) | 372 → 430 |
| Suche als Lernziel (X1a) | −7,2 pp |
| Fine-Tune des ganzen Netzes | C0-Rezept 292 / 297 / 297 gegen 327: systematisch rund −3 pp, auch bei kleiner Lernrate |
| Einzige Trainings-Gewinne bisher | Zielseiten-Spalte +3,7 pp, Spalte gegen wirkungslose Zauber +1,1 pp (Seed A, vorläufig), jeweils alles außer einer Spalte eingefroren |
| Haus-KI als Lehrer (`v6d`) | −7,9 pp |
| Bessere Imitation (Arm c) | −5,1 pp im Spiel |
| v5-Netz gegen Argon | 6 von 70; ob mit Taktik, ist nicht belegt |

Beobachtungen, deren Ursache offen ist:

- Das Netz handelt kaum im gegnerischen Zug (11 Zauber in 1024 Partien, Lehrer 29 bis 51 %
  in denselben Fenstern) und nimmt nie einen Mulligan (0 von 1024, Lehrer 35 %), obwohl beide
  Mulligan-Optionen im Menü stehen.
- Es tappt viel ohne Verwendung (21.031 von 40.377 Taps).
- Die Taktik-Schicht erreicht 497 mit nur 95 Zaubern im gegnerischen Zug; die Regel, die es
  erzwingt, kommt auf 950 Zauber und nur +1,1 pp. Reaktives Spiel ist also nicht der Hebel.

Drei Erklärungen konkurrieren, und **keine ist belegt**:

1. **Menü.** `legal.payable` steht bewusst nicht im Menü
   (`encode_candidates7_grouped`, `features7/actions.rs`), das Netz sieht es nur als Flag
   `f[127]`. Die Regel-Arme deckeln diesen Hebel aber bei wenigen Prozentpunkten.
2. **Greedy-Auswertung.** `P(pass)` liegt im Gegenzug oft bei 0,6 bis 0,9, greedy wählt immer
   passen. Eine Temperatur-Messung fehlt bisher.
3. **Bewertung.** Das Netz wählt schlecht, weil es Optionen falsch einschätzt: wirkungslose
   Zauber nimmt es in 18,6 % der Fälle, Lehrer in 3,9 %. Ein größeres Menü würde das verstärken.

Der Value-Kopf ist schwach, aber nicht wertlos: MSE 0,1525 gegen die Gleichverteilung 0,1569,
auf einem Datenpaket mit nur 13,9 % Duell-Frames. Die Suche stützt sich fast nur auf ihn und
brachte trotzdem +5,6 pp. Eine Messung nur auf Duellen je Spielphase fehlt.

## 2. Das Modell

Ein Modell, ein Kern, mehrere Köpfe. Gelernte Teile und Programm werden als **BayleeAI v6**
zusammen ausgeliefert. Jeder Kopf ist von Anfang an im Vertrag vorgesehen. In die freigegebene
Fassung kommt er, wenn er in Messungen gewinnt.

- **Kern:** Rumpf von B-refresh, eingefroren. Kein Neutraining und keine größere Architektur am
  Anfang, kein Mixture of Experts. Das schützt vor dem systematischen Verlust beim Training des
  ganzen Netzes und behält die 12-GB-Grenze ein.
- **Entscheider:** bestehende Policy. Absichts-Menü nur, wenn Phase A es trägt.
- **Bewerter:** eigener Kopf für die Gewinnchance auf ruhigen Stellungen, mit getrennten
  Eingängen, sodass er den Rumpf nicht verändert.
- **Gegner-Leser:** Verteilung über die gegnerische Hand aus öffentlichen Eingaben. Er ist die
  Voraussetzung für Suche im Live-Spiel, denn die heutige Suche läuft nur im Evaluator und
  kennt die echten Decklisten (`docs/trained-ai.md`, `lookahead-design.md`).
- **Synergetiker:** lernt aus rund 321.000 Listen, verdeckte Karten vorherzusagen, auf dem
  eingefrorenen Kartenkenner. Er arbeitet beim Spielen und beim Deck-Bau. Im Spiel liefert er:
  - **eigene Seite:** Plan-Vektor des Decks und je Permanent und Handkarte, wie viel der Plan an
    ihr hängt, also was geschützt, gehalten oder nicht getauscht werden soll;
  - **gegnerische Seite:** aus den **gesehenen** Karten eine Verteilung über den Rest des Decks,
    dazu die tragenden Teile seiner Synergie, also Ziele für Entfernen, Kontern und Angriffe.
    Der Wert einer Karte ist, wie stark die Vorhersage des Plans fällt, wenn sie fehlt.
  Diese Deck-Verteilung ist zugleich die Grundlage von Gegner-Leser und Vorausdenker. Sie nutzt nur
  Öffentliches. Eingang ins Netz über Spalten, wie bei der Zielseite.
- **Deck-Kopf:** Deck-Vorschläge auf demselben Synergetiker.
- **Denken, drei Formen, nach gemessenem Nutzen:**
  1. **Vorausdenken in der Engine** ist das eigentliche Denken: Züge werden wirklich durchgespielt.
     Es ist die einzige Form mit gemessenem Gewinn (372 → 430), kostete aber das Elffache an
     Zeit, 0,52 s je durchdachter Entscheidung mit GPU. Für das Live-Spiel braucht es
     gezogene Welten statt echter Decklisten, also den Synergetiker, und die Engine im Sitz
     (`lookahead-design.md`).
  2. **Bedenkzeit nach Bedarf:** Gedacht wird nur, wo es zählt: knappe Entscheidung laut
     Entscheider und Bewerter. Heute wurden 21 % der Entscheidungen durchsucht. Das ist auch
     der Regler für die Spielstärken.
  3. **Inneres Denken K:** Heute sehen die Denkschritte nur das Brett, und K brachte nichts.
     Ein kleiner neuer Block, der Brett und Optionen sieht, lässt sich bei eingefrorenem Rumpf
     trainieren. Experiment, kein Versprechen: Der nächste Vorgänger (Arm c) verlor 5,1 pp.
- **Taktiker:** bleibt Programm und äußere Schicht von v6. Ein gelerntes Tor mit Start bei 497
  geht so nicht: `tactics.rs` und `choices.rs` (rund 8.300 Zeilen) arbeiten mit Masken,
  Tausch nach dem Sampling, Zustand und Mehrschritt-Logik, nicht mit Urteilen je Option.
  Maskierte Aktionen liefern zudem keine Daten, aus denen man Fehler der Regel lernen könnte.
  „Nachtrainieren“ heißt deshalb: Die Regelgruppen einzeln an- und abschalten und jede gegen
  den LLM-Mix messen. Regeln, die nur gegen `sharp` helfen, fallen weg. Ein gelerntes Tor ist
  ein eigenes Umbauprojekt nach v6.

## 3. Woraus es lernt

| Quelle | Wofür | Grenze |
| --- | --- | --- |
| LLM-Partien | eingefrorene Spalten, Köpfe, Messlatte | Absicht aus dem Seat-Protokoll als Label; Karten mit falschen Menü-Texten ausgenommen |
| Partieergebnisse roh gespielt | Bewerter, Liga | Taktik-Partien bleiben draußen (Entscheidung vom 6. November) |
| Decklisten | Synergetiker | Test-Decks bleiben draußen |
| verdeckte Karten aus Aufzeichnungen | Lösungsblatt für Gegner-Leser und Bewerter, Suchziele | erlaubt (Eigentümer, 6. November): nur offline, nie als Eingabe, nie im ausgelieferten Pfad, ein Test im Gate hält das fest |

**Gegner-Mix.** Hoch Opus, Astra, Argon; mittel Sonnet, Sol, Terra; niedrig Flash, DeepSeek,
Haiku. Dazu frühere v6-Stände und die Haus-KI, nur als Gegner.

**v3 und v5 nur als Messlatte (Eigentümer, 6. November).** Sie zeigen, wann v6 ihre
Break-even-Punkte durchbricht. Partien gegen sie gehen nie in Training, Liga oder
Datensätze. Modelle liegen unter `/mnt/xdata/baylee/opponents/`. Eine Partie Claude-Mix
kostete im Oktober rund 1,14 $ (60 Partien, 68,50 $), die 160 $ reichen also für gut 130 Partien.
Darum dient die LLM-Leiter nur für Meilensteine, nicht für jede Iteration. Budgets:

| Quelle | Budget | Einsatz |
| --- | --- | --- |
| Anthropic | rund 160 $ bis Ende November | Leiter-Messungen, Stellungs-Labels; Haiku Max für Masse, sobald die Kosten stehen |
| DeepSeek | rund 50 $, gemessen 0,06 $ je Partie | Masse niedrige Stufe |
| Codex | Wochenlimit, Astra 9 %, Reset in 4 Tagen | Bugsuche bis zum Reset, danach Leiter |
| agy | wird beobachtet | Masse und Leiter |

## 4. Plan

**Phase A, Ursachen messen, ohne Training.** Auf B-refresh, gepaart, dev und fresh:

1. Temperatur 0,3 / 0,5 / 0,7 gegen greedy, roh und mit Taktik.
2. Menü-Arm: `payable` zur Laufzeit als Option, Planer bezahlt (`encode_candidates7` kann es
   schon).
3. Kombinationen: `tap,cast,react`, und was die restlichen 44 Siege erklärt.
4. Bewerter nur auf Duellen, je Spielphase, mit Kalibrierung.
5. Denken: K = 0, 1, 2, 4, 8 auf B-refresh (die Exporte gibt es), und Suche auf B-refresh mit
   Taktik, damit bekannt ist, was sie auf dem heutigen Stand bringt.
6. Leiter aufbauen: Haus-Stufen, B-refresh roh und mit Taktik, jede LLM-Stufe; das klärt auch
   die Argon-Zahl.

**Tor A.** Ein Arm zählt, wenn das untere 95-%-Limit über +1 pp liegt. Bringt der Menü-Arm das
nicht, wird das Absichts-Menü gestrichen oder dient nur saubereren Labels.

**Phase B, Vertrag.** `model_version = 6`, nächster freier Encoder (11 nutzt das Experiment
`v6z`). Köpfe, Export, FP16-Parität, `seat4`-Auswahl, Trennung von Rumpf und Köpfen.
Das Netz soll am Ende vor allem auf NPUs laufen (Eigentümer, 6. November). Darum gehört früh eine
Probe auf die XDNA-NPU des Notebooks dazu: Welche Operatoren laufen dort, welche fallen auf die CPU
zurück, wie weit tragen feste Formen und Quantisierung, und was kostet das an Spielstärke?

**Phase C, Köpfe einzeln.** Je Kopf: Rumpf eingefroren, drei Seeds, gepaart gegen B-refresh.
Reihenfolge nach Phase A; vorläufig Spalte gegen wirkungslose Zauber, Gegner-Leser, Bewerter,
Synergetiker. Der Gegner-Leser lernt aus vorhandenen Aufzeichnungen, die Lösung steht in jeder
Partie. Vorrang haben LLM-Partien und eigene Partien, weil er das Verhalten des Gegners mitlernt;
Haus-Partien nur für die Menge, getrennt gemessen. Maßstab: Treffer und Kalibrierung gegen die
Gleichverteilung über die ungesehenen Karten, je Spielphase. **Tor C:** Mittel von drei Seeds, unteres Limit über null, roh; mit Taktik nicht
schlechter als 497 / 506. Ein Kopf, der das nicht schafft, bleibt draußen.

**Phase D, Regeln gegen den Mix.** Taktik-Regelgruppen einzeln gegen die LLM-Leiter messen.

**Phase E, Lernen aus eigenen Partien.** Erst wenn der Lerner für Fenster (`selfplay_pg_v1`,
entscheidungszentriert) gebaut ist. 10⁵ Partien sind beim gemessenen Lerner 100 bis 160 Stunden
je Seed. Während eines Laufs muss die Engine stehen: Ein Katalogwechsel hat schon einmal alle 557
Aufzeichnungen ungültig gemacht. **Abbruch:** keine Wertungs-Steigerung nach 20.000 Partien.
Der Rumpf wird hier zum ersten Mal mittrainiert, mit Anker an das Vergleichsnetz.

**Phase F, Freigabe.** Stufen eichen (Temperatur, Taktik, K, später Suche), Freigabe-Seeds, Promotion
getrennt entscheiden. Suche im Live-Spiel gehört erst dazu, wenn sie ohne echte Decklisten und
außerhalb des Evaluators läuft.

### Stand nach Phase A (6. November)

Gemessen auf B-refresh, eingefroren, gegen `sharp`; Notizen in
`~/baylee-data/v6/train/v6a-phase-a-notes.md`.

- **Greedy ist nicht die Ursache.** Temperatur bringt fehlende Züge zurück und kostet roh 4 bis
  5 pp. Mit Taktik bleibt sie neutral.
- **Das Menü besteht Tor A nicht.** Die vorab festgelegte Prüfung auf 2048 neuen Partien ergab
  +1,7 pp [+0,0, +3,3] auf die Taktik. Es wird kein Kopf; es kann höchstens saubere Labels liefern.
- **K ist flach** von 0 bis 8 und fällt als Stellschraube weg.
- **Suche** bringt roh +4,2 pp [−0,5, +8,9] und mit Taktik −0,5 pp [−5,2, +4,2] auf 192 Partien.
  Beides ist zu ungenau für ein Urteil.
- **Der Bewerter** schlägt die Brett-Grundlinie nicht verlässlich und liegt deutlich zu hoch.
- **Gegen LLMs fehlt am meisten.** Das Netz gewinnt gegen DeepSeek low 2 von 19 roh und 5 von 20
  mit Taktik. Das Netz spielte beide Seiten jedes Deck-Paars, Glück und Paarung erklären das also
  nicht. Die gelesenen Niederlagen zeigen einfache Fehler: 15 von 20 wären mit einem Zug Vorausschau
  sichtbar, wenn die beste Antwort des Gegners mitzählt.

**Neue Reihenfolge (Eigentümer, 6. November):** Phase D zieht vor Phase C. Taktik-Regeln gegen die
gefundenen Fehlerklassen, gemessen an einer größeren Leiter mit DeepSeek, Haiku Max und 3.8 Flash.
Dann Bewerter, dann Spalten für Ziel-, Farb- und Block-Wahl. Gegner-Leser und Synergetiker bleiben
in v6 und kommen danach. Inneres K entfällt.

**16 Bit überall, wo es trägt (Eigentümer, 7. November).** FP16 auf B-refresh kostet nichts (0,0 und
+0,1 pp) und ist 2,2- bis 2,3-mal schneller bei 55 % des VRAM. Es wird Standard für Auswertung und
Auslieferung. Partien fürs Lernen in 16 Bit, sobald die Abweichung der Zug-Wahrscheinlichkeiten
gemessen und klein ist. Gewichte ohne FP32-Kopie nur, wenn ein Versuch das trägt. Für reine CPUs
entscheidet die Messung, viele rechnen FP16 nicht nativ.

### Stand nach dem Bewerter (7. November)

Kopf auf dem eingefrorenen Rumpf von B-refresh, Politik unverändert auf jedem Deal, Notizen in
`~/baylee-data/v6/train/v6e-notes.md`.

- **Das vorab festgelegte Ziel ist knapp verfehlt.** Über alle Züge schlägt der Kopf das Brett beim
  Wechsel zwischen Stellungen (.733 gegen .715 auf fresh, .731 gegen .718 auf neuen Deals, beide
  Intervalle über null), und Brier liegt ohne Nachkalibrierung weit unter der Konstante. Je
  Zugband nicht: 13 bis 18 ist auf fresh ein Unentschieden, und welches Band fehlt, wechselt mit den Deals.
- **Sprachmodell-Partien tragen.** Ein Kopf, der nie eine Partie gegen `sharp` gesehen hat, liest
  sie fast so gut wie einer mit; Haus-Partien liefern nur die Kalibrierung für `sharp`. Auf
  zurückgehaltenen LLM-Partien .742 gegen .695.
- **Offen:** der Kopf ist am Rand zu sicher (Steigung .50); mit gelerntem Maßstab stimmt die Steigung,
  dafür liegt der Mittelwert auf neuen Decks .05 zu hoch. Ob Suche mit Taktik davon profitiert, ist
  nicht gemessen.
- **16 Bit.** FP16 ist Standard für die Auswertung. FP16-Rollouts zeigen keine Verzerrung im
  PPO-Verhältnis, die Zulassung bleibt aber bis zu einem neuen Audit FP32. Reine BF16-Gewichte plus
  BF16-Adam mit stochastischem Runden kosten beim Kopf nichts Messbares; ohne stochastisches Runden
  bleibt das Lernen stehen. Der Rumpf ist nicht getestet.

## 5. Was wir messen

Spielstärke mit Taktik, Lernfortschritt roh. Vergleichsnetz ist B-refresh. Vergleiche nur innerhalb
eines Builds, drei Seeds, gepaart. Die Leiter nur bei Meilensteinen.

## 6. Risiken

| Risiko | Gegenmaßnahme |
| --- | --- |
| Die Diagnose stimmt wieder nicht | Phase A misst, bevor gebaut wird |
| Eingefrorener Rumpf begrenzt die Stärke | Phase E löst ihn, erst wenn Köpfe und Lerner stehen |
| Regel-Eingaben kodieren Hauswissen in Daten | jede Änderung der Regeln kodiert alte Daten um, deshalb sparsam, Eingaben versioniert |
| Der Planer bezahlt schlecht | Bezahlqualität messen, bevor das Menü wächst |
| Überanpassung an einzelne Gegner | Mix, zurückgehaltene Gegner in der Leiter |
| Neue Karten brechen laufende Datensammlung | Engine-Fenster, Kartenarbeit auf eigenem Branch |
| Budget verfällt | Anthropic-Einsatz bis Ende November festlegen |

## 7. Entscheidungen des Eigentümers (6. November)

1. **Zielmarke:** v6 gewinnt mit Taktik die Hälfte der Partien gegen die mittlere LLM-Stufe
   (Sonnet, Sol, Terra), gemessen auf der Leiter.
2. **Engine-Fenster:** Das Lernen aus eigenen Partien läuft auf einem festen Stand, während Karten
   weiter auf `main` landen. Ein neuer Stand kommt nur an einer Iterationsgrenze dazu.
3. **Lösungsblatt:** verdeckte Karten aus Aufzeichnungen sind Trainingsziel, nur offline.
4. **Messlatte:** v3 und v5 spielen nur für Zahlen, nie für das Training.
