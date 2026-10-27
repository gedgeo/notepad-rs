//! Off-screen board export. Fixed pixel budget prevents accidental huge allocations.
use crate::session::MoodItem;
use image::{Rgba, RgbaImage};

pub fn dimensions(
    items: &[MoodItem],
    canvas: Option<[u32; 2]>,
    scale: f32,
) -> Result<[u32; 2], String> {
    if !scale.is_finite() || !(0.25..=4.0).contains(&scale) {
        return Err("Échelle d’export invalide".into());
    }
    let [width, height] = canvas.unwrap_or_else(|| {
        items
            .iter()
            .filter(|i| !i.hidden)
            .fold([1, 1], |[w, h], i| {
                [
                    w.max((i.x + i.width).ceil().max(1.0) as u32),
                    h.max((i.y + i.height).ceil().max(1.0) as u32),
                ]
            })
    });
    let width = (width as f32 * scale).ceil() as u32;
    let height = (height as f32 * scale).ceil() as u32;
    if width == 0 || height == 0 || width as u64 * height as u64 > 32_000_000 {
        return Err(
            "Export limité à 32 millions de pixels. Réduisez la résolution ou le cadre.".into(),
        );
    }
    Ok([width, height])
}

pub fn render(
    items: &[MoodItem],
    canvas: Option<[u32; 2]>,
    scale: f32,
    font_bytes: &[u8],
    background: Option<[u8; 3]>,
) -> Result<RgbaImage, String> {
    let [width, height] = dimensions(items, canvas, scale)?;
    let font = fontdue::Font::from_bytes(font_bytes, fontdue::FontSettings::default())
        .map_err(|e| e.to_string())?;
    let [r, g, b] = background.unwrap_or([255, 255, 255]);
    let mut output = RgbaImage::from_pixel(width, height, Rgba([r, g, b, 255]));
    for item in items.iter().filter(|i| !i.hidden) {
        if [item.x, item.y, item.width, item.height]
            .iter()
            .any(|v| !v.is_finite())
            || item.width <= 0.0
            || item.height <= 0.0
        {
            return Err("Un objet possède des dimensions invalides".into());
        }
        let x = (item.x * scale) as i64;
        let y = (item.y * scale) as i64;
        if x >= width as i64 || y >= height as i64 {
            continue;
        }
        let w = (item.width * scale).ceil().max(1.0) as u32;
        let h = (item.height * scale).ceil().max(1.0) as u32;
        if w as u64 * h as u64 > 32_000_000 {
            return Err("Objet trop grand pour l’export".into());
        }
        if item.image_path.is_some() || item.embedded_image.is_some() {
            let bytes = match &item.embedded_image {
                Some(b) => b.clone(),
                None => std::fs::read(item.image_path.as_ref().unwrap())
                    .map_err(|e| format!("Image inaccessible : {e}"))?,
            };
            let source = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
            let resized = source
                .resize_exact(w, h, image::imageops::FilterType::Lanczos3)
                .to_rgba8();
            image::imageops::overlay(&mut output, &resized, x, y);
        } else {
            let color = match item.color {
                1 => [228, 232, 211, 255],
                2 => [222, 236, 247, 255],
                3 => [245, 222, 225, 255],
                _ => [255, 246, 218, 255],
            };
            let mut note = RgbaImage::from_pixel(w, h, Rgba(color));
            let mut layout =
                fontdue::layout::Layout::new(fontdue::layout::CoordinateSystem::PositiveYDown);
            layout.reset(&fontdue::layout::LayoutSettings {
                x: 12.0 * scale,
                y: 12.0 * scale,
                max_width: Some((w as f32 - 24.0 * scale).max(1.0)),
                ..Default::default()
            });
            layout.append(
                &[&font],
                &fontdue::layout::TextStyle::new(&item.text, 17.0 * scale, 0),
            );
            for glyph in layout.glyphs() {
                let (metrics, bitmap) = font.rasterize_config(glyph.key);
                for gy in 0..metrics.height {
                    for gx in 0..metrics.width {
                        let px = glyph.x as i32 + gx as i32;
                        let py = glyph.y as i32 + gy as i32;
                        if px < 0 || py < 0 || px >= w as i32 || py >= h as i32 {
                            continue;
                        }
                        let alpha = bitmap[gy * metrics.width + gx] as f32 / 255.0;
                        let pixel = note.get_pixel_mut(px as u32, py as u32);
                        for (c, ink) in [23.0, 32.0, 51.0].iter().enumerate() {
                            pixel[c] = (pixel[c] as f32 * (1.0 - alpha) + ink * alpha) as u8;
                        }
                    }
                }
            }
            image::imageops::overlay(&mut output, &note, x, y);
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dimensions_reject_invalid_scale_and_excessive_memory() {
        assert!(dimensions(&[], Some([16384, 16384]), 1.0).is_err());
        assert!(dimensions(&[], Some([100, 100]), f32::NAN).is_err());
        assert_eq!(
            dimensions(&[], Some([1080, 1920]), 2.0).unwrap(),
            [2160, 3840]
        );
    }
    #[test]
    fn hidden_objects_do_not_expand_content_export() {
        let items = [
            MoodItem {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
                ..Default::default()
            },
            MoodItem {
                x: 9000.0,
                hidden: true,
                ..Default::default()
            },
        ];
        assert_eq!(dimensions(&items, None, 1.0).unwrap(), [100, 100]);
    }
    #[test]
    fn export_renders_note_text_and_excludes_hidden_layer() {
        let font = std::fs::read(crate::windows_fonts_dir().join("segoeui.ttf")).unwrap();
        let items = [
            MoodItem {
                x: 0.0,
                y: 0.0,
                width: 160.0,
                height: 90.0,
                text: "Écrire".into(),
                ..Default::default()
            },
            MoodItem {
                x: 0.0,
                y: 0.0,
                width: 160.0,
                height: 90.0,
                hidden: true,
                color: 3,
                ..Default::default()
            },
        ];
        let image = render(&items, Some([200, 100]), 1.0, &font, None).unwrap();
        assert_eq!(image.dimensions(), (200, 100));
        assert_eq!(image.get_pixel(0, 0).0, [255, 246, 218, 255]);
        assert!(image
            .pixels()
            .any(|p| p[0] < 100 && p[1] < 100 && p[2] < 100));
        assert_eq!(image.get_pixel(199, 99).0, [255, 255, 255, 255]);
    }
}
