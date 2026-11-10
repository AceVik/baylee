//! `cards/instants/mv_3/liquify.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

#[test]
fn liquify_counters_spell_with_mana_value_three_or_less_to_exile_and_refuses_mv_four() {
    let card = card_index("dc9ae094-139f-4f28-9d4e-4d6def765744");
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[island(), island(), island()])
        .hand(0, &[index::HILL_GIANT, grizzly_bears()])
        .hand(1, &[card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Cast Hill Giant (MV 4): Liquify cannot target it.
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, index::HILL_GIANT);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, P1);
    engine.refresh_offer();
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority for P1");
    };
    let liquify_in_hand = in_hand(&engine, P1, card).unwrap();
    assert!(
        !legal.castable.contains(&liquify_in_hand),
        "Liquify cannot target a spell with mana value 4"
    );

    // Let Hill Giant resolve, then cast Grizzly Bears (MV 2).
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, index::HILL_GIANT).is_some());

    cast_with_floating(&mut engine, P0, grizzly_bears());
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    let spell = engine.state().zones.list(ZoneLocation::Stack)[0];
    cast_with_floating(&mut engine, P1, card);
    aim_at(&mut engine, P1, spell);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, P0, grizzly_bears()).is_none());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(P0))
            .iter()
            .any(|&id| {
                engine.state().object(id).unwrap().card.map(|c| c.index) == Some(grizzly_bears())
            })
    );
}
