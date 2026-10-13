//! `cards/lands/utility/glacial_chasm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Glacial Chasm: "Cumulative upkeep—Pay 2 life." / "When this land enters, sacrifice a land." / "Creatures you control can't attack." / "Prevent all damage that would be dealt to you."
/// When Glacial Chasm enters the battlefield, its enters-trigger forces its controller to sacrifice a land.
#[test]
fn glacial_chasm_sacrifices_a_land_on_etb() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(225, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[glacial_chasm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forest_obj = on_battlefield(&engine, p0, forest()).expect("Forest deployed");
    let chasm = play_land(&mut engine, p0, glacial_chasm());
    assert!(!entered_tapped(&engine, chasm));

    for _ in 0..8 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "unexpected while waiting for trigger: {:?}",
                engine.pending()
            );
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }

    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected land sacrifice choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert!(options.contains(&forest_obj));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![forest_obj],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, forest()).is_some());
    assert!(on_battlefield(&engine, p0, glacial_chasm()).is_some());
}

use baylee_core::generated::index;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
fn prepared(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = before_first_upkeep(hand, mine, theirs);
    reach_main_phase(&mut e, P0);
    mana(&mut e);
    e
}

fn before_first_upkeep(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    mana(&mut e);
    e
}
fn mana(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}
fn obj(e: &Engine<RegistryLookup>, p: PlayerId, c: CardIndex) -> ObjectId {
    on_battlefield(e, p, c).unwrap()
}
fn priority(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(),Pending::Priority{player,..} if *player==p),
    );
}
fn cast(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex) {
    priority(e, p);
    mana(e);
    let card = in_hand(e, p, c).unwrap();
    e.apply(p, PlayerAction::CastSpell { card }).unwrap();
}
fn targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let p = e.pending().asked().unwrap();
    e.apply(p, PlayerAction::ChooseTargets { objects, players })
        .unwrap();
}

fn answer(e: &mut Engine<RegistryLookup>, yes: bool) {
    let p = e.pending().asked().unwrap();
    e.apply(p, PlayerAction::YesNo(yes)).unwrap();
}
fn choose(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let p = e.pending().asked().unwrap();
    e.apply(p, PlayerAction::ChooseObjects { objects }).unwrap();
}

#[test]
fn chasm_prevents_damage_but_not_life_payment_and_forbids_attacks() {
    let mut e = prepared(
        &[index::GLACIAL_CHASM],
        &[index::FOREST, index::GRIZZLY_BEARS],
        &[index::PRODIGAL_SORCERER],
    );
    let land = in_hand(&e, P0, index::GLACIAL_CHASM).unwrap();
    e.apply(P0, PlayerAction::PlayLand { card: land }).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let forest = obj(&e, P0, index::FOREST);
    choose(&mut e, vec![forest]);
    pass_until(&mut e, stack_is_empty);
    let life = e.state.players[0].life;
    let sorcerer = obj(&e, P1, index::PRODIGAL_SORCERER);
    priority(&mut e, P1);
    e.apply(
        P1,
        PlayerAction::ActivateAbility {
            source: sorcerer,
            ability_index: 0,
        },
    )
    .unwrap();
    targets(&mut e, vec![], vec![P0]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, life);
    let bear = obj(&e, P0, index::GRIZZLY_BEARS);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert!(
        e.apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(P1))]
            }
        )
        .is_err()
    );
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    let before = e.state.players[0].life;
    answer(&mut e, true);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, before - 2);
}

#[test]
fn chasm_cumulative_upkeep_pays_the_whole_age_scaled_price_or_sacrifices() {
    let mut e = prepared(&[index::GLACIAL_CHASM], &[index::FOREST], &[]);
    let chasm = in_hand(&e, P0, index::GLACIAL_CHASM).unwrap();
    e.apply(P0, PlayerAction::PlayLand { card: chasm }).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let forest = obj(&e, P0, index::FOREST);
    choose(&mut e, vec![forest]);
    pass_until(&mut e, stack_is_empty);
    for price in [2, 4] {
        pass_until(
            &mut e,
            |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
        );
        let life = e.state.players[0].life;
        answer(&mut e, true);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state.players[0].life, life - price);
        assert_eq!(e.state.object(chasm).unwrap().zone, Zone::Battlefield);
    }
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    let life = e.state.players[0].life;
    answer(&mut e, false);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, life);
    assert_eq!(e.state.object(chasm).unwrap().zone, Zone::Graveyard);
}

