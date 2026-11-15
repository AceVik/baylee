use super::*;

#[test]
fn original_contract_keeps_its_native_revision_and_house_policy() {
    let (catalog, run, mut cohort, mut opponent) = fixture();
    let run = Run::begin_for(&catalog, run.cards, &[3; 32], 1).unwrap();
    // A retained artifact is explicit, not recreated with today's defaults.
    cohort.executor.rules = "tower-2".into();
    cohort.executor.ai = "tower-house-19".into();
    let cohort = Cohort::for_generation(cohort.executor, &catalog, 1).unwrap();
    assert_eq!(cohort.revision, 2);
    opponent.cohort = cohort.clone();
    let package = Package::reserve(
        &run,
        run.revision,
        &catalog,
        &cohort,
        opponent,
        seeds(),
        4096,
    )
    .unwrap()
    .package;
    let preset = package
        .preset(&catalog, &cohort.executor, printing)
        .unwrap();
    assert_eq!(preset.house_rules.rule_pack.as_ref().unwrap().revision, 2);
    let SeatController::Ai(profile) = preset.seats[1].controller else {
        panic!("echo must be house controlled");
    };
    assert_eq!(profile.generation, 19);
    assert_eq!(profile.foresight, 0);
    for field in ["rules", "ai"] {
        let mut changed = cohort.clone();
        if field == "rules" {
            changed.executor.rules = "tower-3".into();
        } else {
            changed.executor.ai = "tower-house-20".into();
        }
        assert!(changed.validate(&catalog).is_err());
    }
}

#[test]
fn unknown_generations_cannot_start_validate_or_reserve_an_execution() {
    let (catalog, run, cohort, opponent) = fixture();
    let build = Build::freeze(&run, &catalog).unwrap();
    for generator in [0, 2, u16::MAX] {
        assert!(catalog_digest_for(&catalog, generator).is_err());
        assert!(Cohort::for_generation(cohort.executor.clone(), &catalog, generator).is_err());
        assert!(Run::begin_for(&catalog, run.cards.clone(), &[3; 32], generator).is_err());
        assert!(run.validate_for(&catalog, generator).is_err());
        assert!(build.validate_for(&catalog, generator).is_err());
        let mut changed = run.clone();
        changed.shop.generator = generator;
        assert!(changed.validate(&catalog).is_err());
        assert!(
            Package::reserve(
                &changed,
                changed.revision,
                &catalog,
                &cohort,
                opponent.clone(),
                seeds(),
                4096
            )
            .is_err()
        );
        let mut changed_cohort = cohort.clone();
        changed_cohort.generator = generator;
        assert!(changed_cohort.validate(&catalog).is_err());
    }
}
