//! Gameplay rule tests for Wave 19 Engine Lane 4 (`eng-zones-filters`):
//! 1. Decayed (`KeywordSet::DECAYED`, CR 702.147a): can't block, attacks & sacrifices
//!    at end of combat, non-attackers survive, blinked attackers survive (CR 400.7).
//! 2. `Filter::EnteredCurrentZoneThisTurn(Option<ZoneRef>)`: origin-zone filtering
//!    in graveyard/exile, turn reset, and replay-hash `zone_changes.tracked` gating.
//! 3. `Amount::DistinctNames { zone, filter }`: collapsing duplicates, tokens on
//!    battlefield, excluding nameless face-down permanents (CR 201.2a), graveyard cards.
//! 4. `Filter::OwnedBy(PlayerRel::Opponent)` (`OppOwn`) across zones and
//!    `one_per_player_seats` player-scope filtering.
//! 5. Hand-zone static abilities: `Modifier::RevealOpponentsHands`, granted hand
//!    activated abilities (symmetric cycling), `GrantMiracle`, `GrantForetell`
//!    (persists after grantor leaves BF), `GrantWarp`, `GrantEvoke`, `GrantMadnessManaCost`.
//! 6. Graveyard-zone static abilities: `GrantsMayhemManaCost`, `GrantsBeamMeUpManaCost`
//!    (`cant_return_source`), `ZonePlayGrant` (*Serra Paragon* death trigger on both
//!    lands and creatures, `EntryFinality`, `GrantLeaveExileReplacement`, and
//!    graveyard-source *Glimpse the Cosmos* shape).
//! 7. Exile-zone static abilities & mana spending: `Filter::ExiledByYou` +
//!    `AltCastZones::ExiledByYou` (with replay-hash `track_exilers` gating),
//!    `ZonePlayManaSpending::SnowAsAnyColor`, `AnyColor`, and `AnyType`.
#![allow(clippy::too_many_lines, clippy::items_after_statements)]

use super::*;
use crate::choice::CastModeKind;
use baylee_cards_dsl::prelude::spell;
use baylee_cards_dsl::{
    AltCastPrice, AltCastZones, Amount, Condition, Cost, CostPart, CounterKind, Effect, Filter,
    KeywordSet, MiracleGrant, Modifier, PlayerRel, StaticAbility, TargetReq, TargetSpec,
    ZonePlayCastRider, ZonePlayGrant, ZonePlayManaSpending, ZoneRef, ZoneSel,
};
use baylee_core::ids::Defender;
use baylee_core::mana::{ManaColor, ManaPool, ManaSpending};

const C_BASE: u32 = 19_400;

const PERMANENT_CARD: Filter = Filter::Not(&Filter::INSTANT_OR_SORCERY);

const fn stat(filter: Filter, modifier: Modifier) -> AbilityDef {
    AbilityDef::Static(StaticAbility {
        source_zone: ZoneRef::Battlefield,
        layer: modifier.layer(),
        filter,
        modifier,
        condition: None,
    })
}

fn refresh_offer(engine: &mut Bench) {
    if let Pending::Priority { player, .. } = engine.pending().clone() {
        engine.pending = Pending::Priority {
            player,
            legal: Box::new(engine.compute_legal(player)),
        };
    }
}

fn add_pool(engine: &mut Bench, player: PlayerId, color: ManaColor, amount: u32) {
    engine.state.players[usize::from(player.get())]
        .mana_pool
        .add(color, amount);
    refresh_offer(engine);
}

fn add_snow_pool(engine: &mut Bench, player: PlayerId, color: ManaColor, amount: u32) {
    engine.state.players[usize::from(player.get())]
        .mana_pool
        .add_snow(color, amount);
    refresh_offer(engine);
}

fn clear_pool(engine: &mut Bench, player: PlayerId) {
    engine.state.players[usize::from(player.get())].mana_pool = ManaPool::new();
    refresh_offer(engine);
}

fn move_to_gy(engine: &mut Bench, id: ObjectId) {
    let owner = engine.state.object(id).expect("object exists").owner;
    engine
        .state
        .move_object(
            id,
            ZoneLocation::Graveyard(owner),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("move to graveyard");
    engine.sync_static_effects();
    engine.state.refresh_characteristics();
    refresh_offer(engine);
}

fn walk_auto(engine: &mut Bench, mut stop: impl FnMut(&Bench) -> bool) {
    for _ in 0..512 {
        if stop(engine) {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(
                        player,
                        PlayerAction::DeclareAttackers {
                            attackers: Vec::new(),
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(
                        player,
                        PlayerAction::DeclareBlockers {
                            blockers: Vec::new(),
                        },
                    )
                    .unwrap();
            }
            Pending::DiscardChoice { player, count } => {
                let cards: Vec<ObjectId> = engine
                    .state()
                    .zones
                    .list(ZoneLocation::Hand(player))
                    .iter()
                    .copied()
                    .take(usize::from(count))
                    .collect();
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: cards })
                    .unwrap();
            }
            other => panic!("unexpected question while walking: {other:?}"),
        }
    }
    panic!("walk_auto did not reach stop condition");
}

fn to_next_turn_main(engine: &mut Bench, seat: PlayerId) {
    let start_turn = engine.state().turn.number;
    walk_auto(engine, |e| {
        e.state().turn.number > start_turn
            && holds_priority_in(e, seat, crate::turn::Step::Main)
            && e.state().turn.phase == Phase::FirstMain
    });
}

// =========================================================================
// 1. Decayed (CR 702.147a)
// =========================================================================

