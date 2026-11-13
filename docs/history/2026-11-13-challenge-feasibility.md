# Erste Versuche für Daily und Weekly

13.11.2026, lokaler Worktree `codex/open-beta-retention`, Basis `8a1ac57795`.
Plan: [Daily und Weekly für die erste Open Beta](../proposals/daily-weekly-challenges.md).

**Ergebnis:** Deckbau und feste Offline-Partien sind reproduzierbar. In sechs
Paarungen wurden 28 Siege gefunden und durch 28 frische Ausführungen bestätigt.
Eine automatische Bestätigung mehrerer *strategisch* verschiedener Lösungswege
existiert damit noch nicht. Keine dieser Aufgaben ist für Veröffentlichung
zertifiziert; Central, Gateway und Client wurden nicht verändert.

## Was der Versuch ausführt

Die neue Rust-Probe in
[`crates/baylee-train/examples/challenge_probe/`](../../crates/baylee-train/examples/challenge_probe/main.rs)
nutzt vorhandene Generator-, Session-, KI- und Replay-Bausteine. Sie benötigt
weder Datenbank noch ONNX, Modellserver oder Netzwerkdienst.

- Kartenpool: 10.860 implementierte Karten, die `Working::scan` in Testquellen
  findet. Das ist eine Auswahlregel und **kein ausgeführter Test aller Karten**.
- Decks: Constructed, vier Archetypen, Generator-Seeds 0 bis 5. Jedes Deck wird
  zweimal erzeugt und als vollständiger Decktext verglichen; Format und Pool
  werden geprüft. Alle Decktexte und ihre SHA-256-Digests werden gespeichert.
- Spiele: die zwölf erzeugten 60-Karten-Decks werden ohne Zurücklegen in sechs
  aufeinanderfolgende Paare eingeteilt. In jedem Paar bleibt der Spiel-Seed 41.
- Gegner: `STEADY`, Policy-Seed 9001, keine adaptive Host-Vorausschau.
- Spieler: `CASUAL`, `STEADY`, `EXPERT`, jeweils Policy-Seed 101 und 202.
- Beide Policies lesen autorisierte Sitzansichten, die eigene Deckliste und
  die öffentlich für ihre Frage bereitgestellte Erklärung. Keine fremde Hand
  oder reale zukünftige Kartenreihenfolge fließt in ihre Entscheidung ein.
- Jeder Lauf erhält eine eigene UUID v7. Für den Versuch werden beide Sitze
  extern gesteuert und ihre Policies explizit initialisiert; die produktive
  Ableitung aus `Session::describe` wird dadurch **nicht** geändert.
- 2.000 Eingaben und 15 Sekunden zwischen Entscheidungen geprüftes weiches
  Zeitlimit. Das Zeitlimit beendet einen Lauf, verändert aber keine Antwort.
  Ein einzelner blockierter Engine-Schritt bräuchte produktiv zusätzlich einen
  äußeren Prozesswächter.
- Jeder Record wird mit `replay_exact` geprüft. Jeder gefundene Sieg wird in
  einer neuen Session mit neuer Versuchs-ID vollständig erneut gespielt. Alle
  akzeptierten Aktionen beider Seiten, Endzustand, Ergebnis und Fallback-Zähler
  müssen übereinstimmen. Das ist mehr als das Wiederabspielen gespeicherter KI-Züge.

Abgewiesene Aktionen werden als Abbruch mit Grund gemeldet. Die Probe meldet
KI-Notantworten und Limits gesondert; sie ist weder ein automatischer Publisher
noch ein umfassender Kandidaten-Zertifizierer. Ein fataler Replayfehler beendet
das Programm mit Fehler. Synthetische Records liegen ausschließlich unter
`target/`, außerhalb der Versionsverwaltung.

## Deckbau

| Prüfung | Ergebnis |
| --- | --- |
| Decks erfolgreich erzeugt | 24 von 24 |
| Vollständiger Decktext bei Wiederholung identisch | 24 von 24 |
| Constructed-Form und Working-Pool geprüft | 24 von 24 |
| Genau 60 Karten | 12 von 24 |
| Andere Größen | je vier Decks mit 63, 71 und 75 Karten |
| Anfrage `forced_colors = R` | zweimal R; einmal UR; einmal RG; zweimal WR |

