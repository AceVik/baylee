# Kartentests in eigene Crates, durch ein Werkzeug

Stand 6. November 2026. Noch nichts davon ist gebaut.

## Warum

Eine geänderte Karte kostet im täglichen Zyklus (`cargo test -p baylee-engine --lib --no-run`)
56 s, gemessen auf dem M1 Max, etwa ±20 %. Rund 45 s davon sind der Test-Crate der Engine: Die
Kartentests sind Unit-Tests in `engine::card_tests`, also ein Kompilat aus der ganzen Engine und
allen Tests, und es hängt an `baylee-cards`. Die Tests sind 41 von 51 MB im Quellcode der Engine:
6.153 Dateien, 14.323 Tests. Der Schritt läuft seriell und nutzt die Kerne nicht.

## Der Weg

Niemand baut das von Hand um. Ein deterministisches Rust-Werkzeug in `xtask` macht den Umbau und
lässt sich auf jedem Stand wiederholen.

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

## Reihenfolge

1. **Jetzt, ohne Eile:** Das Werkzeug entsteht auf einem eigenen Zweig und wird an einer Kopie
   erprobt. Es ändert im Repository nichts außer sich selbst. Dabei zeigt sich die Liste aus
   Schritt 3 und die echte Zeit.
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
