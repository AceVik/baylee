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
        let profile = crate::v2::generation::contract(self.cohort.generator)?.expert();
        let resources = [
            loadout::resources_for(
                &self.builds[0].active,
                self.builds[0].potions,
                0,
                self.cohort.generator,
            )?,
            loadout::resources_for(
                &self.builds[1].active,
                self.builds[1].potions,
                1,
                self.cohort.generator,
            )?,
        ];
        let mut prints = Vec::new();
        let printing = |index| {
            if self.cohort.generator == crate::v2::generation::RUNIC
                && let Some(art) = catalog
                    .card(index)
                    .and_then(|card| card.run)
                    .and_then(|rules| rules.presentation)
            {
                Some(art.print())
            } else {
                printing(index)
            }
        };
        let seats = vec![
            seat(
                &self.builds[0],
                &resources[0],
                profile,
                &mut prints,
                &printing,
            )?,
            seat(
                &self.builds[1],
                &resources[1],
                profile,
                &mut prints,
                &printing,
            )?,
        ];
        let preset = GamePreset {
            format: FormatId::Custom,
            seed: self.shuffle_seed,
            house_rules: loadout::rules_for(&resources, self.policy_seed, self.cohort.generator)?,
            modifiers: Vec::new(),
            prints,
            rules: RulesMetadata::default(),
            seats,
        };
        loadout::validate_for(&preset, self.cohort.generator)?;
        Ok(preset)
    }
}

fn seat(
    build: &Build,
    resources: &loadout::Resources,
    profile: AIProfile,
    prints: &mut Vec<PrintInfo>,
    printing: &impl Fn(CardIndex) -> Option<PrintInfo>,
) -> Result<SeatSpec, Refusal> {
    let deck = entries(&build.deck, prints, printing)?;
    let (sideboard, commanders) = if let Some(formation) = &build.formation {
        (
            entries(&formation.sideboard, prints, printing)?,
            entries(
                &[crate::cards::Stack {
                    card: formation.commander,
                    copies: 1,
                }],
                prints,
                printing,
            )?,
        )
    } else {
        (Vec::new(), Vec::new())
    };
    Ok(SeatSpec {
        controller: if resources.opening.seat == 0 {
            SeatController::Open
        } else {
            SeatController::Ai(profile)
        },
        capabilities: SeatCapabilities::default(),
        deck,
        sideboard,
        commanders,
        starting_life: Some(resources.life),
        starting_hand: None,
        starting_battlefield: Vec::new(),
        emblems: Vec::new(),
        team: None,
    })
}

fn entries(
    stacks: &[crate::cards::Stack],
    prints: &mut Vec<PrintInfo>,
    printing: &impl Fn(CardIndex) -> Option<PrintInfo>,
) -> Result<Vec<DeckEntry>, Refusal> {
    let mut deck = Vec::new();
    for stack in stacks {
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
    Ok(deck)
}
