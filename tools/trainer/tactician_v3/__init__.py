"""The tactician, stage S1: a set transformer that builds the answer at attacks,
blocks and single targets, thinking in T passes with shared weights.

Reads the rows an imitating collection kept (`eval4 --tactician-imitate`,
`tactician_imitation_v1`: `wNN.jsonl` beside `wNN.f32`), the tactical layer's
answer `R` as the label, and exports `tactician.onnx` + `tactician.json`
(contract `tactician_head_v2`) for `netplay4::tactician::net`. Imitation labels
train the tactician and nothing else; this package reads no other records.

- `records`: the records, their checks, and batches of padded rows.
- `model`: the set transformer, its passes and its confidence.
- `train`: losses at every pass, held-out reports, the confidence threshold.
- `export`: the ONNX graph at any T, checked against PyTorch.

See docs/trained-ai.md, "The tactician, stage S1".
"""
