//! The tests of the cards under `cards/artifacts/equipment/mv_2/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

mod aeronaut_s_wings;
mod banshee_s_blade;
mod bladed_pinions;
mod c_2_mace;
mod cloak_of_the_bat;
mod cobbled_wings;
mod crystal_slipper;
mod cultist_s_staff;
mod daredevil_s_billy_club;
mod dowsing_dagger;
mod echo_circlet;
mod empyrial_plate;
mod fleetfeather_sandals;
mod gilded_pinions;
mod glimmerlight;
mod goggles_of_night;
mod gold_pan;
mod goldvein_pick;
mod gorgon_flail;
mod hard_won_jitte;
mod hero_s_heirloom;
mod jackhammer;
mod kitesail;
mod leering_emblem;
mod lightning_greaves;
mod lightning_spear;
mod marauder_s_axe;
mod mask_of_avacyn;
mod mask_of_memory;
mod mask_of_riddles;
mod nim_deathmantle;
mod no_dachi;
mod ogre_s_cleaver;
mod prowler_s_helm;
mod ranger_s_longbow;
mod raven_wings;
mod riot_gear;
mod rogue_s_gloves;
mod sharpened_pitchfork;
mod short_bow;
mod silverskin_armor;
mod specter_s_shroud;
mod swiftfoot_boots;
mod sword_of_the_meek;
mod torch_gauntlet;
mod true_faith_censer;
mod vanguard_s_shield;
mod veteran_s_sidearm;
mod viridian_claw;
mod vulshok_morningstar;
mod warmonger_s_chariot;

// ---------------------------------------------------------------------------
// The rig the generated Equipment share (this directory's and `mv_3`'s).
//
// An Equipment's printed text is "equipped creature ..." plus an Equip cost,
// so one table answers every one of them: the Equipment already on the
// battlefield, exactly as many Forests as its Equip cost (so that the pool
// reading zero afterwards pins the cost to the card), a creature to hold it,
// a second creature of ours that nobody equips, and one across the table.
// ---------------------------------------------------------------------------

/// Jasmine Boreal, a vanilla legendary Human 4/5.
pub(super) fn jasmine_boreal() -> CardIndex {
    card_index("942407bf-862a-4a0b-b19d-b9f8ee833026")
}

/// Goblin Piker, a vanilla red Goblin 2/1.
pub(super) fn goblin_piker() -> CardIndex {
    card_index("50608184-90d3-43d2-a221-deb186c78323")
}

/// Steel Wall, a 0/4 artifact Wall with defender.
pub(super) fn steel_wall() -> CardIndex {
    card_index("5ccb57e1-ca94-4b5a-8e5f-b8b5e692cfb9")
}

/// A table set for one Equipment.
pub(super) struct Rig {
    pub(super) engine: Engine<RegistryLookup>,
    pub(super) card: CardIndex,
    pub(super) equipment: ObjectId,
    pub(super) host: ObjectId,
    pub(super) bystander: ObjectId,
    pub(super) theirs: ObjectId,
}

/// `lands` Forests, the Equipment, `host` and a bystander (a Goblin Piker,
/// or a Steel Wall when the host is the Piker) for us, and a Llanowar Elves
/// for the opponent.
#[track_caller]
pub(super) fn rig(card: CardIndex, host: CardIndex, lands: usize) -> Rig {
    rig_vs(card, host, lands, &[], &[llanowar_elves()], &[])
}

