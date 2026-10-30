use super::*;

pub(super) fn source(root: &Path, expected: &str) -> Result<(Value, source::Source)> {
    let metadata = read_json(&bound(root, "research-cache.manifest.json")?)?;
    ensure!(
        metadata["schema"] == CACHE_SCHEMA
            && metadata["status"] == "indexed_research_frames_only_unadmitted"
            && metadata["expected_conversion_manifest_sha256"] == expected
            && metadata["admitted"] == false
            && metadata["ppo_eligible"] == false
            && metadata["tensor_cache"] == false
            && metadata["ordinary_target_cache_compatible"] == false
            && metadata["probability_replay_required"] == true
            && metadata["producer_binary_certified"] == false
            && metadata["format"] == format_contract()
            && metadata["builder_executable_sha256"]
                .as_str()
                .is_some_and(valid_digest),
        "unsupported research cache contract"
    );
    let mut content = metadata.clone();
    content
        .as_object_mut()
        .context("cache manifest")?
        .remove("content_fingerprint");
    ensure!(
        metadata["content_fingerprint"] == json_digest(&content)?,
        "research cache fingerprint mismatch"
    );
    ensure!(
        read_json(&bound(root, "research-cache.complete.json")?)? == receipt(root, &metadata)?,
        "incomplete research cache"
    );
    ensure!(
        metadata["output_files"] == serde_json::to_value(output_seals(root)?)?,
        "research cache content seals changed"
    );
    ensure!(
        fs::read_dir(root)?.count() == 11,
        "unlisted research cache output"
    );
    let source = source::Source::load(
        Path::new(
            metadata["source_root"]
                .as_str()
                .context("cache source root")?,
        ),
        expected,
    )?;
    ensure!(
        metadata["source_conversion_manifest"] == source.manifest
            && metadata["source_files"] == serde_json::to_value(&source.seals)?
            && metadata["attempt_counts"] == source.manifest["attempt_counts"]
            && metadata["target_contract"] == crate::research_contract::contract()
            && metadata["frame_contract"] == crate::research_contract::FRAME_CONTRACT
            && fs::read(root.join("attempts.jsonl"))?
                == fs::read(source.root.join("attempts.jsonl"))?,
        "research cache source/attempt identity changed"
    );
    for name in [
        "card_structure.json",
        "card_dsl.bin",
        "card_dsl_back.bin",
        "token_dsl.bin",
        "card_abilities.bin",
    ] {
        ensure!(
            files::sha256(&root.join(name))? == source.manifest["output_files"][name],
            "cached public catalog changed"
        );
    }
    Ok((metadata, source))
}