Der Generator ist für vielfältige Trainingsdecks gebaut. Mehr als 60 Karten
und erweiterte Farben sind hier kein nachgewiesener Regelfehler. Eine Challenge-
Schablone mit „60 Karten, nur Rot“ braucht aber eine harte Nachprüfung oder
explizite neue Generatoroptionen. Ein Feldname allein ist keine Zusicherung.

## Spielresultate der erweiterten Stichprobe

Je sechs Spieler-Policies pro Paar; WB = weiß/schwarz, UG = blau/grün,
UB = blau/schwarz. Zugbereiche umfassen hier auch Niederlagen.

| Paar | Spieler gegen Gegner | Siege | Unterschiedliche rohe Spieler-Aktionsfolgen bei Siegen | Endzug |
| --- | --- | --- | --- | --- |
| 0 | Aggro WB #1 gegen Midrange WB #1 | 6/6 | 6 | 14–18 |
| 1 | Control WB #1 gegen Ramp WB #1 | 6/6 | 5 | 18–20 |
| 2 | Aggro UG #3 gegen Midrange UG #3 | 6/6 | 4 | 16–18 |
| 3 | Control UG #3 gegen Ramp UG #3 | 5/6 | 5 | 16–25 |
| 4 | Aggro UB #4 gegen Midrange UB #4 | 0/6 | 0 | 21 |
| 5 | Control UB #4 gegen Ramp UB #4 | 5/6 | 4 | 21–22 |

36 ursprüngliche Partien, 28 Siege und acht Niederlagen. Alle 36 erreichten
ein reguläres Spielende. Keine abgewiesenen Aktionen, KI-Notantworten, Zeit-
oder Aktionslimit-Abbrüche in dieser Stichprobe.

Alle 28 Siege bestanden die frische Wiederholung. Einschließlich dieser
Wiederholungen wurden **64 vollständige Ausführungen mit exaktem Replay**
geprüft. Die ursprünglichen Spiele benötigten lokal 273–1.545 ms pro Lauf,
ohne anschließendes Replay und ohne Wiederholung. Gleichzeitig liefen Builds;
das ist kein belastbarer Performance-Benchmark und erst recht keine menschliche
Spielzeit.

Die sechs Policies sind eng verwandte Heuristiken, keine sechs unabhängigen
menschlichen Strategen. Die Stichprobe nutzt nur einen Spiel-Seed und verwandte
Deckpaare je Farbe. Insbesondere bedeutet 0/6 für Paar 4 **unbestätigt**, nicht
unlösbar. Ein hoher Sieganteil beweist weder ein gutes Rätsel noch angemessene
Schwierigkeit. Rohe Aktionsfolgen können sich nur durch Manaaktivierungen oder
Karteninstanzen unterscheiden; ihre Anzahl ist kein Diversitätszertifikat.

Vor der Erweiterung wurde ein kleiner Lauf über Paare 0 und 1 ausgeführt:
12 Siege und zwölf erfolgreiche frische Wiederholungen. Diese sind eine
Vorprobe und werden den 36 Partien der obigen Tabelle nicht hinzugerechnet.

## Negativkontrolle

Ein separater Lauf über Paar 0 mit nur **einer erlaubten Eingabe** stoppte
alle sechs Policies mit `answer_cap`. Kein Lauf galt als Sieg, kein Lauf wurde
als frischer Siegesnachweis ausgewiesen. Die sechs unvollständigen Records
ließen sich exakt wiedergeben. Damit wird ein gültiges Teil-Replay nicht mit
einem Beweis für einen Sieg verwechselt.

## Weitere gezielte Prüfungen

Erfolgreich ausgeführt:

- Drei Generator-Basistests in `baylee-cards`: Mana-Mathematik, Synergie und
  Abdeckung eines begrenzten Kartenkorpus.
- Fünf Deckgenerator-Tests in `baylee-train`: darunter drei Formate, vier
  Archetypen und sechs Seeds, Working-/Held-out-Regeln, Text-Roundtrip und
  wiederholbare Erzeugung. Der bestehende Test fordert mindestens 60 erfolgreiche
  Builds aus 72 Versuchen; daraus wird hier keine höhere gemessene Zahl abgeleitet.
- Zwei Gamehost-Integrationstests zur KI-Zufälligkeit: Der Policy-Seed hängt
  nicht am Seed der ausgeteilten Karten; unterschiedliche Spiel-IDs können
  bei identischem Setup unterschiedliche KI-Antworten liefern.
