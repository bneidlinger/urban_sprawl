//! Celestial bodies - visible moon and stars for the night sky.
//!
//! Creates a moon mesh and procedural star field that rotate with the day/night cycle.

use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;

use crate::render::day_night::TimeOfDay;

pub struct CelestialPlugin;

impl Plugin for CelestialPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CelestialConfig>()
            .add_systems(Startup, setup_celestials)
            .add_systems(Update, (update_moon_position, update_stars_visibility));
    }
}

/// Configuration for celestial rendering.
#[derive(Resource)]
pub struct CelestialConfig {
    /// Moon distance from camera (sky dome radius).
    pub sky_radius: f32,
    /// Moon mesh radius.
    pub moon_radius: f32,
    /// Number of stars to generate.
    pub star_count: usize,
    /// Star size range.
    pub star_size_min: f32,
    pub star_size_max: f32,
}

impl Default for CelestialConfig {
    fn default() -> Self {
        Self {
            sky_radius: 800.0,
            moon_radius: 20.0,
            star_count: 200,
            star_size_min: 0.5,
            star_size_max: 2.0,
        }
    }
}

/// Marker for the moon mesh.
#[derive(Component)]
pub struct VisibleMoon;

/// Marker for individual star entities.
#[derive(Component)]
pub struct Star {
    /// Fixed direction in sky (normalized).
    pub direction: Vec3,
    /// Base emissive intensity.
    pub intensity: f32,
    /// Twinkle phase offset.
    pub twinkle_phase: f32,
}

/// Marker for the star parent (rotates with sky).
#[derive(Component)]
pub struct StarField;

fn setup_celestials(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<CelestialConfig>,
) {
    // Moon mesh - a simple sphere with emissive material
    let moon_mesh = meshes.add(Sphere::new(config.moon_radius).mesh().ico(2).unwrap());
    let moon_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.93, 0.85),
        emissive: bevy::color::palettes::css::ANTIQUE_WHITE.into(),
        perceptual_roughness: 0.8,
        metallic: 0.0,
        unlit: true,
        ..default()
    });

    commands.spawn((
        Mesh3d(moon_mesh),
        MeshMaterial3d(moon_material),
        Transform::from_xyz(0.0, config.sky_radius * 0.7, -config.sky_radius * 0.7),
        Visibility::Hidden,
        VisibleMoon,
    ));

    // Star field - small emissive spheres at fixed sky positions
    let star_mesh = meshes.add(create_star_mesh());

    // Generate stars in random directions on the sky dome
    let mut rng_seed = 42u64;
    let star_field_parent = commands
        .spawn((
            Transform::IDENTITY,
            Visibility::Hidden,
            StarField,
        ))
        .id();

    for _ in 0..config.star_count {
        // Simple LCG random for reproducible stars
        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let r1 = ((rng_seed >> 33) as f32) / (u32::MAX as f32);
        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let r2 = ((rng_seed >> 33) as f32) / (u32::MAX as f32);
        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let r3 = ((rng_seed >> 33) as f32) / (u32::MAX as f32);
        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let r4 = ((rng_seed >> 33) as f32) / (u32::MAX as f32);

        // Spherical coordinates for uniform distribution on upper hemisphere
        let theta = r1 * std::f32::consts::TAU; // Azimuth
        let phi = (r2 * 0.5 + 0.2).acos(); // Elevation (favor upper hemisphere)

        let direction = Vec3::new(
            phi.sin() * theta.cos(),
            phi.cos(), // Y is up
            phi.sin() * theta.sin(),
        )
        .normalize();

        // Position on sky dome
        let position = direction * config.sky_radius;

        // Star size varies
        let size = config.star_size_min + r3 * (config.star_size_max - config.star_size_min);

        // Star color varies slightly (blue-white to yellow-white)
        let color_temp = 0.8 + r4 * 0.4; // 0.8 to 1.2
        let star_color = Color::srgb(
            1.0,
            0.9 + (color_temp - 1.0) * 0.2,
            0.7 + color_temp * 0.3,
        );

        // Convert to linear for emissive multiplication
        let star_linear = star_color.to_linear();
        let emissive_linear = bevy::color::LinearRgba::new(
            star_linear.red * 3.0,
            star_linear.green * 3.0,
            star_linear.blue * 3.0,
            1.0,
        );

        let star_material = materials.add(StandardMaterial {
            base_color: star_color,
            emissive: emissive_linear.into(),
            unlit: true,
            ..default()
        });

        let intensity = 0.5 + r3 * 0.5;
        let twinkle_phase = r4 * std::f32::consts::TAU;

        let star_entity = commands
            .spawn((
                Mesh3d(star_mesh.clone()),
                MeshMaterial3d(star_material),
                Transform::from_translation(position).with_scale(Vec3::splat(size)),
                Star {
                    direction,
                    intensity,
                    twinkle_phase,
                },
            ))
            .id();

        commands.entity(star_field_parent).add_child(star_entity);
    }

    info!(
        "Celestials setup: 1 moon, {} stars on sky dome",
        config.star_count
    );
}