#[test]
fn decayed_cannot_block_and_sacrifices_at_end_of_combat_cr_702_147a() {
    let decayed_zombie = C_BASE + 1;
    let idle_decayed = C_BASE + 2;
    let blinked_decayed = C_BASE + 3;
    let opp_attacker = C_BASE + 4;
    let blinker = C_BASE + 5;

    static BLINK_ABILS: [AbilityDef; 1] = [free(
        &[Effect::Blink {
            target: TargetSpec::Object(&Filter::CREATURE),
            owner_control: true,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];

    let cards = vec![
        card(
            decayed_zombie,
            creature_face("Decayed Attacker", "{1}{B}", 2, 2),
            KeywordSet::DECAYED,
            &[],
        ),
        card(
            idle_decayed,
            creature_face("Decayed Sitter", "{1}{B}", 2, 2),
            KeywordSet::DECAYED,
            &[],
        ),
        card(
            blinked_decayed,
            creature_face("Decayed Blinker Target", "{1}{B}", 2, 2),
            KeywordSet::DECAYED,
            &[],
        ),
        card(
            opp_attacker,
            creature_face("Opp Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            blinker,
            face("Blink Device", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &BLINK_ABILS,
        ),
    ];

    let mut engine = bench(
        19_001,
        cards,
        [
            Seat::with(&[decayed_zombie, idle_decayed, blinked_decayed, blinker]),
            Seat::with(&[opp_attacker]),
        ],
    );

    to_main(&mut engine, me());
    let z_att = the(&engine, ZoneLocation::Battlefield, decayed_zombie);
    let z_idle = the(&engine, ZoneLocation::Battlefield, idle_decayed);
    let z_blink = the(&engine, ZoneLocation::Battlefield, blinked_decayed);
    let o_bear = the(&engine, ZoneLocation::Battlefield, opp_attacker);
    let dev = the(&engine, ZoneLocation::Battlefield, blinker);

    // Static half of Decayed: cannot block.
    assert!(
        !crate::combat::can_block(engine.state(), me(), o_bear, z_att),
        "Decayed creature must not be able to block (CR 702.147a)"
    );

    // Advance to me's next turn so attackers don't have summoning sickness.
    to_next_turn_main(&mut engine, me());
    let before_life = life(&engine, them());

    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (z_att, Defender::Player(them())),
                    (z_blink, Defender::Player(them())),
                ],
            },
        )
        .expect("declare decayed attackers");

    // Let the two decayed attack triggers resolve (creating delayed end-of-combat
    // sacrifice triggers), then blink `z_blink` before End of Combat.
    walk_until(&mut engine, |e| {
        e.state().zones.stack_is_empty()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    activate(&mut engine, me(), dev, 0);
    if let Pending::ChooseTargets { player, .. } = engine.pending().clone() {
        engine
            .apply(
                player,
                PlayerAction::ChooseObjects {
                    objects: vec![z_blink],
                },
            )
            .expect("choose blink target");
    }
    settle(&mut engine, me());

    // Advance through combat into Second Main.
    walk_until(&mut engine, |e| {
        holds_priority_in(e, me(), crate::turn::Step::Main)
            && e.state().turn.phase == Phase::SecondMain
    });

    assert_eq!(
        life(&engine, them()),
        before_life - 2,
        "unblinked decayed attacker deals 2 combat damage before dying at end of combat"
    );
    assert_eq!(
        engine.state().object(z_att).unwrap().zone,
        Zone::Graveyard,
        "attacking Decayed creature must be sacrificed at end of combat (CR 702.147a)"
    );
    assert_eq!(
        engine.state().object(z_idle).unwrap().zone,
        Zone::Battlefield,
        "non-attacking Decayed creature must survive combat"
    );
    assert_eq!(
        engine.state().object(z_blink).unwrap().zone,
        Zone::Battlefield,
        "blinked Decayed attacker becomes a new object (CR 400.7) and is not sacrificed"
    );
}

// =========================================================================
// 2. Filter::EnteredCurrentZoneThisTurn + replay-hash gating
// =========================================================================

#[test]
fn entered_current_zone_this_turn_tracks_origin_and_gates_replays() {
    let watcher_idx = C_BASE + 10;
    let victim_idx = C_BASE + 11;
    let milled_idx = C_BASE + 12;
    let exiled_idx = C_BASE + 13;

    static ENTERED_FROM_BF: Filter = Filter::EnteredCurrentZoneThisTurn(Some(ZoneRef::Battlefield));
    static ENTERED_FROM_LIB: Filter = Filter::EnteredCurrentZoneThisTurn(Some(ZoneRef::Library));
    static ENTERED_ANY: Filter = Filter::EnteredCurrentZoneThisTurn(None);

    static WATCHER_ABILS: [AbilityDef; 1] = [stat(
        Filter::EnteredCurrentZoneThisTurn(Some(ZoneRef::Battlefield)),
        Modifier::AddKeyword(KeywordSet::HASTE),
    )];

    // Untracked game: no card references EnteredCurrentZoneThisTurn or zone-change amounts.
    let untracked = bench(
        19_002,
        vec![card(
            victim_idx,
            creature_face("Plain Victim", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        )],
        [Seat::with(&[victim_idx]), Seat::default()],
    );
    assert!(
        !untracked.state().per_turn.zone_changes.tracked,
        "games without zone-change filters must leave zone_changes.tracked = false for bit-identical replays"
    );

    // Tracked game: watcher references Filter::EnteredCurrentZoneThisTurn.
    let mut engine = bench(
        19_003,
        vec![
            card(
                watcher_idx,
                creature_face("Origin Watcher", "{1}{W}", 1, 1),
                KeywordSet::EMPTY,
                &WATCHER_ABILS,
            ),
            card(
                victim_idx,
                creature_face("Died From BF", "{1}{B}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                milled_idx,
                creature_face("Milled Card", "{1}{U}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                exiled_idx,
                creature_face("Exiled From BF", "{1}{R}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [
            Seat::with(&[watcher_idx, victim_idx, exiled_idx]).holding(&[milled_idx]),
            Seat::default(),
        ],
    );
    assert!(
        engine.state().per_turn.zone_changes.tracked,
        "card reading Filter::EnteredCurrentZoneThisTurn must enable zone_changes.tracked"
    );

    to_main(&mut engine, me());
    let watcher = the(&engine, ZoneLocation::Battlefield, watcher_idx);
    let victim = the(&engine, ZoneLocation::Battlefield, victim_idx);
    let exiled = the(&engine, ZoneLocation::Battlefield, exiled_idx);
    let milled = the(&engine, ZoneLocation::Hand(me()), milled_idx);

    move_to_gy(&mut engine, victim);
    engine
        .state
        .move_object(
            exiled,
            ZoneLocation::Exile(me()),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    engine
        .state
        .move_object(
            milled,
            ZoneLocation::Library(me()),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    move_to_gy(&mut engine, milled);

    let v_obj = engine.state().object(victim).unwrap();
    let m_obj = engine.state().object(milled).unwrap();
    let e_obj = engine.state().object(exiled).unwrap();

    assert!(crate::eval::matches(
        &ENTERED_FROM_BF,
        engine.state(),
        v_obj,
        me(),
        watcher
    ));
    assert!(crate::eval::matches(
        &ENTERED_ANY,
        engine.state(),
        v_obj,
        me(),
        watcher
    ));
    assert!(!crate::eval::matches(
        &ENTERED_FROM_LIB,
        engine.state(),
        v_obj,
        me(),
        watcher
    ));

    assert!(crate::eval::matches(
        &ENTERED_FROM_LIB,
        engine.state(),
        m_obj,
        me(),
        watcher
    ));
    assert!(!crate::eval::matches(
        &ENTERED_FROM_BF,
        engine.state(),
        m_obj,
        me(),
        watcher
    ));

    assert!(crate::eval::matches(
        &ENTERED_FROM_BF,
        engine.state(),
        e_obj,
        me(),
        watcher
    ));

    // Next turn: zone_changes clears, so none match anymore.
    to_next_turn_main(&mut engine, them());
    let v_obj = engine.state().object(victim).unwrap();
    let e_obj = engine.state().object(exiled).unwrap();
    assert!(!crate::eval::matches(
        &ENTERED_FROM_BF,
        engine.state(),
        v_obj,
        me(),
        watcher
    ));
    assert!(!crate::eval::matches(
        &ENTERED_FROM_BF,
        engine.state(),
        e_obj,
        me(),
        watcher
    ));
}

// =========================================================================
// 3. Amount::DistinctNames
// =========================================================================

#[test]
fn distinct_names_counts_unique_nonempty_names_across_zones() {
    let alpha = C_BASE + 20;
    let beta = C_BASE + 21;
    let spell_card = C_BASE + 22;

    let mut engine = bench(
        19_004,
        vec![
            card(
                alpha,
                creature_face("Alpha Wolf", "{1}{G}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                beta,
                creature_face("Beta Bear", "{1}{G}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                spell_card,
                face("Not A Creature", "{U}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [
            Seat::with(&[alpha, alpha, beta]).holding(&[alpha, beta, spell_card]),
            Seat::with(&[beta]),
        ],
    );
    to_main(&mut engine, me());
    let src = objects(&engine, ZoneLocation::Battlefield, alpha)[0];

    static BF_YOURS: Amount = Amount::DistinctNames {
        zone: ZoneSel::Battlefield,
        filter: &Filter::ControlledByYou,
    };
    static GY_CREATURES: Amount = Amount::DistinctNames {
        zone: ZoneSel::GraveyardOf(PlayerRel::You),
        filter: &Filter::CREATURE,
    };

    // Two "Alpha Wolf" + one "Beta Bear" = 2 distinct names (opponent's Beta Bear excluded).
    assert_eq!(
        crate::eval::amount(&BF_YOURS, engine.state(), me(), src, None),
        2
    );

    // Make both Alpha Wolves nameless face-down permanents (CR 201.2a) -> only Beta Bear counts.
    for id in objects(&engine, ZoneLocation::Battlefield, alpha) {
        let obj = engine.state.object_mut(id).unwrap();
        obj.status.insert(crate::object::Status::FACE_DOWN);
        obj.base_mut().name = crate::state::NAMELESS;
        obj.cache.clear();
    }
    engine.state.refresh_characteristics();
    assert_eq!(
        crate::eval::amount(&BF_YOURS, engine.state(), me(), src, None),
        1,
        "face-down nameless permanents must not count toward DistinctNames (CR 201.2a)"
    );

    // Move hand cards (Alpha Wolf, Beta Bear, Not A Creature) + one Alpha Wolf into GY.
    let hand_cards = engine.state.zones.list(ZoneLocation::Hand(me())).clone();
    for id in hand_cards {
        move_to_gy(&mut engine, id);
    }
    let one_bf_alpha = objects(&engine, ZoneLocation::Battlefield, alpha)[0];
    move_to_gy(&mut engine, one_bf_alpha);
    // Graveyard now has 2 Alpha Wolf, 1 Beta Bear, 1 instant -> 2 distinct creature names.
    assert_eq!(
        crate::eval::amount(&GY_CREATURES, engine.state(), me(), src, None),
        2
    );
}

// =========================================================================
// 4. Filter::OwnedBy(PlayerRel::Opponent) (OppOwn) + filter_player_scope
// =========================================================================

#[test]
fn opp_own_filter_matches_opponent_owned_objects_and_scopes_targets() {
    let my_card = C_BASE + 30;
    let opp_card = C_BASE + 31;

    static OPP_OWN: Filter = Filter::OwnedBy(PlayerRel::Opponent);

    let mut engine = bench(
        19_005,
        vec![
            card(
                my_card,
                creature_face("My Relic", "{1}", 1, 1),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                opp_card,
                creature_face("Opp Relic", "{1}", 1, 1),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [Seat::with(&[my_card]), Seat::with(&[opp_card])],
    );
    to_main(&mut engine, me());

    let mine = the(&engine, ZoneLocation::Battlefield, my_card);
    let theirs = the(&engine, ZoneLocation::Battlefield, opp_card);

    // Even if `me()` gains control of `theirs`, its owner is still `them()`.
    crate::resolve::gain_control(&mut engine.state, &[(theirs, me())]);
    engine.state.refresh_characteristics();

    assert!(
        crate::eval::matches(
            &OPP_OWN,
            engine.state(),
            engine.state().object(theirs).unwrap(),
            me(),
            mine
        ),
        "opponent-owned permanent controlled by you still matches OppOwn"
    );
    assert!(
        !crate::eval::matches(
            &OPP_OWN,
            engine.state(),
            engine.state().object(mine).unwrap(),
            me(),
            mine
        ),
        "your own permanent must not match OppOwn"
    );

    move_to_gy(&mut engine, theirs);
    assert!(crate::eval::matches(
        &OPP_OWN,
        engine.state(),
        engine.state().object(theirs).unwrap(),
        me(),
        mine
    ));

    // Target player-scope narrowing (`eval::one_per_player_seats`):
    let seats =
        crate::eval::one_per_player_seats(engine.state(), &TargetSpec::Object(&OPP_OWN), me());
    assert_eq!(
        seats,
        vec![them()],
        "OppOwn must scope OnePerPlayer to opponents only"
    );
}

// =========================================================================
// 5. Hand-zone static abilities: RevealOpponentsHands & granted Hand cycling
// =========================================================================

#[test]
fn hand_zone_reveal_opponents_hands_and_granted_hand_cycling() {
    let telepathy = C_BASE + 40;
    let tectonic = C_BASE + 41;
    let hand_land = C_BASE + 42;
    let hand_creature = C_BASE + 43;

    static TELEPATHY_ABILS: [AbilityDef; 1] = [stat(Filter::Any, Modifier::RevealOpponentsHands)];

    static CYCLE_PARTS: [CostPart; 1] = [CostPart::DiscardSelf];
    static CYCLE_EFFECTS: [Effect; 1] = [Effect::DrawCards {
        amount: Amount::Fixed(1),
    }];
    static HAND_CYCLE_DEF: AbilityDef = AbilityDef::Activated {
        cost: Cost {
            mana: ManaCost::parse("{1}"),
            parts: &CYCLE_PARTS,
        },
        effects: &CYCLE_EFFECTS,
        targets: None,
        second_targets: None,
        third_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Hand,
        limit: ActivationLimit::Unlimited,
        x_value: baylee_cards_dsl::ActivationX::Any,
        cost_reduction: None,
    };
    static TECTONIC_ABILS: [AbilityDef; 1] = [stat(
        Filter::LAND,
        Modifier::GrantActivatedDefinition(&HAND_CYCLE_DEF),
    )];

    let mut engine = bench(
        19_006,
        vec![
            card(
                telepathy,
                face("Telepathy Probe", "{U}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                &TELEPATHY_ABILS,
            ),
            card(
                tectonic,
                face("Hand Cycle Granter", "{1}{R}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                &TECTONIC_ABILS,
            ),
            card(
                hand_land,
                land_face("Cycleable Land"),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                hand_creature,
                creature_face("Non-Land Hand Card", "{1}{G}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [
            Seat::with(&[telepathy, tectonic]).holding(&[hand_land, hand_creature]),
            Seat::default().holding(&[hand_land]),
        ],
    );
    to_main(&mut engine, me());

    let tele_id = the(&engine, ZoneLocation::Battlefield, telepathy);
    assert!(
        engine.state().hand_revealed(them()),
        "RevealOpponentsHands must reveal opponent's hand"
    );
    assert!(
        !engine.state().hand_revealed(me()),
        "RevealOpponentsHands must not reveal controller's own hand"
    );

    // Leaving the battlefield stops revealing opponent's hand.
    move_to_gy(&mut engine, tele_id);
    assert!(!engine.state().hand_revealed(them()));

    // Granted hand activated ability (cycling) applies to lands in hand, not creatures.
    let my_land = the(&engine, ZoneLocation::Hand(me()), hand_land);
    let my_cr = the(&engine, ZoneLocation::Hand(me()), hand_creature);
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    let (_, legal) = offer(&engine);
    let grant_idx = legal
        .abilities
        .iter()
        .find(|&&(src, _)| src == my_land)
        .map(|&(_, idx)| idx)
        .expect("land in hand must be offered granted cycling ability");
    assert!(
        !legal.abilities.iter().any(|&(src, _)| src == my_cr),
        "nonland in hand must not receive Filter::LAND hand cycling"
    );

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    activate(&mut engine, me(), my_land, grant_idx);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(my_land).unwrap().zone,
        Zone::Graveyard,
        "cycling discarded the land to graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before,
        "discard 1 + draw 1 keeps hand size constant"
    );

    // Symmetric grant (like Homing Sliver) also works for opponent's hand.
    to_next_turn_main(&mut engine, them());
    let opp_land = the(&engine, ZoneLocation::Hand(them()), hand_land);
    add_pool(&mut engine, them(), ManaColor::Colorless, 1);
    let (_, opp_legal) = offer(&engine);
    assert!(
        opp_legal.abilities.iter().any(|&(src, _)| src == opp_land),
        "symmetric hand ability grant must also be offered in opponent's hand"
    );
}

// =========================================================================
// 6. Hand-zone static keyword grants: Miracle, Foretell, Warp, Evoke, Madness
// =========================================================================

#[test]
fn hand_zone_grant_miracle_applies_to_first_draw_only() {
    let aminatou = C_BASE + 50;
    let big_enchant = C_BASE + 51;

    static AMINATOU_ABILS: [AbilityDef; 1] = [stat(
        Filter::ENCHANTMENT,
        Modifier::GrantMiracle(MiracleGrant::ManaCostLessGeneric(4)),
    )];

    let mut engine = bench(
        19_007,
        vec![
            card(
                aminatou,
                creature_face("Miracle Granter", "{1}{W}{U}{B}", 2, 4),
                KeywordSet::EMPTY,
                &AMINATOU_ABILS,
            ),
            card(
                big_enchant,
                face("Costly Enchantment", "{4}{W}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [
            Seat::with(&[aminatou]).holding(&[big_enchant]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let ench = the(&engine, ZoneLocation::Hand(me()), big_enchant);
    engine
        .state
        .move_object(
            ench,
            ZoneLocation::Library(me()),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();

    // Advance to our next draw step and wait for the Miracle YesNo offer.
    let start_turn = engine.state().turn.number;
    walk_auto(&mut engine, |e| {
        e.state().turn.number > start_turn
            && matches!(e.pending(), Pending::YesNo { player, .. } if *player == me())
    });

    // Give only {W} (NOT enough for {4}{W}, exact for Miracle cost {W}) and accept.
    engine.state.players[usize::from(me().get())]
        .mana_pool
        .add(ManaColor::White, 1);
    engine
        .apply(me(), PlayerAction::YesNo(true))
        .expect("accept granted miracle cast");
    settle(&mut engine, me());

    assert_eq!(
        engine.state().object(ench).unwrap().zone,
        Zone::Battlefield,
        "granted Miracle must cast the {{4}}{{W}} enchantment for {{W}}"
    );
    assert_eq!(
        engine.state().players[usize::from(me().get())]
            .mana_pool
            .total(),
        0,
        "miracle payment consumed exactly {{W}}"
    );
}

#[test]
fn hand_zone_grant_foretell_persists_after_grantor_leaves_battlefield() {
    let devourer = C_BASE + 52;
    let demon = C_BASE + 53;

    static DEVOURER_ABILS: [AbilityDef; 1] = [stat(
        Filter::NONLAND,
        Modifier::GrantForetell { reduce_generic: 2 },
    )];

    let mut engine = bench(
        19_008,
        vec![
            card(
                devourer,
                creature_face("Foretell Granter", "{1}{B}", 0, 3),
                KeywordSet::EMPTY,
                &DEVOURER_ABILS,
            ),
            card(
                demon,
                creature_face("Foretold Demon", "{3}{B}", 4, 4),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [Seat::with(&[devourer]).holding(&[demon]), Seat::default()],
    );
    to_main(&mut engine, me());

    let granter = the(&engine, ZoneLocation::Battlefield, devourer);
    let card_id = the(&engine, ZoneLocation::Hand(me()), demon);

    add_pool(&mut engine, me(), ManaColor::Colorless, 2);
    let (_, legal) = offer(&engine);
    assert!(
        legal.foretellable.contains(&card_id),
        "nonland card in hand must be foretellable under GrantForetell"
    );

    engine
        .apply(me(), PlayerAction::Foretell { card: card_id })
        .expect("foretell card via static grant");
    assert_eq!(engine.state().object(card_id).unwrap().zone, Zone::Exile);

    // Same turn: cannot cast yet (CR 702.143a).
    add_pool(&mut engine, me(), ManaColor::Black, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    let (_, same_turn_legal) = offer(&engine);
    assert!(
        !same_turn_legal.castable.contains(&card_id),
        "foretold card cannot be cast on the turn it was foretold"
    );

    // Remove the grantor BEFORE next turn: granted foretell cost must persist in exile!
    move_to_gy(&mut engine, granter);

    to_next_turn_main(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::Black, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    let (_, next_legal) = offer(&engine);
    assert!(
        next_legal.castable.contains(&card_id),
        "card foretold via GrantForetell must remain castable for reduced cost after grantor leaves"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: card_id })
        .expect("cast foretold card for {1}{B}");
    if let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    {
        let idx = options
            .iter()
            .position(|o| o.kind == CastModeKind::Foretell)
            .expect("Foretell cast mode offered");
        engine.apply(player, PlayerAction::ChooseMode(idx)).unwrap();
    }
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(card_id).unwrap().zone,
        Zone::Battlefield
    );
}

#[test]
fn hand_zone_grant_warp_evoke_and_madness() {
    let warp_granter = C_BASE + 54;
    let evoke_granter = C_BASE + 55;
    let madness_granter = C_BASE + 56;
    let artifact_cr = C_BASE + 57;
    let evoke_cr = C_BASE + 58;
    let madness_cr = C_BASE + 59;
    let discard_outlet = C_BASE + 60;

    static WARP_ABILS: [AbilityDef; 1] = [stat(
        Filter::ARTIFACT,
        Modifier::GrantWarp(Cost {
            mana: ManaCost::parse("{1}{R}"),
            parts: &[],
        }),
    )];
    static EVOKE_ABILS: [AbilityDef; 1] = [stat(
        Filter::CREATURE,
        Modifier::GrantEvoke(Cost {
            mana: ManaCost::parse("{U}"),
            parts: &[],
        }),
    )];
    static MADNESS_ABILS: [AbilityDef; 1] =
        [stat(Filter::CREATURE, Modifier::GrantMadnessManaCost)];
    static DISCARD_ABILS: [AbilityDef; 1] = [free(
        &[Effect::DiscardForPlayers {
            who: PlayerRel::You,
            count: 1,
        }],
        None,
    )];

    // --- Warp + Evoke test ---
    let mut engine = bench(
        19_009,
        vec![
            card(
                warp_granter,
                creature_face("Tannuk Probe", "{2}{R}", 3, 3),
                KeywordSet::EMPTY,
                &WARP_ABILS,
            ),
            card(
                evoke_granter,
                creature_face("Evoke Granter", "{1}{U}", 2, 2),
                KeywordSet::EMPTY,
                &EVOKE_ABILS,
            ),
            card(
                artifact_cr,
                FaceDef {
                    power: Some(5),
                    toughness: Some(5),
                    ..face(
                        "Huge Construct",
                        "{6}",
                        TypeSet::ARTIFACT.union(TypeSet::CREATURE),
                    )
                },
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                evoke_cr,
                creature_face("Evoke Target", "{4}{U}", 3, 3),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [
            Seat::with(&[warp_granter, evoke_granter]).holding(&[artifact_cr, evoke_cr]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let warp_id = the(&engine, ZoneLocation::Hand(me()), artifact_cr);
    let evoke_id = the(&engine, ZoneLocation::Hand(me()), evoke_cr);

    // Cast Huge Construct via granted Warp for {1}{R}.
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    add_pool(&mut engine, me(), ManaColor::Colorless, 1);
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&warp_id),
        "artifact castable via GrantWarp"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: warp_id })
        .unwrap();
    if let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    {
        let idx = options
            .iter()
            .position(|o| o.kind == CastModeKind::Warp)
            .expect("Warp mode offered");
        engine.apply(player, PlayerAction::ChooseMode(idx)).unwrap();
    }
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(warp_id).unwrap().zone,
        Zone::Battlefield
    );

    // Cast Evoke Target via granted Evoke for {U} -> enters BF then ETB trigger sacrifices it.
    add_pool(&mut engine, me(), ManaColor::Blue, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: evoke_id })
        .unwrap();
    if let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    {
        let idx = options
            .iter()
            .position(|o| matches!(o.kind, CastModeKind::Permission(_)))
            .expect("Evoke grant mode offered");
        engine.apply(player, PlayerAction::ChooseMode(idx)).unwrap();
    }
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(evoke_id).unwrap().zone,
        Zone::Graveyard,
        "evoked creature must be sacrificed by synthetic ETB trigger (CR 702.74a)"
    );

    // Advance past end step: warped permanent must be exiled at end step.
    to_next_turn_main(&mut engine, them());
    assert_eq!(
        engine.state().object(warp_id).unwrap().zone,
        Zone::Exile,
        "warped permanent must exile itself at end step (CR 702.185a)"
    );

    // --- Madness test ---
    let mut mad_engine = bench(
        19_010,
        vec![
            card(
                madness_granter,
                creature_face("Gorger Probe", "{R}", 2, 1),
                KeywordSet::EMPTY,
                &MADNESS_ABILS,
            ),
            card(
                madness_cr,
                creature_face("Mad Vampire", "{1}{R}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                discard_outlet,
                face("Discard Outlet", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                &DISCARD_ABILS,
            ),
        ],
        [
            Seat::with(&[madness_granter, discard_outlet]).holding(&[madness_cr]),
            Seat::default(),
        ],
    );
    to_main(&mut mad_engine, me());
    let mad_id = the(&mad_engine, ZoneLocation::Hand(me()), madness_cr);
    let outlet = the(&mad_engine, ZoneLocation::Battlefield, discard_outlet);

    add_pool(&mut mad_engine, me(), ManaColor::Red, 1);
    add_pool(&mut mad_engine, me(), ManaColor::Colorless, 1);
    activate(&mut mad_engine, me(), outlet, 0);
    // Resolve discard ability -> choose mad_id -> madness exiles the card and asks YesNo to cast for {1}{R}.
    for _ in 0..64 {
        if matches!(mad_engine.pending(), Pending::YesNo { player, .. } if *player == me()) {
            break;
        }
        match mad_engine.pending().clone() {
            Pending::Priority { player, .. } => {
                mad_engine
                    .apply(player, PlayerAction::PassPriority)
                    .unwrap();
            }
            Pending::ChooseCards { player, .. } => {
                mad_engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![mad_id],
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected question during madness discard: {other:?}"),
        }
    }
    assert!(
        matches!(mad_engine.pending(), Pending::YesNo { player, .. } if *player == me()),
        "granted madness must ask YesNo to cast"
    );
    assert_eq!(
        mad_engine.state().object(mad_id).unwrap().zone,
        Zone::Exile,
        "discarded card under GrantMadnessManaCost must be exiled instead (CR 702.35a)"
    );
    mad_engine
        .apply(me(), PlayerAction::YesNo(true))
        .expect("cast via granted madness");
    settle(&mut mad_engine, me());
    assert_eq!(
        mad_engine.state().object(mad_id).unwrap().zone,
        Zone::Battlefield,
        "granted madness cast resolves onto battlefield"
    );
}

// =========================================================================
// 7. Graveyard statics: Mayhem, BeamMeUp, ZonePlayGrant (Serra, Finality,
//    LeaveExile, Graveyard-source Glimpse)
// =========================================================================

#[test]
fn graveyard_mayhem_and_beam_me_up_cant_return_source() {
    let mayhem_granter = C_BASE + 70;
    let mayhem_cr = C_BASE + 71;
    let non_discarded_cr = C_BASE + 72;
    let discard_outlet = C_BASE + 73;
    let scotty = C_BASE + 74;
    let helper_cr = C_BASE + 75;
    let bolt_idx = C_BASE + 76;

    static MAYHEM_ABILS: [AbilityDef; 1] = [stat(Filter::CREATURE, Modifier::GrantsMayhemManaCost)];
    static DISCARD_ABILS: [AbilityDef; 1] = [free(
        &[Effect::DiscardForPlayers {
            who: PlayerRel::You,
            count: 1,
        }],
        None,
    )];
    static SCOTTY_ABILS: [AbilityDef; 1] = [stat(
        Filter::INSTANT_OR_SORCERY,
        Modifier::GrantsBeamMeUpManaCost {
            cant_return_source: true,
        },
    )];
    static BOLT_ABILS: [AbilityDef; 1] = [spell!(&[Effect::LoseLife {
        amount: Amount::Fixed(3),
        target: PlayerRel::EachOpponent,
    }])];

    // --- Mayhem grant ---
    let mut engine = bench(
        19_011,
        vec![
            card(
                mayhem_granter,
                creature_face("Mayhem Granter", "{1}{B}", 2, 2),
                KeywordSet::EMPTY,
                &MAYHEM_ABILS,
            ),
            card(
                mayhem_cr,
                creature_face("Discarded Goblin", "{R}", 1, 1),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                non_discarded_cr,
                creature_face("Milled Goblin", "{R}", 1, 1),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                discard_outlet,
                face("Outlet", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                &DISCARD_ABILS,
            ),
        ],
        [
            Seat::with(&[mayhem_granter, discard_outlet, non_discarded_cr]).holding(&[mayhem_cr]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let disc_id = the(&engine, ZoneLocation::Hand(me()), mayhem_cr);
    let non_disc_id = the(&engine, ZoneLocation::Battlefield, non_discarded_cr);
    let outlet = the(&engine, ZoneLocation::Battlefield, discard_outlet);

    move_to_gy(&mut engine, non_disc_id);
    activate(&mut engine, me(), outlet, 0);
    for _ in 0..64 {
        if engine.state().zones.stack_is_empty()
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == me())
        {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards { player, .. } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![disc_id],
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected question during mayhem discard: {other:?}"),
        }
    }

    add_pool(&mut engine, me(), ManaColor::Red, 1);
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&disc_id),
        "discarded card must be castable via GrantsMayhemManaCost"
    );
    assert!(
        !legal.castable.contains(&non_disc_id),
        "card put into GY without being discarded must not be castable via Mayhem"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: disc_id })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(disc_id).unwrap().zone,
        Zone::Battlefield
    );

    // --- BeamMeUp grant with cant_return_source: true (Scotty) ---
    let mut beam = bench(
        19_012,
        vec![
            card(
                scotty,
                creature_face("Scotty Probe", "{2}{U}", 2, 3),
                KeywordSet::EMPTY,
                &SCOTTY_ABILS,
            ),
            card(
                helper_cr,
                creature_face("Away Team", "{1}{W}", 1, 1),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                bolt_idx,
                face("Phaser Blast", "{R}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                &BOLT_ABILS,
            ),
        ],
        [
            Seat::with(&[scotty]).holding(&[bolt_idx, helper_cr]),
            Seat::default(),
        ],
    );
    to_main(&mut beam, me());
    let scotty_id = the(&beam, ZoneLocation::Battlefield, scotty);
    let bolt_id = the(&beam, ZoneLocation::Hand(me()), bolt_idx);
    let helper_id = the(&beam, ZoneLocation::Hand(me()), helper_cr);
    move_to_gy(&mut beam, bolt_id);

    // Only Scotty is on BF -> cant_return_source blocks returning Scotty, so bolt is NOT offered.
    add_pool(&mut beam, me(), ManaColor::Red, 1);
    let (_, only_scotty) = offer(&beam);
    assert!(
        !only_scotty.castable.contains(&bolt_id),
        "Scotty cannot be returned to pay its own granted BeamMeUp cost"
    );

    // Put another creature on BF -> now bolt IS offered, and return options exclude Scotty.
    beam.state.object_mut(helper_id).unwrap().kind = crate::object::ObjectKind::Permanent;
    beam.state
        .move_object(
            helper_id,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    beam.sync_static_effects();
    refresh_offer(&mut beam);

    let ret_opts = crate::casting::beam_return_options(beam.state(), me(), bolt_id);
    assert_eq!(
        ret_opts,
        vec![helper_id],
        "return options must contain only the other creature, never Scotty"
    );
    let (_, with_helper) = offer(&beam);
    assert!(with_helper.castable.contains(&bolt_id));

    let opp_life_before = life(&beam, them());
    beam.apply(me(), PlayerAction::CastSpell { card: bolt_id })
        .unwrap();
    if let Pending::ChooseCards { player, .. } = beam.pending().clone() {
        beam.apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![helper_id],
            },
        )
        .unwrap();
    }
    settle(&mut beam, me());
    assert_eq!(
        beam.state().object(scotty_id).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(beam.state().object(helper_id).unwrap().zone, Zone::Hand);
    assert_eq!(
        beam.state().object(bolt_id).unwrap().zone,
        Zone::Exile,
        "BeamMeUp exiles the spell on resolution"
    );
    assert_eq!(life(&beam, them()), opp_life_before - 3);
}

#[test]
fn graveyard_zone_play_grant_serra_paragon_finality_leave_exile_and_gy_source() {
    let serra = C_BASE + 80;
    let gy_land = C_BASE + 81;
    let small_cr = C_BASE + 82;
    let big_cr = C_BASE + 83;
    let eighth_doc = C_BASE + 84;
    let leave_granter = C_BASE + 85;
    let glimpse = C_BASE + 86;
    let bounce_dev = C_BASE + 87;
    let destroy_dev = C_BASE + 88;

    static SERRA_ABILS: [AbilityDef; 1] = [stat(
        PERMANENT_CARD,
        Modifier::ZonePlayGrant(ZonePlayGrant {
            who: PlayerRel::You,
            zones: AltCastZones::Zones(&[ZoneRef::Graveyard]),
            grant_zone_permission: true,
            allow_lands: true,
            with_flash: false,
            max_mana_value: Some(Amount::Fixed(3)),
            once_per_turn: true,
            price: AltCastPrice::Normal,
            mana_spending: ZonePlayManaSpending::Exact,
            rider: ZonePlayCastRider::GrantDeathExileAndGainLife(2),
            your_turn_only: true,
            as_sorcery_only: false,
            mana_cost_must_contain: None,
        }),
    )];

    static FINALITY_ABILS: [AbilityDef; 1] = [stat(
        Filter::CREATURE,
        Modifier::ZonePlayGrant(ZonePlayGrant {
            who: PlayerRel::You,
            zones: AltCastZones::Zones(&[ZoneRef::Graveyard]),
            grant_zone_permission: true,
            allow_lands: false,
            with_flash: false,
            max_mana_value: None,
            once_per_turn: true,
            price: AltCastPrice::Normal,
            mana_spending: ZonePlayManaSpending::Exact,
            rider: ZonePlayCastRider::EntryFinality,
            your_turn_only: true,
            as_sorcery_only: false,
            mana_cost_must_contain: None,
        }),
    )];

    static LEAVE_EXILE_ABILS: [AbilityDef; 1] = [stat(
        Filter::CREATURE,
        Modifier::ZonePlayGrant(ZonePlayGrant {
            who: PlayerRel::You,
            zones: AltCastZones::Zones(&[ZoneRef::Graveyard]),
            grant_zone_permission: true,
            allow_lands: false,
            with_flash: false,
            max_mana_value: None,
            once_per_turn: false,
            price: AltCastPrice::Normal,
            mana_spending: ZonePlayManaSpending::Exact,
            rider: ZonePlayCastRider::GrantLeaveExileReplacement,
            your_turn_only: false,
            as_sorcery_only: false,
            mana_cost_must_contain: None,
        }),
    )];

    static GLIMPSE_ABILS: [AbilityDef; 2] = [
        spell!(&[Effect::DrawCards {
            amount: Amount::Fixed(1),
        }]),
        AbilityDef::Static(StaticAbility {
            source_zone: ZoneRef::Graveyard,
            layer: baylee_cards_dsl::Layer::Text,
            filter: Filter::This,
            modifier: Modifier::ZonePlayGrant(ZonePlayGrant {
                who: PlayerRel::You,
                zones: AltCastZones::Zones(&[ZoneRef::Graveyard]),
                grant_zone_permission: true,
                allow_lands: false,
                with_flash: false,
                max_mana_value: None,
                once_per_turn: false,
                price: AltCastPrice::Mana(ManaCost::parse("{U}")),
                mana_spending: ZonePlayManaSpending::Exact,
                rider: ZonePlayCastRider::ExileInsteadOfGraveyard,
                your_turn_only: false,
                as_sorcery_only: true,
                mana_cost_must_contain: None,
            }),
            condition: Some(Condition::ControlCount(&Filter::CREATURE, 1)),
        }),
    ];

    static BOUNCE_ABILS: [AbilityDef; 1] = [free(
        &[Effect::ReturnToHand {
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static DESTROY_ABILS: [AbilityDef; 1] = [free(
        &[Effect::Destroy {
            target: TargetSpec::Object(&PERMANENT_CARD),
            no_regen: false,
        }],
        Some(TargetReq::one(TargetSpec::Object(&PERMANENT_CARD))),
    )];

    // --- Serra Paragon: land + creature death trigger + once_per_turn + max_mana_value ---
    let mut engine = bench(
        19_013,
        vec![
            card(
                serra,
                creature_face("Serra Probe", "{2}{W}{W}", 3, 4),
                KeywordSet::EMPTY,
                &SERRA_ABILS,
            ),
            card(gy_land, land_face("Paragon Land"), KeywordSet::EMPTY, &[]),
            card(
                small_cr,
                creature_face("Small Knight", "{1}{W}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                big_cr,
                creature_face("Too Big Angel", "{3}{W}", 4, 4),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                destroy_dev,
                face("Destroy Device", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                &DESTROY_ABILS,
            ),
        ],
        [
            Seat::with(&[serra, destroy_dev]).holding(&[gy_land, small_cr, big_cr]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let destroyer = the(&engine, ZoneLocation::Battlefield, destroy_dev);
    let land_id = the(&engine, ZoneLocation::Hand(me()), gy_land);
    let small_id = the(&engine, ZoneLocation::Hand(me()), small_cr);
    let big_id = the(&engine, ZoneLocation::Hand(me()), big_cr);
    move_to_gy(&mut engine, land_id);
    move_to_gy(&mut engine, small_id);
    move_to_gy(&mut engine, big_id);

    add_pool(&mut engine, me(), ManaColor::White, 4);
    let (_, legal) = offer(&engine);
    assert!(
        legal.lands.contains(&land_id),
        "land in GY playable via Serra Paragon"
    );
    assert!(
        legal.castable.contains(&small_id),
        "MV 2 permanent castable via Serra Paragon"
    );
    assert!(
        !legal.castable.contains(&big_id),
        "MV 4 permanent must be rejected by max_mana_value: 3"
    );

    // Play the land -> uses once_per_turn, so small_id is no longer castable this turn!
    engine
        .apply(me(), PlayerAction::PlayLand { card: land_id })
        .expect("play land via Serra Paragon");
    let (_, after_land) = offer(&engine);
    assert!(
        !after_land.castable.contains(&small_id),
        "once_per_turn allowance consumed by playing a land"
    );

    // Destroy that land from BF -> GY: triggers exile + gain 2 life!
    let life_before = life(&engine, me());
    activate(&mut engine, me(), destroyer, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![land_id],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(land_id).unwrap().zone,
        Zone::Exile,
        "Serra Paragon land death trigger must exile it from graveyard"
    );
    assert_eq!(
        life(&engine, me()),
        life_before + 2,
        "Serra Paragon land death trigger must gain 2 life"
    );

    // Next turn: cast small_id from GY, then destroy it BF -> GY -> exiles + gains 2 life!
    to_next_turn_main(&mut engine, me());
    add_pool(&mut engine, me(), ManaColor::White, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: small_id })
        .expect("cast small creature via Serra Paragon");
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(small_id).unwrap().zone,
        Zone::Battlefield
    );

    let life_before_cr = life(&engine, me());
    activate(&mut engine, me(), destroyer, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![small_id],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(engine.state().object(small_id).unwrap().zone, Zone::Exile);
    assert_eq!(life(&engine, me()), life_before_cr + 2);

    // --- EntryFinality + GrantLeaveExileReplacement + Graveyard-source Glimpse ---
    let mut fin_engine = bench(
        19_014,
        vec![
            card(
                eighth_doc,
                creature_face("Finality Granter", "{2}{U}", 2, 2),
                KeywordSet::EMPTY,
                &FINALITY_ABILS,
            ),
            card(
                small_cr,
                creature_face("Reanimated Buddy", "{W}", 1, 1),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [
            Seat::with(&[eighth_doc]).holding(&[small_cr]),
            Seat::default(),
        ],
    );
    to_main(&mut fin_engine, me());
    let buddy = the(&fin_engine, ZoneLocation::Hand(me()), small_cr);
    move_to_gy(&mut fin_engine, buddy);
    add_pool(&mut fin_engine, me(), ManaColor::White, 1);
    fin_engine
        .apply(me(), PlayerAction::CastSpell { card: buddy })
        .unwrap();
    settle(&mut fin_engine, me());
    assert_eq!(
        fin_engine
            .state()
            .object(buddy)
            .unwrap()
            .counters
            .get(CounterKind::Finality),
        1,
        "EntryFinality rider must put a finality counter on entering permanent"
    );

    let mut leave_engine = bench(
        19_015,
        vec![
            card(
                leave_granter,
                creature_face("Leave Exile Granter", "{2}{B}", 2, 2),
                KeywordSet::EMPTY,
                &LEAVE_EXILE_ABILS,
            ),
            card(
                small_cr,
                creature_face("Unbounceable Buddy", "{W}", 1, 1),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                bounce_dev,
                face("Bounce Device", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                &BOUNCE_ABILS,
            ),
        ],
        [
            Seat::with(&[leave_granter, bounce_dev]).holding(&[small_cr]),
            Seat::default(),
        ],
    );
    to_main(&mut leave_engine, me());
    let unbounce = the(&leave_engine, ZoneLocation::Hand(me()), small_cr);
    let bouncer = the(&leave_engine, ZoneLocation::Battlefield, bounce_dev);
    move_to_gy(&mut leave_engine, unbounce);
    add_pool(&mut leave_engine, me(), ManaColor::White, 1);
    leave_engine
        .apply(me(), PlayerAction::CastSpell { card: unbounce })
        .unwrap();
    settle(&mut leave_engine, me());
    assert_eq!(
        leave_engine.state().object(unbounce).unwrap().zone,
        Zone::Battlefield
    );
    activate(&mut leave_engine, me(), bouncer, 0);
    if let Pending::ChooseTargets { player, .. } = leave_engine.pending().clone() {
        leave_engine
            .apply(
                player,
                PlayerAction::ChooseObjects {
                    objects: vec![unbounce],
                },
            )
            .unwrap();
    }
    settle(&mut leave_engine, me());
    assert_eq!(
        leave_engine.state().object(unbounce).unwrap().zone,
        Zone::Exile,
        "GrantLeaveExileReplacement must exile permanent instead of returning to hand"
    );

    // Graveyard-source static (Glimpse the Cosmos shape):
    let mut gy_src = bench(
        19_016,
        vec![
            card(
                glimpse,
                face("Glimpse Probe", "{1}{U}", TypeSet::SORCERY),
                KeywordSet::EMPTY,
                &GLIMPSE_ABILS,
            ),
            card(
                small_cr,
                creature_face("Giant Stand-In", "{W}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [Seat::with(&[small_cr]).holding(&[glimpse]), Seat::default()],
    );
    to_main(&mut gy_src, me());
    let g_id = the(&gy_src, ZoneLocation::Hand(me()), glimpse);
    let cr_id = the(&gy_src, ZoneLocation::Battlefield, small_cr);
    move_to_gy(&mut gy_src, g_id);
    add_pool(&mut gy_src, me(), ManaColor::Blue, 1);

    let (_, with_cr) = offer(&gy_src);
    assert!(
        with_cr.castable.contains(&g_id),
        "graveyard-source ZonePlayGrant must activate when condition holds"
    );
    move_to_gy(&mut gy_src, cr_id);
    let (_, without_cr) = offer(&gy_src);
    assert!(
        !without_cr.castable.contains(&g_id),
        "graveyard-source ZonePlayGrant must lapse when condition fails"
    );
    gy_src.state.object_mut(cr_id).unwrap().kind = crate::object::ObjectKind::Permanent;
    gy_src
        .state
        .move_object(
            cr_id,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    gy_src.sync_static_effects();
    refresh_offer(&mut gy_src);
    gy_src
        .apply(me(), PlayerAction::CastSpell { card: g_id })
        .unwrap();
    settle(&mut gy_src, me());
    assert_eq!(
        gy_src.state().object(g_id).unwrap().zone,
        Zone::Exile,
        "ExileInsteadOfGraveyard rider must exile graveyard-cast sorcery on resolution"
    );
}

// =========================================================================
// 8. Exile-zone static (ExiledByYou + replay gating) & SnowAsAnyColor / AnyType
// =========================================================================

#[test]
fn exiled_by_you_and_snow_mana_spending_and_replay_gating() {
    let haldan = C_BASE + 90;
    let exiler_spell = C_BASE + 91;
    let opp_spell = C_BASE + 92;
    let opp_other = C_BASE + 93;
    let kaldring = C_BASE + 94;
    let blue_spell = C_BASE + 95;
    let colorless_spell = C_BASE + 96;
    let any_type_granter = C_BASE + 97;

    static HALDAN_ABILS: [AbilityDef; 1] = [stat(
        Filter::NONCREATURE,
        Modifier::ZonePlayGrant(ZonePlayGrant {
            who: PlayerRel::You,
            zones: AltCastZones::ExiledByYou,
            grant_zone_permission: true,
            allow_lands: false,
            with_flash: false,
            max_mana_value: None,
            once_per_turn: false,
            price: AltCastPrice::Normal,
            mana_spending: ZonePlayManaSpending::AnyColor,
            rider: ZonePlayCastRider::None,
            your_turn_only: false,
            as_sorcery_only: false,
            mana_cost_must_contain: None,
        }),
    )];
    static EXILE_DEV_ABILS: [AbilityDef; 1] = [free(
        &[Effect::Exile {
            target: TargetSpec::Object(&PERMANENT_CARD),
        }],
        Some(TargetReq::one(TargetSpec::Object(&PERMANENT_CARD))),
    )];
    static KALDRING_ABILS: [AbilityDef; 1] = [stat(
        PERMANENT_CARD,
        Modifier::ZonePlayGrant(ZonePlayGrant {
            who: PlayerRel::You,
            zones: AltCastZones::Zones(&[ZoneRef::Graveyard]),
            grant_zone_permission: true,
            allow_lands: false,
            with_flash: false,
            max_mana_value: None,
            once_per_turn: false,
            price: AltCastPrice::Normal,
            mana_spending: ZonePlayManaSpending::SnowAsAnyColor,
            rider: ZonePlayCastRider::None,
            your_turn_only: false,
            as_sorcery_only: false,
            mana_cost_must_contain: None,
        }),
    )];
    static ANY_TYPE_ABILS: [AbilityDef; 1] = [stat(
        PERMANENT_CARD,
        Modifier::ZonePlayGrant(ZonePlayGrant {
            who: PlayerRel::You,
            zones: AltCastZones::Zones(&[ZoneRef::Graveyard]),
            grant_zone_permission: true,
            allow_lands: false,
            with_flash: false,
            max_mana_value: None,
            once_per_turn: false,
            price: AltCastPrice::Normal,
            mana_spending: ZonePlayManaSpending::AnyType,
            rider: ZonePlayCastRider::None,
            your_turn_only: false,
            as_sorcery_only: false,
            mana_cost_must_contain: None,
        }),
    )];

    // Replay gating: without any ExiledByYou card, track_exilers stays false.
    let untracked = bench(
        19_017,
        vec![card(
            opp_spell,
            face("Plain Artifact", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &[],
        )],
        [Seat::with(&[opp_spell]), Seat::default()],
    );
    assert!(
        !untracked.state().per_turn.track_exilers.0,
        "games without ExiledByYou must leave track_exilers = false for bit-identical replays"
    );

    // ExiledByYou test: Haldan + exile by me vs exile by opponent.
    let mut engine = bench(
        19_018,
        vec![
            card(
                haldan,
                creature_face("Haldan Probe", "{1}{G}{U}", 1, 4),
                KeywordSet::EMPTY,
                &HALDAN_ABILS,
            ),
            card(
                exiler_spell,
                face("Exile Wand", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                &EXILE_DEV_ABILS,
            ),
            card(
                opp_spell,
                face("Stolen Artifact", "{1}{U}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                opp_other,
                face("Opp Self-Exiled Artifact", "{1}{U}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [
            Seat::with(&[haldan, exiler_spell]),
            Seat::with(&[exiler_spell, opp_spell, opp_other]),
        ],
    );
    assert!(engine.state().per_turn.track_exilers.0);
    to_main(&mut engine, me());

    let my_wand = objects(&engine, ZoneLocation::Battlefield, exiler_spell)
        .into_iter()
        .find(|&id| engine.state().object(id).unwrap().controller == me())
        .unwrap();
    let opp_wand = objects(&engine, ZoneLocation::Battlefield, exiler_spell)
        .into_iter()
        .find(|&id| engine.state().object(id).unwrap().controller == them())
        .unwrap();
    let stolen = the(&engine, ZoneLocation::Battlefield, opp_spell);
    let opp_exiled = the(&engine, ZoneLocation::Battlefield, opp_other);

    // `me()` exiles `stolen` with my_wand.
    activate(&mut engine, me(), my_wand, 0);
    if let Pending::ChooseTargets { player, .. } = engine.pending().clone() {
        engine
            .apply(
                player,
                PlayerAction::ChooseObjects {
                    objects: vec![stolen],
                },
            )
            .unwrap();
    }
    settle(&mut engine, me());

    // `them()` exiles `opp_exiled` with opp_wand on their turn.
    to_next_turn_main(&mut engine, them());
    activate(&mut engine, them(), opp_wand, 0);
    if let Pending::ChooseTargets { player, .. } = engine.pending().clone() {
        engine
            .apply(
                player,
                PlayerAction::ChooseObjects {
                    objects: vec![opp_exiled],
                },
            )
            .unwrap();
    }
    settle(&mut engine, them());

    to_next_turn_main(&mut engine, me());
    assert!(
        crate::eval::matches(
            &Filter::ExiledByYou,
            engine.state(),
            engine.state().object(stolen).unwrap(),
            me(),
            my_wand,
        ),
        "card exiled by me must match Filter::ExiledByYou"
    );
    assert!(
        !crate::eval::matches(
            &Filter::ExiledByYou,
            engine.state(),
            engine.state().object(opp_exiled).unwrap(),
            me(),
            my_wand,
        ),
        "card exiled by opponent must not match Filter::ExiledByYou"
    );

    // Under Haldan's AnyColor spending, {R}{R} pays {1}{U} for `stolen`, NOT for `opp_exiled`!
    add_pool(&mut engine, me(), ManaColor::Red, 2);
    let (_, legal) = offer(&engine);
    assert!(
        legal.castable.contains(&stolen),
        "opponent's card exiled by you must be castable via AltCastZones::ExiledByYou + AnyColor"
    );
    assert!(
        !legal.castable.contains(&opp_exiled),
        "card exiled by opponent must not be castable via AltCastZones::ExiledByYou"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: stolen })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(stolen).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(engine.state().object(stolen).unwrap().controller, me());

    // --- SnowAsAnyColor (Kaldring) + AnyType test ---
    let mut snow_engine = bench(
        19_019,
        vec![
            card(
                kaldring,
                face("Kaldring Probe", "{1}{U}{B}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                &KALDRING_ABILS,
            ),
            card(
                blue_spell,
                creature_face("Blue Snow Bear", "{1}{U}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                colorless_spell,
                creature_face("Eldrazi Relic", "{1}{C}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                any_type_granter,
                face("AnyType Relic", "{3}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                &ANY_TYPE_ABILS,
            ),
        ],
        [
            Seat::with(&[kaldring]).holding(&[blue_spell, colorless_spell, any_type_granter]),
            Seat::default(),
        ],
    );
    to_main(&mut snow_engine, me());
    let u_id = the(&snow_engine, ZoneLocation::Hand(me()), blue_spell);
    let c_id = the(&snow_engine, ZoneLocation::Hand(me()), colorless_spell);
    let at_id = the(&snow_engine, ZoneLocation::Hand(me()), any_type_granter);
    move_to_gy(&mut snow_engine, u_id);
    move_to_gy(&mut snow_engine, c_id);

    // Non-snow {G}{G} CANNOT pay {1}{U} under SnowAsAnyColor!
    add_pool(&mut snow_engine, me(), ManaColor::Green, 2);
    let (_, non_snow_legal) = offer(&snow_engine);
    assert!(
        !non_snow_legal.castable.contains(&u_id),
        "non-snow green mana must not pay {{U}} under SnowAsAnyColor"
    );

    // Snow {G} + non-snow {G} CAN pay {1}{U}, and LegalActions publishes snow_as_any_color!
    clear_pool(&mut snow_engine, me());
    add_snow_pool(&mut snow_engine, me(), ManaColor::Green, 1);
    add_pool(&mut snow_engine, me(), ManaColor::Green, 1);
    let (_, snow_legal) = offer(&snow_engine);
    assert!(
        snow_legal.castable.contains(&u_id),
        "snow green + non-snow green must pay {{1}}{{U}} under SnowAsAnyColor"
    );
    let published =
        snow_legal.spell_spending_for(crate::choice::Spend::cast(u_id, CastModeKind::Normal));
    assert!(
        published.is_some_and(ManaSpending::snow_as_any_color),
        "LegalActions must publish snow_as_any_color for planners"
    );

    // Snow {G}{G} CANNOT pay {1}{C} (colorless pip stays strict per CR 106.1b)!
    clear_pool(&mut snow_engine, me());
    add_snow_pool(&mut snow_engine, me(), ManaColor::Green, 2);
    let (_, snow_c_legal) = offer(&snow_engine);
    assert!(
        !snow_c_legal.castable.contains(&c_id),
        "SnowAsAnyColor must not let snow green pay colorless {{C}} (CR 106.1b)"
    );

    // Cast {1}{U} using snow {G} + non-snow {G}!
    clear_pool(&mut snow_engine, me());
    add_snow_pool(&mut snow_engine, me(), ManaColor::Green, 1);
    add_pool(&mut snow_engine, me(), ManaColor::Green, 1);
    snow_engine
        .apply(me(), PlayerAction::CastSpell { card: u_id })
        .expect("cast {1}{U} with snow {G} + non-snow {G}");
    settle(&mut snow_engine, me());
    assert_eq!(
        snow_engine.state().object(u_id).unwrap().zone,
        Zone::Battlefield
    );

    // Remove Kaldring and put AnyType granter on BF -> now non-snow {R}{R} CAN pay {1}{C}!
    let kaldring_id = the(&snow_engine, ZoneLocation::Battlefield, kaldring);
    move_to_gy(&mut snow_engine, kaldring_id);
    snow_engine.state.object_mut(at_id).unwrap().kind = crate::object::ObjectKind::Permanent;
    snow_engine
        .state
        .move_object(
            at_id,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    snow_engine.sync_static_effects();
    clear_pool(&mut snow_engine, me());
    add_pool(&mut snow_engine, me(), ManaColor::Red, 2);
    let (_, any_type_legal) = offer(&snow_engine);
    assert!(
        any_type_legal.castable.contains(&c_id),
        "ZonePlayManaSpending::AnyType must let red mana pay colorless {{C}}"
    );
    snow_engine
        .apply(me(), PlayerAction::CastSpell { card: c_id })
        .expect("cast {1}{C} with {R}{R} under AnyType");
    settle(&mut snow_engine, me());
    assert_eq!(
        snow_engine.state().object(c_id).unwrap().zone,
        Zone::Battlefield
    );
}

// =========================================================================
// 9. Deferred land entry riders (tag 573), Evoke abilities_lost (CR 702.74a),
//    GrantWarp non-mana CostParts, untargeted sweep gating, & hash 565..=573
// =========================================================================

#[test]
fn deferred_land_entry_preserves_serra_paragon_and_finality_riders_and_hashes_tag_573() {
    let serra = C_BASE + 120;
    let finality_land_grant = C_BASE + 121;
    let leave_exile_land_grant = C_BASE + 122;
    let plain_gy_land_grant = C_BASE + 123;
    let shock_land = C_BASE + 124;
    let destroy_dev = C_BASE + 125;

    static SERRA_LAND_ABILS: [AbilityDef; 1] = [stat(
        PERMANENT_CARD,
        Modifier::ZonePlayGrant(ZonePlayGrant {
            who: PlayerRel::You,
            zones: AltCastZones::Zones(&[ZoneRef::Graveyard]),
            grant_zone_permission: true,
            allow_lands: true,
            with_flash: false,
            max_mana_value: Some(Amount::Fixed(3)),
            once_per_turn: true,
            price: AltCastPrice::Normal,
            mana_spending: ZonePlayManaSpending::Exact,
            rider: ZonePlayCastRider::GrantDeathExileAndGainLife(2),
            your_turn_only: true,
            as_sorcery_only: false,
            mana_cost_must_contain: None,
        }),
    )];
    static FINALITY_LAND_ABILS: [AbilityDef; 1] = [stat(
        PERMANENT_CARD,
        Modifier::ZonePlayGrant(ZonePlayGrant {
            who: PlayerRel::You,
            zones: AltCastZones::Zones(&[ZoneRef::Graveyard]),
            grant_zone_permission: true,
            allow_lands: true,
            with_flash: false,
            max_mana_value: None,
            once_per_turn: false,
            price: AltCastPrice::Normal,
            mana_spending: ZonePlayManaSpending::Exact,
            rider: ZonePlayCastRider::EntryFinality,
            your_turn_only: true,
            as_sorcery_only: false,
            mana_cost_must_contain: None,
        }),
    )];
    static LEAVE_EXILE_LAND_ABILS: [AbilityDef; 1] = [stat(
        PERMANENT_CARD,
        Modifier::ZonePlayGrant(ZonePlayGrant {
            who: PlayerRel::You,
            zones: AltCastZones::Zones(&[ZoneRef::Graveyard]),
            grant_zone_permission: true,
            allow_lands: true,
            with_flash: false,
            max_mana_value: None,
            once_per_turn: false,
            price: AltCastPrice::Normal,
            mana_spending: ZonePlayManaSpending::Exact,
            rider: ZonePlayCastRider::GrantLeaveExileReplacement,
            your_turn_only: true,
            as_sorcery_only: false,
            mana_cost_must_contain: None,
        }),
    )];
    static PLAIN_LAND_ABILS: [AbilityDef; 1] = [stat(
        PERMANENT_CARD,
        Modifier::ZonePlayGrant(ZonePlayGrant {
            who: PlayerRel::You,
            zones: AltCastZones::Zones(&[ZoneRef::Graveyard]),
            grant_zone_permission: true,
            allow_lands: true,
            with_flash: false,
            max_mana_value: None,
            once_per_turn: false,
            price: AltCastPrice::Normal,
            mana_spending: ZonePlayManaSpending::Exact,
            rider: ZonePlayCastRider::None,
            your_turn_only: true,
            as_sorcery_only: false,
            mana_cost_must_contain: None,
        }),
    )];
    static DESTROY_ABILS: [AbilityDef; 1] = [free(
        &[Effect::Destroy {
            target: TargetSpec::Object(&PERMANENT_CARD),
            no_regen: false,
        }],
        Some(TargetReq::one(TargetSpec::Object(&PERMANENT_CARD))),
    )];
    static SHOCK_MODS: [baylee_cards_dsl::EnterModifier; 1] =
        [baylee_cards_dsl::EnterModifier::TappedOrPayLife(2)];

    let make_shock_face = || FaceDef {
        enter_modifiers: &SHOCK_MODS,
        ..land_face("Shock Crypt")
    };

    // --- Deferred shockland via Serra Paragon ---
    let mut serra_eng = bench(
        19_020,
        vec![
            card(
                serra,
                creature_face("Serra Probe", "{2}{W}{W}", 3, 4),
                KeywordSet::EMPTY,
                &SERRA_LAND_ABILS,
            ),
            card(shock_land, make_shock_face(), KeywordSet::EMPTY, &[]),
            card(
                destroy_dev,
                face("Destroy Device", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                &DESTROY_ABILS,
            ),
        ],
        [
            Seat::with(&[serra, destroy_dev]).holding(&[shock_land]),
            Seat::default(),
        ],
    );
    to_main(&mut serra_eng, me());
    let destroyer = the(&serra_eng, ZoneLocation::Battlefield, destroy_dev);
    let land_id = the(&serra_eng, ZoneLocation::Hand(me()), shock_land);
    move_to_gy(&mut serra_eng, land_id);

    serra_eng
        .apply(me(), PlayerAction::PlayLand { card: land_id })
        .expect("play deferred shockland via Serra Paragon");
    assert!(
        matches!(serra_eng.pending(), Pending::YesNo { player, .. } if *player == me()),
        "shockland must defer entry and ask YesNo for paying 2 life"
    );
    assert_eq!(
        serra_eng.state().object(land_id).unwrap().zone,
        Zone::Graveyard,
        "deferred shockland is still in graveyard while entry replacement is pending"
    );
    let deferred_serra_hash = serra_eng.state().snapshot_hash();

    // Decline paying 2 life -> enters tapped AND keeps GrantDeathExileAndGainLife(2)!
    serra_eng
        .apply(me(), PlayerAction::YesNo(false))
        .expect("decline shockland life payment");
    assert_eq!(
        serra_eng.state().object(land_id).unwrap().zone,
        Zone::Battlefield
    );
    assert!(
        serra_eng
            .state()
            .object(land_id)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
    let life_before = life(&serra_eng, me());
    activate(&mut serra_eng, me(), destroyer, 0);
    serra_eng
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![land_id],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut serra_eng, me());
    assert_eq!(
        serra_eng.state().object(land_id).unwrap().zone,
        Zone::Exile,
        "deferred Serra Paragon shockland must still exile itself on death"
    );
    assert_eq!(
        life(&serra_eng, me()),
        life_before + 2,
        "deferred Serra Paragon shockland must still gain 2 life on death"
    );

    // --- Deferred shockland via EntryFinality ---
    let mut fin_eng = bench(
        19_021,
        vec![
            card(
                finality_land_grant,
                creature_face("Finality Land Granter", "{2}{U}", 2, 2),
                KeywordSet::EMPTY,
                &FINALITY_LAND_ABILS,
            ),
            card(shock_land, make_shock_face(), KeywordSet::EMPTY, &[]),
        ],
        [
            Seat::with(&[finality_land_grant]).holding(&[shock_land]),
            Seat::default(),
        ],
    );
    to_main(&mut fin_eng, me());
    let fin_land = the(&fin_eng, ZoneLocation::Hand(me()), shock_land);
    move_to_gy(&mut fin_eng, fin_land);
    fin_eng
        .apply(me(), PlayerAction::PlayLand { card: fin_land })
        .expect("play deferred shockland via EntryFinality");
    assert!(matches!(fin_eng.pending(), Pending::YesNo { .. }));
    fin_eng.apply(me(), PlayerAction::YesNo(false)).unwrap();
    assert_eq!(
        fin_eng.state().object(fin_land).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        fin_eng
            .state()
            .object(fin_land)
            .unwrap()
            .counters
            .get(CounterKind::Finality),
        1,
        "deferred shockland must enter with Finality counter"
    );

    // --- Deferred shockland via GrantLeaveExileReplacement vs None (tag 573 hash sensitivity) ---
    let mut leave_eng = bench(
        19_022,
        vec![
            card(
                leave_exile_land_grant,
                creature_face("Leave Exile Granter", "{2}{W}", 2, 2),
                KeywordSet::EMPTY,
                &LEAVE_EXILE_LAND_ABILS,
            ),
            card(shock_land, make_shock_face(), KeywordSet::EMPTY, &[]),
            card(
                destroy_dev,
                face("Destroy Device", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                &DESTROY_ABILS,
            ),
        ],
        [
            Seat::with(&[leave_exile_land_grant, destroy_dev]).holding(&[shock_land]),
            Seat::default(),
        ],
    );
    to_main(&mut leave_eng, me());
    let leave_dest = the(&leave_eng, ZoneLocation::Battlefield, destroy_dev);
    let leave_land = the(&leave_eng, ZoneLocation::Hand(me()), shock_land);
    move_to_gy(&mut leave_eng, leave_land);
    leave_eng
        .apply(me(), PlayerAction::PlayLand { card: leave_land })
        .unwrap();
    let deferred_leave_hash = leave_eng.state().snapshot_hash();
    assert_ne!(
        deferred_serra_hash, deferred_leave_hash,
        "tag 573 must distinguish deferred Attempt::zone_play_rider in snapshot_hash"
    );
    leave_eng.apply(me(), PlayerAction::YesNo(false)).unwrap();
    activate(&mut leave_eng, me(), leave_dest, 0);
    leave_eng
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![leave_land],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut leave_eng, me());
    assert_eq!(
        leave_eng.state().object(leave_land).unwrap().zone,
        Zone::Exile,
        "deferred GrantLeaveExileReplacement shockland must exile on leaving BF"
    );

    // Compare deferred Attempt with ZonePlayCastRider::None vs GrantLeaveExileReplacement directly
    // on an identical state setup.
    let mut plain_eng = bench(
        19_022,
        vec![
            card(
                plain_gy_land_grant,
                creature_face("Plain Land Granter", "{2}{W}", 2, 2),
                KeywordSet::EMPTY,
                &PLAIN_LAND_ABILS,
            ),
            card(shock_land, make_shock_face(), KeywordSet::EMPTY, &[]),
        ],
        [
            Seat::with(&[plain_gy_land_grant]).holding(&[shock_land]),
            Seat::default(),
        ],
    );
    to_main(&mut plain_eng, me());
    let plain_land = the(&plain_eng, ZoneLocation::Hand(me()), shock_land);
    move_to_gy(&mut plain_eng, plain_land);
    plain_eng
        .apply(me(), PlayerAction::PlayLand { card: plain_land })
        .unwrap();
    let h_none = plain_eng.state().snapshot_hash();
    let f_none = plain_eng.fingerprint();
    crate::entry::set_entry_zone_play_rider(
        &mut plain_eng.state,
        plain_land,
        ZonePlayCastRider::GrantDeathExileAndGainLife(2),
    );
    let h_death = plain_eng.state().snapshot_hash();
    let f_death = plain_eng.fingerprint();
    crate::entry::set_entry_zone_play_rider(
        &mut plain_eng.state,
        plain_land,
        ZonePlayCastRider::GrantLeaveExileReplacement,
    );
    let h_leave = plain_eng.state().snapshot_hash();
    let f_leave = plain_eng.fingerprint();
    assert_ne!(
        h_none, h_death,
        "tag 573 changes snapshot_hash for None vs DeathExile"
    );
    assert_ne!(
        h_death, h_leave,
        "tag 573 changes snapshot_hash for DeathExile vs LeaveExile"
    );
    assert_ne!(
        f_none, f_death,
        "tag 573 changes Engine::fingerprint for None vs DeathExile"
    );
    assert_ne!(
        f_death, f_leave,
        "tag 573 changes Engine::fingerprint for DeathExile vs LeaveExile"
    );
}

#[test]
fn evoke_grant_suppressed_by_lose_all_abilities_and_grant_warp_pays_cost_parts() {
    let evoke_granter = C_BASE + 130;
    let dress_down = C_BASE + 131;
    let evoke_cr = C_BASE + 132;
    let warp_granter = C_BASE + 133;
    let warp_artifact = C_BASE + 134;

    static EVOKE_ABILS: [AbilityDef; 1] = [stat(
        Filter::CREATURE,
        Modifier::GrantEvoke(Cost {
            mana: ManaCost::parse("{U}"),
            parts: &[],
        }),
    )];
    static HUMILITY_ABILS: [AbilityDef; 1] = [stat(Filter::CREATURE, Modifier::LoseAllAbilities)];
    static WARP_PARTS: [CostPart; 1] = [CostPart::PayLife(2)];
    static WARP_WITH_LIFE_ABILS: [AbilityDef; 1] = [stat(
        Filter::ARTIFACT,
        Modifier::GrantWarp(Cost {
            mana: ManaCost::parse("{1}{R}"),
            parts: &WARP_PARTS,
        }),
    )];

    // --- CR 702.74a: EvokedGrant must NOT trigger sacrifice when abilities are lost ---
    let mut evoke_eng = bench(
        19_023,
        vec![
            card(
                evoke_granter,
                face("Evoke Shrine", "{1}{U}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                &EVOKE_ABILS,
            ),
            card(
                dress_down,
                face("Dress Down Probe", "{1}{U}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                &HUMILITY_ABILS,
            ),
            card(
                evoke_cr,
                creature_face("Evoked Elemental", "{4}{U}", 3, 3),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [
            Seat::with(&[evoke_granter, dress_down]).holding(&[evoke_cr]),
            Seat::default(),
        ],
    );
    to_main(&mut evoke_eng, me());
    let cr_id = the(&evoke_eng, ZoneLocation::Hand(me()), evoke_cr);
    add_pool(&mut evoke_eng, me(), ManaColor::Blue, 1);
    evoke_eng
        .apply(me(), PlayerAction::CastSpell { card: cr_id })
        .expect("cast via GrantEvoke under LoseAllAbilities");
    if let Pending::ChooseCastMode {
        player, options, ..
    } = evoke_eng.pending().clone()
    {
        let idx = options
            .iter()
            .position(|o| matches!(o.kind, CastModeKind::Permission(_)))
            .expect("Evoke mode offered");
        evoke_eng
            .apply(player, PlayerAction::ChooseMode(idx))
            .unwrap();
    }
    settle(&mut evoke_eng, me());
    assert_eq!(
        evoke_eng.state().object(cr_id).unwrap().zone,
        Zone::Battlefield,
        "evoked creature entering under LoseAllAbilities must NOT trigger ETB sacrifice (CR 702.74a)"
    );

    // --- Modifier::GrantWarp(Cost { mana, parts }) pays non-mana CostParts ---
    let mut warp_eng = bench(
        19_024,
        vec![
            card(
                warp_granter,
                creature_face("Life-Warp Probe", "{2}{R}", 3, 3),
                KeywordSet::EMPTY,
                &WARP_WITH_LIFE_ABILS,
            ),
            card(
                warp_artifact,
                FaceDef {
                    power: Some(4),
                    toughness: Some(4),
                    ..face(
                        "Warped Automaton",
                        "{6}",
                        TypeSet::ARTIFACT.union(TypeSet::CREATURE),
                    )
                },
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [
            Seat::with(&[warp_granter]).holding(&[warp_artifact]),
            Seat::default(),
        ],
    );
    to_main(&mut warp_eng, me());
    let art_id = the(&warp_eng, ZoneLocation::Hand(me()), warp_artifact);
    add_pool(&mut warp_eng, me(), ManaColor::Red, 1);
    add_pool(&mut warp_eng, me(), ManaColor::Colorless, 1);

    // Cannot pay 2 life at 1 life -> Warp cast must NOT be offered.
    warp_eng.state.players[usize::from(me().get())].life = 1;
    refresh_offer(&mut warp_eng);
    let (_, low_life_legal) = offer(&warp_eng);
    assert!(
        !low_life_legal.castable.contains(&art_id),
        "GrantWarp with CostPart::PayLife(2) must not be castable at 1 life"
    );

    // At 20 life -> Warp cast IS offered and deducts 2 life on payment.
    warp_eng.state.players[usize::from(me().get())].life = 20;
    refresh_offer(&mut warp_eng);
    let (_, ok_legal) = offer(&warp_eng);
    assert!(
        ok_legal.castable.contains(&art_id),
        "GrantWarp with CostPart::PayLife(2) must be castable at 20 life"
    );
    warp_eng
        .apply(me(), PlayerAction::CastSpell { card: art_id })
        .unwrap();
    if let Pending::ChooseCastMode {
        player, options, ..
    } = warp_eng.pending().clone()
    {
        let idx = options
            .iter()
            .position(|o| o.kind == CastModeKind::Warp)
            .expect("Warp mode offered");
        warp_eng
            .apply(player, PlayerAction::ChooseMode(idx))
            .unwrap();
    }
    settle(&mut warp_eng, me());
    assert_eq!(
        warp_eng.state().object(art_id).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        life(&warp_eng, me()),
        18,
        "GrantWarp must pay its non-mana CostPart::PayLife(2)"
    );
}

#[test]
fn untargeted_sweep_and_amount_nested_filters_activate_replay_gating() {
    let faiths_reward = C_BASE + 140;
    let draw_by_entered = C_BASE + 141;
    let exile_play_sweep = C_BASE + 142;

    static ENTERED_FROM_BF: Filter = Filter::EnteredCurrentZoneThisTurn(Some(ZoneRef::Battlefield));
    static FAITH_ABILS: [AbilityDef; 1] = [spell!(&[Effect::ReturnFromGraveyardOf {
        whose: PlayerRel::You,
        filter: &ENTERED_FROM_BF,
    }])];
    static DRAW_AMOUNT_ABILS: [AbilityDef; 1] = [spell!(&[Effect::DrawCards {
        amount: Amount::DistinctNames {
            zone: ZoneSel::GraveyardOf(PlayerRel::You),
            filter: &ENTERED_FROM_BF,
        },
    }])];
    static EXILE_PLAY_ABILS: [AbilityDef; 1] = [spell!(&[Effect::Play {
        source: baylee_cards_dsl::PlaySource::Zone {
            zone: ZoneRef::Exile,
            owner: PlayerRel::EachPlayer,
        },
        filter: &Filter::ExiledByYou,
        controller: PlayerRel::You,
        count: baylee_cards_dsl::PlayCount::One,
        cost: baylee_cards_dsl::PlayCostMode::Normal,
        cast_only: false,
        copy: false,
        exile_instead_of_graveyard: false,
        optional: true,
    }])];

    let faith_eng = bench(
        19_025,
        vec![card(
            faiths_reward,
            face("Faith's Reward Probe", "{3}{W}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            &FAITH_ABILS,
        )],
        [Seat::default().holding(&[faiths_reward]), Seat::default()],
    );
    assert!(
        faith_eng.state().per_turn.zone_changes.tracked,
        "untargeted ReturnFromGraveyardOf sweep must activate per_turn.zone_changes.tracked"
    );
    assert!(!faith_eng.state().per_turn.track_exilers.0);

    let amt_eng = bench(
        19_026,
        vec![card(
            draw_by_entered,
            face("Amount Nested Probe", "{2}{U}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            &DRAW_AMOUNT_ABILS,
        )],
        [Seat::default().holding(&[draw_by_entered]), Seat::default()],
    );
    assert!(
        amt_eng.state().per_turn.zone_changes.tracked,
        "Amount::DistinctNames filter inside DrawCards must activate per_turn.zone_changes.tracked"
    );

    let exile_eng = bench(
        19_027,
        vec![card(
            exile_play_sweep,
            face("Exile Play Probe", "{2}{R}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            &EXILE_PLAY_ABILS,
        )],
        [
            Seat::default().holding(&[exile_play_sweep]),
            Seat::default(),
        ],
    );
    assert!(
        exile_eng.state().per_turn.track_exilers.0,
        "untargeted Effect::Play with Filter::ExiledByYou must activate per_turn.track_exilers"
    );
}

#[test]
fn hash_sensitivity_covers_all_tags_565_through_572() {
    use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
    use crate::object::Rider;

    let bear = C_BASE + 150;
    let mut engine = bench(
        19_028,
        vec![card(
            bear,
            creature_face("Hash Probe Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        )],
        [Seat::with(&[bear]), Seat::default()],
    );
    to_main(&mut engine, me());
    let bear_id = the(&engine, ZoneLocation::Battlefield, bear);

    // Tags 567..=572: Rider hashes in BOTH snapshot_hash() and loop_signature().
    let base_snap = engine.state().snapshot_hash();
    let base_loop = engine.state().loop_signature();
    let riders_to_test: Vec<(&str, Rider)> = vec![
        (
            "567a GrantedForetellReduce(2)",
            Rider::GrantedForetellReduce(2),
        ),
        (
            "567b GrantedForetellReduce(3)",
            Rider::GrantedForetellReduce(3),
        ),
        ("568 GrantedMadness", Rider::GrantedMadness),
        ("569a ExiledBy(me)", Rider::ExiledBy(me())),
        ("569b ExiledBy(them)", Rider::ExiledBy(them())),
        ("570 EvokedGrant", Rider::EvokedGrant),
        (
            "571a GrantDeathExileAndGainLife(2)",
            Rider::GrantDeathExileAndGainLife(2),
        ),
        (
            "571b GrantDeathExileAndGainLife(3)",
            Rider::GrantDeathExileAndGainLife(3),
        ),
        (
            "572 GrantLeaveExileReplacement",
            Rider::GrantLeaveExileReplacement,
        ),
    ];
    let mut seen_snap = std::collections::BTreeSet::new();
    let mut seen_loop = std::collections::BTreeSet::new();
    seen_snap.insert(base_snap);
    seen_loop.insert(base_loop);
    for (label, rider) in riders_to_test {
        let mut copy = engine.state.clone();
        copy.object_mut(bear_id).unwrap().riders.push(rider);
        let snap = copy.snapshot_hash();
        let lsig = copy.loop_signature();
        assert!(
            seen_snap.insert(snap),
            "snapshot_hash collision for rider {label}"
        );
        assert!(
            seen_loop.insert(lsig),
            "loop_signature collision for rider {label}"
        );
    }

    // Tags 565..=566: Filter::EnteredCurrentZoneThisTurn and Filter::ExiledByYou
    // inside ContinuousEffect filters in snapshot_hash().
    static F_NONE: Filter = Filter::EnteredCurrentZoneThisTurn(None);
    static F_BF: Filter = Filter::EnteredCurrentZoneThisTurn(Some(ZoneRef::Battlefield));
    static F_LIB: Filter = Filter::EnteredCurrentZoneThisTurn(Some(ZoneRef::Library));
    static F_EXILED_BY: Filter = Filter::ExiledByYou;
    static F_ANY: Filter = Filter::Any;

    let filters: [(&str, &'static Filter); 5] = [
        ("565a EnteredCurrentZoneThisTurn(None)", &F_NONE),
        ("565b EnteredCurrentZoneThisTurn(Battlefield)", &F_BF),
        ("565c EnteredCurrentZoneThisTurn(Library)", &F_LIB),
        ("566 ExiledByYou", &F_EXILED_BY),
        ("baseline Any", &F_ANY),
    ];
    let mut filter_snaps = std::collections::BTreeSet::new();
    for (label, filter) in filters {
        let mut copy = engine.state.clone();
        copy.effects.register(ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: Some(bear_id),
            origin: EffectOrigin::Resolution,
            controller: me(),
            layer: baylee_cards_dsl::Layer::Ability,
            filter: EffectFilter::Dsl(filter),
            modifier: Modifier::AddKeyword(KeywordSet::HASTE),
            duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
            timestamp: 1,
        });
        assert!(
            filter_snaps.insert(copy.snapshot_hash()),
            "snapshot_hash collision for filter {label}"
        );
    }
}
