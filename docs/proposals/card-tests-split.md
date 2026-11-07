# Kartentests in eigene Crates, durch ein Werkzeug

Stand 6. November 2026. Schritte 1 und 2 sind am 7. November 2026 gebaut (siehe §"Ergebnis"),
Schritt 3, das Regelnummern-Werkzeug, steht noch aus.

## Warum

Eine geänderte Karte kostet im täglichen Zyklus (`cargo test -p baylee-engine --lib --no-run`)
56 s, gemessen auf dem M1 Max, etwa ±20 %. Rund 45 s davon sind der Test-Crate der Engine: Die
Kartentests sind Unit-Tests in `engine::card_tests`, also ein Kompilat aus der ganzen Engine und
allen Tests, und es hängt an `baylee-cards`. Die Tests sind 41 von 51 MB im Quellcode der Engine:
6.153 Dateien, 14.323 Tests. Der Schritt läuft seriell und nutzt die Kerne nicht.

## Der Weg

Niemand baut das von Hand um. Ein deterministisches Rust-Werkzeug macht den Umbau und lässt sich
auf jedem Stand wiederholen. Es ist Wegwerfcode (Eigentümer, 6. November): Es liegt außerhalb des
Repositorys und wird nach dem Lauf gelöscht. Ins Repository kommt nur sein Ergebnis.

1. Es erzeugt etwa 16 Test-Crates nach den Ordnern von `card_tests`. Die Testdateien bleiben liegen
   und werden über `#[path]` eingebunden.
2. Es schreibt Pfade mechanisch um (`crate::…` zu `baylee_engine::…`, die Hilfsmodule wie
   `legends_kit` in einen gemeinsamen Test-Crate).
3. Engine-Interna, die Tests brauchen, kommen hinter das vorhandene `test-support`-Feature. Die
   Liste liefert der Compiler; das ist der einzige Teil, der nicht rein mechanisch ist.
4. Es prüft am Ende: dieselben Testnamen, 14.323 oder der dann aktuelle Stand, keiner fehlt.

Die Engine hängt dann im Testlauf nicht mehr an den Karten, und die Test-Crates bauen parallel.
Geschätzt, nicht gemessen: 56 s werden 10 bis 20 s. Eine reine Teständerung baut nur noch einen
Crate. Eine Kartenänderung baut weiter alle Test-Crates, weil sie an `baylee-cards` hängen.

## Was das Gate wirklich kostet

Der Schritt `test` im Gate vom 6. November dauerte 949 s: 233 s Bauen, 702 s Laufen. Der Umbau hilft
also vor allem dem Bauen; das Laufen entscheiden wenige langsame Tests, und das ist billiger zu
beheben und kommt deshalb zuerst.

- `streamed_frames_match_collecting_conversion_with_bounded_memory` lief allein 630 s und bestimmt
  damit fast die ganze Laufzeit.
- Die Bench-Ziele laufen als Tests mit: `baylee-ai` 1290 s, `baylee-engine` 464 s,
  `baylee-client-core` 265 s Testzeit.
- Rund zwanzig weitere Tests liegen über 100 s (`duel_flow`, `sound`, `attempt_ledger`,
  `every_implemented_card_survives_a_game_*`).

Die Zeiten stammen von einem belasteten Rechner. Ein einzelner Kartentest brauchte im Leerlauf
0,04 s, im Gate im Mittel 0,16 s.

## Reihenfolge

0. **Zuerst:** die langsamen Tests und Bench-Ziele oben, ohne Abdeckung zu verlieren.
1. **Jetzt, ohne Eile:** Das Werkzeug entsteht außerhalb des Repositorys und wird an einer Kopie
   erprobt. Dabei zeigt sich die Liste aus Schritt 3 und die echte Zeit.
2. **Wenn die laufende Kartenarbeit gemergt ist:** Kurzer Stopp für neue Karten, Werkzeug auf dem
   aktuellen Stand laufen lassen, Gates, Landen. Danach schreibt `docs/card-agent-brief.md` vor, wo
   ein neuer Kartentest liegt.
3. **Im selben Stopp:** Das Regelnummern-Werkzeug läuft mit. Die Fassung vom 6. November 2026 hat
   713 Meld Cards eingefügt: 566 Verweise rücken eins hoch, bis zu 179 in CR 712 brauchen eine
   Zuordnung je Regel. Beide Läufe berühren viele Dateien und sollen nur einmal stören.

## Offen

- Wie lang die Liste aus Schritt 3 wird.
- Ob 16 die richtige Zahl ist. Das misst das Werkzeug an der Kopie.
- Ob Kartendateien später ebenso geteilt werden (`baylee-cards` in Shards, kalt 72 s auf
  geschätzt 20 s). Das ist ein eigener Schritt und läuft über die Codegen.

## Ergebnis (7. November 2026)

Das Werkzeug lag außerhalb des Repositorys, wurde auf Kopien von `main` erprobt und lief einmal auf
dem Stand, der gelandet ist. Was jetzt gilt, steht in `docs/cards.md` §"Where a card test compiles".

- **17 Test-Crates statt 16**, dazu `baylee-card-tests-verification` und das gemeinsame
  `baylee-card-testkit`. Nach Ordnern geschnitten wären es sieben ungleiche Crates gewesen: Alle
  Kreaturentests in einem Crate kamen auf bigace in neun Minuten nicht durch `cargo check`,
  53.000 Zeilen Hexereien in unter einer. Das Werkzeug packt deshalb die Kinder eines Ordners in
  Crates von höchstens etwa 55.000 Zeilen.
- **Dieselben Tests:** alle 16.934 Testnamen des alten Engine-Testprogramms finden sich wieder,
  keiner fehlt. Fünf Kartentests, die einen Helfer der Engine-Tests brauchten, liegen jetzt bei
  diesem Helfer (`mechanics_tests::untap_scope`). 27 Tests laufen doppelt: die Subjekte der
  Prüf-Hooks, die das Verifikations-Crate unter kürzeren Namen ein zweites Mal kompiliert.
- **Die Liste aus Schritt 3:** 118 Stellen der Engine, alle vom Compiler genannt. Felder, Module
  und Re-Exporte sind nur unter `test-support` offen, Funktionen und Typen `#[doc(hidden)] pub`;
  ohne das Feature erreicht man mit ihnen keinen Zustand. Das Feature erreicht keinen
  ausgelieferten Build, `cargo dev gate shipped-features` prüft das.
- **Gemessen auf dem MacBook unter Last** (Last 7 bis 8, nacheinander auf derselben Maschine):

  | Änderung | vorher | nachher |
  |---|---|---|
  | eine Testdatei | 42 s | 3 s |
  | eine Karte, ihr Crate | 52 s | 15 s |
  | eine Karte, alle Crates | 52 s | 28 s |

  Alle Kartentests bauen in 95 s und laufen in 31 s.
