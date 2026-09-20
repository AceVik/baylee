Du schreibst **einen** Engine-Test für die Karte unten. Oben steht
alles, was du dafür brauchst: das Testkit (`Duel`, `keep_mulligans`,
`reach_main_phase`, `play_land`, `cast_from_hand`, `cast_with_floating`,
`activate`, `tap_all_mana`, `tap_all_mana_but`, `tap_mana_except`,
`card_index`, …), die geteilten Helfer aus `card_tests/mod.rs`, die
Frage-Taxonomie (`Pending`, `LegalActions`, `PlayerAction`) und eine ganze
fertige Testdatei als Vorbild.

Regeln:

1. **Spiele die Karte wirklich.** Nicht die Datei lesen, nicht `CardDef`-Felder
   vergleichen — die Karte muss in einer Partie ankommen und das tun, was ihr
   gedruckter Text sagt. Ein Test, der nur prüft, dass eine Karte existiert,
   ist wertlos.
2. **Nur Helfer, die oben vorkommen.** Erfinde keine Funktion, die du nicht
   gelesen hast. Wenn dir für einen sauberen Test etwas fehlt, schreibe
   stattdessen `SKIP: <was fehlt>` und sonst nichts.
3. **Die Karte ist teilweise umgesetzt** (`Coverage::Partial`), wenn das im
   Kartenkopf steht — dann teste **das, was da ist**, und nicht die Klausel,
   die in der `// NOT SUPPORTED:`-Zeile steht.
4. Ein Doc-Kommentar über dem Test, zwei bis fünf Sätze: was die Karte druckt
   und warum genau dieses Szenario etwas beweist. Kein `mod`, kein `use`.
5. Der Kartengriff gehört dazu:
   `fn <slug>() -> CardIndex { card_index("<oracle_id>") }`.
6. **Erst Mana in den Pool, dann behaupten.** Ob eine Fähigkeit angeboten
   oder ein Zauber spielbar ist, liest die Engine am *Mana-Pool* ab und
   nicht daran, was man noch tappen könnte. Also `tap_all_mana` vor jeder
   solchen Behauptung — und wenn eine Steuer wirklich *gefragt* werden
   soll (`PlayerMayPayOr`), genug Länder, dass der Pool sie deckt: einem
   Spieler, der nicht zahlen kann, wird gar nicht erst die Frage gestellt.
7. **`tap_all_mana` tappt jede Mana-Fähigkeit, deren ganzer Preis ihr
   eigenes `{T}` ist** (#159) — die Grundlandtypen aus CR 305.6 *und* das
   gedruckte `{T}: Add …` eines Mana Vault, eines Mox, eines Sol Rings,
   einer Mana-Kreatur. Nicht gedrückt wird ein **größerer** Preis: Wall of
   Roots zahlt eine −0/−1-Marke, ein Filterland `{1}, {T}`; die aktivierst
   du von Hand mit `activate(&mut engine, seat, card, i)`. Tappe
   **nach** einem `tap_all_mana` nichts mehr von Hand — die Quelle ist
   schon getappt und das `apply` wird abgelehnt, was sich wie ein
   Kartendefekt liest und keiner ist. Soll eine Quelle stehen bleiben,
   nenne sie: `tap_mana_except(&mut engine, seat, object)` für ein Objekt,
   `tap_all_mana_but(&mut engine, seat, Some(<slug>()))` für eine Karte,
   danach `cast_with_floating`.
8. **Die Frageform steht in `choice.rs`, rate sie nicht.** Ein Ziel, das
   nur ein Spieler ist, kommt als `Pending::ChoosePlayer`; eine Opfergabe
   als Kosten als `ChooseCards { prompt: CostSacrifice }`; eine
   Alternativkosten-Wahl als `ChooseCastMode` — und die nur, wenn mehr als
   ein Modus bezahlbar ist.
9. **Ein Gegenbeweis braucht eine saubere Kontrolle.** Wenn du die fehlende
   Klausel einer `Coverage::Partial`-Karte zeigst, wähle etwas, das das
   behauptete Ergebnis nicht aus einem *anderen* Grund erzeugt — ein Land,
   das von sich aus getappt ins Spiel kommt, beweist nichts über eine Karte,
   die Länder tappt.
10. **Aktivierungs-Reihenfolge: erst das Ziel, dann die Kosten.** Ziele
   werden beim Ankündigen gewählt (CR 601.2c), die Kosten sind der
   *letzte* Schritt (CR 601.2h). Solange also `Pending::ChooseTargets`
   offen ist, ist die Marke noch auf der Kreatur, das Mana noch im Pool
   und das Opfer noch auf dem Schlachtfeld — die Kosten-Behauptung
   gehört hinter das `apply`, das die Ziel-Frage beantwortet.

Antworte mit **genau einem** ```rust-Block: der Kartengriff und die eine
`#[test]`-Funktion. Kein weiterer Text.
