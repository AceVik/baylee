//! The tests of the cards under `cards/creatures/artifacts/mv_6/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

mod arachnoid;
mod beast_of_burden;
mod chardalyn_dragon;
mod composite_golem;
mod darksteel_sentinel;
mod duplicant;
mod enatu_golem;
mod flowstone_sculpture;
mod geistcatcher_s_rig;
mod goblin_dirigible;
mod gold_forged_sentinel;
mod iron_man_futurist_paragon;
mod limestone_golem;
mod lumengrid_gargoyle;
mod malachite_golem;
mod mirror_golem;
mod pardic_wanderer;
mod phyrexian_hulk;
mod prophet_of_the_peak;
mod sharuum_the_hegemon;
mod tetravus;
mod the_reaper_king_no_more;
mod thundering_tanadon;
mod tribal_golem;
mod triskelion;
mod urza_s_avenger;
mod workhorse;

/// A seat's life total.
fn life_of(engine: &Engine<RegistryLookup>, seat: usize) -> i32 {
    engine.state().players[seat].life
}

/// Declares `attacker` alone against `defender`, stopping at the declaration.
#[track_caller]
fn attack_alone(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    attacker: ObjectId,
    defender: PlayerId,
) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == seat),
    );
    engine
        .apply(
            seat,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(defender))],
            },
        )
        .expect("the attack is legal");
}

/// Plays the rest of the combat out, nobody blocking, to the second main.
#[track_caller]
fn through_combat(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
}

/// Casts `card` from `seat`'s hand off every land it has and lets the spell
/// and whatever it triggered resolve; answers the permanent it became.
#[track_caller]
fn cast_resolved(engine: &mut Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> ObjectId {
    cast_from_hand(engine, seat, card);
    pass_until(engine, stack_is_empty);
    on_battlefield(engine, seat, card).expect("the permanent spell resolved")
}

/// Whether the permanent is, as the layers read it, an artifact creature.
fn is_artifact_creature(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    let types = engine
        .state()
        .object(id)
        .expect("on the table")
        .characteristics()
        .types;
    types.contains(TypeSet::ARTIFACT) && types.contains(TypeSet::CREATURE)
}

/// The artifact creature `card` cast for `lands` and resolved, as a 6-mana
/// body test needs it.
#[track_caller]
fn cast_for_six(card: CardIndex) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[card])
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let id = cast_resolved(&mut engine, p0, card);
    (engine, id)
}

/// "Flying": against a ground Ogre and a flying Envoy, only the Envoy may
/// block `card` when it attacks.
#[track_caller]
fn only_flyers_may_block(card: CardIndex) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[card])
        .battlefield(1, &[gray_ogre(), aven_envoy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let flyer = on_battlefield(&engine, p0, card).expect("seated");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("seated");
    let envoy = on_battlefield(&engine, p1, aven_envoy()).expect("seated");
    assert!(keywords(&engine, flyer).contains(KeywordSet::FLYING));
    let blocks = attack_and_collect_blocks(&mut engine, flyer, p1);
    let can_block = |who: ObjectId| {
        blocks
            .iter()
            .any(|b| b.blocker == who && b.attackers.contains(&flyer))
    };
    assert!(can_block(envoy), "a flyer may block a flyer");
    assert!(!can_block(ogre), "a ground creature may not block it");
}

/// "Trample": a 2/2 Ogre chump-blocks `card`, which assigns the Ogre lethal
/// damage and tramples the rest of its power over to the player.
#[track_caller]
fn tramples_over_a_blocker(card: CardIndex, power: i32) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[card])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let trampler = on_battlefield(&engine, p0, card).expect("seated");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("seated");
    assert!(keywords(&engine, trampler).contains(KeywordSet::TRAMPLE));
    assert_eq!(pt(&engine, ogre).1, 2, "the chump blocker has toughness 2");
    attack_alone(&mut engine, p0, trampler, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(ogre, trampler)],
            },
        )
        .expect("the Ogre blocks");
    through_combat(&mut engine);
    assert!(
        in_graveyard(&engine, p1, gray_ogre()).is_some(),
        "the blocker died"
    );
    assert_eq!(
        life_of(&engine, 1),
        20 - (power - 2),
        "the excess tramples over"
    );
}
mod phyrexian_devourer;
