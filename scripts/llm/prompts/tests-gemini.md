Du schreibst **Engine-Tests** für fertig implementierte Magic-Karten in
diesem Repository. Du implementierst keine Karten und änderst keine
Kartendatei.

## Was du zuerst liest

1. `crates/baylee-engine/src/engine/testkit.rs` — `Duel`, `keep_mulligans`,
   `reach_main_phase`, `walk_to_own_main`, `reach_their_main_phase`,
   `play_land`, `cast_from_hand`, `activate`, `tap_all_mana`, `pass_until`,
   `card_index`, `stack_is_empty`.
2. `crates/baylee-engine/src/engine/card_tests/mod.rs` — die geteilten
   Helfer (`on_battlefield`, `all_on_battlefield`, `in_hand`, `in_graveyard`,
   `pt`, `keywords`, `is_tapped`, `counters_on`, `library_size`, …) und die
   Kartengriffe, die es schon gibt.
3. `crates/baylee-engine/src/choice.rs` — die Frage-Taxonomie (`Pending`,
   `LegalActions`, `PlayerAction`). **Rate keine Frageform**, lies sie dort
   nach.
4. Eine fertige Testdatei als Vorbild, z. B.
   `crates/baylee-engine/src/engine/card_tests/artifacts.rs`.

## Die Regeln

1. **Spiele die Karte wirklich.** Nicht die Kartendatei vergleichen, nicht
   `CardDef`-Felder lesen: die Karte muss in einer Partie ankommen und das
   tun, was ihr gedruckter `//! Oracle:`-Text sagt.
2. **Nur Helfer, die es gibt.** Erfinde keine Funktion. Fehlt dir etwas für
   einen sauberen Test, schreibe stattdessen `SKIP: <was fehlt>`.
3. **Steht `Coverage::Partial` im Kopf, teste das, was da ist** — und nicht
   die Klausel in der `// NOT SUPPORTED:`-Zeile. Die Lücke darfst du
   *gegenprüfen* (sie passiert nicht), aber dann mit einer Kontrolle, die das
   behauptete Ergebnis nicht aus einem anderen Grund erzeugt.
4. **Erst Mana in den Pool, dann behaupten.** Ob eine Fähigkeit angeboten
   oder ein Zauber spielbar ist, liest die Engine am *Mana-Pool* ab, nicht
   daran, was man noch tappen könnte: also `tap_all_mana` davor. Soll eine
   Steuer wirklich *gefragt* werden, braucht der Gegner genug Länder, dass
   sein Pool sie deckt.
5. **`tap_all_mana` ist die Liste der Grundland-Typen** (CR 305.6). Die
   gedruckte `{T}`-Fähigkeit einer Nicht-Land-Karte tappt sie nicht; die
   aktivierst du mit `activate(&mut engine, seat, card, index)`.
6. **Eine aktivierte Fähigkeit geht über den Stack, und die Reihenfolge
   ist: erst das Ziel, dann die Kosten.** Ziele werden beim Ankündigen
   gewählt (CR 601.2c), die Kosten sind der *letzte* Schritt der
   Aktivierung (CR 601.2h). Solange also `Pending::ChooseTargets` offen
   ist, ist die Marke noch auf der Kreatur, das Mana noch im Pool und das
   Opfer noch auf dem Schlachtfeld — eine Kosten-Behauptung gehört hinter
   das `apply`, das die Ziel-Frage beantwortet. Die Fragen des *Effekts*
   stellt dann erst die Verrechnung; dazwischen liegt Priorität.
7. **„Du darfst suchen" ist eine Frage, nicht zwei**: ein optionales
   `Effect::SearchLibrary` kommt als `ChooseCards { min: 0, max: 1 }`.
8. Ein Doc-Kommentar über dem Test, zwei bis fünf Sätze: was die Karte
   druckt, und warum genau dieses Szenario etwas beweist.

## Was du abgibst

Für **jede** Karte genau eine Datei, geschrieben mit deinem Schreib-Werkzeug:

```
<SCRATCH>/<slug>.rs
```

Inhalt: der Kartengriff
`fn <slug>() -> CardIndex { card_index("<oracle_id>") }` (die Oracle-ID steht
im `//! `-Kopf der Kartendatei) und **eine** `#[test]`-Funktion. Kein `mod`,
kein `use`, kein weiterer Text in der Datei.

Im Doc-Kommentar **jeden Rust-Namen in Backticks** setzen, besonders
`Coverage::Partial` und `Coverage::Implemented`. Clippys `doc_markdown` ist
im Gate ein Fehler, nicht eine Warnung, und ein blanker Name dort hat jede
bisherige Runde denselben Nachlauf gekostet — zuletzt 25 Zeilen in einer.

Mana mit einer Ausgabebeschränkung ("spend this mana only to …") landet
**nicht** in `pool.available(color)`, sondern in `pool.restricted()`. Eine
Zusicherung über `available` ist dort nicht bloß falsch, sie ginge auch für
eine Karte durch, die die Beschränkung verloren hat.

Der Index in `activate(engine, seat, card, i)` zählt **alle** Fähigkeiten
der Karte in der Reihenfolge, in der sie im `abilities`-Feld stehen —
ausgelöste mitgezählt. Steht ein `triggered!` an Position 0, hat die erste
aktivierbare Fähigkeit den Index 1.

Die Frage einer **ausgelösten** Fähigkeit kommt an, wenn der Trigger
*abgearbeitet* wird, nicht wenn die Karte ankommt. Nach
`PlayerAction::PlayLand` steht der Trigger auf dem Stack und
`engine.pending()` ist Priorität. Also nicht in der nächsten Zeile
`Pending::ChooseCards` erwarten, sondern erst
`pass_until(&mut engine, |e| matches!(e.pending(), Pending::ChooseCards { .. }))`.
Und `pass_until` bricht mit „unexpected while passing" ab, sobald irgendeine
Frage ungeantwortet dazwischensteht — ein `pass_until(.., stack_is_empty)`
über einen Scry oder Surveil hinweg ist deshalb immer ein Fehlschlag.

Mana für einen Aktivierungspreis muss **vor** dem `activate` im Pool
liegen, und `tap_mana_except` / `tap_all_mana` tappen nur die Länder mit
Grundland-Typ (CR 305.6). Ein Nichtgrundland, das sein eigenes
`{T}: Add {C}` druckt, tappst du von Hand mit
`PlayerAction::ActivateAbility { source, ability_index: 0 }`.

Führe **kein** `cargo` aus — der Build gehört dem Koordinator. Am Ende gibst
du eine Tabelle aus, eine Zeile pro Karte:

```
<Kartenname> | written | skip | <ein Satz>
```

## Die Karten
