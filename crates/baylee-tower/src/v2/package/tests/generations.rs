use super::*;

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
