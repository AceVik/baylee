//! `cards/artifacts/mv_4/urza_s_filter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn urza_s_filter() -> CardIndex {
    card_index("46484242-28ed-43d2-b33a-1e5c1771acca")
}

fn flowstone_charger() -> CardIndex {
    card_index("a19e0a7f-d356-4d2c-8414-70a40ce18674")
}

fn watchwolf() -> CardIndex {
    card_index("a35c2e20-eb90-4132-b65f-be1fcb569819")
}

/// Urza's Filter (46484242-28ed-43d2-b33a-1e5c1771acca) — {4} Artifact:
/// "Multicolored spells cost {2} less to cast."
///
/// Reduces generic cost of multicolored spells by {2} for BOTH players
/// (Flowstone Charger {2}{R}{W} -> {R}{W}), does not reduce colored mana
/// (Watchwolf {G}{W}), and does not discount monocolored (Hill Giant {3}{R})
/// or colorless (Millstone {2}) spells.
#[test]
fn urza_s_filter_reduces_multicolored_spells_for_both_players_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[urza_s_filter(), mountain(), plains()])
        .hand(
            0,
            &[flowstone_charger(), watchwolf(), hill_giant(), millstone()],
        )
        .battlefield(1, &[mountain(), plains()])
        .hand(1, &[flowstone_charger()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let charger = in_hand(&engine, p0, flowstone_charger()).unwrap();
    let wolf = in_hand(&engine, p0, watchwolf()).unwrap();
    let giant = in_hand(&engine, p0, hill_giant()).unwrap();
    let stone = in_hand(&engine, p0, millstone()).unwrap();

    // With 0 mana floating: colorless Millstone ({2}) is NOT discounted to {0}.
    assert!(
        !priority_offer(&engine).castable.contains(&stone),
        "colorless spells do not get Urza's Filter's reduction"
    );

    // Float {R}{W}: multicolored Flowstone Charger ({2}{R}{W}) drops to {R}{W};
    // monocolored Hill Giant ({3}{R}) and {G}{W} Watchwolf remain uncastable.
    tap_all_mana(&mut engine, p0);
    let castable = &priority_offer(&engine).castable;
    assert!(
        castable.contains(&charger),
        "multicolored {{2}}{{R}}{{W}} costs {{R}}{{W}} with Urza's Filter"
    );
    assert!(
        !castable.contains(&giant),
        "monocolored {{3}}{{R}} is NOT reduced to {{1}}{{R}}"
    );
    assert!(
        !castable.contains(&wolf),
        "colored pips ({{G}}{{W}}) are not reduced"
    );

    cast_with_floating(&mut engine, p0, flowstone_charger());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, flowstone_charger()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    // Symmetric check: opponent also casts Flowstone Charger ({2}{R}{W}) off {R}{W}.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, flowstone_charger());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, flowstone_charger()).is_some(),
        "Urza's Filter reduces multicolored spells for the opponent too"
    );
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
}
