#[allow(clippy::wildcard_imports)] // shared integration fixture vocabulary
use super::*;

/// **The tripwire fired on 20.09.2026 and this is what is left of it.**
///
/// It used to assert a zero. The counter-clock rows of #73 were answered by
/// two `baylee-ai` unit tests and by no game, because no card in this pool
/// could be *made* to put a lore or a time counter anywhere: both clocks
/// were advanced by the engine itself (`progress.rs`), never by an effect a
/// player activates. Its own message said what the day a card arrived would
/// owe — the same decision reachable in a real game, and that game as a
/// test.
///
/// Trenzalore Clocktower is that card: `{T}: Add {U}. Put a time counter on
/// Trenzalore Clocktower.` **#166 was opened on the prediction that
/// `clock_score` would score it backwards, and that prediction is wrong.**
/// The `Time` arm is written for suspend, and it says so before it reads
/// anything: it asks whether the card underneath prints `Suspend` and
/// answers 0 when it does not, which is the same refusal it already gives
/// vanishing. `a_time_counter_is_not_a_delay_on_a_card_that_is_not_counting_down`
/// pins it with the deciding shape — the Clocktower carrying **three**
/// counters against a suspended card's **four**, so a rule reading the count
/// alone would take the Clocktower and this one does not.
///
/// It is not reached at all, either. The counter is inside the mana ability
/// and has no target, so `targets` — the only caller of `clock_score` — is
/// never consulted about it. Both halves had to be measured, because either
/// one alone would have been the wrong reason.
///
/// **So the list stays, and deleting it is no longer part of closing #166.**
/// The ticket said to delete it once the game was written, on the premise
/// that this card made the decision reachable; it does not, and a tripwire
/// that has not yet caught what it watches for is not one to take down. What
/// it is now waiting for is narrower and worth saying: a lore or time
/// counter effect that **targets**, which is the only shape that reaches the
/// rule. The played game the ticket asked for exists —
/// `a_mana_land_that_also_counts_pays_for_the_spell` in
/// `ai_decisions/mana.rs` — and now proves that the planner taps this land
/// and casts the spell despite the mana ability's counter rider (#170).
///
/// The zero is a **pinned list** now, the same shape as
/// `LANDS_THAT_WOULD_COUNT_THEMSELVES`: it names what is reachable, it is
/// red the day a second card joins, and it carries the ticket. Relaxing it
/// is not on the table, and neither is adding a name to the list without
/// reading, for that name, the two questions asked above — does anything
/// target it, and what does the arm answer.
///
/// Time Bomb joined on 14.10.2026. Its upkeep puts a time counter on
/// itself without targeting, and its sacrifice damages each creature and
/// player without targeting. It prints no `Suspend`, so `clock_score` would
/// answer 0 for it as well. `time_bomb_counts_its_own_upkeeps_and_uses_sacrificed_source_counters_for_damage`
/// plays both abilities, including the departed source's counter count.
/// `the_ai_declines_time_bombs_unread_sweep_even_with_mana_and_enemy_creatures`
/// pins the actual AI limitation: the offered counter-dependent sweep is
/// unread, even with mana floating. This arrival does not close #166 or
/// make a targeted clock effect reachable.
///
/// Infinite Hourglass is held back after the Argon review (23.10.2026):
/// its earlier counter manipulation omitted the printed creature-power
/// payoff. The canonical stub has no abilities and therefore no reachable
/// clock effect or meaningful card-specific AI regression. Reassess it
/// here when the full card is implemented; see
/// `docs/history/2026-10-23-argon-review.md`.
///
/// The two assertions above the list are what make it mean something.
/// `signed` proves the walk reaches real `AddCounter` effects at all — the
/// pool is full of +1/+1 — and `seen` is the population it read, so a walker
/// blinded by a refactor fails here instead of reporting a quiet nought.
#[test]
fn only_the_named_cards_put_a_lore_or_time_counter_on_anything() {
    // Named by the card that puts them, so another arrival is a name in a
    // failure message and not a number to bump.
    const REACHABLE_CLOCKS: &[&str] = &["Time Bomb", "Trenzalore Clocktower"];
    let mut seen = 0;
    let mut kinds = Vec::new();
    for def in baylee_cards::all() {
        for effects in abilities(def).flat_map(ability_effects) {
            counters_put(effects, &mut seen, &mut kinds);
        }
    }
    let mut clocks: Vec<&str> = Vec::new();
    for def in baylee_cards::all() {
        for effects in abilities(def).flat_map(ability_effects) {
            let mut here = 0;
            let mut mine = Vec::new();
            counters_put(effects, &mut here, &mut mine);
            if mine
                .iter()
                .any(|kind| matches!(kind, CounterKind::Lore | CounterKind::Time))
            {
                clocks.push(def.faces[0].name);
            }
        }
    }
    clocks.sort_unstable();
    clocks.dedup();
    let signed = kinds
        .iter()
        .filter(|kind| matches!(kind, CounterKind::Plus { .. } | CounterKind::Minus { .. }))
        .count();

    // 2731 on 19.09.2026. The floor is the population and not the number,
    // so a card added or removed is not news and a walker that stopped
    // descending is.
    assert!(
        seen > 2_000,
        "the effect walk visited {seen} effects, which is too few to have \
         read this pool: the nesting shapes it follows have gone stale"
    );
    assert!(
        signed > 0,
        "the walk found no +1/+1 or -1/-1 counter in the whole pool, so it \
         is not reaching `AddCounter` at all and the zero below is empty"
    );
    assert_eq!(
        clocks, REACHABLE_CLOCKS,
        "the set of pool cards that put a lore or time counter on something \
         has changed. Check whether its counter effect targets and whether \
         `tactics::clock_score` reads it as a clock, then write its real \
         AI/game regression before extending this list (#166)."
    );
}
