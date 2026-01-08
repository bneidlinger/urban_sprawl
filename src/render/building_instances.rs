//! Building instance buffer management for GPU instancing.
//!
//! This module provides the data structures and resources for managing
//! building instances that are rendered via GPU instancing.

use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};

use crate::procgen::building_factory::{BuildingArchetype, BuildingShape, FacadeStyle};

pub struct BuildingInstancesPlugin;

impl Plugin for BuildingInstancesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BuildingInstanceBuffer>()
            .init_resource::<BuildingMaterialPalette>()
            .add_systems(Startup, initialize_material_palette);
    }
}

/// Initialize the shared material palette at startup.
fn initialize_material_palette(
    mut palette: ResMut<BuildingMaterialPalette>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    palette.initialize(&mut materials);
    info!("Building material palette initialized: {} materials",
        palette.brick.len() + palette.concrete.len() + palette.glass.len() +
        palette.metal.len() + palette.painted.len());
}

/// Extended per-instance data for GPU instancing.
///
/// This structure is designed for efficient GPU rendering with full transform
/// support and material parameters. Total size: 128 bytes (aligned to 16 bytes).
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
#[repr(C)]
pub struct BuildingInstanceData {
    /// Full 4x4 transform matrix (64 bytes)
    /// Allows non-uniform scaling for different building dimensions
    pub transform: [[f32; 4]; 4],

    /// RGBA color (16 bytes)
    pub color: [f32; 4],

    /// Material parameters (16 bytes)
    /// x = roughness (0.0-1.0)
    /// y = metallic (0.0-1.0)
    /// z = emissive intensity (0.0+)
    /// w = facade type index (0-4: Brick, Concrete, Glass, Metal, Painted)
    pub material_params: [f32; 4],

    /// Bounding information (16 bytes)
    /// xyz = half-extents of AABB
    /// w = mesh type index (0=Box, 1=LShape, 2=TowerBase, 3=Stepped)
    pub bounds: [f32; 4],

    /// Additional data (16 bytes)
    /// x = building archetype (0=Residential, 1=Commercial, 2=Industrial)
    /// y = lot index (for lookup)
    /// z = flags (bit 0 = visible, bit 1 = selected, etc.)
    /// w = reserved
    pub extra: [f32; 4],
}

impl Default for BuildingInstanceData {
    fn default() -> Self {
        Self {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            color: [1.0, 1.0, 1.0, 1.0],
            material_params: [0.8, 0.0, 0.0, 0.0],
            bounds: [0.5, 0.5, 0.5, 0.0],
            extra: [0.0, 0.0, 1.0, 0.0], // visible by default
        }
    }
}

impl BuildingInstanceData {
    /// Create instance data from building parameters.
    pub fn new(
        position: Vec3,
        scale: Vec3,
        rotation: Quat,
        color: Color,
        facade: FacadeStyle,
        archetype: BuildingArchetype,
        shape: BuildingShape,
        lot_index: usize,
    ) -> Self {
        let transform = Mat4::from_scale_rotation_translation(scale, rotation, position);
        let (roughness, metallic) = facade_material_params(facade);
        let linear_color = color.to_linear();

        Self {
            transform: transform.to_cols_array_2d(),
            color: linear_color.to_f32_array(),
            material_params: [roughness, metallic, 0.0, facade_to_index(facade) as f32],
            bounds: [scale.x / 2.0, scale.y / 2.0, scale.z / 2.0, shape_to_index(shape) as f32],
            extra: [
                archetype_to_index(archetype) as f32,
                lot_index as f32,
                1.0, // visible
                0.0,
            ],
        }
    }

    /// Create instance data from a Transform.
    pub fn from_transform(
        transform: &Transform,
        color: Color,
        facade: FacadeStyle,
        archetype: BuildingArchetype,
        shape: BuildingShape,
        lot_index: usize,
    ) -> Self {
        Self::new(
            transform.translation,
            transform.scale,
            transform.rotation,
            color,
            facade,
            archetype,
            shape,
            lot_index,
        )
    }

    /// Get the position from the transform matrix.
    pub fn position(&self) -> Vec3 {
        Vec3::new(self.transform[3][0], self.transform[3][1], self.transform[3][2])
    }

    /// Get the bounding sphere radius (approximate).
    pub fn bounding_radius(&self) -> f32 {
        let half_extents = Vec3::new(self.bounds[0], self.bounds[1], self.bounds[2]);
        half_extents.length()
    }

    /// Check if this instance is marked as visible.
    pub fn is_visible(&self) -> bool {
        (self.extra[2] as u32 & 1) != 0
    }

    /// Set visibility flag.
    pub fn set_visible(&mut self, visible: bool) {
        let flags = self.extra[2] as u32;
        self.extra[2] = if visible {
            (flags | 1) as f32
        } else {
            (flags & !1) as f32
        };
    }
}

