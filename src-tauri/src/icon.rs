use crate::services::{ServiceData, ServiceStatus};
use std::collections::HashMap;
use std::f64::consts::PI;
use tauri::{Manager, Runtime};

const SIZE: u32 = 32;
const OUTER_R: f64 = 13.5;
const INNER_R: f64 = 6.5;
const CENTER: f64 = 15.5;

fn status_color(s: &ServiceStatus) -> [u8; 4] {
    match s {
        ServiceStatus::Green => [34, 197, 94, 255],
        ServiceStatus::Yellow => [245, 158, 11, 255],
        ServiceStatus::Red => [239, 68, 68, 255],
        ServiceStatus::Unknown => [100, 116, 139, 180],
    }
}

pub fn generate_pie_icon(statuses: &[ServiceStatus]) -> Vec<u8> {
    let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];
    let n = statuses.len();

    if n == 0 {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let dx = x as f64 + 0.5 - CENTER;
                let dy = y as f64 + 0.5 - CENTER;
                let r = (dx * dx + dy * dy).sqrt();
                if r >= INNER_R && r <= OUTER_R {
                    let idx = ((y * SIZE + x) * 4) as usize;
                    pixels[idx..idx + 4].copy_from_slice(&[100, 116, 139, 180]);
                }
            }
        }
        return encode_png_rgba(&pixels, SIZE, SIZE);
    }

    let slice_angle = 2.0 * PI / n as f64;
    let gap = if n > 1 { 0.06_f64 } else { 0.0 };

    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f64 + 0.5 - CENTER;
            let dy = y as f64 + 0.5 - CENTER;
            let r = (dx * dx + dy * dy).sqrt();

            if r < INNER_R || r > OUTER_R {
                continue;
            }

            // Angle from top, clockwise (screen coords: y increases downward)
            let mut angle = dy.atan2(dx) + PI / 2.0;
            if angle < 0.0 {
                angle += 2.0 * PI;
            }

            let slice_idx = ((angle / slice_angle) as usize).min(n - 1);
            let within = angle - slice_idx as f64 * slice_angle;

            if within < gap / 2.0 || within > slice_angle - gap / 2.0 {
                continue;
            }

            let color = status_color(&statuses[slice_idx]);
            let idx = ((y * SIZE + x) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&color);
        }
    }

    encode_png_rgba(&pixels, SIZE, SIZE)
}

fn encode_png_rgba(pixels: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(pixels).unwrap();
    drop(writer);
    out
}

pub fn update_tray_icon<R: Runtime>(
    app: &tauri::AppHandle<R>,
    service_data: &HashMap<String, ServiceData>,
) where
    tauri::AppHandle<R>: Manager<R>,
{
    const ORDER: &[&str] = &["claude", "openai", "gemini", "grok", "perplexity"];

    let statuses: Vec<ServiceStatus> = ORDER
        .iter()
        .filter_map(|id| service_data.get(*id).map(|d| d.status.clone()))
        .collect();

    let png_bytes = generate_pie_icon(&statuses);

    if let Ok(icon) = tauri::image::Image::from_bytes(&png_bytes) {
        if let Some(tray) = app.tray_by_id("main-tray") {
            let _ = tray.set_icon(Some(icon));
            let _ = tray.set_icon_as_template(false);
        }
    }
}
