# Open items (c41/play-cleanup, 2026-10-05)

What the eight quick commits on fea144e0 left open after the cleanup. Each
item names who it is for.

## Engine lane (baylee-engine; not touched on this branch)

1. **Miracle: a declined or short payment window does not give the mana
   back.** The client now pays a miracle's window in one press ("Pay (tap
   N)" taps what is owed, then passes) or declines it ("Don't pay").
   Declining, or passing short, ends in
   `crates/baylee-engine/src/engine/cast_wizard.rs:1925`
   (`finish_miracle_payment`): `finish_cast` fails, `finish_nested_cast`
   runs, and the lands tapped in the window stay tapped with their mana
   floating (it empties at the step's end). The owner asked for the taps to
   be rolled back. The rules ground is the reversal of an illegal casting
   (Comprehensive Rules, "Handling Illegal Actions": the mana abilities
   activated during it are reversed too; look the number up, sections
   renumber). Wanted: when the window closes without a cast, untap what
   was tapped for it since `cast_or_make_miracle_mana`
   (`cast_wizard.rs:1873`) opened it and take back the mana it made. The
   same holds for `PaymentContinuation::Cast` in `close_mana_window`
   (`crates/baylee-engine/src/engine/actions.rs:1826`).
2. **"Sent to the engine only on completion"** can only be met as far as
   the protocol allows: every mana ability is its own action in a CR 605.3a
   window, so the client sends each tap and then the pass. If the owner
   wants one atomic answer, the engine needs an action like
   `PayWith { sources }` (the taps and the settle in one apply), which
   would also make item 1 unnecessary for the client's own path.
3. **A real miracle test.** `combo_tests/miracle_bug.rs` was an empty,
   unwired stub and is deleted. Wanted in `combo_tests/` or
   `card_tests/`: a miracle card drawn on an opponent's turn, "yes", the
   window opened with `PlayerView::owed`, one manual tap, the rest paid,
   the pass casting it; and a decline leaving the card in hand (and, after
   item 1, the lands untapped).
4. **Teferi, Time Raveler's +1: no engine defect.** `casting::timing_allows`
   (`crates/baylee-engine/src/casting.rs:1145`) already lets the
   controller's sorceries through and lets the opponent's static win;
   its tests are at `casting.rs` around line 2744. The defect was the
   client's: `timing::allows` did not know the +1, so no sorcery lit off
   the player's own main phase. The view now carries
   `PlayerView::sorceries_have_flash` (defaulted, so `VIEW_VERSION` stays
   53, per `docs/protocol.md`'s rule for additive fields) and the client
   reads it.

## Owner decisions

5. **Spend caps were switched off** in `crates/baylee-seat/src/llm.rs`
   (`Tally::spent_under`, `Tally::hold`: "Budget limit disabled per user
   request"). Restored, because the bridge's caps are documented and
   tested and a settings file without caps already has none. If unlimited
   play is wanted, leave the caps out of `llm-seat.json` rather than the
   code.
6. **One CLI process per turn** is restored (`cli.rs`, `CliMind::start`);
   89f8a1d2 kept one process across turns for prompt caching, against
   `docs/llm-seat.md` §"One process per turn" and its tests. Caching across
   turns needs its own design (conversation budget, the notes a new turn
   is told) before it comes back.
7. **The AI log reaches a teammate only while the AI seat shares its hand
   with them** (`Session::shows_hand`; `docs/protocol.md` §"An AI seat's
   reasoning"). Nothing in `baylee-seat` sends `SeatSetting::ShareHand`
   yet, so today nothing is delivered; a bridge that should be watched by
   its teammate has to share its hand. Watching an *opponent's* LLM (the
   dev-table case) stays the bridge's own transcripts
   (`target/seat-transcripts/`), never the table. Residual: a teammate
   shown the hand also reads whatever else the model mentions (a scry).

## Client

8. **The remaining cost is not drawn as a remainder.** In a payment window
   the pool strip shows the owed total beside the floating pips, and the
   pay button says how many lands are left to tap ("Pay (tap
   N)"); the plan is for the remainder, so hand-tapped mana is counted.
   A pip row of what is still owed (owed minus pool, with hybrid and
   restricted mana) is not written.
9. **Not checked live.** The miracle window, the overload chooser and the
   AI log panel were tested in client-core and `baylee-client` unit tests,
   not through dev-control against a running client.
