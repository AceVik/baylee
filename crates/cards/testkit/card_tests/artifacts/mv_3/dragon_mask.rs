//! `cards/artifacts/mv_3/dragon_mask.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn dragon_mask() -> CardIndex {
    card_index("fce29dd8-23b1-465c-a977-526f716dc8f6")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn dragon_mask_pumps_your_creature_and_returns_it_to_hand_at_next_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                grizzly_bears(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[dragon_mask()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let their_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, dragon_mask());
    pass_until(&mut engine, stack_is_empty);

    let mask = on_battlefield(&engine, p0, dragon_mask()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mask,
                ability_index: 0,
            },
        )
        .unwrap();
    let menu = aim_at(&mut engine, p0, bear);
    assert!(menu.contains(&elf), "any creature you control");
    assert!(
        !menu.contains(&their_bear),
        "\"target creature you control\": the opponent's Bears is no target"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (4, 4));

    reach_their_main_phase(&mut engine, p1);
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_none());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .iter()
            .any(|&id| {
                engine
                    .state()
                    .object(id)
                    .unwrap()
                    .card
                    .is_some_and(|c| c.index == grizzly_bears())
            })
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "only the target goes back to hand"
    );
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());
    assert_eq!(pt(&engine, elf), (1, 1), "the Elves was never pumped");
}
