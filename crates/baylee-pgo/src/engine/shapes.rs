//! Tower's fixed human seat must be driven by frames, never an AI-only pump.
use super::{Failure, PROVEN_ON, TRAINED_ON};

pub(super) struct Shapes {
    pub trained: &'static [&'static str],
    pub proven: &'static [&'static str],
}

impl Shapes {
    pub fn for_args(args: &[String]) -> Result<Self, Failure> {
        let mut server = false;
        let mut workload = false;
        let mut next_features = false;
        for arg in args {
            let features = if next_features {
                Some(arg.as_str())
            } else {
                arg.strip_prefix("--features=")
                    .or_else(|| arg.strip_prefix("-F"))
            };
            next_features = arg == "--features" || arg == "-F";
            for feature in features.into_iter().flat_map(|list| list.split([',', ' '])) {
                match feature {
                    "baylee-engine-server/tower" => server = true,
                    "baylee-perf/tower" => workload = true,
                    "tower" => {
                        return Err(Failure(
                            "name both Tower package features explicitly".into(),
                        ));
                    }
                    _ => {}
                }
            }
        }
        if server != workload {
            return Err(Failure(
                "Tower shipping and workload registries must match".into(),
            ));
        }
        Ok(if server {
            Self {
                trained: &["frames"],
                proven: &["frames"],
            }
        } else {
            Self {
                trained: &TRAINED_ON,
                proven: &PROVEN_ON,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EngineArtifact;

    #[test]
    fn tower_drives_a_human_seat_and_never_proves_a_different_registry() {
        for artifact in EngineArtifact::ALL {
            let shapes = Shapes::for_args(&artifact.cargo_args()).unwrap();
            if artifact == EngineArtifact::Tower {
                assert_eq!(shapes.trained, ["frames"]);
                assert_eq!(shapes.proven, ["frames"]);
            } else {
                assert_eq!(shapes.trained, TRAINED_ON);
                assert_eq!(shapes.proven, PROVEN_ON);
            }
        }
        for feature in ["tower", "baylee-engine-server/tower", "baylee-perf/tower"] {
            for prefix in ["--features=", "-F"] {
                assert!(Shapes::for_args(&[format!("{prefix}{feature}")]).is_err());
            }
            assert!(Shapes::for_args(&["--features".into(), feature.into()]).is_err());
        }
    }
}
