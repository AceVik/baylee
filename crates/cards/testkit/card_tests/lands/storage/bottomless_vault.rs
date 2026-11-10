//! `cards/lands/storage/bottomless_vault.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bottomless Vault, whole: `This land enters tapped.` / `You may choose
/// not to untap this land during your untap step.` / `At the beginning of
/// your upkeep, if this land is tapped, put a storage counter on it.` /
/// `{T}, Remove any number of storage counters from this land: Add {B} for
/// each storage counter removed this way.`
///
/// The Fallen Empires storage cycle is the pool's one printing of an
/// intervening-`if` clause on a land, and the whole card is built around
/// the player's answer to CR 502.3's determination: a land left tapped
/// banks a counter at the next upkeep, and a land that untaps does not.
/// Both answers are played here in one game, because either on its own
/// proves nothing — a trigger that never fires passes the second half, and
/// a trigger that ignores its clause passes the first.
///
/// The counters are then spent, which is what says the two halves are one
/// card: the number announced at CR 601.2b is the number of {B} that
/// arrives.
#[test]
fn a_storage_land_banks_a_counter_only_on_the_upkeeps_it_spent_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(884, forest())
        .hand(0, &[bottomless_vault()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, bottomless_vault());
    assert!(
        entered_tapped(&engine, land),
        "Bottomless Vault prints `This land enters tapped`"
    );
    assert_eq!(
        counters_on(&engine, land, counters::STORAGE),
        0,
        "and arrives empty"
    );

    // First own untap step: the land is the one permanent that prints the
    // sentence, and the answer is to leave it tapped.
    let (player, options) = walk_to_the_untap_question(&mut engine);
    assert_eq!(player, p0, "the active player makes the determination");
    assert_eq!(
        options,
        vec![land],
        "the Forests untap without being asked about"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .expect("leaving it tapped is an answer to the question asked");
    on_to_this_turn_s_main(&mut engine, p0);
    assert!(is_tapped(&engine, land), "it stayed tapped");
    assert_eq!(
        counters_on(&engine, land, counters::STORAGE),
        1,
        "so the upkeep trigger's clause was true and it banked one"
    );

    // Second own untap step, answered the other way: the land untaps, the
    // clause is false at the beginning of the upkeep, and nothing is banked.
    let (_, options) = walk_to_the_untap_question(&mut engine);
    assert_eq!(options, vec![land], "the same question, a turn later");
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("untapping it is the other answer");
    on_to_this_turn_s_main(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "it untapped");
    assert_eq!(
        counters_on(&engine, land, counters::STORAGE),
        1,
        "and an untapped land banks nothing: the clause is checked, not \
         assumed"
    );

    // And the counter is spendable, one {B} for one counter.
    let (min, max) = spend_storage(&mut engine, p0, land, 2, 1);
    assert_eq!(
        (min, max),
        (0, 1),
        "`any number` is bounded above by what the land actually carries"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "one storage counter removed this way is one {{B}}"
    );
    assert_eq!(
        counters_on(&engine, land, counters::STORAGE),
        0,
        "and the counter is gone, because removing it was the cost"
    );
}
