use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Rect, TextureOptions, Ui};

fn get_raw_logo_bytes(app_id: &str) -> Option<&'static [u8]> {
    match app_id {
        "photocraft" => Some(include_bytes!("../../assets/logos/photocraft.png")),
        "vectorcraft" => Some(include_bytes!("../../assets/logos/vectorcraft.png")),
        "filmcraft" => Some(include_bytes!("../../assets/logos/filmcraft.png")),
        "lightcraft" => Some(include_bytes!("../../assets/logos/lightcraft.png")),
        "pdfcraft" => Some(include_bytes!("../../assets/logos/pdfcraft.png")),
        "effectcraft" => Some(include_bytes!("../../assets/logos/effectcraft.png")),
        "designcraft" => Some(include_bytes!("../../assets/logos/designcraft.png")),
        "soundcraft" => Some(include_bytes!("../../assets/logos/soundcraft.png")),
        "cadcraft" => Some(include_bytes!("../../assets/logos/cadcraft.png")),
        "deckcraft" => Some(include_bytes!("../../assets/logos/deckcraft.png")),
        "gridcraft" => Some(include_bytes!("../../assets/logos/gridcraft.png")),
        "wordcraft" => Some(include_bytes!("../../assets/logos/wordcraft.png")),
        "github" => Some(include_bytes!("../../assets/logos/github.png")),
        _ => None,
    }
}

/// Render the official GitHub octocat mark as a clickable icon button with a specific persistent ID.
/// Smoothly eases up to 5% larger on hover and eases back down when unhovered.
pub fn render_github_icon_with_id(ui: &mut Ui, id_str: &str, size: f32, tint: Color32) -> egui::Response {
    let texture_id = "opencloud_logo_github";
    let texture = ui.ctx().data_mut(|d| {
        d.get_temp::<egui::TextureHandle>(egui::Id::new(texture_id))
    });

    let texture = match texture {
        Some(t) => t,
        None => {
            if let Some(color_img) = get_app_color_image("github") {
                let handle = ui.ctx().load_texture(texture_id, color_img, TextureOptions::LINEAR);
                ui.ctx().data_mut(|d| {
                    d.insert_temp(egui::Id::new(texture_id), handle.clone());
                });
                handle
            } else {
                return ui.button(egui::RichText::new("GH").strong());
            }
        }
    };

    let btn_id = ui.make_persistent_id(id_str);
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), egui::Sense::click());

    // Smooth easing: eases up 5% on hover, down 5% when unhovered
    let hover_t = ui.ctx().animate_bool_with_time(btn_id, response.hovered(), 0.15);
    let scale = 1.0 + 0.05 * hover_t;
    let anim_size = size * scale;
    let anim_rect = Rect::from_center_size(rect.center(), vec2(anim_size, anim_size));

    // Subtle neutral hover halo (no blue!)
    if hover_t > 0.01 {
        let alpha = (hover_t * 28.0) as u8;
        ui.painter().circle_filled(
            rect.center(),
            (size * 0.54) * scale,
            Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
        );
    }

    // Clean neutral tint: slightly brighter on hover, absolutely NO blue
    let actual_tint = if hover_t > 0.01 {
        Color32::WHITE
    } else {
        tint
    };

    ui.painter().image(
        texture.id(),
        anim_rect,
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        actual_tint,
    );

    response
}

/// Render the official GitHub octocat mark as a clickable icon button in the header
pub fn render_github_icon(ui: &mut Ui, size: f32, tint: Color32) -> egui::Response {
    render_github_icon_with_id(ui, "opencloud_github_header_btn", size, tint)
}

pub fn get_app_color_image(app_id: &str) -> Option<egui::ColorImage> {
    let bytes = get_raw_logo_bytes(app_id)?;
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    // Find bounding box of non-transparent content (alpha > 15)
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;

    for y in 0..height {
        for x in 0..width {
            let p = rgba.get_pixel(x, y);
            if p[3] > 15 {
                if x < min_x { min_x = x; }
                if x > max_x { max_x = x; }
                if y < min_y { min_y = y; }
                if y > max_y { max_y = y; }
            }
        }
    }

    if min_x >= max_x || min_y >= max_y {
        let size = [width as usize, height as usize];
        return Some(egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()));
    }

    let crop_w = (max_x - min_x + 1) as usize;
    let crop_h = (max_y - min_y + 1) as usize;
    let mut cropped = Vec::with_capacity(crop_w * crop_h * 4);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = rgba.get_pixel(x, y);
            cropped.extend_from_slice(&p.0);
        }
    }

    Some(egui::ColorImage::from_rgba_unmultiplied([crop_w, crop_h], &cropped))
}

/// Render the official upstream logo for an application with synchronous texture upload and caching.
pub fn render_logo(ui: &mut Ui, app_id: &str, token: &str, accent_color: [u8; 3], size: f32) -> egui::Response {
    let texture_id = format!("opencloud_logo_{}", app_id);
    let texture = ui.ctx().data_mut(|d| {
        d.get_temp::<egui::TextureHandle>(egui::Id::new(&texture_id))
    });

    let texture = match texture {
        Some(t) => t,
        None => {
            if let Some(color_img) = get_app_color_image(app_id) {
                let handle = ui.ctx().load_texture(&texture_id, color_img, TextureOptions::LINEAR);
                ui.ctx().data_mut(|d| {
                    d.insert_temp(egui::Id::new(&texture_id), handle.clone());
                });
                handle
            } else {
                // Fallback mnemonic badge if logo is unavailable
                let (rect, resp) = ui.allocate_exact_size(vec2(size, size), egui::Sense::click());
                let accent = Color32::from_rgb(accent_color[0], accent_color[1], accent_color[2]);
                ui.painter().rect_filled(rect, CornerRadius::same(8), accent);
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    token,
                    FontId::proportional(size * 0.42),
                    Color32::WHITE,
                );
                return resp;
            }
        }
    };

    ui.add(
        egui::Image::new(&texture)
            .fit_to_exact_size(vec2(size, size))
            .corner_radius(CornerRadius::same(8))
            .sense(egui::Sense::click()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_12_apps_have_logos() {
        let app_ids = [
            "photocraft",
            "vectorcraft",
            "filmcraft",
            "lightcraft",
            "pdfcraft",
            "effectcraft",
            "designcraft",
            "soundcraft",
            "cadcraft",
            "deckcraft",
            "gridcraft",
            "wordcraft",
            "github",
        ];

        for id in app_ids {
            let color_img = get_app_color_image(id);
            assert!(color_img.is_some(), "ColorImage must decode successfully for app: {}", id);
            let img = color_img.unwrap();
            assert!(img.width() > 0 && img.height() > 0);
        }
    }
}
