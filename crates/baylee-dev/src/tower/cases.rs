//! Exportable inputs with manually specified expected results, stable within a revision.
use super::{
    execution::{self, Artifact, Contract, Detection, Mode, Variant},
    recipes::{self, Piece, Rarity},
};
use anyhow::{Result, ensure};
use serde::Serialize;
use std::collections::BTreeSet;

pub(super) const CONTRACT_REVISION: u32 = 1;

#[derive(Clone, Serialize)]
pub(super) struct FusionCase {
    pub id: String,
    pub input: recipes::Input,
    pub expected: Option<recipes::Output>,
}

#[derive(Clone, Serialize)]
pub(super) struct ExecutionCase {
    pub id: String,
    pub input: execution::Input,
    pub expected: Option<Variant>,
}

#[derive(Clone, Serialize)]
pub(super) struct ResaleCase {
    pub id: String,
    pub gold_basis: u32,
    pub catalog_ceiling: u32,
    pub expected: u32,
}

#[derive(Clone, Serialize)]
pub(super) struct Suite {
    pub schema: u32,
    pub contract_revision: u32,
    pub scope: &'static str,
    pub fusions: Vec<FusionCase>,
    pub resales: Vec<ResaleCase>,
    pub executions: Vec<ExecutionCase>,
}

impl Suite {
    pub fn check(&self) -> Result<()> {
        super::schema(self.schema, self.contract_revision)?;
        let mut ids = BTreeSet::new();
        for id in self
            .fusions
            .iter()
            .map(|c| &c.id)
            .chain(self.resales.iter().map(|c| &c.id))
            .chain(self.executions.iter().map(|c| &c.id))
        {
            ensure!(
                super::bounded_label(id) && ids.insert(id),
                "invalid or duplicate reference ID"
            );
        }
        ensure!(
            !self.fusions.is_empty() && !self.resales.is_empty() && !self.executions.is_empty(),
            "empty reference corpus"
        );
        for case in &self.fusions {
            ensure!(
                recipes::quote(&case.input) == case.expected,
                "fusion reference differs: {}",
                case.id
            );
        }
        for case in &self.executions {
            ensure!(
                execution::select(&case.input) == case.expected,
                "execution reference differs: {}",
                case.id
            );
        }
        for case in &self.resales {
            ensure!(
                recipes::resale(case.gold_basis, case.catalog_ceiling) == case.expected,
                "resale reference differs: {}",
                case.id
            );
        }
        Ok(())
    }
}

pub(super) fn suite() -> Suite {
    Suite {
        schema: 1,
        contract_revision: CONTRACT_REVISION,
        scope: "reference_contract_only",
        fusions: fusion_cases(),
        resales: resale_cases(),
        executions: execution_cases(),
    }
}

fn resale_cases() -> Vec<ResaleCase> {
    [
        ("resale-granted", 0, 25, 0),
        ("resale-odd-basis", 13, 25, 6),
        ("resale-catalog-cap", 60, 25, 25),
        ("resale-half-basis", 50, 100, 25),
        ("resale-nonsellable", 100, 0, 0),
    ]
    .into_iter()
    .map(|(id, gold_basis, catalog_ceiling, expected)| ResaleCase {
        id: id.into(),
        gold_basis,
        catalog_ceiling,
        expected,
    })
    .collect()
}

fn fusion(
    id: &str,
    rarity: Rarity,
    count: usize,
    basis: u32,
    potions: Vec<u32>,
    expected: Option<(Rarity, u32)>,
) -> FusionCase {
    FusionCase {
        id: id.into(),
        input: recipes::Input {
            pieces: vec![
                Piece {
                    family: "heartwood".into(),
                    definition: 1,
                    rarity,
                    gold_basis: basis
                };
                count
            ],
            potion_bases: potions,
        },
        expected: expected.map(|(rarity, gold_basis)| recipes::Output { rarity, gold_basis }),
    }
}

