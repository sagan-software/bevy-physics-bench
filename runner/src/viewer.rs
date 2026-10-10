//! Native wireframe rendering outside the benchmark timing boundary.

use bevy_math::{Quat, Vec2, Vec3};
use bevy_physics_bench::{
    BodyCount, BodyKind, Dimension, Engine, Scenario, Settings, SettingsInput, Shape, Simulation,
    Snapshot, StepCount,
};
use clap::Args;
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::{io::Write, path::PathBuf};

/// Configure a live native window or one headless frame from the same scene
/// and engine adapters.
#[derive(Debug, Args)]
pub(super) struct ViewOptions {
    /// Physics integration selected when the viewer first opens.
    #[arg(long, default_value = "avian")]
    engine: Engine,
    /// Shared scene to render; inverse kinematics requires the Bevy engine.
    #[arg(long, default_value = "pyramid")]
    scenario: Scenario,
    /// Planar or spatial projection using the corresponding engine adapter.
    #[arg(long, default_value = "3d")]
    dimension: Dimension,
    /// Validated number of scene bodies, excluding the common floor.
    #[arg(long, default_value = "128")]
    count: BodyCount,
    /// Optional complete scene settings, including seed, materials, and
    /// solver effort.
    #[arg(long)]
    settings: Option<PathBuf>,
    /// Write one binary PPM frame without opening a window.
    #[arg(long)]
    frame: Option<PathBuf>,
}

/// Use fixed framebuffer dimensions so display resizing cannot change
/// physics work or sample settings.
const WIDTH: usize = 960;
/// Keep a three-to-two aspect ratio for both captured frames and live
/// native windows.
const HEIGHT: usize = 640;
/// Primitive colours distinguish active bodies from static colliders and
/// constrained links.
const ACTIVE: u32 = 0x247666;

/// Render primitive edges into a reused framebuffer with a fixed
/// initial-scene camera.
struct Canvas {
    /// Packed RGB pixels owned by the renderer and reused between
    /// simulation steps.
    pixels: Vec<u32>,
    /// Pixels per metre derived from the initial projected scene bounds.
    scale: f32,
    /// Projected scene centre, including zero height so falling bodies
    /// remain visible.
    centre: Vec2,
    /// Closed projection mode, matching the selected physics dimension.
    dimension: Dimension,
}

/// Construct one native simulation and keep rendering outside every
/// explicit physics step call.
pub(super) fn run(options: ViewOptions) -> Result<(), Box<dyn std::error::Error>> {
    let settings = if let Some(path) = options.settings {
        std::fs::read_to_string(path)?.parse()?
    } else {
        let defaults = SettingsInput::default();
        Settings::try_from(SettingsInput {
            scenario: options.scenario,
            dimension: options.dimension,
            count: options.count,
            ..defaults
        })?
    };
    let mut engine = options.engine;
    let mut simulation = Simulation::new(engine, settings)?;
    let mut canvas = Canvas::new(&simulation.snapshot(), settings.input().dimension);
    // Frame export exercises the actual renderer without requiring a display
    // server in CI.
    if let Some(path) = options.frame {
        canvas.draw(&simulation.snapshot());
        let mut output = std::io::BufWriter::new(std::fs::File::create(path)?);
        write!(output, "P6\n{WIDTH} {HEIGHT}\n255\n")?;
        for pixel in &canvas.pixels {
            output.write_all(&pixel.to_be_bytes()[1..])?;
        }
        return Ok(());
    }
    // minifb 0.28 presents the same RGB buffer on each supported native window
    // system.
    // https://docs.rs/minifb/0.28.0/minifb/struct.Window.html
    let mut window = Window::new(
        "Bevy physics bench",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )?;
    window.set_target_fps(60);
    let mut is_playing = true;
    let step = StepCount::try_from(1)?;
    let mut previous = std::time::Instant::now();
    let mut accumulated = std::time::Duration::ZERO;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if window.is_key_pressed(Key::Space, KeyRepeat::No) {
            is_playing = !is_playing;
        }
        let selected = [
            (Key::Key1, Engine::Avian),
            (Key::Key2, Engine::Rapier),
            (Key::Key3, Engine::Jolt),
            (Key::Key4, Engine::Bevy),
        ]
        .into_iter()
        .find_map(|(key, engine)| window.is_key_pressed(key, KeyRepeat::No).then_some(engine));
        if let Some(selected) = selected {
            match Simulation::new(selected, settings) {
                Ok(replacement) => {
                    simulation = replacement;
                    engine = selected;
                }
                Err(error) => eprintln!("{error}"),
            }
        }
        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            simulation = Simulation::new(engine, settings)?;
        }
        // Bound catch-up work so an overloaded renderer cannot accumulate an
        // unlimited backlog.
        let now = std::time::Instant::now();
        accumulated += now
            .duration_since(previous)
            .min(std::time::Duration::from_millis(100));
        previous = now;
        if is_playing {
            for _ in 0..4 {
                if accumulated < settings.timestep() {
                    break;
                }
                simulation.advance(step)?;
                accumulated -= settings.timestep();
            }
            accumulated = accumulated.min(settings.timestep());
        } else {
            accumulated = std::time::Duration::ZERO;
        }
        if window.is_key_pressed(Key::N, KeyRepeat::No) {
            is_playing = false;
            simulation.advance(step)?;
        }
        let snapshot = simulation.snapshot();
        canvas.draw(&snapshot);
        let tick = snapshot.tick;
        window.set_title(&format!("{engine:?} | tick {tick} | Space: pause | N: step | R: reset | 1 Avian / 2 Rapier / 3 Jolt / 4 Bevy"));
        window.update_with_buffer(&canvas.pixels, WIDTH, HEIGHT)?;
    }
    Ok(())
}

