use super::*;
use serde_json::json;

mod optional;

fn metadata() -> Value {
    let mut packages = Vec::new();
    let mut nodes = Vec::new();
    for (name, version, dependencies) in [
        ("central-crypto", "0.1.0", vec!["opaque-ke"]),
        ("opaque-ke", "4.0.1", vec!["voprf", "elliptic-curve"]),
        ("voprf", "0.5.0", vec!["elliptic-curve"]),
        ("elliptic-curve", "0.13.8", vec!["crypto-bigint"]),
        ("crypto-bigint", "0.5.5", vec![]),
    ] {
        let source = if name == "central-crypto" {
            format!(
                "git+https://github.com/AceVik/baylee-central.git?rev={CENTRAL_REV}#{CENTRAL_REV}"
            )
        } else {
            REGISTRY.to_owned()
        };
        packages.push(json!({"name":name,"version":version,"source":source,"id":name}));
        nodes.push(json!({"id":name,"dependencies":dependencies}));
    }
    json!({"packages":packages,"resolve":{"nodes":nodes}})
}

#[test]
fn the_reviewed_sdk_chain_is_accepted() {
    verify(&metadata()).unwrap();
}

#[test]
fn every_reviewed_source_and_version_must_still_match() {
    for index in 0..5 {
        for field in ["version", "source"] {
            let mut changed = metadata();
            changed["packages"][index][field] = json!("different");
            assert!(verify(&changed).is_err(), "package {index}, {field}");
        }
    }
}

#[test]
fn an_additional_consumer_cannot_inherit_the_exception() {
    for target in ["opaque-ke", "voprf", "elliptic-curve", "crypto-bigint"] {
        let mut changed = metadata();
        changed["resolve"]["nodes"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":"new-consumer","dependencies":[target]}));
        assert!(verify(&changed).is_err(), "new consumer of {target}");
    }
}

#[test]
fn duplicate_or_missing_packages_and_incomplete_graphs_are_rejected() {
    for index in 0..5 {
        let mut missing = metadata();
        missing["packages"].as_array_mut().unwrap().remove(index);
        assert!(verify(&missing).is_err());
        let mut duplicate = metadata();
        let package = duplicate["packages"][index].clone();
        duplicate["packages"].as_array_mut().unwrap().push(package);
        assert!(verify(&duplicate).is_err());
    }
    for changed in [
        json!({}),
        json!({"packages":[]}),
        json!({"packages":[],"resolve":{"nodes":[]}}),
    ] {
        assert!(verify(&changed).is_err());
    }
    let mut malformed = metadata();
    malformed["resolve"]["nodes"][0]["dependencies"] = Value::Null;
    assert!(verify(&malformed).is_err());
}
