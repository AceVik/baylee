//! `cards/creatures/mv_2/theorist_s_proxy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Theorist's Proxy — {1}{U}, a 0/3 Illusion — prints flash, "When this
/// creature enters, empower Jace 3." and "{U}, Sacrifice this creature: The
/// next spell you cast this turn can't be countered."
///
/// Flash is the written half, and p0's own end step is the board that reads
/// it: five tapped Islands pay for either spell in hand, so the only thing
/// separating the 0/3 from the {2}{U} Aether Channeler beside it is that one
/// of them may be cast once the main phases are gone. The other two lines are
/// the `Coverage::Partial` gap and are read as absences — no Jace token
/// arrives with it, and the sacrifice line stays unoffered with the creature
/// on the table and the {U} it costs still floating in the pool.
#[test]
fn theorist_s_proxy_flashes_in_after_the_main_phase_and_offers_neither_of_its_other_lines() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[theorist_s_proxy(), aether_channeler()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Out of the main phases and into the end step, which is instant timing:
    // sorcery speed ends with the postcombat main, and the cleanup the
    // nine-card opening hand owes lies beyond the priority asked here.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    // Mana first: `castable` is verified against the pool, and five blue
    // cover both spells at once.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let card = in_hand(&engine, p0, theorist_s_proxy()).expect("the Proxy is in hand");
    let channeler = in_hand(&engine, p0, aether_channeler()).expect("the Channeler is in hand");
    assert!(
        legal.castable.contains(&card),
        "flash: a creature may be cast with no main phase open, and the pool \
         covers it: {:?}",
        legal.castable
    );
    assert!(
        !legal.castable.contains(&channeler),
        "the same five blue buy the Channeler's {{2}}{{U}} and it is still \
         not offered, so what keeps it off this list is the timing and not \
         the mana: {:?}",
        legal.castable
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("{1}{U} out of the pool pays for the flashed creature");
    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    let proxy = on_battlefield(&engine, p0, theorist_s_proxy()).expect("the Proxy resolved");
    assert_eq!(pt(&engine, proxy), (0, 3), "the body the card prints");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "empower is the `Coverage::Partial` gap: `crate::tokens` holds no Jace \
         planeswalker token, so nothing arrives beside it"
    );

    // The second gap, with its cost payable: three of the five Islands' blue
    // are left over, and the creature the sacrifice would eat is the Proxy.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the Proxy's controller holds priority again: {:?}",
            engine.pending()
        )
    };
    assert!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue)
            >= 1,
        "the {{U}} the sacrifice asks for is right there in the pool"
    );
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == proxy),
        "and `{{U}}, Sacrifice this creature` is offered nowhere all the same: \
         that line is not written at all: {:?}",
        legal.abilities
    );
}
