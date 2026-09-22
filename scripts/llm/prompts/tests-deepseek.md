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
   solchen Behauptung. Eine **Steuer** ist davon die Ausnahme:
   `PlayerMayPayOr` (Ward, Esper Sentinel) stellt die Frage, ob Mana
   schwebt oder nicht — CR 605.3a lässt den Spieler es noch in der Frage
   machen —, und ob der Pool sie deckt, entscheidet erst die Antwort. Dem
   Gegner dafür extra Länder hinzustellen war der alte Workaround und
   prüft die Karte nicht mehr. Der Zwilling `PlayerMayPayCostOr` (die
   Karoo-Länder) antwortet umgekehrt: gibt es keinen bezahlbaren Preis,
   wird gar nicht gefragt.
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
   Der häufigste andere Grund ist der **Preis**: `legal.abilities` ist
   hinter `can_afford(player, id, cost)` gefiltert, und das liest den
   Mana-Pool und nicht die ungetappten Länder. Eine fehlende Fähigkeit mit
   irgendeiner Manakosten-Zeile fehlt im Angebot also sowieso, und
   `assert_eq!(…count(), 1)` über ein Brett ohne schwebendes Mana bliebe
   grün, wenn die Klausel morgen geschrieben würde. Vorher
   `tap_mana_except(&mut engine, seat, <die Karte selbst>)`. Der zweite
   Grund ist das **Ziel**: für `!legal.castable.contains(&x)` nimm einen
   Zauber ohne Ziel, sonst lehnt die Engine ihn ab, weil nichts da ist,
   worauf er zeigen kann.
10. **Aktivierungs-Reihenfolge: erst das Ziel, dann die Kosten.** Ziele
   werden beim Ankündigen gewählt (CR 601.2c), die Kosten sind der
   *letzte* Schritt (CR 601.2h). Solange also `Pending::ChooseTargets`
   offen ist, ist die Marke noch auf der Kreatur, das Mana noch im Pool
   und das Opfer noch auf dem Schlachtfeld — die Kosten-Behauptung
   gehört hinter das `apply`, das die Ziel-Frage beantwortet.
11. **Eine Kreatur mit `{T}: Add …` zählt im Pool mit.** Fünf Tests der
   Runde H haben `mana_pool.total()` gegen die Zahl der *Länder* behauptet
   und standen neben zwei Llanowar Elves, die `tap_all_mana_but` genauso
   tappt. Zähle jede Quelle auf dem Brett — oder stelle für ein Szenario,
   in dem die Kreaturen danach angreifen sollen, gar keine Mana-Kreatur
   hin: getappt fürs Mana heißt nicht mehr angreifen.
12. **Ein Land, das getappt ankommt, gibt in diesem Zug nichts.** Es steht
   erst im Enttappschritt seines Beherrschers wieder auf, und bis dahin ist
   eine Fähigkeit mit `{T}` nicht einmal im Angebot. Willst du die Manazeile
   lesen, geh einen Zug weiter:
   `reach_their_main_phase(&mut engine, PlayerId::new(1))`, dann
   `reach_their_main_phase(&mut engine, p0)`. Dasselbe gilt für ein Land,
   das sich **selbst** zur Kreatur macht: von da an ist es eine Kreatur mit
   Einsatzschwäche (CR 302.6), und sein `{T}` ist im Ankunftszug
   unbezahlbar.
13. **„Spend this mana only …" ist `RestrictedMana`.** `ManaPool::available`
   liest den *einfachen* Pool und findet davon nichts. Lies
   `mana_pool.restricted()` und summiere die Einträge der Farbe. Ein Test,
   der hier 0 behauptet, ist grün und beschreibt ein Land, das nichts
   produziert.
14. **`tap_all_mana` und `tap_all_mana_but` geben nichts zurück.** Was sie
   bewirkt haben, steht im Pool:
   `engine.state().players[0].mana_pool.total()`.
15. **Mana-Symbole in einer Formatzeichenkette werden verdoppelt** —
   `"{{T}}"`, nie `"{T}"` —, und binde keine lokale Variable auf den Namen
   eines Kartengriffs: `let forest = …` verdeckt `fn forest()` für den Rest
   der Funktion.
16. **Behaupte nur, was das Brett hergibt.** Wer Kampfschaden erwartet,
   braucht einen Angreifer mit Stärke (ein 0/2 richtet nichts aus); wer
   „target **Dwarf** you control" prüft, braucht einen Zwerg auf der
   eigenen Seite — und einen auf der anderen, damit „you control" auch
   geprüft und nicht angenommen ist.
17. **`tap_all_mana` drückt auch die Karte, um die es geht.** Creeping Tar
   Pit druckt `{T}: Add {U} or {B}` — ganzer Preis ist das eigene Tapsymbol,
   also tappt der Helfer sie mit, und die Aktivierung danach wird für ein
   bereits getapptes Land abgelehnt. Das liest sich wie ein Kartendefekt und
   ist keiner: `tap_all_mana_but(&mut engine, seat, Some(<slug>()))`.
18. **Eine erzeugbare Farbe ist keine Wahl.** „Add one mana of any type that
   a land you control could produce" fragt nur, wenn mehr als eine Sorte
   erzeugbar ist: über zwei Wäldern hat die Engine eine Antwort und fragt
   nichts. Wer dort `Pending::ChooseColor` erwartet, beschreibt ein Brett,
   das er nicht gebaut hat — Wald **und** Insel hinstellen.
19. **Schwebendes Mana wird mitgezählt.** `tap_mana_except` und
   `tap_all_mana` lassen Mana im Pool stehen, also liest „genau vier
   schwarze" danach fünf. Ist der ganze Preis `{T}` plus ein Opfer, tappe
   **gar nichts** vorher — der leere Pool macht „vier und sonst nichts" erst
   zu einer exakten Aussage.

Antworte mit **genau einem** ```rust-Block: der Kartengriff und die eine
`#[test]`-Funktion. Kein weiterer Text.

**Alle Kommentare, Doc-Kommentare und `assert!`-Meldungen im Rust-Code sind auf Englisch.** Dieses Repository ist durchgehend englisch geschrieben; dieser Prompt ist die einzige deutsche Datei in der Kette, und eine deutsche Zeile im Baum ist eine, die jemand von Hand übersetzen muss.
