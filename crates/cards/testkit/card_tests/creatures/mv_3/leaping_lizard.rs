//! `cards/creatures/mv_3/leaping_lizard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Leaping Lizard — {1}{G}{G} 2/3 Lizard: "{1}{G}: This creature gets -0/-1
/// and gains flying until end of turn."
///
/// Both halves of the pump are read off one activation, and each is something
/// the card file alone cannot say happened: the toughness moves (2, 3) →
/// (2, 2) while the power stays, and `KeywordSet::FLYING` appears only once
/// the layers have projected it. The Elf beside it is the control —
/// `Filter::This` is not "creatures you control", so a filter that had widened
/// would have shrunk and lifted the Elf too. The two Forests are named as the
/// printing kept untapped, so the pool the ability empties is exactly the
/// {1}{G} it charges.
#[test]
fn leaping_lizard_trades_toughness_for_flying_and_for_nothing_else() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), leaping_lizard(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lizard = on_battlefield(&engine, p0, leaping_lizard()).expect("the Lizard is out");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert_eq!(
        pt(&engine, lizard),
        (2, 3),
        "a printed 2/3 before anything is asked"
    );
    assert!(
        !keywords(&engine, lizard).contains(KeywordSet::FLYING),
        "and nothing has granted it flying yet"
    );

    // `legal.abilities` is filtered through `can_afford`, and that reads the
    // pool rather than the untapped lands: no {1}{G}, no offer.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(lizard, 0)),
        "with an empty pool the {{1}}{{G}} is unpayable, so the one line the \
         card prints is not offered: {:?}",
        legal.abilities
    );

    // The Elf is named as the printing kept back: it prints its own
    // `{T}: Add {G}` (#159), and a pool of three would make the payment below
    // a claim about mana nothing on this board accounted for.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests tapped, and the Elf left standing"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(lizard, 0)),
        "with {{1}}{{G}} floating the whole price is payable: {:?}",
        legal.abilities
    );

    // Ability 0 is the only ability the card prints.
    activate(&mut engine, p0, leaping_lizard(), 0);
    assert!(
        !stack_is_empty(&engine),
        "shrinking a creature and granting it flying is no mana ability, so it \
         is on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, lizard),
        (2, 2),
        "\"gets -0/-1\": the toughness moves and the power does not"
    );
    assert!(
        keywords(&engine, lizard).contains(KeywordSet::FLYING),
        "and the resolution grants flying, which only the layers can project"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{G}} it charges came out of the pool"
    );

    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "the Elf nothing named never moved"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FLYING),
        "and gained none of the keyword"
    );
    assert!(
        !is_tapped(&engine, lizard),
        "the price is mana and not a tap, so the Lizard may still attack"
    );
}
