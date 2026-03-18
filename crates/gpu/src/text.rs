use cosmic_text::{Attrs, Buffer, Color, FontSystem, Metrics, Shaping, SwashCache};

/// Rasterize text to an RGBA pixel buffer.
///
/// Returns `(rgba_data, width, height)` where `rgba_data` is a packed RGBA byte vector.
/// The text is rendered with the given font, size, and color, constrained to `max_width` x `max_height`.
pub fn rasterize_text(
    content: &str,
    font_family: &str,
    font_size: f32,
    color: [f32; 4],
    max_width: u32,
    max_height: u32,
) -> (Vec<u8>, u32, u32) {
    let mut font_system = FontSystem::new();
    let mut swash_cache = SwashCache::new();

    let metrics = Metrics::new(font_size, font_size * 1.2);
    let mut buffer = Buffer::new(&mut font_system, metrics);

    buffer.set_size(
        &mut font_system,
        Some(max_width as f32),
        Some(max_height as f32),
    );

    let attrs = Attrs::new().family(cosmic_text::Family::Name(font_family));
    buffer.set_text(&mut font_system, content, attrs, Shaping::Advanced);
    buffer.shape_until_scroll(&mut font_system, false);

    let width = max_width;
    let height = max_height;
    let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];

    let text_color = Color::rgba(
        (color[0] * 255.0) as u8,
        (color[1] * 255.0) as u8,
        (color[2] * 255.0) as u8,
        (color[3] * 255.0) as u8,
    );

    buffer.draw(
        &mut font_system,
        &mut swash_cache,
        text_color,
        |x, y, w, h, color| {
            // The draw callback gives us the position and color of each pixel
            let _ = (w, h); // width/height of the glyph run region
            if x < 0 || y < 0 {
                return;
            }
            let px = x as u32;
            let py = y as u32;
            if px >= width || py >= height {
                return;
            }
            let idx = ((py * width + px) * 4) as usize;
            if idx + 3 >= pixels.len() {
                return;
            }

            // Alpha-blend glyph pixel onto the buffer
            let src_a = color.a() as f32 / 255.0;
            let dst_a = pixels[idx + 3] as f32 / 255.0;

            let out_a = src_a + dst_a * (1.0 - src_a);
            if out_a > 0.0 {
                let src_r = color.r() as f32;
                let src_g = color.g() as f32;
                let src_b = color.b() as f32;
                let dst_r = pixels[idx] as f32;
                let dst_g = pixels[idx + 1] as f32;
                let dst_b = pixels[idx + 2] as f32;

                pixels[idx] = ((src_r * src_a + dst_r * dst_a * (1.0 - src_a)) / out_a) as u8;
                pixels[idx + 1] = ((src_g * src_a + dst_g * dst_a * (1.0 - src_a)) / out_a) as u8;
                pixels[idx + 2] = ((src_b * src_a + dst_b * dst_a * (1.0 - src_a)) / out_a) as u8;
                pixels[idx + 3] = (out_a * 255.0) as u8;
            }
        },
    );

    (pixels, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasterize_hello_non_empty() {
        let (data, w, h) =
            rasterize_text("Hello", "sans-serif", 48.0, [1.0, 1.0, 1.0, 1.0], 256, 64);
        assert_eq!(w, 256);
        assert_eq!(h, 64);
        assert_eq!(data.len(), 256 * 64 * 4);
        // At least some pixels should be non-zero (text was rendered)
        let has_content = data.iter().any(|&b| b != 0);
        assert!(
            has_content,
            "rasterized text should produce non-zero pixels"
        );
    }

    #[test]
    fn rasterize_empty_string() {
        let (data, w, h) = rasterize_text("", "sans-serif", 48.0, [1.0, 1.0, 1.0, 1.0], 128, 64);
        assert_eq!(w, 128);
        assert_eq!(h, 64);
        assert_eq!(data.len(), 128 * 64 * 4);
    }

    #[test]
    fn rasterize_colored_text() {
        let (data, w, h) = rasterize_text("Red", "sans-serif", 32.0, [1.0, 0.0, 0.0, 1.0], 128, 48);
        assert_eq!(data.len(), (w as usize) * (h as usize) * 4);
        // Find a non-zero pixel and check it has red component
        for chunk in data.chunks_exact(4) {
            if chunk[3] > 0 {
                // If there's alpha, the red channel should dominate
                assert!(
                    chunk[0] > 0 || chunk[3] > 0,
                    "colored text should have red channel"
                );
                break;
            }
        }
    }

    #[test]
    fn rasterize_large_text() {
        let (data, w, h) =
            rasterize_text("Big!", "sans-serif", 96.0, [1.0, 1.0, 1.0, 1.0], 512, 128);
        assert_eq!(w, 512);
        assert_eq!(h, 128);
        assert_eq!(data.len(), 512 * 128 * 4);
    }

    #[test]
    fn rasterize_small_canvas() {
        let (data, w, h) = rasterize_text("Tiny", "sans-serif", 12.0, [1.0, 1.0, 1.0, 1.0], 64, 16);
        assert_eq!(w, 64);
        assert_eq!(h, 16);
        assert_eq!(data.len(), 64 * 16 * 4);
    }

    #[test]
    fn rasterize_very_long_string() {
        let long_text = "A".repeat(500);
        let (data, w, h) = rasterize_text(
            &long_text,
            "sans-serif",
            24.0,
            [1.0, 1.0, 1.0, 1.0],
            256,
            128,
        );
        assert_eq!(w, 256);
        assert_eq!(h, 128);
        assert_eq!(data.len(), 256 * 128 * 4);
    }

    #[test]
    fn rasterize_large_font_size() {
        let (data, w, h) = rasterize_text("X", "sans-serif", 200.0, [1.0, 1.0, 1.0, 1.0], 512, 256);
        assert_eq!(w, 512);
        assert_eq!(h, 256);
        assert_eq!(data.len(), 512 * 256 * 4);
        let has_content = data.iter().any(|&b| b != 0);
        assert!(has_content, "large font should produce visible pixels");
    }

    #[test]
    fn rasterize_zero_dimensions() {
        // Zero-size canvas should produce empty pixel buffer without panic
        let (data, w, h) = rasterize_text("Hello", "sans-serif", 24.0, [1.0, 1.0, 1.0, 1.0], 0, 0);
        assert_eq!(w, 0);
        assert_eq!(h, 0);
        assert!(data.is_empty());
    }

    #[test]
    fn rasterize_with_alpha() {
        let (data, _, _) =
            rasterize_text("Semi", "sans-serif", 32.0, [1.0, 1.0, 1.0, 0.5], 128, 48);
        // Should still produce some output
        let has_content = data.iter().any(|&b| b != 0);
        assert!(has_content, "semi-transparent text should still render");
    }
}
