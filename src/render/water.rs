//! Water surface rendering with animated waves.
//!
//! Creates an animated water surface for rivers using a custom shader.

use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
        render_resource::{AsBindGroup, ShaderRef},
    },
};

use crate::procgen::river::{River, RiverGenerated};

pub struct WaterPlugin;

impl Plugin for WaterPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<WaterMaterial>::default())
            .init_resource::<WaterConfig>()
            .init_resource::<WaterSpawned>()
            .add_systems(Update, (spawn_water_mesh, update_water_animation));
    }
}

/// Configuration for water appearance.
#[derive(Resource, Clone)]
pub struct WaterConfig {
    /// Wave animation speed.
    pub wave_speed: f32,
    /// Wave height amplitude.
    pub wave_amplitude: f32,
    /// Wave frequency (higher = more waves).
    pub wave_frequency: f32,
    /// Deep water color.
    pub deep_color: Color,
    /// Shallow/edge water color.
    pub shallow_color: Color,
    /// Water opacity (0-1).
    pub opacity: f32,
    /// Foam intensity at edges.
    pub foam_intensity: f32,
}

impl Default for WaterConfig {
    fn default() -> Self {
        Self {
            wave_speed: 0.8,
            wave_amplitude: 0.12,
            wave_frequency: 0.4,
            deep_color: Color::srgb(0.1, 0.25, 0.4),
            shallow_color: Color::srgb(0.2, 0.45, 0.55),
            opacity: 0.85,
            foam_intensity: 0.3,
        }
    }
}

/// Custom material for animated water surface.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct WaterMaterial {
    #[uniform(0)]
    pub time: f32,
    #[uniform(0)]
    pub wave_speed: f32,
    #[uniform(0)]
    pub wave_amplitude: f32,
    #[uniform(0)]
    pub wave_frequency: f32,
    #[uniform(0)]
    pub deep_color: LinearRgba,
    #[uniform(0)]
    pub shallow_color: LinearRgba,
    #[uniform(0)]
    pub foam_intensity: f32,
    #[uniform(0)]
    pub opacity: f32,
}

impl Material for WaterMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/water.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

impl Default for WaterMaterial {
    fn default() -> Self {
        let config = WaterConfig::default();
        Self {
            time: 0.0,
            wave_speed: config.wave_speed,
            wave_amplitude: config.wave_amplitude,
            wave_frequency: config.wave_frequency,
            deep_color: config.deep_color.to_linear(),
            shallow_color: config.shallow_color.to_linear(),
            foam_intensity: config.foam_intensity,
            opacity: config.opacity,
        }
    }
}

/// Marker component for water surface entities.
#[derive(Component)]
pub struct WaterSurface;

/// Marker resource indicating water has been spawned.
#[derive(Resource, Default)]
pub struct WaterSpawned(pub bool);

/// Spawn water mesh when river is generated.
fn spawn_water_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<WaterMaterial>>,
    river: Res<River>,
    river_generated: Res<RiverGenerated>,
    config: Res<WaterConfig>,
    mut spawned: ResMut<WaterSpawned>,
) {
    if spawned.0 || !river_generated.0 || river.centerline.is_empty() {
        return;
    }

    // Create water mesh from river banks
    let water_mesh = create_water_mesh(&river);

    // Create water material
    let water_material = materials.add(WaterMaterial {
        time: 0.0,
        wave_speed: config.wave_speed,
        wave_amplitude: config.wave_amplitude,
        wave_frequency: config.wave_frequency,
        deep_color: config.deep_color.to_linear(),
        shallow_color: config.shallow_color.to_linear(),
        foam_intensity: config.foam_intensity,
        opacity: config.opacity,
    });

    // Spawn water surface entity
    commands.spawn((
        Mesh3d(meshes.add(water_mesh)),
        MeshMaterial3d(water_material),
        Transform::from_xyz(0.0, river.water_level, 0.0),
        WaterSurface,
    ));

    spawned.0 = true;
    info!(
        "Water surface spawned at level {:.1}",
        river.water_level
    );
}

/// Create water mesh as a quad strip following river banks.
fn create_water_mesh(river: &River) -> Mesh {
    let num_points = river.left_bank.len();
    if num_points < 2 {
        // Return empty mesh if river is too short
        return Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );
    }

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(num_points * 2);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(num_points * 2);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(num_points * 2);

    // Generate vertices along both banks
    for i in 0..num_points {
        let left = river.left_bank[i];
        let right = river.right_bank[i];

        // Position at water level (Y=0, transform will offset)
        positions.push([left.x, 0.0, left.y]);
        positions.push([right.x, 0.0, right.y]);

        // Water surface normal points up
        normals.push([0.0, 1.0, 0.0]);
        normals.push([0.0, 1.0, 0.0]);

        // UV: X = 0 for left bank, 1 for right bank
        // UV: Y = progress along river (0 to 1)
        let v = i as f32 / (num_points - 1) as f32;
        uvs.push([0.0, v]);
        uvs.push([1.0, v]);
    }

    // Generate triangle indices (quad strip)
    let mut indices: Vec<u32> = Vec::with_capacity((num_points - 1) * 6);
    for i in 0..(num_points - 1) {
        let base = (i * 2) as u32;
        // Two triangles per quad (CCW winding for top-facing)
        // First triangle
        indices.push(base);
        indices.push(base + 2);
        indices.push(base + 1);
        // Second triangle
        indices.push(base + 1);
        indices.push(base + 2);
        indices.push(base + 3);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    mesh
}

/// Update water animation by advancing time uniform.
fn update_water_animation(
    time: Res<Time>,
    mut materials: ResMut<Assets<WaterMaterial>>,
    query: Query<&MeshMaterial3d<WaterMaterial>, With<WaterSurface>>,
) {
    for handle in query.iter() {
        if let Some(material) = materials.get_mut(handle) {
            material.time = time.elapsed_secs();
        }
    }
}
