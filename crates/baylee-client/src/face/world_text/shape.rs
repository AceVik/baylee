//! Drive the public text pipeline without requiring a visible 2D camera.
use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    sprite::Text2dReader,
    text::{
        ComputedTextBlock, FontAtlasSet, FontCx, LayoutCx, ScaleCx, TextBounds, TextLayoutInfo,
        TextPipeline,
    },
};

#[derive(Component)]
pub(super) struct Retry(bool);

#[derive(SystemParam)]
pub(super) struct Shaper<'w> {
    fonts: Res<'w, Assets<Font>>,
    atlas: ResMut<'w, FontAtlasSet>,
    images: ResMut<'w, Assets<Image>>,
    pipeline: ResMut<'w, TextPipeline>,
    fonts_ctx: ResMut<'w, FontCx>,
    layout_ctx: ResMut<'w, LayoutCx>,
    scale_ctx: ResMut<'w, ScaleCx>,
    rem: Res<'w, RemSize>,
}

#[allow(clippy::type_complexity)] // text layout inputs and outputs stay in one query
pub(super) fn shape(
    mut commands: Commands,
    mut text: Query<
        (
            Entity,
            Ref<Text2d>,
            Ref<TextLayout>,
            Ref<TextBounds>,
            &FontHinting,
            &mut TextLayoutInfo,
            &mut ComputedTextBlock,
            Option<&Retry>,
        ),
        With<super::super::WorldFace>,
    >,
    mut reader: Text2dReader,
    mut shaper: Shaper,
) {
    for (entity, words, block, bounds, hinting, mut info, mut computed, retry) in &mut text {
        if !words.is_changed()
            && !block.is_changed()
            && !bounds.is_changed()
            && !computed.needs_rerender(false, false)
            && retry.is_some_and(|r| !r.0)
        {
            continue;
        }
        // WorldFace uses fixed Px fonts and explicit line breaks. Two atlas
        // texels per logical text unit keep zoomed cards readable; no viewport
        // resize or camera movement needs new geometry or glyph allocation.
        let physical = TextBounds {
            width: bounds.width.map(|x| x * 2.0),
            height: bounds.height.map(|y| y * 2.0),
        };
        let result = shaper
            .pipeline
            .update_buffer(
                &shaper.fonts,
                reader.iter(entity),
                block.linebreak,
                block.justify,
                physical,
                2.0,
                &mut computed,
                &mut shaper.fonts_ctx,
                &mut shaper.layout_ctx,
                Vec2::splat(1000.0),
                *shaper.rem,
            )
            .and_then(|()| {
                shaper.pipeline.update_text_layout_info(
                    &mut info,
                    &mut shaper.atlas,
                    &mut shaper.images,
                    &mut computed,
                    &mut shaper.scale_ctx,
                    physical,
                    block.justify,
                    *hinting,
                )
            });
        let failed = result.is_err();
        let again = match result {
            Ok(()) => false,
            Err(TextError::NoSuchFont | TextError::NoSuchFontFamily(_)) => true,
            Err(error) => {
                bevy::log::warn_once!("Unable to shape card text: {error}");
                false
            }
        };
        if failed && !info.glyphs.is_empty() {
            info.clear();
        }
        commands.entity(entity).insert(Retry(again));
    }
}
