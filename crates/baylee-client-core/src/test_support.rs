//! Builders for constructing views in tests.
//!
//! Kept in the crate rather than in each test module so that every test
//! describes only the thing it is testing: a test about token grouping should
//! not also be a test about how to spell a `PlayerView`.

use baylee_core::color::ColorSet;
use baylee_core::ids::{CardIndex, ObjectId, PlayerId, PrintRef};
use baylee_core::types::{SupertypeSet, TypeSet};
use baylee_view::{
    AttackerView, BlockerView, CardIdentity, CombatView, Finish, GameStatic, HandObject,
    ObjectStatus, Phase, PlayerView, PrintEntry, PublicObject, SeatIdentity, SeatView, Step,
};

/// A bare creature token controlled by `controller`.
#[must_use]
pub fn token(slot: u32, controller: u8, name: &str, power: i16, toughness: i16) -> PublicObject {
    PublicObject {
        id: ObjectId::new(slot, 0),
        card: None,
        name: name.to_string(),
        controller: PlayerId::new(controller),
        owner: PlayerId::new(controller),
        status: ObjectStatus::NONE,
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        colors: ColorSet::default(),
        keywords: 0,
        power: Some(power),
        toughness: Some(toughness),
        loyalty: None,
        damage: 0,
        counters: Vec::new(),
        attached_to: None,
        targets: Vec::new(),
        stack_item: None,
        summoning_sick: false,
    }
}

/// A card-backed permanent, so tests can exercise the image path.
#[must_use]
pub fn printed(slot: u32, controller: u8, name: &str, print: u16) -> PublicObject {
    let mut obj = token(slot, controller, name, 2, 2);
    obj.card = Some(CardIdentity {
        index: CardIndex::new(u32::from(print)),
        print: PrintRef::new(print),
        face: 0,
    });
    obj
}

/// Assembles a [`PlayerView`] one clause at a time.
pub struct ViewBuilder {
    view: PlayerView,
}

impl ViewBuilder {
    /// A quiet main phase with `seats` players, viewed from seat 0.
    #[must_use]
    pub fn new(seats: u8) -> Self {
        let n = seats as usize;
        Self {
            view: PlayerView {
                seq: 1,
                seat: PlayerId::new(0),
                turn: 3,
                phase: Phase::FirstMain,
                step: Step::Main,
                active: PlayerId::new(0),
                priority: Some(PlayerId::new(0)),
                monarch: None,
                seats: (0..seats)
                    .map(|i| SeatView {
                        player: PlayerId::new(i),
                        life: 40,
                        poison: 0,
                        energy: 0,
                        hand_count: 7,
                        library_count: 80,
                        graveyard_count: 2,
                        has_lost: false,
                        commander_casts: vec![0; n],
                    })
                    .collect(),
                hand: Vec::new(),
                battlefield: Vec::new(),
                stack: Vec::new(),
                graveyards: vec![Vec::new(); n],
                exile: vec![Vec::new(); n],
                command: vec![Vec::new(); n],
                combat: CombatView::default(),
            },
        }
    }

    /// Adds permanents controlled by `controller`.
    #[must_use]
    pub fn with_battlefield(
        mut self,
        controller: u8,
        objects: impl IntoIterator<Item = PublicObject>,
    ) -> Self {
        self.view
            .battlefield
            .extend(objects.into_iter().map(|mut o| {
                o.controller = PlayerId::new(controller);
                o.owner = PlayerId::new(controller);
                o
            }));
        self
    }

    /// Sets the stack, index 0 = bottom.
    #[must_use]
    pub fn with_stack(mut self, objects: Vec<PublicObject>) -> Self {
        self.view.stack = objects;
        self
    }

    /// Sets declared combat.
    #[must_use]
    pub fn with_combat(mut self, attackers: Vec<AttackerView>, blockers: Vec<BlockerView>) -> Self {
        self.view.combat = CombatView {
            attackers,
            blockers,
        };
        self
    }

    /// Fills the local hand from `(name, mana value, object slot)` triples.
    #[must_use]
    pub fn with_hand(mut self, cards: Vec<(&str, u32, u32)>) -> Self {
        self.view.hand = cards
            .into_iter()
            .map(|(name, mana_value, slot)| HandObject {
                id: ObjectId::new(slot, 0),
                card: CardIdentity {
                    index: CardIndex::new(slot),
                    print: PrintRef::new(slot as u16),
                    face: 0,
                },
                name: name.to_string(),
                mana_value,
                colors: ColorSet::default(),
                types: TypeSet::CREATURE,
            })
            .collect();
        self
    }

    /// Overrides who holds priority.
    #[must_use]
    pub fn with_priority(mut self, player: Option<u8>) -> Self {
        self.view.priority = player.map(PlayerId::new);
        self
    }

    /// Finishes the view.
    #[must_use]
    pub fn build(self) -> PlayerView {
        self.view
    }
}

/// A print table covering the first `count` print references.
#[must_use]
pub fn statics(count: u16) -> GameStatic {
    GameStatic {
        view_version: baylee_view::VIEW_VERSION,
        game_id: "test-game".to_string(),
        your_seat: PlayerId::new(0),
        seats: vec![SeatIdentity {
            player: PlayerId::new(0),
            display_name: "You".to_string(),
            is_ai: false,
            team: None,
        }],
        prints: (0..count)
            .map(|i| PrintEntry {
                // A deterministic but well-formed Scryfall-shaped id.
                scryfall_id: format!("{i:08x}-124f-4125-87ab-609be40e774c"),
                lang: "EN".to_string(),
                finish: Finish::Normal,
            })
            .collect(),
    }
}