/// Get material parameters for a facade style.
fn facade_material_params(facade: FacadeStyle) -> (f32, f32) {
    match facade {
        FacadeStyle::Brick => (0.9, 0.0),
        FacadeStyle::Concrete => (0.85, 0.0),
        FacadeStyle::Glass => (0.2, 0.1),
        FacadeStyle::Metal => (0.35, 0.6),
        FacadeStyle::Painted => (0.75, 0.0),
    }
}

/// Convert facade style to index for shader.
fn facade_to_index(facade: FacadeStyle) -> u32 {
    match facade {
        FacadeStyle::Brick => 0,
        FacadeStyle::Concrete => 1,
        FacadeStyle::Glass => 2,
        FacadeStyle::Metal => 3,
        FacadeStyle::Painted => 4,
    }
}

/// Convert building archetype to index.
fn archetype_to_index(archetype: BuildingArchetype) -> u32 {
    match archetype {
        BuildingArchetype::Residential => 0,
        BuildingArchetype::Commercial => 1,
        BuildingArchetype::Industrial => 2,
    }
}

/// Convert building shape to index.
fn shape_to_index(shape: BuildingShape) -> u32 {
    match shape {
        BuildingShape::Box => 0,
        BuildingShape::LShape => 1,
        BuildingShape::TowerOnBase => 2,
        BuildingShape::Stepped => 3,
    }
}

/// Resource containing all building instance data.
#[derive(Resource, Default)]
pub struct BuildingInstanceBuffer {
    /// CPU-side instance data
    pub instances: Vec<BuildingInstanceData>,

    /// Dirty flag - set when instances need to be re-uploaded to GPU
    pub dirty: bool,

    /// Statistics
    pub stats: InstanceStats,
}

/// Statistics about the instance buffer.
#[derive(Default, Clone, Debug)]
pub struct InstanceStats {
    pub total_instances: usize,
    pub visible_instances: usize,
    pub box_count: usize,
    pub l_shape_count: usize,
    pub tower_count: usize,
    pub stepped_count: usize,
}

impl BuildingInstanceBuffer {
    /// Add a new building instance.
    pub fn add(&mut self, instance: BuildingInstanceData) -> usize {
        let index = self.instances.len();
        self.instances.push(instance);
        self.dirty = true;
        index
    }

    /// Update an existing instance.
    pub fn update(&mut self, index: usize, instance: BuildingInstanceData) {
        if index < self.instances.len() {
            self.instances[index] = instance;
            self.dirty = true;
        }
    }

    /// Clear all instances.
    pub fn clear(&mut self) {
        self.instances.clear();
        self.dirty = true;
        self.stats = InstanceStats::default();
    }

    /// Recalculate statistics.
    pub fn update_stats(&mut self) {
        self.stats = InstanceStats {
            total_instances: self.instances.len(),
            visible_instances: self.instances.iter().filter(|i| i.is_visible()).count(),
            box_count: self.instances.iter().filter(|i| i.bounds[3] as u32 == 0).count(),
            l_shape_count: self.instances.iter().filter(|i| i.bounds[3] as u32 == 1).count(),
            tower_count: self.instances.iter().filter(|i| i.bounds[3] as u32 == 2).count(),
            stepped_count: self.instances.iter().filter(|i| i.bounds[3] as u32 == 3).count(),
        };
    }

    /// Get instance data as bytes for GPU upload.
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.instances)
    }
}

/// Pre-computed material palette for buildings.
/// Stores handles to StandardMaterial for each facade/color combination.
#[derive(Resource)]
pub struct BuildingMaterialPalette {
    /// Brick materials (3 color variants)
    pub brick: Vec<Handle<StandardMaterial>>,
    /// Concrete materials (3 color variants)
    pub concrete: Vec<Handle<StandardMaterial>>,
    /// Glass materials (3 color variants)
    pub glass: Vec<Handle<StandardMaterial>>,
    /// Metal materials (3 color variants)
    pub metal: Vec<Handle<StandardMaterial>>,
    /// Painted materials (3 color variants)
    pub painted: Vec<Handle<StandardMaterial>>,
}

impl Default for BuildingMaterialPalette {
    fn default() -> Self {
        Self {
            brick: Vec::new(),
            concrete: Vec::new(),
            glass: Vec::new(),
            metal: Vec::new(),
            painted: Vec::new(),
        }
    }
}

