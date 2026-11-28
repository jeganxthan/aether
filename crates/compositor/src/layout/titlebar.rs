use anyhow::ensure;
use pango::FontDescription;
use pangocairo::cairo::{self, ImageSurface};
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::gles::{GlesRenderer, GlesTexture};
use smithay::utils::{Logical, Point, Transform};

use crate::layout::CornerRadius;
use crate::render_helpers::primary_gpu_texture::PrimaryGpuTextureRenderElement;
use crate::render_helpers::texture::{TextureBuffer, TextureRenderElement};

pub const HEIGHT: f64 = 36.;
pub const CONTROL_WIDTH: f64 = 78.;
const BUTTON_START_X: f64 = 18.;
const BUTTON_SPACING: f64 = 20.;
const BUTTON_RADIUS: f64 = 6.;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Close,
    Minimize,
    Maximize,
}

/// Returns which titlebar button circle contains `point` (in titlebar-local logical coordinates).
pub fn hit_test(point: Point<f64, Logical>) -> Option<Control> {
    let cy = HEIGHT / 2.;
    let radius_sq = BUTTON_RADIUS * BUTTON_RADIUS;
    let controls = [Control::Close, Control::Minimize, Control::Maximize];

    for (index, control) in controls.into_iter().enumerate() {
        let cx = BUTTON_START_X + index as f64 * BUTTON_SPACING;
        let dx = point.x - cx;
        let dy = point.y - cy;
        if dx * dx + dy * dy <= radius_sq {
            return Some(control);
        }
    }

    None
}

const FONT: &str = "sans 13px";

#[derive(Debug, Default)]
pub struct TitleBar {
    cache: Option<CachedTitleBar>,
}

#[derive(Debug)]
struct CachedTitleBar {
    title: String,
    active: bool,
    scale: f64,
    width: f64,
    corner_radius: CornerRadius,
    buffer: TextureBuffer<GlesTexture>,
}

