use baylee_core::ids::ObjectId;
use baylee_view::{PlayerView, PublicObject};
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Default)]
pub(super) struct Objects {
    object_ids: BTreeSet<ObjectId>,
    card_faces: BTreeSet<(u32, u8)>,
    tokens: BTreeSet<u16>,
}

impl Objects {
    pub(super) fn observe(&mut self, view: &PlayerView) {
        for object in view
            .battlefield
            .iter()
            .chain(&view.stack)
            .chain(&view.library_tops)
            .chain(&view.looking_at)
            .chain(view.graveyards.iter().flatten())
            .chain(view.exile.iter().flatten())
            .chain(view.command.iter().flatten())
        {
            self.object(object);
        }
        for object in view.hand.iter().chain(
            view.shared_hands
                .iter()
                .chain(&view.controlled_hands)
                .flat_map(|h| &h.cards),
        ) {
            self.object_ids.insert(object.id);
            self.card_faces
                .insert((object.card.index.get(), object.card.face));
        }
        for source in view.target_objects.iter().chain(&view.damage_sources) {
            if let Some(card) = source.card {
                self.card_faces.insert((card.index.get(), card.face));
            }
            if let Some(rules) = source.rules {
                self.card_faces.insert((rules.card.get(), rules.face));
            }
            self.tokens.extend(source.token);
        }
    }

    fn object(&mut self, object: &PublicObject) {
        self.object_ids.insert(object.id);
        if let Some(card) = object.card {
            self.card_faces.insert((card.index.get(), card.face));
        }
        for rules in object.rules.iter().chain(&object.prepared_spell) {
            self.card_faces.insert((rules.card.get(), rules.face));
        }
        self.tokens.extend(object.token);
    }

    pub(super) fn report(&self) -> Value {
        json!({
            "unique_object_handles":self.object_ids.len(),
            "card_faces":self.card_faces,
            "token_indices":self.tokens,
        })
    }
}