- 18 Gamehost-Tests zur Vorausschau, einschließlich deterministischer
  Arbeitsgrenzen, unverändertem Verhalten bei anderer verdeckter Information
  sowie der heutigen Rückfälle auf einfachere Antworten bei fehlenden
  Host-Slots oder überschrittener Wandzeit. Alle 18 bestanden in 129 Sekunden.

Damit bestanden **28 vorhandene gezielt ausgewählte Tests**, zusätzlich zu
den beschriebenen Spiel- und Replay-Experimenten. Die neuen Rust-Dateien
bestehen `rustfmt --check` und `cargo clippy --quiet --locked -p baylee-train
--example challenge_probe -- -D warnings` ohne Warnungen.
Desktop-, Feature- und Datenbank-Gates sind
für diesen lokalen Machbarkeitslauf nicht ausgeführt; es wurde nichts gepusht,
gelandet oder produktiv bereitgestellt.

## Konsequenzen für den Plan

1. **Gegneridentität einfrieren.** Challenge-Policy-Seed getrennt von der
   Versuchs-ID einführen und im Record festhalten. Sonst erhält jeder Retry
   trotz gleichem Setup einen leicht anderen Gegner.
2. **Harte Aufgabenbedingungen prüfen.** Deckgröße, Farben, Mechaniken und
   getestete Karten müssen vor der Suche passen.
3. **Mehrwege-Nachweis als eigenes Arbeitspaket.** Erst semantisch normalisierte
   Entscheidungen und ein eng definierter Schablonenprüfer erlauben die
   gewünschte Zusicherung von drei Siegen aus mindestens zwei Familien.
4. **Vorausschau nicht lastabhängig wechseln.** Rechenbudget und Policy gehören
   zur Aufgabe. Ein belegter Host wartet oder beendet technisch; er macht den
   Gegner nicht stillschweigend leichter.
5. **Menschen kalibrieren Schwierigkeit.** Die vorläufigen Daily-/Weekly-Zeiten
   und Badge-Schwellen bleiben Hypothesen bis zu ersten Spieltests.
6. **Policy-Eingaben vom Bedienverlauf trennen.** Die Quellprüfung zeigt:
   `policy::noise` nutzt `PlayerView.seq`, und `Session::answer` erhöht die
   Sequenz auch für geänderte Einstellungen ohne Spielaktion. Gleiche
   Spielentscheidungen bei verschiedenem Bedien-/Reconnect-Verlauf benötigen
   daher einen eigenen Fairness-Test. Die aktuelle Offline-Probe sendet keine
   solchen zusätzlichen Einstellungen und deckt diesen Fall nicht ab.

Der vorhandene kosmetische `GameStatic.table_seed` wurde ebenfalls bis zur
Ableitung geprüft: gekürzter SHA-256-Digest, kein durchgereichter Spiel-Seed.
Private Challenge-Pakete, Seeds und vollständige Lösungs-Replays müssen trotzdem
bis Ende der Live-Wertung serverseitig bleiben.

## Reproduktion

Aus dem Repository-Root; `--out` muss ein noch nicht vorhandenes Verzeichnis
sein. Die Befehle erfordern den Kartenpool im Build, aber kein Codegen und
keinen externen Karten-Corpus.

```text
cargo test --locked -p baylee-cards --lib deckgen:: -- --nocapture
cargo test --locked -p baylee-train --lib deckgen:: -- --nocapture
cargo test --locked -p baylee-gamehost --features simulation --test integration randomness:: -- --nocapture
cargo test --locked -p baylee-gamehost --features simulation --lib session::tests::foresight:: -- --nocapture
cargo run --locked -p baylee-train --example challenge_probe -- --out target/challenge-probe-expanded-2026-11-13 --pairs 6
cargo run --locked -p baylee-train --example challenge_probe -- --out target/challenge-probe-cap-2026-11-13 --pairs 1 --max-answers 1
cargo clippy --quiet --locked -p baylee-train --example challenge_probe -- -D warnings
```

Lokale Rohberichte:
[Erweiterte Stichprobe](../../target/challenge-probe-expanded-2026-11-13/report.json),
[Negativkontrolle](../../target/challenge-probe-cap-2026-11-13/report.json).
Diese Links gelten für den Versuchs-Worktree; nach einem frischen Checkout
müssen die oben genannten Befehle erneut ausgeführt werden. Der im JSON gemeldete
Build ist die Repository-Basis; der zusätzliche Probe-Quelltext wird mit diesem
Bericht versioniert.
