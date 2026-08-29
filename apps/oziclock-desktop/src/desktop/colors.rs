use slint::Color;

pub(super) fn parse_color(value: &str) -> Color {
    let value = value.trim_start_matches('#');
    let rgb = match value.len() {
        6 => u32::from_str_radix(value, 16)
            .ok()
            .map(|rgb| (rgb >> 16, rgb >> 8, rgb)),
        8 => u32::from_str_radix(value, 16)
            .ok()
            .map(|argb| (argb >> 16, argb >> 8, argb)),
        _ => None,
    };

    rgb.map_or(Color::from_rgb_u8(255, 255, 255), |(red, green, blue)| {
        Color::from_rgb_u8(red as u8, green as u8, blue as u8)
    })
}

pub(super) fn color_to_hsv(value: &str) -> (f32, f32, f32) {
    let color = parse_color(value);
    let red = f32::from(color.red()) / 255.0;
    let green = f32::from(color.green()) / 255.0;
    let blue = f32::from(color.blue()) / 255.0;
    let maximum = red.max(green).max(blue);
    let minimum = red.min(green).min(blue);
    let chroma = maximum - minimum;
    let hue = if chroma == 0.0 {
        0.0
    } else if maximum == red {
        60.0 * ((green - blue) / chroma).rem_euclid(6.0)
    } else if maximum == green {
        60.0 * ((blue - red) / chroma + 2.0)
    } else {
        60.0 * ((red - green) / chroma + 4.0)
    };
    let saturation = if maximum == 0.0 {
        0.0
    } else {
        chroma / maximum * 100.0
    };
    (hue, saturation, maximum * 100.0)
}

pub(super) fn hsv_hex(hue: f32, saturation: f32, value: f32) -> String {
    let color = hsv_color(hue, saturation, value);
    format!(
        "#{:02X}{:02X}{:02X}",
        color.red(),
        color.green(),
        color.blue()
    )
}

pub(super) fn hsv_color(hue: f32, saturation: f32, value: f32) -> Color {
    let chroma = value / 100.0 * saturation / 100.0;
    let segment = hue / 60.0;
    let secondary = chroma * (1.0 - ((segment % 2.0) - 1.0).abs());
    let (red, green, blue) = match segment as i32 {
        0 => (chroma, secondary, 0.0),
        1 => (secondary, chroma, 0.0),
        2 => (0.0, chroma, secondary),
        3 => (0.0, secondary, chroma),
        4 => (secondary, 0.0, chroma),
        _ => (chroma, 0.0, secondary),
    };
    let offset = value / 100.0 - chroma;
    Color::from_rgb_u8(
        ((red + offset) * 255.0) as u8,
        ((green + offset) * 255.0) as u8,
        ((blue + offset) * 255.0) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_hex_uses_white_fallback() {
        assert_eq!(parse_color("invalid"), Color::from_rgb_u8(255, 255, 255));
    }

    #[test]
    fn hsv_red_has_expected_hex_value() {
        assert_eq!(hsv_hex(0.0, 100.0, 100.0), "#FF0000");
    }
}