/// The same table with the opponent's board and hand chosen; `theirs` is the
/// first creature named.
#[track_caller]
pub(super) fn rig_vs(
    card: CardIndex,
    host: CardIndex,
    lands: usize,
    our_hand: &[CardIndex],
    their_board: &[CardIndex],
    their_hand: &[CardIndex],
) -> Rig {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let bystander_card = if host == goblin_piker() {
        steel_wall()
    } else {
        goblin_piker()
    };
    let mut board = vec![forest(); lands];
    board.extend([card, host, bystander_card]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, our_hand)
        .battlefield(1, their_board)
        .hand(1, their_hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let equipment = on_battlefield(&engine, p0, card).expect("the Equipment is out");
    let host = on_battlefield(&engine, p0, host).expect("the host is out");
    let bystander = on_battlefield(&engine, p0, bystander_card).expect("the bystander is out");
    let theirs = on_battlefield(&engine, p1, their_board[0]).expect("their creature is out");
    Rig {
        engine,
        card,
        equipment,
        host,
        bystander,
        theirs,
    }
}

impl Rig {
    pub(super) fn attached_to(&self) -> Option<ObjectId> {
        self.engine
            .state()
            .object(self.equipment)
            .and_then(|o| o.attached_to)
    }

    pub(super) fn pool(&self) -> u64 {
        self.engine.state().players[0].mana_pool.total()
    }

    /// Pays the Equip cost out of the Forests and attaches to the host.
    #[track_caller]
    pub(super) fn equip(&mut self, cost: u64) {
        self.equip_keeping(cost, 0);
    }

    /// The same with `spare` more Forests than the cost: the Equip takes its
    /// cost and the rest stays in the pool.
    #[track_caller]
    pub(super) fn equip_keeping(&mut self, cost: u64, spare: u64) {
        let p0 = PlayerId::new(0);
        assert_eq!(self.attached_to(), None, "it enters holding nobody");
        tap_all_mana_but(&mut self.engine, p0, Some(llanowar_elves()));
        assert_eq!(
            self.pool(),
            cost + spare,
            "the Forests make exactly the Equip cost{}",
            if spare > 0 { " and the spare" } else { "" }
        );
        self.equip_now();
        assert_eq!(self.pool(), spare, "the Equip cost left the pool");
    }

    /// Activates Equip (ability 0) with whatever mana floats, answering as a
    /// player would: through the offered targets, choosing the host.
    #[track_caller]
    pub(super) fn equip_now(&mut self) {
        let p0 = PlayerId::new(0);
        activate(&mut self.engine, p0, self.card, 0);
        let Pending::ChooseTargets { options, .. } = self.engine.pending().clone() else {
            panic!("equip targets a creature, got {:?}", self.engine.pending())
        };
        assert!(
            options.contains(&self.host) && options.contains(&self.bystander),
            "both creatures we control are legal: {options:?}"
        );
        assert!(
            !options.contains(&self.theirs) && !options.contains(&self.equipment),
            "\"target creature you control\" offers neither their creature nor the Equipment: {options:?}"
        );
        assert_eq!(options.len(), 2, "and those two are the whole menu");
        let (equipment, host) = (self.equipment, self.host);
        self.engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![host],
                },
            )
            .unwrap();
        pass_until(&mut self.engine, |e| {
            e.state()
                .object(equipment)
                .is_some_and(|o| o.attached_to == Some(host))
        });
    }

    /// Passes priority until the opponent is asked to block and hands back
    /// that question; whatever the attack was has been declared already.
    #[track_caller]
    pub(super) fn run_to_the_blocks(&mut self) -> Pending {
        for _ in 0..40 {
            match self.engine.pending().clone() {
                blocks @ Pending::ChooseBlockers { .. } => return blocks,
                Pending::Priority { player, .. } => {
                    self.engine
                        .apply(player, PlayerAction::PassPriority)
                        .unwrap();
                }
                other => panic!("unexpected on the way to the blockers: {other:?}"),
            }
        }
        panic!("never reached the declare-blockers question")
    }

    /// The host attacks; returns the blockers question the opponent faces.
    #[track_caller]
    pub(super) fn attack_to_the_blocks(&mut self) -> Pending {
        self.attack_with_host();
        self.run_to_the_blocks()
    }

    /// The opponent attacks us with `attackers` on their turn; returns the
    /// blockers question we face.
    #[track_caller]
    pub(super) fn defend_against(&mut self, attackers: &[ObjectId]) -> Pending {
        let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
        reach_their_main_phase(&mut self.engine, p1);
        pass_until(&mut self.engine, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        self.engine
            .apply(
                p1,
                PlayerAction::DeclareAttackers {
                    attackers: attackers
                        .iter()
                        .map(|a| (*a, Defender::Player(p0)))
                        .collect(),
                },
            )
            .unwrap();
        self.run_to_the_blocks()
    }

    /// How many cards sit in our hand.
    pub(super) fn hand_size(&self) -> usize {
        self.engine
            .state()
            .zones
            .list(ZoneLocation::Hand(PlayerId::new(0)))
            .len()
    }

    /// Declares the host as the only attacker against the opponent.
    #[track_caller]
    pub(super) fn attack_with_host(&mut self) {
        let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
        pass_until(&mut self.engine, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        let host = self.host;
        self.engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(host, Defender::Player(p1))],
                },
            )
            .unwrap();
    }

    /// Attacks the opponent with the host and lets the turn run to its end
    /// step, past combat damage (CR 510.2); "you may" is answered yes on the
    /// way.
    #[track_caller]
    pub(super) fn swing_unblocked(&mut self) {
        self.attack_with_host();
        pass_until(&mut self.engine, |e| {
            matches!(e.state().turn.phase, Phase::Ending)
        });
    }
}

