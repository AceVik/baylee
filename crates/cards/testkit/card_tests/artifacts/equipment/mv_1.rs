//! The tests of the cards under `cards/artifacts/equipment/mv_1/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

mod adventuring_gear;
mod avacyn_s_collar;
mod basilisk_collar;
mod bladed_bracers;
mod bonesplitter;
mod boots_of_speed;
mod bronze_sword;
mod colossus_hammer;
mod darksteel_axe;
mod dead_iron_sledge;
mod golem_skin_gauntlets;
mod gorgon_s_head;
mod greataxe;
mod hedgewitch_s_mask;
mod honed_khopesh;
mod kor_halberd;
mod lead_pipe;
mod leonin_scimitar;
mod masterwork_of_ingenuity;
mod neurok_hoversail;
mod peregrine_mask;
mod prying_blade;
mod short_sword;
mod shuko;
mod silver_inlaid_dagger;
mod skateboard;
mod slagwurm_armor;
mod sylvok_lifestaff;
mod trusty_machete;
mod vanquisher_s_axe;
mod vibranium_energy_daggers;
mod well_worn_spatula;
mod witches_eye;
mod zephyr_boots;

/// A plain Human, the bystander that tells "a Human" from "a creature".
fn tragic_poet() -> CardIndex {
    card_index("97bdfbe0-7048-4157-8a58-5b17410750ab")
}

fn pool_total(engine: &Engine<RegistryLookup>) -> u64 {
    engine.state().players[0].mana_pool.total()
}

/// Seat 0 at its first main phase with `forests` Forests, `mine` beside
/// them and `hand` in hand; seat 1 holds `theirs`.
fn rig(
    forests: usize,
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut board = vec![forest(); forests];
    board.extend_from_slice(mine);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, hand)
        .battlefield(0, &board)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    engine
}

/// Taps exactly `n` untapped Forests of seat 0 for mana, and nothing else
/// (the Elves beside them keep their own `{T}` ability).
#[track_caller]
fn float(engine: &mut Engine<RegistryLookup>, n: usize) {
    let p0 = PlayerId::new(0);
    let chosen: Vec<ObjectId> = all_on_battlefield(engine, p0, forest())
        .into_iter()
        .filter(|id| !is_tapped(engine, *id))
        .take(n)
        .collect();
    assert_eq!(chosen.len(), n, "enough untapped Forests to float {n}");
    let before = pool_total(engine);
    tap_mana_where(engine, p0, |id| chosen.contains(&id));
    assert_eq!(pool_total(engine), before + n as u64, "{n} mana floated");
}

/// Pays `Equip {cost}` of the Equipment `card` (its ability 0) onto `host`,
/// the way a player does: the offer is absent one mana short, present at the
/// price, the target menu is the controller's creatures and no other, and
/// the pool is empty afterwards. Returns the Equipment.
#[track_caller]
fn equip(
    engine: &mut Engine<RegistryLookup>,
    card: CardIndex,
    cost: usize,
    host: ObjectId,
) -> ObjectId {
    let p0 = PlayerId::new(0);
    let gear = on_battlefield(engine, p0, card).expect("the Equipment is out");
    let offered = |e: &Engine<RegistryLookup>| priority_offer(e).abilities.contains(&(gear, 0));
    assert_eq!(pool_total(engine), 0, "an empty pool to start from");
    assert!(!offered(engine), "Equip is not free");
    float(engine, cost - 1);
    assert!(!offered(engine), "one mana short of Equip {{{cost}}}");
    float(engine, 1);
    assert!(offered(engine), "Equip {{{cost}}} is paid");
    activate(engine, p0, card, 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host),
        "the host is a legal target: {options:?}"
    );
    assert!(!options.contains(&gear), "the Equipment is no creature");
    assert!(
        options.iter().all(|o| engine
            .state()
            .object(*o)
            .is_some_and(|o| o.controller == p0)),
        "\"target creature you control\": {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the host is chosen");
    pass_until(engine, |e| {
        e.state()
            .object(gear)
            .is_some_and(|o| o.attached_to == Some(host))
    });
    assert_eq!(pool_total(engine), 0, "the price was the whole pool");
    gear
}

/// Seat 1 answers the open declare-blockers question with `blockers`
/// (blocker, attacker) and combat is played to the second main phase.
#[track_caller]
fn block_and_finish(engine: &mut Engine<RegistryLookup>, blockers: Vec<(ObjectId, ObjectId)>) {
    engine
        .apply(PlayerId::new(1), PlayerAction::DeclareBlockers { blockers })
        .expect("the blocks come out of the offer");
    pass_until(engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
}

/// The shared reading of an Equipment whose whole text is "Equipped creature
/// gets +`dp`/+`dt`" and "Equip {`cost`}": it changes the host and nobody
/// else, the price is real, and moving it takes the bonus along.
#[track_caller]
fn plain_pump(card: CardIndex, cost: usize, dp: i16, dt: i16) {
    let elf = llanowar_elves();
    let mut engine = rig(2 * cost, &[], &[card, elf, elf], &[elf]);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let elves = all_on_battlefield(&engine, p0, elf);
    let (host, other) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, elf).expect("their Elf");
    let gear = on_battlefield(&engine, p0, card).expect("the Equipment");
    assert!(
        engine
            .state()
            .object(gear)
            .is_some_and(|o| o.attached_to.is_none()),
        "it enters holding nobody"
    );
    equip(&mut engine, card, cost, host);
    assert_eq!(pt(&engine, host), (1 + dp, 1 + dt), "the host wears it");
    assert_eq!(pt(&engine, other), (1, 1), "the bare Elf is untouched");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "so is the one across the table"
    );
    equip(&mut engine, card, cost, other);
    assert_eq!(
        pt(&engine, other),
        (1 + dp, 1 + dt),
        "re-equipped: the new host wears it"
    );
    assert_eq!(pt(&engine, host), (1, 1), "and the old host does not");
}
