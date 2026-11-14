//! Exact recipes evaluated against owned instances, with one final-state validation.
use super::{Definition, Deserialize, Economy, Refusal, Relic, Serialize, Slot};
use crate::v2::catalog::Rarity;

/// Player-selected inputs and explicit output location; output and price are derived.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    /// Three identical Common/Uncommon instances, or one Rare.
    pub relics: Vec<u32>,
    /// Required exactly once for Rare-to-Mythic, forbidden for other recipes.
    pub mythic: Option<u32>,
    /// Active or reserve output slot; must be valid after inputs are consumed.
    pub destination: Slot,
}

impl Economy {
    /// Consume an exact recipe atomically, deriving the new definition and basis.
    /// # Errors
    /// Missing/duplicate/wrong inputs, invalid recipe or impossible final loadout.
    pub fn fused(&self, recipe: &Recipe) -> Result<Self, Refusal> {
        self.changed(|next| next.fuse(recipe))
    }

    /// Purchase one recipe input and fuse in one transaction, including a full bag.
    /// `owned.relics` contains only existing inputs; the new copy is appended once.
    /// The authority must consume the saved shop offer in the same transaction.
    /// # Errors
    /// Invalid purchase/recipe or final state. No intermediate item is committed.
    pub fn bought_and_fused(
        &self,
        definition: Definition,
        owned: &Recipe,
    ) -> Result<Self, Refusal> {
        if owned.relics.len() > 2 {
            return Err(Refusal::Invalid);
        }
        self.changed(|next| {
            let id = next.purchase(definition, owned.destination)?;
            let mut recipe = owned.clone();
            recipe.relics.push(id);
            next.fuse(&recipe)
        })
    }

    fn fuse(&mut self, recipe: &Recipe) -> Result<(), Refusal> {
        if !matches!(recipe.relics.len(), 1 | 3) || !recipe.destination.valid() {
            return Err(Refusal::Invalid);
        }
        let mut selected = Vec::with_capacity(recipe.relics.len());
        for (index, id) in recipe.relics.iter().enumerate() {
            if recipe.relics[..index].contains(id) {
                return Err(Refusal::Invalid);
            }
            selected.push(
                self.inventory
                    .relics
                    .iter()
                    .find(|item| item.id == *id)
                    .ok_or(Refusal::Unavailable)?,
            );
        }
        let definition = selected[0].definition;
        if selected.iter().any(|item| item.definition != definition) {
            return Err(Refusal::Invalid);
        }
        let rarity = match (definition.rarity, selected.len(), recipe.mythic) {
            (Rarity::Common, 3, None) => Rarity::Uncommon,
            (Rarity::Uncommon, 3, None) => Rarity::Rare,
            (Rarity::Rare, 1, Some(_)) => Rarity::MythicRare,
            _ => return Err(Refusal::Invalid),
        };
        let mut basis = selected.iter().map(|item| item.basis).sum::<u16>();
        if let Some(id) = recipe.mythic {
            let index = self
                .inventory
                .materials
                .iter()
                .position(|item| item.id == id)
                .ok_or(Refusal::Unavailable)?;
            basis += self.inventory.materials.remove(index).basis;
        }
        self.inventory
            .relics
            .retain(|item| !recipe.relics.contains(&item.id));
        let id = self.inventory.allocate()?;
        self.inventory.relics.push(Relic {
            id,
            definition: Definition {
                rarity,
                ..definition
            },
            slot: recipe.destination,
            basis,
        });
        Ok(())
    }
}