/// The whole of a plain static Equipment: before it holds anything the host
/// is untouched; after the Equip, the host (and only the host) has `delta`
/// more power/toughness and every keyword in `gained`.
#[track_caller]
pub(super) fn equipped_gets(
    card: CardIndex,
    cost: u64,
    host: CardIndex,
    delta: (i16, i16),
    gained: KeywordSet,
) -> Rig {
    let mut r = rig(card, host, usize::try_from(cost).unwrap());
    let before = pt(&r.engine, r.host);
    let bystander_before = pt(&r.engine, r.bystander);
    let bystander_keywords = keywords(&r.engine, r.bystander);
    assert_eq!(
        keywords(&r.engine, r.host).bits() & gained.bits(),
        0,
        "the host has none of the granted keywords yet"
    );
    r.equip(cost);
    assert_eq!(
        pt(&r.engine, r.host),
        (before.0 + delta.0, before.1 + delta.1),
        "the host gets {delta:?}"
    );
    assert!(
        gained.bits() == 0 || keywords(&r.engine, r.host).contains(gained),
        "the host has the granted keywords"
    );
    assert_eq!(
        pt(&r.engine, r.bystander),
        bystander_before,
        "the creature nobody equipped is untouched"
    );
    assert_eq!(
        keywords(&r.engine, r.bystander),
        bystander_keywords,
        "and gains no keyword"
    );
    assert_eq!(pt(&r.engine, r.theirs), (1, 1), "nor does theirs");
    assert_eq!(
        keywords(&r.engine, r.equipment).bits() & gained.bits(),
        0,
        "the Equipment grants the keywords and keeps none"
    );
    r
}

/// The subtypes a battlefield object has after the layers.
pub(super) fn subtypes_of(
    engine: &Engine<RegistryLookup>,
    id: ObjectId,
) -> baylee_core::types::SubtypeSet {
    engine
        .state()
        .object(id)
        .expect("object exists")
        .characteristics()
        .subtypes
}

/// The colors a battlefield object has after the layers.
pub(super) fn colors_of(
    engine: &Engine<RegistryLookup>,
    id: ObjectId,
) -> baylee_core::color::ColorSet {
    engine
        .state()
        .object(id)
        .expect("object exists")
        .characteristics()
        .colors
}

/// The plain "the host gets `delta`" test for a host that does **not** meet
/// a conditional static's condition: the keywords the condition would give
/// stay away.
#[track_caller]
pub(super) fn equipped_stays_without(
    card: CardIndex,
    cost: u64,
    host: CardIndex,
    delta: (i16, i16),
    withheld: KeywordSet,
) -> Rig {
    let r = equipped_gets(card, cost, host, delta, KeywordSet::EMPTY);
    assert_eq!(
        keywords(&r.engine, r.host).bits() & withheld.bits(),
        0,
        "this host does not meet the condition, so it gets none of {withheld:?}"
    );
    r
}
