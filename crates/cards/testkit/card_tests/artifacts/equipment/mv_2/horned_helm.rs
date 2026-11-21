//! `cards/artifacts/equipment/mv_2/horned_helm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn horned_helm() -> CardIndex {
    card_index("127b438d-e5c2-47ba-9554-d9d5af018d09")
}

/// Horned Helm (127b438d-e5c2-47ba-9554-d9d5af018d09) — {2} Equipment:
/// "Equipped creature gets +1/+1 and has trample."
/// "{G}{G}: Attach this Equipment to target creature you control."
/// "Equip {1}"
///
/// Equip {1} gives +1/+1 and Trample, verified in combat where excess damage
/// tramples over a 1/1 blocker onto the opponent.
#[test]
fn horned_helm_equips_for_one_gives_plus_one_and_tramples_in_combat() {
    let p1 = PlayerId::new(1);
    let mut r = equipped_gets(
        horned_helm(),
        1,
        jasmine_boreal(),
        (1, 1),
        KeywordSet::TRAMPLE,
    );
    assert_eq!(pt(&r.engine, r.host), (5, 6));

    // Attack with 5/6 Trample Jasmine; opponent blocks with 1/1 Llanowar Elves.
    let _ = r.attack_to_the_blocks();
    r.engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(r.theirs, r.host)],
            },
        )
        .unwrap();
    pass_until(&mut r.engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });

    assert_eq!(
        r.engine.state().object(r.theirs).unwrap().zone,
        Zone::Graveyard,
        "1/1 blocker took lethal damage"
    );
    assert_eq!(
        life_of(&r.engine, p1),
        16,
        "4 excess trample damage hit opponent (5 - 1 = 4, 20 -> 16)"
    );
}

/// "{G}{G}: Attach this Equipment to target creature you control" activates at
/// instant speed on the opponent's turn (when Equip {1} is illegal) and moves
/// +1/+1 and Trample to the new host.
#[test]
fn horned_helm_gg_attaches_at_instant_speed_on_opponents_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                horned_helm(),
                jasmine_boreal(),
                goblin_piker(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let helm = on_battlefield(&engine, p0, horned_helm()).unwrap();
    let jasmine = on_battlefield(&engine, p0, jasmine_boreal()).unwrap();
    let piker = on_battlefield(&engine, p0, goblin_piker()).unwrap();
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    // Equip {1} onto Jasmine Boreal in own main phase, leaving 2 Forests untapped for {G}{G}.
    let my_forests = forests(&engine, p0, 3);
    tap_mana_where(&mut engine, p0, |id| id == my_forests[0]);
    activate(&mut engine, p0, horned_helm(), 0);
    aim_at(&mut engine, p0, jasmine);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, jasmine), (5, 6));
    assert!(keywords(&engine, jasmine).contains(KeywordSet::TRAMPLE));

    // Opponent's turn: p1 passes priority to p0, float {G}{G}.
    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);

    let offer = priority_offer(&engine);
    assert!(
        !ability_offered(&engine, helm, 0),
        "Equip {{1}} is sorcery-speed only"
    );
    assert!(
        offer.abilities.contains(&(helm, 3)),
        "{{G}}{{G}} attach activates at instant speed on opponent's turn"
    );

    activate(&mut engine, p0, horned_helm(), 3);
    let menu = aim_at(&mut engine, p0, piker);
    assert!(menu.contains(&piker) && menu.contains(&jasmine));
    assert!(!menu.contains(&their_elf));
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, piker), (3, 2), "Piker gained +1/+1");
    assert!(keywords(&engine, piker).contains(KeywordSet::TRAMPLE));
    assert_eq!(pt(&engine, jasmine), (4, 5), "Jasmine lost +1/+1");
    assert!(!keywords(&engine, jasmine).contains(KeywordSet::TRAMPLE));
}