fn fusion_cases() -> Vec<FusionCase> {
    use Rarity::{Common, Mythic, Rare, Uncommon};
    let mut cases = vec![
        fusion("fusion-common", Common, 3, 4, vec![], Some((Uncommon, 12))),
        fusion("fusion-uncommon", Uncommon, 3, 10, vec![], Some((Rare, 30))),
        fusion("fusion-mythic", Rare, 1, 26, vec![24], Some((Mythic, 50))),
        fusion("fusion-granted", Rare, 1, 0, vec![0], Some((Mythic, 0))),
        fusion("fusion-empty", Common, 0, 0, vec![], None),
        fusion("fusion-two-common", Common, 2, 4, vec![], None),
        fusion("fusion-three-rare", Rare, 3, 26, vec![24], None),
        fusion("fusion-no-potion", Rare, 1, 26, vec![], None),
        fusion("fusion-extra-potion", Rare, 1, 26, vec![24, 24], None),
        fusion("fusion-wrong-potion-stage", Common, 3, 4, vec![24], None),
        fusion("fusion-above-mythic", Mythic, 3, 50, vec![24], None),
        fusion("fusion-overflow", Common, 3, u32::MAX, vec![], None),
    ];
    let mut mixed = fusion("fusion-mixed-family", Common, 3, 4, vec![], None);
    mixed.input.pieces[1].family = "compass".into();
    cases.push(mixed);
    let mut mixed = fusion("fusion-mixed-definition", Common, 3, 4, vec![], None);
    mixed.input.pieces[1].definition = 2;
    cases.push(mixed);
    let mut mixed = fusion("fusion-mixed-rarity", Common, 3, 4, vec![], None);
    mixed.input.pieces[1].rarity = Uncommon;
    cases.push(mixed);
    cases
}

fn artifact(variant: Variant) -> Artifact {
    let tower = variant == Variant::Tower;
    Artifact {
        contract: Contract {
            variant,
            build: "synthetic-build-1".into(),
            feature_fingerprint: match variant {
                Variant::Standard => "synthetic-standard-off",
                Variant::StandardLoops => "synthetic-standard-loops",
                Variant::Tower => "synthetic-tower-off",
            }
            .into(),
            rule_pack: if tower { "tower-1" } else { "standard-1" }.into(),
            card_pool: "synthetic-cards-1".into(),
            ai_policy: if tower {
                "tower-expert-1"
            } else {
                "standard-ai-1"
            }
            .into(),
        },
        loop_detection: variant == Variant::StandardLoops,
        tower_rules: tower,
        tower_ai: tower,
    }
}

fn execution(id: &str, mode: Mode, detection: Detection, variant: Variant) -> ExecutionCase {
    ExecutionCase {
        id: id.into(),
        input: execution::Input {
            mode,
            detection,
            required: artifact(variant).contract,
            installed: vec![
                artifact(Variant::Standard),
                artifact(Variant::StandardLoops),
                artifact(Variant::Tower),
            ],
            free_agent_slots: 1,
        },
        expected: Some(variant),
    }
}

fn execution_cases() -> Vec<ExecutionCase> {
    let mut cases = vec![
        execution(
            "execution-standard-default",
            Mode::Standard,
            Detection::Off,
            Variant::Standard,
        ),
        execution(
            "execution-standard-enabled",
            Mode::Standard,
            Detection::Break,
            Variant::StandardLoops,
        ),
        execution(
            "execution-standard-draw-policy",
            Mode::Standard,
            Detection::Draw,
            Variant::StandardLoops,
        ),
        execution(
            "execution-tower",
            Mode::Tower,
            Detection::Off,
            Variant::Tower,
        ),
    ];
    let base = cases[3].clone();
    let mut refuse = |id: &str, change: fn(&mut execution::Input)| {
        let mut case = base.clone();
        case.id = id.into();
        case.expected = None;
        change(&mut case.input);
        cases.push(case);
    };
    refuse("execution-tower-detection", |i| {
        i.detection = Detection::Break;
    });
    refuse("execution-no-fallback", |i| {
        i.installed.pop();
    });
    refuse("execution-shared-capacity", |i| i.free_agent_slots = 0);
    refuse("execution-build-mismatch", |i| {
        i.required.build = "other-build".into();
    });
    refuse("execution-feature-mismatch", |i| {
        i.required.feature_fingerprint = "other-features".into();
    });
    refuse("execution-pack-mismatch", |i| {
        i.required.rule_pack = "other-pack".into();
    });
    refuse("execution-cards-mismatch", |i| {
        i.required.card_pool = "other-cards".into();
    });
    refuse("execution-ai-mismatch", |i| {
        i.required.ai_policy = "other-ai".into();
    });
    refuse("execution-variant-mismatch", |i| {
        i.required.variant = Variant::Standard;
    });
    refuse("execution-detection-leak", |i| {
        i.installed[2].loop_detection = true;
    });
    refuse("execution-missing-tower-ai", |i| {
        i.installed[2].tower_ai = false;
    });
    refuse("execution-duplicate", |i| {
        i.installed.push(i.installed[2].clone());
    });
    refuse("execution-path-instead-of-id", |i| {
        i.required.build = "../../engine".into();
    });
    refuse("execution-too-many-artifacts", |i| {
        i.installed = vec![i.installed[2].clone(); 65];
    });
    cases
}
