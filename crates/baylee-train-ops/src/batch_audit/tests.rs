use super::*;
#[test]
fn sampling_covers_whole_epoch_and_singletons() -> Result<()> {
    let ids = indices(5000, 48)?;
    ensure!(ids.len() == 48 && ids[0] == 0 && ids[47] == 4999);
    ensure!(indices(2, 48)? == [0, 1] && indices(1, 48)? == [0]);
    ensure!(indices(0, 48).is_err());
    Ok(())
}
#[test]
fn cross_lane_padding_accounts_for_all_owners_and_scratch() -> Result<()> {
    let result = boundary(
        &json!({"kind":"estimate","schemas":[{"x":{"shape":[2,8],"element_size":4},"z":{"shape":[],"element_size":1}},{"x":{"shape":[4,3],"element_size":4},"z":{"shape":[],"element_size":1}}],"frame_peak_bytes":10,"unroll":3,"budget_mib":1}),
    )?;
    ensure!(
        result["predicted_collated_bytes"] == 258
            && result["conservative_tensor_peak_bytes"] == 690
            && result["measure"] == true
    );
    Ok(())
}
#[test]
fn native_report_uses_lower_order_quantiles_and_preserves_output() -> Result<()> {
    use crate::files;
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("source");
    std::fs::write(&source, "sealed")?;
    let out = directory.path().join("audit.json");
    let mut records = Vec::new();
    for (index, size) in [8u64, 128 << 20, 256 << 20, 512 << 20]
        .into_iter()
        .enumerate()
    {
        let mask =
            json!({"padded_width":1,"max_actual":1,"actual_entries":1,"allocated_entries":1});
        records.push(json!({"batch_index":index,"segments":[],"estimate":{"measure":true,"predicted_collated_bytes":size,"conservative_tensor_peak_bytes":size*3},"measurement":{"raw_batch_bytes":size,"logical_batch_bytes":size,"batch_size":1,"batch_unroll":1,"valid_frames":1,"active_timesteps":1,"thinking_decision_rows":1,"decision_head_counts":{"0":1,"1":0,"2":0,"3":0},"entities":mask,"edges":mask,"candidates":mask,"pairs":mask,"entity_attention_E2":{"actual_entries":1}}}));
    }
    let request = json!({"kind":"finish","plan":{"out":out,"input_seals":{source.to_str().context("source path")?:files::sha256(&source)?},"samples":4,"budget_mib":2048},"metadata":{"epoch_batches":4,"dataset":"fixture","data_fingerprint":"fixture","namespace":"fixture","metadata_inputs":{},"sampler_version":1},"records":records,"elapsed_seconds":1.0});
    boundary(&request)?;
    let report = files::read_json(&out)?;
    ensure!(
        report["fractions"]["le128MiB"] == 0.5
            && report["fractions"]["gt128MiB"] == 0.5
            && report["fractions"]["gt256MiB"] == 0.25
    );
    ensure!(report["benchmark_windows"] == json!({"first":0,"median":1,"high_p95":2,"argmax":3}));
    ensure!(boundary(&request).is_err());
    Ok(())
}