impl BuildingMaterialPalette {
    /// Initialize the palette with actual materials.
    pub fn initialize(&mut self, materials: &mut Assets<StandardMaterial>) {
        // Brick colors
        let brick_colors = [
            Color::srgb(0.74, 0.48, 0.38),
            Color::srgb(0.68, 0.42, 0.32),
            Color::srgb(0.8, 0.54, 0.42),
        ];
        self.brick = brick_colors
            .iter()
            .map(|&c| {
                materials.add(StandardMaterial {
                    base_color: c,
                    perceptual_roughness: 0.9,
                    metallic: 0.0,
                    ..default()
                })
            })
            .collect();

        // Concrete colors
        let concrete_colors = [
            Color::srgb(0.65, 0.65, 0.65),
            Color::srgb(0.72, 0.72, 0.72),
            Color::srgb(0.58, 0.6, 0.62),
        ];
        self.concrete = concrete_colors
            .iter()
            .map(|&c| {
                materials.add(StandardMaterial {
                    base_color: c,
                    perceptual_roughness: 0.85,
                    metallic: 0.0,
                    ..default()
                })
            })
            .collect();

        // Glass colors
        let glass_colors = [
            Color::srgb(0.4, 0.6, 0.75),
            Color::srgb(0.35, 0.55, 0.7),
            Color::srgb(0.45, 0.65, 0.8),
        ];
        self.glass = glass_colors
            .iter()
            .map(|&c| {
                materials.add(StandardMaterial {
                    base_color: c,
                    perceptual_roughness: 0.2,
                    metallic: 0.1,
                    ..default()
                })
            })
            .collect();

        // Metal colors
        let metal_colors = [
            Color::srgb(0.55, 0.55, 0.58),
            Color::srgb(0.5, 0.5, 0.52),
            Color::srgb(0.48, 0.5, 0.55),
        ];
        self.metal = metal_colors
            .iter()
            .map(|&c| {
                materials.add(StandardMaterial {
                    base_color: c,
                    perceptual_roughness: 0.35,
                    metallic: 0.6,
                    ..default()
                })
            })
            .collect();

        // Painted colors
        let painted_colors = [
            Color::srgb(0.85, 0.78, 0.62),
            Color::srgb(0.92, 0.86, 0.72),
            Color::srgb(0.76, 0.82, 0.72),
        ];
        self.painted = painted_colors
            .iter()
            .map(|&c| {
                materials.add(StandardMaterial {
                    base_color: c,
                    perceptual_roughness: 0.75,
                    metallic: 0.0,
                    ..default()
                })
            })
            .collect();
    }

    /// Get a material for a facade style with random color variant.
    pub fn get(&self, facade: FacadeStyle, variant: usize) -> Option<Handle<StandardMaterial>> {
        let palette = match facade {
            FacadeStyle::Brick => &self.brick,
            FacadeStyle::Concrete => &self.concrete,
            FacadeStyle::Glass => &self.glass,
            FacadeStyle::Metal => &self.metal,
            FacadeStyle::Painted => &self.painted,
        };
        palette.get(variant % palette.len()).cloned()
    }

    /// Get the color for a facade style variant (for instance data).
    pub fn get_color(facade: FacadeStyle, variant: usize) -> Color {
        let colors = match facade {
            FacadeStyle::Brick => [
                Color::srgb(0.74, 0.48, 0.38),
                Color::srgb(0.68, 0.42, 0.32),
                Color::srgb(0.8, 0.54, 0.42),
            ],
            FacadeStyle::Concrete => [
                Color::srgb(0.65, 0.65, 0.65),
                Color::srgb(0.72, 0.72, 0.72),
                Color::srgb(0.58, 0.6, 0.62),
            ],
            FacadeStyle::Glass => [
                Color::srgb(0.4, 0.6, 0.75),
                Color::srgb(0.35, 0.55, 0.7),
                Color::srgb(0.45, 0.65, 0.8),
            ],
            FacadeStyle::Metal => [
                Color::srgb(0.55, 0.55, 0.58),
                Color::srgb(0.5, 0.5, 0.52),
                Color::srgb(0.48, 0.5, 0.55),
            ],
            FacadeStyle::Painted => [
                Color::srgb(0.85, 0.78, 0.62),
                Color::srgb(0.92, 0.86, 0.72),
                Color::srgb(0.76, 0.82, 0.72),
            ],
        };
        colors[variant % colors.len()]
    }
}

/// Component marking an entity as using instanced rendering.
/// The entity should have a reference to the instance buffer index.
#[derive(Component)]
pub struct InstancedBuilding {
    /// Index into BuildingInstanceBuffer
    pub instance_index: usize,
    /// Building shape for mesh selection
    pub shape: BuildingShape,
}

/// Component for lightweight building representation (no mesh, just instance data).
#[derive(Component)]
pub struct BuildingRef {
    pub lot_index: usize,
    pub instance_index: usize,
    pub archetype: BuildingArchetype,
    pub facade: FacadeStyle,
    pub shape: BuildingShape,
}
