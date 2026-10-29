//! Goblin Charbelcher — `{4}` Artifact (MRD).
//! Oracle: "{3}, {T}: Reveal cards from the top of your library until you reveal a land card. This artifact deals damage equal to the number of nonland cards revealed this way to any target. If the revealed land card was a Mountain, this artifact deals double that damage instead. Put the revealed cards on the bottom of your library in any order."

use super::*;

fn goblin_charbelcher() -> CardIndex {
    card_index("c9b1557b-70b1-44f2-9fc2-7aa0b035b187")
}

#[test]
fn goblin_charbelcher_reveals_until_land_and_doubles_damage_for_a_mountain() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let belcher = goblin_charbelcher();
    let elf = llanowar_elves();
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[elf, elf])
        .battlefield(0, &[belcher, mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Put the two nonland cards (Llanowar Elves) on top of p0's library above the Mountains.
    let hand_elves: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p0))
        .iter()
        .copied()
        .filter(|&id| engine.state().object(id).unwrap().card.unwrap().index == elf)
        .collect();
    for id in hand_elves {
        engine
            .dev_state_mut(p0)
            .unwrap()
            .move_object(
                id,
                ZoneLocation::Library(p0),
                ZonePosition::Top,
                Cause::Setup,
            )
            .unwrap();
    }

    let belcher_id = on_battlefield(&engine, p0, belcher).unwrap();
    tap_all_mana_but(&mut engine, p0, None);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: belcher_id,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { player, cards, .. } = engine.pending().clone() else {
        unreachable!();
    };
    engine
        .apply(player, PlayerAction::Arrange { piles: vec![cards] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // 2 nonland cards revealed above a Mountain -> 2 * 2 = 4 damage to p1.
    assert_eq!(engine.state().players[1].life, 16);
}

#[test]
fn goblin_charbelcher_orders_the_bottom_cards_after_damage_replacement_choices() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let belcher = goblin_charbelcher();
    let elf = llanowar_elves();
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[elf, elf])
        .battlefield(0, &[belcher, mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_elves: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p0))
        .iter()
        .copied()
        .filter(|&id| engine.state().object(id).unwrap().card.unwrap().index == elf)
        .collect();
    for id in hand_elves {
        engine
            .dev_state_mut(p0)
            .unwrap()
            .move_object(
                id,
                ZoneLocation::Library(p0),
                ZonePosition::Top,
                Cause::Setup,
            )
            .unwrap();
    }
    for _ in 0..2 {
        engine
            .dev_state_mut(p0)
            .unwrap()
            .shields
            .push(crate::prevention::Shield {
                protects: crate::prevention::Shielded::Player(p1),
                kind: crate::prevention::ShieldKind::Next(1),
                controller: p1,
            });
    }

    let belcher_id = on_battlefield(&engine, p0, belcher).unwrap();
    tap_all_mana_but(&mut engine, p0, None);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: belcher_id,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    answer_damage_choices(&mut engine);
    let Pending::Arrange { player, cards, .. } = engine.pending().clone() else {
        panic!("the revealed cards still need their bottom-order choice");
    };
    let ordered: Vec<ObjectId> = cards.into_iter().rev().collect();
    engine
        .apply(
            player,
            PlayerAction::Arrange {
                piles: vec![ordered.clone()],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 18);
    let library = engine.state().zones.list(ZoneLocation::Library(p0));
    let bottom_to_top: Vec<ObjectId> = ordered.into_iter().rev().collect();
    assert_eq!(&library[..bottom_to_top.len()], bottom_to_top);
}

fn answer_damage_choices(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..12 {
        if matches!(engine.pending(), Pending::Arrange { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseDamageEffect {
                player,
                choice,
                options,
                ..
            } => engine
                .apply(
                    player,
                    PlayerAction::ChooseDamageEffect {
                        choice,
                        effect: options[0].id,
                    },
                )
                .unwrap(),
            Pending::AllocatePrevention {
                player,
                choice,
                damage,
                total,
                ..
            } => {
                assert_eq!(damage.len(), 1);
                engine
                    .apply(
                        player,
                        PlayerAction::AllocatePrevention {
                            choice,
                            allocation: vec![(damage[0].id, total.min(damage[0].amount))],
                        },
                    )
                    .unwrap();
            }
            pending => panic!("expected a damage replacement choice: {pending:?}"),
        }
    }
}