impl Canvas {
    /// Fit the original scene once, retaining the floor height in the
    /// visible region.
    fn new(snapshot: &Snapshot, dimension: Dimension) -> Self {
        let mut minimum = Vec2::ZERO;
        let mut maximum = Vec2::ZERO;
        for body in &snapshot.bodies {
            let point = project(Vec3::from_array(body.position), dimension);
            minimum = minimum.min(point);
            maximum = maximum.max(point);
        }
        let size = maximum - minimum + Vec2::splat(4.0);
        Self {
            pixels: vec![0; WIDTH * HEIGHT],
            scale: (WIDTH as f32 / size.x).min(HEIGHT as f32 / size.y),
            centre: (minimum + maximum) * 0.5,
            dimension,
        }
    }
    /// Draw finite geometry and passive links without changing
    /// simulation state or allocating per body.
    fn draw(&mut self, snapshot: &Snapshot) {
        self.pixels.fill(0x14212b);
        for body in &snapshot.bodies {
            let centre = Vec3::from_array(body.position);
            let rotation = Quat::from_array(body.rotation);
            let colour = if body.kind == BodyKind::Static {
                0x718492
            } else {
                ACTIVE
            };
            match body.shape {
                Shape::Box { half_extents } => {
                    let extent = Vec3::from_array(half_extents);
                    for vertex in 0..8_u8 {
                        let corner = |bits: u8| {
                            centre
                                + rotation
                                    * (extent
                                        * Vec3::new(
                                            if bits & 1 == 0 { -1.0 } else { 1.0 },
                                            if bits & 2 == 0 { -1.0 } else { 1.0 },
                                            if bits & 4 == 0 { -1.0 } else { 1.0 },
                                        ))
                        };
                        for axis in [1, 2, 4] {
                            if vertex & axis == 0 {
                                self.line(corner(vertex), corner(vertex | axis), colour);
                            }
                        }
                    }
                }
                Shape::Ball { radius } => {
                    for plane in 0..3 {
                        for segment in 0..24 {
                            let point = |index: u32| {
                                let angle = index as f32 * std::f32::consts::TAU / 24.0;
                                let (sin, cos) = angle.sin_cos();
                                let circle = match plane {
                                    0 => Vec3::new(cos, sin, 0.0),
                                    1 => Vec3::new(cos, 0.0, sin),
                                    _ => Vec3::new(0.0, cos, sin),
                                };
                                centre + rotation * circle * radius
                            };
                            self.line(point(segment), point(segment + 1), colour);
                        }
                    }
                }
            }
        }
        for &[parent, child] in &snapshot.joints {
            if let (Some(parent), Some(child)) =
                (snapshot.bodies.get(parent), snapshot.bodies.get(child))
            {
                self.line(
                    Vec3::from_array(parent.position),
                    Vec3::from_array(child.position),
                    0xe8edf0,
                );
            }
        }
    }
    /// Rasterize a projected edge with bounded work and discard pixels
    /// outside the framebuffer.
    fn line(&mut self, first: Vec3, second: Vec3, colour: u32) {
        let screen = |point| {
            (project(point, self.dimension) - self.centre) * self.scale
                + Vec2::new(WIDTH as f32, HEIGHT as f32) * 0.5
        };
        let first = screen(first);
        let second = screen(second);
        if !first.is_finite() || !second.is_finite() {
            return;
        }
        let count = (second - first)
            .abs()
            .max_element()
            .ceil()
            .clamp(1.0, 1920.0) as u32;
        for index in 0..=count {
            let point = first.lerp(second, index as f32 / count as f32);
            if point.x >= 0.0 && point.y >= 0.0 && point.x < WIDTH as f32 && point.y < HEIGHT as f32
            {
                self.pixels[point.y as usize * WIDTH + point.x as usize] = colour;
            }
        }
    }
}

/// Project metre coordinates into a planar or isometric view before scaling
/// them into pixels.
fn project(point: Vec3, dimension: Dimension) -> Vec2 {
    match dimension {
        Dimension::Two => Vec2::new(point.x, -point.y),
        Dimension::Three => Vec2::new(
            (point.x - point.z) * std::f32::consts::FRAC_1_SQRT_2,
            (point.x + point.z) * 0.35 - point.y * 0.866,
        ),
    }
}
