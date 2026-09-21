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
   daran, was man noch tappen könnte: also `tap_all_mana` davor. Eine
   **Steuer** ist davon die Ausnahme und braucht gar nichts:
   `PlayerMayPayOr` (Ward, Esper Sentinel) stellt die Frage, ob Mana
   schwebt oder nicht — CR 605.3a lässt den Spieler es noch in der Frage
   machen —, und ob der Pool sie deckt, entscheidet erst die Antwort. Dem
   Gegner dafür extra Länder hinzustellen war der alte Workaround und
   prüft die Karte nicht mehr. Der Zwilling `PlayerMayPayCostOr` (die
   Karoo-Länder) antwortet umgekehrt: gibt es keinen bezahlbaren Preis,
   wird gar nicht gefragt.
5. **`tap_all_mana` tappt jede Mana-Fähigkeit, deren ganzer Preis ihr
   eigenes `{T}` ist** — die Grundlandtypen aus CR 305.6 *und* das gedruckte
   `{T}: Add …` eines Nichtgrundlands, eines Sol Rings, einer Mana-Kreatur.
   Nicht gedrückt wird eine Mana-Fähigkeit mit einem **größeren** Preis:
   Wall of Roots zahlt eine −0/−1-Marke, Ashnod's Altar eine Kreatur, ein
   Filterland `{1}, {T}`. Die aktivierst du von Hand mit
   `activate(&mut engine, seat, card, index)` — beim Filterland erst, wenn
   das Mana dafür schon schwebt. Eine `{T}`-Fähigkeit, die **kein** Mana
   macht (Riptide Laboratory holt einen Wizard zurück), rührt der Helfer
   ohnehin nicht an.
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
9. **Eine Kreatur mit `{T}: Add …` zählt im Pool mit.** Fünf Tests der
   Runde H haben `mana_pool.total()` gegen die Zahl der *Länder* behauptet
   und standen neben zwei Llanowar Elves, die `tap_all_mana_but` genauso
   tappt. Zähle jede Quelle auf dem Brett — oder stelle für ein Szenario,
   in dem die Kreaturen danach angreifen sollen, gar keine Mana-Kreatur
   hin: getappt fürs Mana heißt nicht mehr angreifen.
10. **Ein Land, das getappt ankommt, gibt in diesem Zug nichts.** Es steht
   erst im Enttappschritt seines Beherrschers wieder auf, und bis dahin ist
   eine Fähigkeit mit `{T}` nicht einmal im Angebot. Willst du die Manazeile
   lesen, geh einen Zug weiter:
   `reach_their_main_phase(&mut engine, PlayerId::new(1))`, dann
   `reach_their_main_phase(&mut engine, p0)`. Dasselbe gilt für ein Land,
   das sich **selbst** zur Kreatur macht: von da an ist es eine Kreatur mit
   Einsatzschwäche (CR 302.6), und sein `{T}` ist im Ankunftszug
   unbezahlbar.
11. **„Spend this mana only …" ist `RestrictedMana`.** `ManaPool::available`
   liest den *einfachen* Pool und findet davon nichts. Lies
   `mana_pool.restricted()` und summiere die Einträge der Farbe. Ein Test,
   der hier 0 behauptet, ist grün und beschreibt ein Land, das nichts
   produziert.
12. **`tap_all_mana` und `tap_all_mana_but` geben nichts zurück.** Was sie
   bewirkt haben, steht im Pool:
   `engine.state().players[0].mana_pool.total()`.
13. **Mana-Symbole in einer Formatzeichenkette werden verdoppelt** —
   `"{{T}}"`, nie `"{T}"` —, und binde keine lokale Variable auf den Namen
   eines Kartengriffs: `let forest = …` verdeckt `fn forest()` für den Rest
   der Funktion.
14. **Behaupte nur, was das Brett hergibt.** Wer Kampfschaden erwartet,
   braucht einen Angreifer mit Stärke (ein 0/2 richtet nichts aus); wer
   „target **Dwarf** you control" prüft, braucht einen Zwerg auf der
   eigenen Seite — und einen auf der anderen, damit „you control" auch
   geprüft und nicht angenommen ist.

15. **`tap_all_mana` drückt auch die Karte, um die es geht.** Creeping Tar
   Pit druckt `{T}: Add {U} or {B}` — ganzer Preis ist das eigene Tapsymbol,
   also tappt der Helfer sie mit, und die Aktivierung danach wird für ein
   bereits getapptes Land abgelehnt. Das liest sich wie ein Kartendefekt und
   ist keiner. Soll die Karte stehen bleiben, nenne sie:
   `tap_all_mana_but(&mut engine, seat, Some(<slug>()))`.
16. **Eine erzeugbare Farbe ist keine Wahl.** „Add one mana of any type that
   a land you control could produce" fragt nur, wenn mehr als eine Sorte
   erzeugbar ist: über zwei Wäldern hat die Engine eine Antwort und fragt
   nichts. Ein Test, der dort `Pending::ChooseColor` erwartet, beschreibt ein
   Brett, das er nicht gebaut hat — stell einen Wald **und** eine Insel hin.
17. **Schwebendes Mana wird mitgezählt.** `tap_mana_except` und
   `tap_all_mana` lassen Mana im Pool stehen, also liest eine Behauptung
   „genau vier schwarze" danach fünf. Ist der ganze Preis der Fähigkeit
   `{T}` plus ein Opfer, tappe **gar nichts** vorher — der leere Pool ist
   das, was „vier und sonst nichts" zu einer exakten Aussage macht.

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
liegen, und `tap_all_mana` holt dafür jede Quelle, deren Mana-Fähigkeit
genau `{T}` kostet. Tappe deshalb **nach** einem `tap_all_mana` nichts mehr
von Hand: das Land ist schon getappt, und das `apply` wird abgelehnt — das
ist derselbe Fehlschlag, den eine nicht angebotene Fähigkeit erzeugt, und
er liest sich wie ein Kartendefekt. Soll eine Quelle stehen bleiben, nenne
sie: `tap_mana_except(&mut engine, seat, object)` für ein Objekt,
`tap_all_mana_but(&mut engine, seat, Some(<slug>()))` für eine Karte — und
schreibe in einem Kommentar, warum. `cast_with_floating` zaubert dann aus
dem, was schwebt.

Führe **kein** `cargo` aus — der Build gehört dem Koordinator. Am Ende gibst
du eine Tabelle aus, eine Zeile pro Karte:

```
<Kartenname> | written | skip | <ein Satz>
```

## Die Karten
