//! `cards/artifacts/mv_0/mana_crypt.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::event::GameEvent;

fn mana_crypt() -> CardIndex {
    card_index("2c63e4e1-89d2-4bc6-a232-94e75c4b1c8a")
}

/// Every flip the journal holds, as (who, won).
fn flips(engine: &Engine<RegistryLookup>) -> Vec<(PlayerId, bool)> {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter_map(|e| match e.event {
            GameEvent::CoinFlipped { player, won } => Some((player, won)),
            _ => None,
        })
        .collect()
}

/// A game with the Crypt on seat 0's side, run until its upkeep trigger has
/// flipped and resolved.
fn after_the_upkeep_flip(seed: u64) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &[mana_crypt()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| !flips(e).is_empty() && stack_is_empty(e));
    engine
}

/// Mana Crypt (2c63e4e1-89d2-4bc6-a232-94e75c4b1c8a) — "At the beginning of
/// your upkeep, flip a coin. If you lose the flip, this artifact deals 3
/// damage to you."
///
/// The coin comes from the game's seeded stream, so which seeds win is not
/// something to hard-code: forty games are played and each is judged by its
/// own journal. A won flip costs nothing, a lost flip costs exactly 3, and
/// both outcomes turn up in the sweep (a sweep that only ever won would
/// prove nothing about the damage).
#[test]
fn mana_crypt_flips_at_upkeep_and_deals_3_only_on_a_lost_flip() {
    let p0 = PlayerId::new(0);
    let (mut won_seen, mut lost_seen) = (false, false);
    for seed in 1..=40 {
        let engine = after_the_upkeep_flip(seed);
        let flipped = flips(&engine);
        assert_eq!(
            flipped.len(),
            1,
            "seed {seed}: one flip at the one upkeep, got {flipped:?}"
        );
        let (flipper, won) = flipped[0];
        assert_eq!(flipper, p0, "seed {seed}: the Crypt's controller flips");
        let life = engine.state().players[0].life;
        if won {
            won_seen = true;
            assert_eq!(life, 20, "seed {seed}: a won flip costs nothing");
        } else {
            lost_seen = true;
            assert_eq!(life, 17, "seed {seed}: a lost flip is 3 damage");
        }
        assert!(
            on_battlefield(&engine, p0, mana_crypt()).is_some(),
            "seed {seed}: the Crypt stays either way"
        );
    }
    assert!(won_seen, "some seed won the flip");
    assert!(lost_seen, "some seed lost the flip");
}

/// The trigger is "your upkeep": the opponent's upkeep flips nothing. Exactly
/// one flip has happened by the time the second player's turn comes round.
#[test]
fn mana_crypt_does_not_flip_in_the_opponents_upkeep() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mana_crypt()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| !flips(e).is_empty() && stack_is_empty(e));
    // Walk to the other seat's main phase and through the rest of its turn:
    // the Crypt must not flip again there.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        flips(&engine).len(),
        1,
        "the opponent's upkeep did not flip the Crypt"
    );
}

/// "{T}: Add {C}{C}." The mana ability taps the Crypt and puts two colorless
/// in the pool without using the stack.
#[test]
fn mana_crypt_taps_for_two_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mana_crypt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);
    let crypt = on_battlefield(&engine, p0, mana_crypt()).expect("the Crypt is out");
    assert!(!is_tapped(&engine, crypt));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    assert_eq!(
        tap_all_mana(&mut engine, p0),
        1,
        "one mana route: the Crypt"
    );
    assert!(is_tapped(&engine, crypt));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2,
        "{{C}}{{C}}"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert!(
        stack_is_empty(&engine),
        "a mana ability does not use the stack"
    );
}
