# Auftrag: dreißig Magic-Karten fertig schreiben

Du arbeitest in `/Users/viktor/Projects/baylee`, einer Rust-Engine für Magic:
The Gathering. Die Karten dieses Projekts sind Code, kein Datenformat — jede
liegt als eigene Datei unter `crates/baylee-cards/src/cards/` und wird mit
einem Makro-DSL geschrieben.

Dreißig dieser Dateien sind heute generierte Stubs. Deine Aufgabe ist, sie
fertigzuschreiben. Die Liste steht unten.

## Orientiere dich selbst

Ich sage dir nicht, wie eine Karte aussieht — das steht im Projekt, und zwar
besser, als ich es hier wiederholen könnte:

- **`docs/card-dsl.md`** ist der Autorenvertrag. Das ist die erste Datei, die
  du liest, und die wichtigste.
- **`CLAUDE.md`**, Abschnitt „Cards: generated stubs, hand-finished,
  machine-checked" — warum die Regeln unten so sind, wie sie sind.
- **`crates/baylee-cards-dsl/src/`** — `effect.rs`, `cost.rs`, `ability.rs`,
  `filter.rs`, `static_ability.rs`, `build.rs`. Was dort als Variante steht,
  ist genau das, was das DSL sagen kann. Nicht mehr.
- **`crates/baylee-cards/src/cards/`** — 1500 Karten, davon Hunderte fertig.
  Zu fast jeder Zeile, die du schreiben willst, gibt es hier schon eine Karte,
  die etwas Ähnliches druckt. Suche sie, lies sie, triff ihre Redeweise.
- **`crates/baylee-cards/src/filters.rs`** und `crate::tokens` — die
  pool-eigenen Bausteine, damit du keinen zwanzigsten gleichnamigen `static`
  erfindest.

Arbeite in der Reihenfolge, die du für sinnvoll hältst. Karten, die sich
ähneln, gemeinsam zu machen, ist meist schneller — aber das ist deine
Entscheidung.

## Die vier Regeln, die nicht verhandelbar sind

**1. Starte niemals `cargo`.** Kein `build`, `check`, `test`, `clippy`, `fmt`,
`run`. Der Build gehört dem Koordinator; ein zweiter Lauf blockiert am
Target-Lock. Du
liest und schreibst Dateien, sonst nichts. Das Übersetzen und die Tests
übernehme ich, nachdem du fertig bist.

**2. Eine ungelesene Klausel, und die Karte bleibt ein Stub.** Das ist die
Regel, an der dieses Projekt hängt. Wenn das DSL einen gedruckten Satz der
Karte nicht sagen kann, dann schreibe die Karte **nicht ungefähr**:

- Kannst du den Rest sauber bauen, dann
  `coverage = Coverage::Partial("<was fehlt, in einem Halbsatz>")` und eine
  Zeile `// NOT SUPPORTED: <die Klausel>`.
- Kannst du gar nichts bauen, dann lass die Datei **unverändert** und nenne
  sie im Schlussbericht mit dem Grund.

Eine falsch als `Coverage::Implemented` markierte Karte ist schlimmer als ein
Stub, weil der Deckbauer sie dann als spielbar anbietet und ein Spieler sie
mitten in einer Partie kaputt vorfindet. Erfinde **niemals** eine
DSL-Variante, die du nicht in `baylee-cards-dsl` gelesen hast.

**Eine weggelassene *Kosten*-Klausel ist dabei niemals `Partial`.** Die Regel
oben gilt für Klauseln, deren Fehlen die Karte **schwächer** macht — eine
Fähigkeit, die der Spieler dann eben nicht bekommt. Eine Klausel, die etwas
**kostet**, macht die Karte beim Weglassen **stärker** als den Druck, und
dafür gibt es keine ehrliche Teilfassung: dann bleibt die Datei unverändert.
Dazu zählen „As an additional cost to cast this spell, …", jeder Teil einer
Aktivierungskosten-Zeile links vom Doppelpunkt, jedes „unless you pay …",
„you lose N life", „sacrifice …", „discard …", „exile …", und jede
Timing-Einschränkung („Activate only as a sorcery"). Prüfe vor jedem
`Partial`: *ist die Karte, die ich schreibe, besser als die gedruckte?* Wenn
ja, ist sie ein Stub. Crop Rotation kam einmal als `Partial` zurück, ohne
sein „sacrifice a land" — ein Ein-Mana-Tutor umsonst, und jeder gespielte
Test dazu wäre grün gewesen.

**3. Der generierte Kopf bleibt Byte für Byte stehen.** Die `//!`-Zeilen, und
in `card!` die Felder `index`, `oracle_id`, `scryfall_id`, `faces` samt
`name`, `mana_cost`, `types`, `supertypes`, `subtypes`, `power`, `toughness`.
Die fasst ein Generator an, nicht du. Du setzt `coverage`, `keywords`,
`abilities`, ersetzt die Zeile `// GENERATED STUB — …` durch einen kurzen
Kommentar, der sagt, was du gebaut hast, und löschst die `// TODO(card):`-Zeile
am Ende.

Der `//! Oracle:`-Kopf ist der **gedruckte Text** der Karte und damit deine
einzige Quelle dafür, was sie tut. Nicht deine Erinnerung an die Karte.

**4. Schreibe nie einen Standardwert hin, den das Makro schon setzt.** Die
Makro-Voreinstellungen sind Regelvoreinstellungen — Instant-Geschwindigkeit
ist CR 602.2, das Schlachtfeld CR 113.6, `mana_ability = false` ist CR 605.1.
Eine Karte, die sie wiederholt, sagt nichts und verdeckt, wo sie abweicht.

**5. Kein Keyword, das keine Regel liest.** Die Engine liest genau diese
Bits: flying, first strike, double strike, deathtouch, haste, hexproof,
shroud, indestructible, lifelink, menace, reach, trample, vigilance,
defender, flash, prowess, changeling, unblockable, uncounterable, rebound,
daybound, nightbound (die Liste heisst `ENFORCED` in
`crates/baylee-engine/src/engine/keyword_tests.rs`). Jedes andere —
intimidate, undying, persist, infect, fear, … — ist ein totes Bit: weder auf
einer `face!` noch über `Modifier::AddKeyword`, das `keywords`-Feld eines
Pumps oder ein `CopyMod`. Solche Klauseln kommen als `// NOT SUPPORTED:` in
die Karte und in den `Coverage::Partial`-Grund. Eine Karte, die ein totes Bit
behauptet, sieht fertig aus und ändert am Tisch nichts.

## Was du am Ende abgibst

Eine Tabelle, eine Zeile pro Karte, in dieser Form:

```
<Kartenname> | implemented | partial | stub | <ein Satz>
```

Bei `partial` sagt der Satz, welche gedruckte Klausel dich dazu gezwungen hat.
Bei `stub` sagt er, welche DSL-Variante fehlen würde, damit die Karte sagbar
wäre — das ist für mich die wertvollste Zeile im ganzen Bericht, weil sie mir
sagt, was ich als Nächstes in die Engine baue.

Keine Zusammenfassung des Projekts, keine Wiederholung dieser Anweisung, keine
Vorschläge für weitere Arbeit.


**Alle Kommentare, Doc-Kommentare und `assert!`-Meldungen im Rust-Code sind auf Englisch.** Dieses Repository ist durchgehend englisch geschrieben; dieser Prompt ist die einzige deutsche Datei in der Kette, und eine deutsche Zeile im Baum ist eine, die jemand von Hand übersetzen muss.

## Die Karten

