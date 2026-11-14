// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The window background: a hyperbola sky and a 3D block world, software
//! rasterised here (with a direct GPU path in [`gl`]) and crossed by the user's
//! custom background images. The world renders on a worker thread.

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
    /// through its edges. They are white on the dark flavours and black on Latte.
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
