//! `cards/lands/pain/ancient_tomb.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ancient Tomb prints one line — "{T}: Add {C}{C}. This land deals 2 damage
/// to you." — and the two halves are one activation, which is what makes
/// reading the card useless: a tap that produced the pair and skipped the
/// price reads correctly off the pool and wrongly off a life total. The
/// scenario is written so the pool has exactly one possible source (the
/// Tomb, played as a land drop on an otherwise empty board), and the life is
/// read twice: still twenty after the land arrives, eighteen after the tap —
/// which is what tells a price paid on activation from one paid on entry.
/// The mana ability uses no stack (CR 605.3b), so both happen the moment the
/// answer is applied, and "you" is pinned against the opponent's untouched
/// twenty.
#[test]
fn ancient_tomb_taps_for_two_colorless_and_charges_its_controller_two_life() {
    let p0 = PlayerId::new(0);
    let _p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[ancient_tomb()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Played and not seeded: a land drop is how a land arrives, and this one
    // has to be on the battlefield before its {T} means anything.
    let tomb = play_land(&mut engine, p0, ancient_tomb());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing else on this board makes mana, so the pool is empty and the \
         two below can only come off the Tomb"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "entering is not the price: the land is on the table and nobody has \
         paid anything yet"
    );

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "the land drop hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that played the land");
    assert!(
        legal.abilities.contains(&(tomb, 0)),
        "the Tomb prints its own {{T}}: Add {{C}}{{C}}, so it is an ordinary \
         entry in `abilities` rather than the CR 305.6 shortcut: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, ancient_tomb(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        2,
        "{{C}}{{C}} off the one tap"
    );
    assert_eq!(pool.total(), 2, "one tap, two mana and nothing else");
    assert_eq!(
        engine.state().players[0].life,
        18,
        "\"this land deals 2 damage to you\" — the price lands on the seat \
         that tapped it, off the same activation that made the mana"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and \"you\" is its controller, not the table: a reading that hit the \
         opponent instead would leave this side at twenty"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana and the damage \
         are already done"
    );
    assert!(is_tapped(&engine, tomb), "{{T}} was the whole cost");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
}
