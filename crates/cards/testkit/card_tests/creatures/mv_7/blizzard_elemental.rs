//! `cards/creatures/mv_7/blizzard_elemental.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blizzard Elemental prints a 5/5 flying body for {5}{U}{U} and one activated
/// line: "{3}{U}: Untap this creature." An untap is invisible on a creature
/// that is already standing, so the Elemental is seated before the game starts
/// (nothing summoning sick) and sent at the opponent — attacking is the only
/// thing on this board that turns it sideways, and it also puts the printed
/// 5/5 body to work. The two readings that carry the ability are the offer and
/// the price: with an empty pool the line is absent from `legal.abilities`,
/// because `can_afford` reads the mana pool and not four untapped Islands, and
/// once the Islands are tapped the whole four leave the pool as the Elemental
/// stands back up.
#[test]
fn blizzard_elemental_taps_for_three_and_a_blue_to_stand_itself_back_up() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[blizzard_elemental(), island(), island(), island(), island()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elemental =
        on_battlefield(&engine, p0, blizzard_elemental()).expect("the Elemental is on the table");
    assert_eq!(pt(&engine, elemental), (5, 5), "the body the card prints");
    assert!(
        keywords(&engine, elemental).contains(KeywordSet::FLYING),
        "and the printed flying reaches the permanent"
    );

    // The price is read off the *pool*, not off the four untapped Islands:
    // while nothing floats, the {3}{U} is unpayable and the line is absent from
    // the offer rather than refused by it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(elemental, 0)),
        "an empty pool pays no {{3}}{{U}}, so the untap is not offered at all: {:?}",
        legal.abilities
    );

    // The only thing here that taps a creature is attacking, and an untapped
    // 5/5 flier is what the printed body is for.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        attackers.contains(&elemental),
        "an untapped 5/5 flier may attack: {attackers:?}"
    );
    engine
        .apply(
            player,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elemental, Defender::Player(p1))],
            },
        )
        .expect("the attacker came out of the list that offered it");
    pass_until(&mut engine, |e| at_rest(e, p0) && is_tapped(e, elemental));
    assert!(
        is_tapped(&engine, elemental),
        "declaring it as an attacker is what turned it sideways"
    );

    // Mana before the claim: four Islands fill the pool, and the whole price of
    // the ability is mana — the Elemental prints no `{T}: Add …` that
    // `tap_all_mana` would have spent instead (#159).
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Islands, and the Elemental contributed nothing to the pool"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(elemental, 0)),
        "with {{3}}{{U}} floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, blizzard_elemental(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{U}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "untapping is no mana ability, so the ability is waiting on the stack"
    );
    assert!(
        is_tapped(&engine, elemental),
        "and nothing has happened yet: the Elemental stands up on resolution"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, elemental),
        "\"Untap this creature\" — the same 5/5, on its feet again"
    );
}
