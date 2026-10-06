use super::*;
use baylee_core::color::ColorSet;
use baylee_core::ids::{AbilityRef, CardIndex, Defender, ObjectId, PlayerId, PrintRef, SeatSet};
use baylee_core::mana::ManaCost;
use baylee_core::types::{SubtypeSet, SupertypeSet, TypeSet};
use std::borrow::Cow;

mod accessors;
mod display;
mod objects;
mod table;
mod wire_shape;

fn obj(id: u32, controller: u8) -> PublicObject {
    PublicObject {
        mana_value: 0,
        id: ObjectId::new(id, 0),
        card: None,
        rules: None,
        name: "Soldier".to_string(),
        controller: PlayerId::new(controller),
        owner: PlayerId::new(controller),
        commander: false,
        status: ObjectStatus::NONE,
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::default(),
        subtypes: SubtypeSet::EMPTY,
        chosen_subtype: None,
        chosen_name: None,
        chosen_opponent: None,
        unlocked_doors: None,
        suspended: false,
        token: None,
        colors: ColorSet::default(),
        keywords: 0,
        power: Some(1),
        toughness: Some(1),
        base_power: None,
        base_toughness: None,
        loyalty: None,
        damage: 0,
        counters: vec![],
        attached_to: None,
        targets: vec![],
        stack_item: None,
        summoning_sick: false,
        granted_mana: None,
        board_mana: None,
        flashback: None,
        grants: Vec::new(),
        word_changes: Vec::new(),
    }
}

fn view(seats: u8) -> PlayerView {
    PlayerView {
        seq: 1,
        seat: PlayerId::new(0),
        turn: 1,
        phase: Phase::FirstMain,
        step: Step::Main,
        active: PlayerId::new(0),
        awaiting: Some(PlayerId::new(0)),
        deciding: SeatSet::new(),
        decision_remaining_ms: None,
        priority_held: false,
        policy_acts: Vec::new(),
        monarch: None,
        day_night: None,
        seats: (0..seats)
            .map(|i| SeatView {
                mana_pool: ManaPoolView::default(),
                player: PlayerId::new(i),
                life: 40,
                poison: 0,
                energy: 0,
                hand_count: 7,
                no_max_hand_size: false,
                library_count: 93,
                graveyard_count: 0,
                loss: None,
                house_answered: None,
                commanders: vec![],
                commander_damage: vec![],
            })
            .collect(),
        hand: vec![],
        shared_hands: vec![],
        controlled_hands: vec![],
        decision_player: None,
        hand_shared_with: SeatSet::new(),
        hand_requests: SeatSet::new(),
        hand_requested: SeatSet::new(),
        battlefield: vec![],
        stack: vec![],
        graveyards: vec![vec![]; seats as usize],
        exile: vec![vec![]; seats as usize],
        command: vec![vec![]; seats as usize],
        combat: CombatView::default(),
        looking_at: Vec::new(),
        damage_sources: Vec::new(),
        target_objects: Vec::new(),
        library_tops: Vec::new(),
        owed: None,
        targeting: None,
        sorcery_lock: None,
        sorceries_have_flash: false,
    }
}
