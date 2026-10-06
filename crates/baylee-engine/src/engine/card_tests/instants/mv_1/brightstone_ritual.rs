//! `cards/instants/mv_1/brightstone_ritual.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Brightstone Ritual ({R}, Instant): "Add {R} for each Goblin on the
/// battlefield." The count is the whole card, so the board is built to make
/// three readings disagree at once: two Goblins under the caster, one across
/// the table, and an Elf that is a creature but no Goblin. Three red in the
/// pool is the only answer that reads "Goblin" over the *whole* battlefield —
/// two would drop the opponent's Goblin, four would count the Elf — and the
/// Elf is kept untapped for exactly that, since its own `{T}: Add {G}` would
/// otherwise leave green beside the red this test is counting.
#[test]
fn brightstone_ritual_adds_one_red_for_each_goblin_on_the_battlefield() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                festering_goblin(),
                festering_goblin(),
                quiet_creature(),
            ],
        )
        .battlefield(1, &[festering_goblin()])
        .hand(0, &[brightstone_ritual()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The Elf is named as the printing kept back: its whole price is its own
    // {T}, so `tap_all_mana` would have tapped it too and the pool would hold
    // a green that has nothing to do with the card under test.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "one Mountain pays the {{R}}"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        0,
        "and the Elf stayed out of it"
    );

    cast_with_floating(&mut engine, p0, brightstone_ritual());
    assert!(
        !stack_is_empty(&engine),
        "an instant goes on the stack, so the mana it adds is not here yet"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{R}} is spent, so whatever the pool holds next the Ritual put there"
    );
    pass_until(&mut engine, stack_is_empty);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        3,
        "one red for each of the three Goblins on the battlefield — two of \
         this seat's and one of the opponent's"
    );
    assert_eq!(pool.total(), 3, "and nothing else came with it");
    assert!(
        on_battlefield(&engine, p0, festering_goblin()).is_some()
            && on_battlefield(&engine, p1, festering_goblin()).is_some(),
        "the Ritual adds mana and moves no Goblin"
    );
}
