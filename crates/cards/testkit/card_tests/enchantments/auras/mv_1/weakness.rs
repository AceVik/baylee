//! `cards/enchantments/auras/mv_1/weakness.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Weakness — {B}, Aura: "Enchant creature. Enchanted creature gets -2/-1."
///
/// The two printed sentences part company on exactly one question: *which*
/// creature. "Enchant creature" carries no controller restriction, so the
/// offer is expected to name both sides of the table (CR 303.4a), while the
/// static's `Filter::AttachedToBySource` may name only the one the Aura is
/// holding. Both are read in one cast: the opponent's 7/5 takes the -2/-1 and
/// becomes a 5/4, and the Llanowar Elves under Weakness' own controller is
/// still the 1/1 it was printed as — the bystander is what tells a modifier
/// that lost its filter from one that never left the host. The host survives
/// the shrink on purpose, so the numbers are readable where they landed
/// rather than inferred from a graveyard.
#[test]
fn weakness_shrinks_the_creature_it_enchants_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), llanowar_elves()])
        .hand(0, &[weakness()])
        .battlefield(1, &[a_seven_five_wurm()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let bystander = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let host = on_battlefield(&engine, p1, a_seven_five_wurm()).expect("their Wurm is out");
    assert_eq!(pt(&engine, host), (7, 5), "the body the card prints");
    assert_eq!(pt(&engine, bystander), (1, 1), "and the bystander's");

    // Mana first, then the claim: the offer is read off the pool. The Elves'
    // own `{T}` is a mana ability `tap_all_mana` takes as well, so nothing
    // here counts the pool — the Swamp is what pays the {B}.
    tap_all_mana(&mut engine, p0);
    let spell = in_hand(&engine, p0, weakness()).expect("the Aura is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "an Aura with a creature on the table is castable: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, weakness());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but a target choice")
    };
    assert_eq!(player, p0, "the caster chooses what it enchants");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "\"enchant creature\" names both creatures on the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Wurm was one of the options it published");

    pay_life_ward(&mut engine, p0, 7);
    pass_until(&mut engine, |e| on_battlefield(e, p0, weakness()).is_some());
    let aura = on_battlefield(&engine, p0, weakness()).expect("the Aura resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature its target became"
    );
    assert_eq!(
        pt(&engine, host),
        (5, 4),
        "-2/-1 on the creature it enchants: 7/5 becomes 5/4"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "and the creature it does not — the static reads the source's own \
         attachment and never the whole table"
    );
}
