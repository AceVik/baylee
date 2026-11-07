//! Which export of a net an evaluation runs. FP16 is the default where it
//! holds: on the GPU, for an evaluation, when `export_onnx4.py` wrote one next
//! to the FP32 graphs (`MODEL/fp16`). Collection keeps FP32, because PPO
//! admission refuses anything else, and the CPU keeps it because FP16 graphs
//! gain nothing there.
use super::collector::Purpose;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub(super) enum Precision {
    /// `float16` when the rule above allows it, the model as given otherwise.
    Auto,
    /// `MODEL/fp16`, or an error: a number must not silently change its precision.
    Float16,
    /// The model as given.
    Float32,
}

fn half_export(model: &Path) -> Option<PathBuf> {
    let half = model.join("fp16");
    half.join("model.onnx.json").is_file().then_some(half)
}

/// The graphs `model` means under these settings.
///
/// # Errors
/// `--precision float16` on a collection, or with no export to read.
pub(super) fn resolve(
    model: &Path,
    precision: Precision,
    purpose: Purpose,
    on_gpu: bool,
) -> anyhow::Result<PathBuf> {
    let evaluation = purpose == Purpose::Evaluation;
    match precision {
        Precision::Float32 => Ok(model.to_path_buf()),
        Precision::Auto if !evaluation || !on_gpu => Ok(model.to_path_buf()),
        Precision::Auto => Ok(half_export(model).unwrap_or_else(|| model.to_path_buf())),
        Precision::Float16 => {
            anyhow::ensure!(
                evaluation,
                "a collection stays FP32: PPO admission reads it"
            );
            half_export(model).ok_or_else(|| {
                anyhow::anyhow!(
                    "{} has no fp16 export: run export_onnx4.py, or pass --precision float32",
                    model.display()
                )
            })
        }
    }
}

/// Whether a player's sessions run on the GPU (`netplay4::load_session`).
pub(super) fn on_gpu() -> bool {
    std::env::var("BAYLEE_EP").as_deref() == Ok("cuda")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A model directory for one test, gone when it ends.
    struct Model(PathBuf);

    impl Model {
        fn new(name: &str, with_half: bool) -> Self {
            let root = std::env::temp_dir().join(format!("eval4-{}-{name}", std::process::id()));
            std::fs::create_dir_all(root.join("fp16")).unwrap();
            if with_half {
                std::fs::write(root.join("fp16/model.onnx.json"), "{}").unwrap();
            }
            Self(root)
        }
    }

    impl Drop for Model {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn an_evaluation_on_the_gpu_reads_the_half_export() {
        let model = Model::new("gpu", true);
        let chosen = resolve(&model.0, Precision::Auto, Purpose::Evaluation, true).unwrap();
        assert_eq!(chosen, model.0.join("fp16"));
    }

    #[test]
    fn float32_stays_selectable() {
        let model = Model::new("full", true);
        let chosen = resolve(&model.0, Precision::Float32, Purpose::Evaluation, true).unwrap();
        assert_eq!(chosen, model.0);
    }

    #[test]
    fn the_cpu_and_a_model_with_no_export_keep_what_was_given() {
        let with = Model::new("cpu", true);
        let without = Model::new("none", false);
        for (model, gpu) in [(&with, false), (&without, true)] {
            let chosen = resolve(&model.0, Precision::Auto, Purpose::Evaluation, gpu).unwrap();
            assert_eq!(chosen, model.0);
        }
    }

    #[test]
    fn a_collection_never_reads_the_half_export() {
        let model = Model::new("collect", true);
        let chosen = resolve(&model.0, Precision::Auto, Purpose::OnPolicyTraining, true);
        assert_eq!(chosen.unwrap(), model.0);
        assert!(
            resolve(
                &model.0,
                Precision::Float16,
                Purpose::OnPolicyTraining,
                true
            )
            .is_err()
        );
    }

    #[test]
    fn asking_for_a_half_export_that_is_not_there_fails() {
        let model = Model::new("missing", false);
        assert!(resolve(&model.0, Precision::Float16, Purpose::Evaluation, true).is_err());
    }
}
