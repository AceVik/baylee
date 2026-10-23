use super::{Forest, TreeEncode};
use baylee_cards_dsl::{AbilityDef, CardDef};

impl Forest {
    fn ability(&mut self, key: &str, ability: &AbilityDef) {
        let root = ability.tree(self);
        self.roots.insert(key.into(), root);
        let modes = match ability {
            AbilityDef::ModalSpell { modes, .. }
            | AbilityDef::ModalActivated { modes, .. }
            | AbilityDef::ModalTriggered { modes, .. } => *modes,
            _ => return,
        };
        for (i, mode) in modes.iter().enumerate() {
            let root = mode.tree(self);
            self.roots.insert(format!("{key}/m/{i}"), root);
        }
    }
    fn card(&mut self, def: &CardDef) {
        let mut faces = vec![];
        for (face, value) in def.faces.iter().enumerate() {
            let children = vec![
                value.tree(self),
                def.keywords_for_face(face).tree(self),
                def.abilities_for_face(face).tree(self),
            ];
            faces.push(self.node("FaceRules", &[], children));
        }
        let all_faces = self.node("AllFaces", &[], faces);
        for (face, value) in def.faces.iter().enumerate() {
            let children = vec![
                value.tree(self),
                def.color_identity.tree(self),
                def.commander.tree(self),
                def.partner.tree(self),
                def.keywords_for_face(face).tree(self),
                def.abilities_for_face(face).tree(self),
                all_faces,
            ];
            let root = self.node("CardFace", &[], children);
            self.roots
                .insert(format!("c/{}/{face}", def.index.get()), root);
            let identity = self.node("CardIdentity", &def.index.get().to_le_bytes(), vec![]);
            let identity = self.node("FaceIdentity", &(face as u32).to_le_bytes(), vec![identity]);
            self.roots
                .insert(format!("i/{}/{face}", def.index.get()), identity);
            for (i, ab) in def.abilities_for_face(face).iter().enumerate() {
                self.ability(&format!("a/{}/{face}/{i}", def.index.get()), ab);
            }
            for (i, cost) in value.alternative_costs.iter().enumerate() {
                let root = cost.tree(self);
                self.roots
                    .insert(format!("alt/{}/{face}/{i}", def.index.get()), root);
            }
        }
    }
}

/// Build the exact structure catalog. The caller supplies its versioned public
/// vocabulary; every printed ability and every mode is retained.
///
/// # Errors
/// If a supplied public card identity has no compiled definition.
pub fn catalog(cards: &[u32]) -> anyhow::Result<Forest> {
    let mut forest = Forest::new();
    for special in (u32::MAX - 11)..=u32::MAX {
        let node = forest.node("ReservedAbility", &special.to_le_bytes(), vec![]);
        forest.roots.insert(format!("q/{special}"), node);
    }
    let mut cards = cards.to_vec();
    cards.sort_unstable();
    cards.dedup();
    for cid in cards {
        let def = baylee_cards::by_index(baylee_core::ids::CardIndex::new(cid))
            .ok_or_else(|| anyhow::anyhow!("missing card definition {cid}"))?;
        forest.card(def);
    }
    for (i, tok) in baylee_cards::tokens::ALL.iter().enumerate() {
        let root = tok.tree(&mut forest);
        forest.roots.insert(format!("t/{i}"), root);
        for (j, ab) in tok.abilities.iter().enumerate() {
            forest.ability(&format!("ta/{i}/{j}"), ab);
        }
    }
    Ok(forest)
}