impl TitleBar {
    pub fn render(
        &mut self,
        renderer: &mut GlesRenderer,
        title: &str,
        active: bool,
        width: f64,
        scale: f64,
        corner_radius: CornerRadius,
        location: Point<f64, Logical>,
    ) -> anyhow::Result<PrimaryGpuTextureRenderElement> {
        self.render_cached(
            renderer,
            title,
            active,
            width,
            scale,
            corner_radius,
            location,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_cached(
        &mut self,
        renderer: &mut GlesRenderer,
        title: &str,
        active: bool,
        width: f64,
        scale: f64,
        corner_radius: CornerRadius,
        location: Point<f64, Logical>,
    ) -> anyhow::Result<PrimaryGpuTextureRenderElement> {
        let needs_update = self.cache.as_ref().is_none_or(|cache| {
            cache.title != title
                || cache.active != active
                || cache.scale != scale
                || cache.width != width
                || cache.corner_radius != corner_radius
        });

        if needs_update {
            self.cache = Some(CachedTitleBar {
                title: title.to_owned(),
                active,
                scale,
                width,
                corner_radius,
                buffer: render_title_bar(renderer, title, active, width, scale, corner_radius)?,
            });
        }

        let buffer = self
            .cache
            .as_ref()
            .expect("title bar cache was initialized");
        Ok(PrimaryGpuTextureRenderElement(
            TextureRenderElement::from_texture_buffer(
                buffer.buffer.clone(),
                location,
                1.,
                None,
                None,
                Kind::Unspecified,
            ),
        ))
    }
}

fn render_title_bar(
    renderer: &mut GlesRenderer,
    title: &str,
    active: bool,
    width: f64,
    scale: f64,
    corner_radius: CornerRadius,
) -> anyhow::Result<TextureBuffer<GlesTexture>> {
    let physical_width = (width * scale).round().max(1.) as i32;
    let physical_height = (HEIGHT * scale).round().max(1.) as i32;
    let size = (physical_width, physical_height);
    let font_size = (13. * scale * f64::from(pango::SCALE)).round() as i32;

    let surface = ImageSurface::create(cairo::Format::ARgb32, physical_width, physical_height)?;
    let cr = cairo::Context::new(&surface)?;
    let radius = corner_radius.fit_to(width as f32, HEIGHT as f32);
    rounded_top_path(
        &cr,
        f64::from(physical_width),
        f64::from(physical_height),
        f64::from(radius.top_left) * scale,
        f64::from(radius.top_right) * scale,
    );
    cr.clip();

    cr.set_source_rgb(0.10, 0.105, 0.12);
    cr.paint()?;

    cr.set_source_rgb(0.22, 0.23, 0.25);
    cr.rectangle(
        0.,
        f64::from(physical_height) - 1.,
        f64::from(physical_width),
        1.,
    );
    cr.fill()?;

    let button_colors = [(0.96, 0.29, 0.26), (0.98, 0.75, 0.22), (0.19, 0.78, 0.34)];
    let button_alpha = if active { 1. } else { 0.58 };
    for (index, (red, green, blue)) in button_colors.into_iter().enumerate() {
        let x = (BUTTON_START_X + index as f64 * BUTTON_SPACING) * scale;
        cr.set_source_rgba(red, green, blue, button_alpha);
        cr.arc(
            x,
            f64::from(physical_height) / 2.,
            BUTTON_RADIUS * scale,
            0.,
            std::f64::consts::TAU,
        );
        cr.fill()?;
    }

    let layout = pangocairo::functions::create_layout(&cr);
    layout.context().set_round_glyph_positions(false);
    let mut font = FontDescription::from_string(FONT);
    font.set_absolute_size(font_size.into());
    layout.set_font_description(Some(&font));
    layout.set_single_paragraph_mode(true);
    layout.set_ellipsize(pango::EllipsizeMode::End);
    layout.set_width(
        ((f64::from(physical_width) - CONTROL_WIDTH * 2. * scale).max(0.) * f64::from(pango::SCALE))
            as i32,
    );
    layout.set_text(title);

    let (_, text_height) = layout.pixel_size();
    cr.set_source_rgba(0.92, 0.92, 0.94, if active { 1. } else { 0.72 });
    cr.move_to(
        (f64::from(physical_width) - f64::from(layout.pixel_size().0)) / 2.,
        (f64::from(physical_height) - f64::from(text_height)) / 2.,
    );
    pangocairo::functions::show_layout(&cr, &layout);
    drop(cr);

    let data = surface.take_data().unwrap();
    ensure!(data.len() >= (physical_width * physical_height * 4) as usize);
    TextureBuffer::from_memory(
        renderer,
        &data,
        Fourcc::Argb8888,
        size,
        false,
        scale,
        Transform::Normal,
        Vec::new(),
    )
    .map_err(Into::into)
}

fn rounded_top_path(cr: &cairo::Context, width: f64, height: f64, top_left: f64, top_right: f64) {
    let top_left = top_left.min(width / 2.).min(height);
    let top_right = top_right.min(width / 2.).min(height);

    cr.move_to(0., height);
    cr.line_to(0., top_left);
    if top_left > 0. {
        cr.curve_to(0., top_left * 0.45, top_left * 0.45, 0., top_left, 0.);
    } else {
        cr.line_to(0., 0.);
    }

    cr.line_to(width - top_right, 0.);
    if top_right > 0. {
        cr.curve_to(
            width - top_right * 0.45,
            0.,
            width,
            top_right * 0.45,
            width,
            top_right,
        );
    } else {
        cr.line_to(width, 0.);
    }
    cr.line_to(width, height);
    cr.close_path();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_test_matches_button_circles_only() {
        // Inside the close circle (center (18, 18), radius 6).
        assert_eq!(hit_test(Point::from((18., 18.))), Some(Control::Close));
        assert_eq!(hit_test(Point::from((13., 18.))), Some(Control::Close));

        // Outside the close circle (even in x < 28), must not trigger Close.
        assert_eq!(hit_test(Point::from((5., 5.))), None);
        assert_eq!(hit_test(Point::from((18., 10.))), None);
        assert_eq!(hit_test(Point::from((18., 26.))), None);
        assert_eq!(hit_test(Point::from((27., 18.))), None);

        // Minimize and maximize circles.
        assert_eq!(hit_test(Point::from((38., 18.))), Some(Control::Minimize));
        assert_eq!(hit_test(Point::from((58., 18.))), Some(Control::Maximize));
        assert_eq!(hit_test(Point::from((70., 18.))), None);
    }
}
