//! Verify encoder-specific fixed global input width on every loaded depth graph.
use anyhow::{Context, ensure};
use ort::session::Session;
pub(super) const fn widths(encoder: u32) -> (usize, usize) {
    match encoder {
        9 => (
            crate::features7::ENTITY_FEAT_WIDTH7,
            crate::features9::GLOBAL_FEAT_WIDTH9,
        ),
        7 | 8 => (
            crate::features7::ENTITY_FEAT_WIDTH7,
            crate::features7::GLOBAL_FEAT_WIDTH7,
        ),
        _ => (
            crate::features4::ENTITY_FEAT_WIDTH,
            crate::features4::GLOBAL_FEAT_WIDTH,
        ),
    }
}
pub(super) fn validate(session: &Session, encoder: u32) -> anyhow::Result<()> {
    let width = match encoder {
        7 | 8 => 80,
        9 => 82,
        _ => return Ok(()),
    };
    {
        let input = session
            .inputs()
            .iter()
            .find(|input| input.name() == "global_feat")
            .context("Graph lacks global_feat")?;
        let shape = input
            .dtype()
            .tensor_shape()
            .context("global_feat is not a tensor")?;
        ensure!(
            shape.len() == 2 && shape[1] == width,
            "Encoder {encoder} graph global_feat must have fixed width {width}"
        );
        if encoder == 9 {
            ensure!(
                input.dtype().tensor_type() == Some(ort::value::TensorElementType::Float32),
                "Encoder 9 global_feat must be FP32"
            );
            let entity = session
                .inputs()
                .iter()
                .find(|input| input.name() == "entity_feat")
                .context("Encoder 9 graph lacks entity_feat")?;
            let shape = entity
                .dtype()
                .tensor_shape()
                .context("entity_feat is not a tensor")?;
            ensure!(
                shape.len() == 3 && shape[2] == 128,
                "Encoder 9 entity width remains 128"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn encoder_abi_keeps_entity128_when_global_width_grows() {
        assert_eq!(super::widths(7), (128, 80));
        assert_eq!(super::widths(8), (128, 80));
        assert_eq!(super::widths(9), (128, 82));
    }
}
