use super::*;
use crate::features4::HeadKind4;
use baylee_core::{
    ids::{ObjectId, PlayerId, PrintRef},
    preset::{AIProfile, DeckEntry, FormatId},
};
use baylee_engine::choice::PlayerAction;
use baylee_gamehost::Session;
use baylee_view::Phase;
use serde_json::{Value, json};
pub(super) const OWNER: PlayerId = PlayerId::new(0);
pub(super) const OTHER: PlayerId = PlayerId::new(1);
fn card(name: &str) -> DeckEntry {
    DeckEntry {
        card: baylee_cards::all()
            .find(|c| c.name() == name)
            .unwrap()
            .index,
        print: PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    }
}
pub(super) struct Fixture {
    pub session: Session,
    knowledge: Vec<SeatKnowledge4>,
    pub expected: Vec<Value>,
}
impl Fixture {
    pub fn before_x() -> (Self, ObjectId) {
        let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
        let mut preset =
            crate::selfplay::table_for(12, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
        preset.format = FormatId::Freeform;
        preset.modifiers.clear();
        for seat in &mut preset.seats {
            seat.deck = vec![card("Forest"); 32];
            seat.commanders.clear();
            seat.sideboard.clear();
            seat.starting_life = Some(20);
            seat.starting_hand = Some(vec![]);
            seat.starting_battlefield = vec![];
        }
        preset.seats[0].starting_hand = Some(vec![card("Fireball")]);
        preset.seats[0].starting_battlefield =
            vec![card("Mountain"), card("Wastes"), card("Wastes")];
        let mut session =
            Session::new_recorded(&preset, "encoder9-fixed-announcement-fixture").unwrap();
        session.take_over(OWNER);
        session.take_over(OTHER);
        let knowledge = (0..2)
            .map(|s| SeatKnowledge4::from_preset(&preset, PlayerId::new(s)))
            .collect();
        let mut f = Self {
            session,
            knowledge,
            expected: vec![],
        };
        for _ in 0..64 {
            if f.session.observe_view(OWNER).phase == Phase::FirstMain
                && matches!(f.session.pending(),Pending::Priority{player,..}if *player==OWNER)
            {
                break;
            }
            let action = match f.session.pending() {
                Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                Pending::Priority { .. } => PlayerAction::PassPriority,
                p => panic!("{p:?}"),
            };
            f.act(action);
        }
        let sources = f
            .session
            .observe_view(OWNER)
            .battlefield
            .iter()
            .filter(|o| o.controller == OWNER)
            .map(|o| o.id)
            .collect::<Vec<_>>();
        for source in sources {
            let action = if matches!(f.session.pending(),Pending::Priority{legal,..}if legal.mana_abilities.contains(&source))
            {
                PlayerAction::ActivateManaAbility { source }
            } else {
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 0,
                }
            };
            f.act(action);
        }
        let source = f
            .session
            .observe_view(OWNER)
            .hand
            .iter()
            .find(|o| o.name == "Fireball")
            .unwrap()
            .id;
        f.act(PlayerAction::CastSpell { card: source });
        while matches!(f.session.pending(), Pending::ChooseCastMode { .. }) {
            f.act(PlayerAction::ChooseMode(0));
        }
        assert!(matches!(f.session.pending(), Pending::ChooseNumber { .. }));
        (f, source)
    }
    pub fn act(&mut self, action: PlayerAction) {
        let actor = self.session.pending().asked().unwrap();
        let (pending, view) = self
            .session
            .view_for_with_announcement_context(actor)
            .unwrap();
        assert_eq!(
            view,
            self.session.observe_view_with_announcement_context(actor)
        );
        for s in 0..2 {
            let seat = PlayerId::new(s);
            let passive = if seat == actor {
                view.clone()
            } else {
                self.session.observe_view_with_announcement_context(seat)
            };
            if self.knowledge[usize::from(s)].observe_with_announcement_context(&passive) {
                self.expected.push(json!({"seat":s,"is_decision":false,"state":encode_state9(&passive,None,Some(&self.knowledge[usize::from(s)]),&[])}));
            }
        }
        let state = encode_state9(
            &view,
            Some(&pending),
            Some(&self.knowledge[usize::from(actor.get())]),
            &[],
        );
        let candidates = crate::features7::encode_candidates7_grouped(&state, &view, &pending);
        let forced = match candidates.head {
            HeadKind4::SingleChoice => candidates.candidates.len() <= 1,
            HeadKind4::SubsetPick => candidates.candidates.is_empty(),
            HeadKind4::PairPick => candidates.pairs.is_empty(),
            HeadKind4::ScalarValue => candidates.max_choices <= candidates.min_choices,
        };
        if !forced {
            self.expected
                .push(json!({"seat":actor.get(),"is_decision":true,"state":state}));
        }
        self.session.act(actor, action).unwrap();
    }
}
