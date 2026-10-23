//! `cards/sorceries/mv_3/soul_burn.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn soul_burn() -> CardIndex {
    card_index("063b0f5d-af27-4681-87b9-b553f6887061")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Soul Burn (063b0f5d-af27-4681-87b9-b553f6887061) — {X}{2}{B} Sorcery:
/// "Soul Burn deals X damage to any target. You gain life equal to the damage dealt,
/// but not more than ... the creature's toughness."
/// (Coverage::Partial — X is paid with black mana only; printed black and/or red.)
#[test]
fn soul_burn_deals_x_damage_and_caps_life_gain_at_target_creature_toughness() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[soul_burn()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    // Tap 8 Swamps and cast Soul Burn with X = 5 targeting a 2/2 Grizzly Bears.
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, soul_burn());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine.apply(P0, PlayerAction::ChooseNumber(5)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P1, grizzly_bears()).is_none()
            && in_graveyard(&engine, P1, grizzly_bears()).is_some(),
        "5 damage from Soul Burn destroys Grizzly Bears"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "life gain is capped at Grizzly Bears's toughness (2), not the full 5 damage dealt"
    );
}

/// "Spend only black and/or red mana on X": with three Swamps and four
/// Mountains the `{2}` and `{B}` take the Mountains and one Swamp, so only the
/// two Swamps left over may pay X. The card reads the restriction as black
/// only, so X stops at 2 and not at the 4 an unrestricted `{X}{2}{B}` would
/// allow.
#[test]
fn soul_burn_x_cannot_be_paid_with_mountains() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[soul_burn()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, soul_burn());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!("expected the X question");
    };
    assert_eq!(
        (min, max),
        (0, 2),
        "only the Swamps left after {{2}}{{B}} may pay X; the Mountains may not"
    );
}