/// Create a simple star mesh (small octahedron for sparkle effect).
fn create_star_mesh() -> Mesh {
    // Octahedron vertices
    let vertices: Vec<[f32; 3]> = vec![
        [0.0, 1.0, 0.0],  // Top
        [1.0, 0.0, 0.0],  // Right
        [0.0, 0.0, 1.0],  // Front
        [-1.0, 0.0, 0.0], // Left
        [0.0, 0.0, -1.0], // Back
        [0.0, -1.0, 0.0], // Bottom
    ];

    let indices = vec![
        0, 1, 2, // Top front right
        0, 2, 3, // Top front left
        0, 3, 4, // Top back left
        0, 4, 1, // Top back right
        5, 2, 1, // Bottom front right
        5, 3, 2, // Bottom front left
        5, 4, 3, // Bottom back left
        5, 1, 4, // Bottom back right
    ];

    let normals: Vec<[f32; 3]> = vertices
        .iter()
        .map(|v| {
            let len: f32 = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            [v[0] / len, v[1] / len, v[2] / len]
        })
        .collect();

    let uvs: Vec<[f32; 2]> = vertices.iter().map(|_| [0.5, 0.5]).collect();

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

/// Update moon position based on time of day.
fn update_moon_position(
    tod: Option<Res<TimeOfDay>>,
    config: Res<CelestialConfig>,
    mut moon_query: Query<(&mut Transform, &mut Visibility), With<VisibleMoon>>,
) {
    let Some(tod) = tod else { return };

    let hour = tod.hour();

    // Moon is visible from 6 PM to 6 AM (night hours)
    let is_night = hour < 6.0 || hour > 18.0;

    for (mut transform, mut visibility) in moon_query.iter_mut() {
        if is_night {
            *visibility = Visibility::Visible;

            // Moon angle - rises when sun sets
            // At 6 PM (18:00): moon at eastern horizon
            // At midnight (0:00): moon at zenith
            // At 6 AM (6:00): moon at western horizon
            let moon_time = if hour >= 18.0 {
                (hour - 18.0) / 12.0 // 18-24 maps to 0.0-0.5
            } else {
                (hour + 6.0) / 12.0 // 0-6 maps to 0.5-1.0
            };

            let moon_angle = moon_time * std::f32::consts::PI; // 0 to PI

            // Moon position on arc
            let height = moon_angle.sin() * config.sky_radius * 0.8;
            let x = moon_angle.cos() * config.sky_radius * 0.9;

            transform.translation = Vec3::new(x, height, -config.sky_radius * 0.3);
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

/// Update star visibility based on time of day.
fn update_stars_visibility(
    tod: Option<Res<TimeOfDay>>,
    mut star_field_query: Query<&mut Visibility, With<StarField>>,
) {
    let Some(tod) = tod else { return };

    let hour = tod.hour();

    // Stars visible during night and twilight
    let stars_visible = hour < 6.5 || hour > 18.5;

    // Fade during twilight (slightly before/after moon)
    for mut visibility in star_field_query.iter_mut() {
        *visibility = if stars_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
