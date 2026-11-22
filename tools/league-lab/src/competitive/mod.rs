mod calibration;
mod deck_cycle;
mod examples;
mod experiment;
mod fixtures;
mod metrics;
mod queue;
mod rating;
pub(crate) fn run(selection: Option<&str>) {
    experiment::run(selection);
}
pub(crate) fn rating_trials() {
    calibration::run();
}
pub(crate) fn examples() {
    examples::run();
}
pub(crate) fn deck_trials() {
    experiment::deck_trials();
}
