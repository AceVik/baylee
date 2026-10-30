# CPU tensor batch audit

The native CLI owns job parsing, deterministic sample selection, shape estimates,
budget decisions, quantiles and atomic reports:

```text
audit_batches5 --job JOB.json --out NEW.json --python TORCH_INTERPRETER \
  --samples 48 --budget-mib 512 --timeout-seconds 900
```

Build with `cargo build --release -p baylee-train-ops --bin audit_batches5`.
The job must contain the existing literal `Python -u SCRIPT --data ... --unroll
... --batch-size ... --seed ...` arguments. Only the named frozen runtime's
PyTorch dataset/sampler are loaded; the job's training command is never run.
The associated Python worker decodes tensor frames, collates only batches admitted
by the native allocation estimate, and measures Tensor storage/counts. It forces
CPU execution and performs no model forward, backward, optimizer step or export.
The Rust owner enforces a hard worker deadline and verifies frozen input seals
before publishing a new report. Existing reports are preserved.

Report schema 2 labels the integer-even-v1 sampling rule explicitly. Sampled
maxima are not global maxima; attention squares are geometry estimates rather
than measured model allocations. Zero-width masks have null padding fractions.
The tensor budget excludes fixed catalog/index/interpreter RSS. Dataset cache
admission belongs to the selected loader; this audit is not an independent full
payload CRC scan or a quality evaluation.
