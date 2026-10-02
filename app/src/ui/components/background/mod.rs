// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The window background.
//!
//! Two layers, composited over the window's own colour (the palette's `crust`):
//! the hyperbola sky at 30% opacity, then the 3D block world at 30%. Above them
//! are the user's custom backgrounds — the global one from Settings →
//! Appearance, or the current instance's — as up to two images that cross-fade.
//!
//! Slint's declarative API has no custom-shader hook and no way to express tens
//! of thousands of depth-ordered quads, so the sky and the world are
//! software-rasterised here into `SharedPixelBuffer`s that Slint uploads and
//! composites on the GPU; the world also has a direct GPU path (see [`gl`]).
//!
//! * [`scene`] is the world's terrain, trees and face culling.
//! * [`raster`] is the z-buffered quad rasteriser the world's two draw passes
//!   are expressed in.
//! * [`world`] projects the scene and runs those two passes.
//! * [`sky`] draws the hyperbolae.
//!
//! The layer opacities (0.3) are baked into the pixels, which is what the GPU
//! would compute for `opacity: 0.3` on the element anyway; the elements
//! themselves then only need `visible` toggles and the fades between custom
//! backgrounds.
//!
//! Rendering happens on a worker thread (see `controller::Renderer`) so a frame
//! that takes longer than a display frame cannot stall the UI, and the camera
//! only advances when the world can actually be seen.

pub mod controller;
pub mod gl;
pub mod raster;
pub mod scene;
pub mod sky;
pub mod world;

#[cfg(test)]
mod tests {
    use super::*;

    /// The outlines are the whole picture on a dark palette: every fill is the
    /// crust, which is also the window's own colour, so a face is visible only
    /// through its edges. They are white on the dark flavours and black on
    /// Latte (`const c = latte ? 0 : 255`); with that backwards the world
    /// disappears into a flat window colour.
    ///
    /// Rendering the same scene both ways leaves the fills identical and only
    /// the outlines different, so those pixels can be compared directly.
    #[test]
    fn outlines_contrast_with_the_crust() {
        let (width, height) = (400u32, 300u32);
        let crust = [
            0x18 as f32 / 255.0,
            0x19 as f32 / 255.0,
            0x26 as f32 / 255.0,
        ];
        let mut renderer = world::WorldRenderer::new();
        let render = |renderer: &mut world::WorldRenderer, dark: bool| {
            renderer
                .render(&world::FrameRequest {
                    width,
                    height,
                    edge_scale: 1.0,
                    cam_z: 12.34,
                    crust,
                    dark,
                })
                .as_bytes()
                .to_vec()
        };
        let dark = render(&mut renderer, true);
        let latte = render(&mut renderer, false);

        let (mut both, mut darker, mut lighter, mut count) = (0u32, 0u32, 0u32, 0u32);
        for (one, two) in dark.as_chunks::<4>().0.iter().zip(latte.as_chunks::<4>().0) {
            if one == two {
                continue;
            }
            count += 1;
            let brightness =
                |pixel: &[u8]| u32::from(pixel[0]) + u32::from(pixel[1]) + u32::from(pixel[2]);
            both += brightness(one) + brightness(two);
            if brightness(one) > brightness(two) {
                lighter += 1;
            } else {
                darker += 1;
            }
        }
        assert!(
            count > 0,
            "the two renders are identical — no outlines at all"
        );
        assert_eq!(
            (lighter, darker),
            (count, 0),
            "the dark flavours' outlines are not the lighter of the two"
        );
        // And they are, on average, brighter than the crust they sit on.
        let average = both / (count * 2);
        let crust_value = ((crust[0] + crust[1] + crust[2]) * 255.0 * scene::LAYER_ALPHA) as u32;
        assert!(
            average > crust_value,
            "outlines average {average}, which is not brighter than the crust ({crust_value})"
        );
    }

    /// What the background costs per frame at the size a maximised window on a
    /// Retina display gives it. Run with `--release`: the numbers only mean
    /// anything optimised.
    ///
    /// ```text
    /// cargo test --release -p conic-launcher timings -- --nocapture
    /// ```
    #[test]
    fn timings() {
        // The renderer's own per-pass log is at debug level.
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
        let (width, height) = (2048u32, 1296u32);
        let crust = [
            0x18 as f32 / 255.0,
            0x19 as f32 / 255.0,
            0x26 as f32 / 255.0,
        ];
        let mut renderer = world::WorldRenderer::new();

        let started = std::time::Instant::now();
        // `CONIC_TIMING_FRAMES` makes it long enough to attach a profiler to.
        let frames = std::env::var("CONIC_TIMING_FRAMES")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(60);
        let mut cam_z = 0.0f32;
        for _ in 0..frames {
            // 2 blocks a second at 60fps.
            cam_z += crate::ui::components::background::scene::CAMERA_SPEED / 60.0;
            renderer.render(&world::FrameRequest {
                width,
                height,
                edge_scale: 1.08,
                cam_z,
                crust,
                dark: true,
            });
        }
        let per_frame = started.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
        println!("world {width}x{height}: {per_frame:.1} ms/frame");

        let started = std::time::Instant::now();
        sky::render(&sky::SkyRequest {
            width,
            height,
            scale: 1.08,
            dark: true,
        });
        println!(
            "sky (cached, on resize): {:.1} ms",
            started.elapsed().as_secs_f64() * 1000.0
        );
    }
}
