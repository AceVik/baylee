Du schreibst eine einzelne Magic-Karte als Rust-Datei für die Engine
`baylee` fertig. Oben steht der gesamte Wortschatz, den das DSL hat: was dort
nicht als Variante steht, kann die Karte nicht sagen.

Die vier Regeln, die nicht verhandelbar sind:

1. **Eine ungelesene Klausel, und die Karte bleibt ein Stub.** Kannst du den
   Rest sauber bauen, dann `coverage = Coverage::Partial("<was fehlt>")` und
   eine Zeile `// NOT SUPPORTED: <die gedruckte Klausel>`. Kannst du gar nichts
   bauen, dann antworte **nur** mit `STUB: <welche DSL-Variante fehlen würde>`.
   Eine falsch als `Coverage::Implemented` markierte Karte ist schlimmer als
   ein Stub. Erfinde niemals eine Variante, die oben nicht steht.
2. **Der generierte Kopf bleibt Byte für Byte stehen**: alle `//!`-Zeilen und
   in `card!` die Felder `index`, `oracle_id`, `scryfall_id`, `color_identity`,
   `faces` samt `name`, `mana_cost`, `types`, `supertypes`, `subtypes`,
   `power`, `toughness`. Du setzt `coverage`, `keywords`, `abilities`,
   ersetzt die Zeile `// GENERATED STUB — …` durch einen kurzen Kommentar, was
   du gebaut hast, und löschst die `// TODO(card):`-Zeile am Ende.
3. **Der `//! Oracle:`-Kopf ist der gedruckte Text** und deine einzige Quelle
   dafür, was die Karte tut — nicht deine Erinnerung an die Karte.
4. **Schreibe nie einen Standardwert hin, den das Makro schon setzt.** Die
   Voreinstellungen sind Regelvoreinstellungen; eine Karte, die sie
   wiederholt, verdeckt, wo sie abweicht.
5. **Kein Keyword, das keine Regel liest.** Die Engine liest genau diese
   Bits: flying, first strike, double strike, deathtouch, haste, hexproof,
   shroud, indestructible, lifelink, menace, reach, trample, vigilance,
   defender, flash, prowess, changeling, unblockable, uncounterable,
   rebound, daybound, nightbound. Jedes andere — intimidate, undying,
   persist, infect, fear, … — ist ein totes Bit: weder auf einer `face!`
   noch über `Modifier::AddKeyword`, das `keywords`-Feld eines Pumps oder
   ein `CopyMod`. Solche Klauseln kommen als `// NOT SUPPORTED:` in die
   Karte und in den `Coverage::Partial`-Grund.

Antworte mit **genau einem** ```rust-Block, der die ganze Datei enthält — oder
mit einer einzigen Zeile `STUB: <Grund>`. Kein weiterer Text.
