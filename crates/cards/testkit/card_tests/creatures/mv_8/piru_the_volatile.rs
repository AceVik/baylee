//! `cards/creatures/mv_8/piru_the_volatile.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn piru() -> CardIndex {
    card_index("dcd13eab-7ed7-4e95-bdae-d140c1ff84df")
}

fn griselbrand() -> CardIndex {
    card_index("f759d112-76db-4091-a22b-b9f19ab6fa5f")
}

/// Piru is a 7/7 for {2}{R}{R}{W}{W}{B}{B} with flying and lifelink.
#[test]
fn piru_is_cast_for_eight_with_flying_and_lifelink() {
    let mut engine = at_main(
        &[
            mountain(),
            mountain(),
            plains(),
            plains(),
            swamp(),
            swamp(),
            forest(),
            forest(),
        ],
        &[piru()],
    );
    let dragon = cast_and_settle(&mut engine, piru());
    assert_eq!(pt(&engine, dragon), (7, 7));
    assert!(keywords(&engine, dragon).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, dragon).contains(KeywordSet::LIFELINK));
}

/// A board for Piru's upkeep: the Bears of each side, the opponent's
/// Griselbrand (legendary, 7/7), and {R}{W}{B} floating.
fn piru_upkeep_board() -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[piru(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears(), griselbrand()])
        .start();
    keep_mulligans(&mut engine);
    for c in [ManaColor::Red, ManaColor::White, ManaColor::Black] {
        give_mana(&mut engine, p0, c, 1);
    }
    engine
}

/// "At the beginning of your upkeep, sacrifice Piru unless you pay
/// {R}{W}{B}": paying keeps him and spends exactly that mana.
#[test]
fn piru_stays_when_the_upkeep_is_paid() {
    let p0 = PlayerId::new(0);
    let mut engine = piru_upkeep_board();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, piru()).is_some(), "paid, kept");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        [ManaColor::Red, ManaColor::White, ManaColor::Black].map(|c| pool.available(c)),
        [0, 0, 0],
        "{{R}}{{W}}{{B}} spent"
    );
}

/// Declining sacrifices Piru; his death trigger deals 7 to each nonlegendary
/// creature, so both Bears die and the legendary Griselbrand lives.
#[test]
fn piru_dies_and_wipes_the_nonlegendary_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = piru_upkeep_board();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, piru()).is_some(), "sacrificed");
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_none(),
        "own Bears die"
    );
    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "theirs too"
    );
    assert!(
        on_battlefield(&engine, p1, griselbrand()).is_some(),
        "the legendary creature takes nothing"
    );
}
