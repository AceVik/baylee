use super::{
    Build, CardIndex, Catalog, GamePreset, Identity, Package, PrintInfo, Refusal, loadout,
};
use baylee_core::{
    card_metadata::{DeckChoices, RulesMetadata},
    ids::PrintRef,
    preset::{AIProfile, DeckEntry, FormatId, SeatCapabilities, SeatController, SeatSpec},
};

impl Package {
    /// Derive the complete native preset from frozen, validated resources.
    /// `printing` must resolve the exact executor's canonical registry, not player URLs.
    /// # Errors
    /// Wrong artifact/catalog, unavailable printing or invalid native admission.
    pub fn preset(
        &self,
        catalog: &Catalog,
        executor: &Identity,
        printing: impl Fn(CardIndex) -> Option<PrintInfo>,
    ) -> Result<GamePreset, Refusal> {
        self.validate(catalog, executor)?;
        let resources = [
            loadout::resources(&self.builds[0].active, self.builds[0].potions, 0)?,
            loadout::resources(&self.builds[1].active, self.builds[1].potions, 1)?,
        ];
        let mut prints = Vec::new();
        let seats = vec![
            seat(&self.builds[0], &resources[0], &mut prints, &printing)?,
            seat(&self.builds[1], &resources[1], &mut prints, &printing)?,
        ];
        let preset = GamePreset {
            format: FormatId::Custom,
            seed: self.shuffle_seed,
            house_rules: loadout::rules(&resources, self.policy_seed)?,
            modifiers: Vec::new(),
            prints,
            rules: RulesMetadata::default(),
            seats,
        };
        loadout::validate(&preset)?;
        Ok(preset)
    }
}

fn seat(
    build: &Build,
    resources: &loadout::Resources,
    prints: &mut Vec<PrintInfo>,
    printing: &impl Fn(CardIndex) -> Option<PrintInfo>,
) -> Result<SeatSpec, Refusal> {
    let mut deck = Vec::new();
    for stack in &build.deck {
        let print = PrintRef::new(u16::try_from(prints.len()).map_err(|_| Refusal::Capacity)?);
        prints.push(printing(stack.card).ok_or(Refusal::Unavailable)?);
        deck.extend(std::iter::repeat_n(
            DeckEntry {
                card: stack.card,
                print,
                rule_print: None,
                choices: DeckChoices::default(),
            },
            usize::from(stack.copies),
        ));
    }
    Ok(SeatSpec {
        controller: if resources.opening.seat == 0 {
            SeatController::Open
        } else {
            SeatController::Ai(AIProfile::TOWER_EXPERT)
        },
        capabilities: SeatCapabilities::default(),
        deck,
        sideboard: Vec::new(),
        commanders: Vec::new(),
        starting_life: Some(resources.life),
        starting_hand: None,
        starting_battlefield: Vec::new(),
        emblems: Vec::new(),
        team: None,
    })
}
