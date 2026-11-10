//! `cards/creatures/mv_5/latulla_keldon_overseer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn latulla_keldon_overseer() -> CardIndex {
    card_index("86b2879b-6c54-4d96-ab81-a3f2c87d42d9")
}

/// Latulla, Keldon Overseer ({3}{R}{R} 3/3 Legendary Creature — Human Spellshaper):
/// "{X}{R}, {T}, Discard two cards: Latulla deals X damage to any target."
#[test]
fn latulla_keldon_overseer_discards_two_cards_to_deal_x_damage_to_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                latulla_keldon_overseer(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[grizzly_bears(), index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, latulla_keldon_overseer(), 0);
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::ChooseNumber { .. } => {
                engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
            }
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p1],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards { options, .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .unwrap();
            }
            _ => break,
        }
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, p0, index::HILL_GIANT).is_some());
    assert_eq!(engine.state().players[1].life, 17);
}

/// Latulla, Keldon Overseer can target a creature and destroy it with lethal X damage.
#[test]
fn latulla_keldon_overseer_destroys_target_creature_with_x_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                latulla_keldon_overseer(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[mountain(), forest()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, latulla_keldon_overseer(), 0);
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::ChooseNumber { .. } => {
                engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
            }
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![bear],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards { options, .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .unwrap();
            }
            _ => break,
        }
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
}