#[test]
fn chasm_entry_requires_one_land_and_may_sacrifice_itself() {
    let mut e = prepared(&[index::GLACIAL_CHASM], &[], &[]);
    let chasm = in_hand(&e, P0, index::GLACIAL_CHASM).unwrap();
    e.apply(P0, PlayerAction::PlayLand { card: chasm }).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending()
    else {
        unreachable!()
    };
    assert_eq!((*min, *max), (1, 1));
    assert!(options.contains(&chasm));
    let before = e.snapshot_hash();
    assert!(
        e.apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
            .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    choose(&mut e, vec![chasm]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(chasm).unwrap().zone, Zone::Graveyard);
}

#[test]
fn doubling_season_doubles_age_counters_and_the_whole_life_price() {
    let mut e = before_first_upkeep(&[], &[index::GLACIAL_CHASM, index::DOUBLING_SEASON], &[]);
    let chasm = obj(&e, P0, index::GLACIAL_CHASM);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    let Pending::YesNo { prompt, .. } = e.pending() else {
        unreachable!()
    };
    assert_eq!(*prompt, crate::choice::YesNoPrompt::PayLife { amount: 4 });
    assert_eq!(
        e.state
            .object(chasm)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        2
    );
    let life = e.state.players[0].life;
    answer(&mut e, true);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, life - 4);
}

#[test]
fn countering_cumulative_upkeep_adds_no_age_and_demands_no_payment() {
    let mut e = before_first_upkeep(&[index::TISHANA_S_TIDEBINDER], &[index::GLACIAL_CHASM], &[]);
    let chasm = obj(&e, P0, index::GLACIAL_CHASM);
    pass_until(&mut e, |e| {
        e.state.turn.active == P0
            && e.state.turn.step == Step::Upkeep
            && !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    let trigger = *e.state.zones.list(ZoneLocation::Stack).last().unwrap();
    let life = e.state.players[0].life;
    cast(&mut e, P0, index::TISHANA_S_TIDEBINDER);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    targets(&mut e, vec![trigger], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state
            .object(chasm)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        0
    );
    assert_eq!(e.state.players[0].life, life);
    // Tidebinder does not remove a land's abilities; next upkeep starts at one.
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    let Pending::YesNo { prompt, .. } = e.pending() else {
        unreachable!()
    };
    assert_eq!(*prompt, crate::choice::YesNoPrompt::PayLife { amount: 2 });
    answer(&mut e, true);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, life - 2);
}

#[test]
fn large_chasm_upkeep_never_truncates_the_full_life_price() {
    for enough in [false, true] {
        let mut e = before_first_upkeep(&[], &[index::GLACIAL_CHASM], &[]);
        let chasm = obj(&e, P0, index::GLACIAL_CHASM);
        // Transparent high-value setup; upkeep trigger and payment are real.
        e.state
            .object_mut(chasm)
            .unwrap()
            .counters
            .set(baylee_cards_dsl::counters::AGE, 32_767);
        e.state.players[0].life = if enough { 100_000 } else { 65_535 };
        e.refresh_offer();
        pass_until(&mut e, |e| {
            e.state.turn.active == P0
                && e.state.turn.step == Step::Upkeep
                && !stack_is_empty(e)
                && matches!(e.pending(), Pending::Priority { .. })
        });
        if enough {
            pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
            let Pending::YesNo { prompt, .. } = e.pending() else {
                unreachable!()
            };
            assert_eq!(
                *prompt,
                crate::choice::YesNoPrompt::PayLife { amount: 65_536 }
            );
            answer(&mut e, true);
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state.players[0].life, 34_464);
            assert_eq!(e.state.object(chasm).unwrap().zone, Zone::Battlefield);
        } else {
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state.players[0].life, 65_535);
            assert_eq!(e.state.object(chasm).unwrap().zone, Zone::Graveyard);
        }
    }
}

/// "Creatures you control can't attack": under our Chasm the opponent's
/// creature is still offered as an attacker on their turn, and ours is not
/// offered on ours.
#[test]
fn chasm_forbids_only_its_controllers_creatures_from_the_attack_offer() {
    let mut e = prepared(
        &[index::GLACIAL_CHASM],
        &[index::FOREST, index::GRIZZLY_BEARS],
        &[index::GRIZZLY_BEARS],
    );
    let land = in_hand(&e, P0, index::GLACIAL_CHASM).unwrap();
    e.apply(P0, PlayerAction::PlayLand { card: land }).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let forest = obj(&e, P0, index::FOREST);
    choose(&mut e, vec![forest]);
    pass_until(&mut e, stack_is_empty);
    let mine = obj(&e, P0, index::GRIZZLY_BEARS);
    let theirs = obj(&e, P1, index::GRIZZLY_BEARS);

    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let Pending::ChooseAttackers { attackers, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert!(
        !attackers.contains(&mine),
        "\"Creatures you control can't attack\": our Grizzly Bears is not offered: {attackers:?}"
    );

    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P1),
    );
    let Pending::ChooseAttackers { attackers, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert!(
        attackers.contains(&theirs),
        "the opponent's creatures are not ours, so theirs may still attack: {attackers:?}"
    );
}
