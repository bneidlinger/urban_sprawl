//! Shared mesh pools for GPU instancing.
//!
//! Instead of creating unique meshes per building, this module provides
//! canonical mesh templates that are shared across all instances.

use bevy::prelude::*;

pub struct MeshPoolsPlugin;

impl Plugin for MeshPoolsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_mesh_pools);
    }
}

/// Shared mesh pool for building shapes.
/// All buildings of the same shape share these mesh handles.
#[derive(Resource)]
pub struct BuildingMeshPool {
    /// Unit cube (1x1x1) - scaled via instance transform for Box buildings
    pub box_mesh: Handle<Mesh>,

    /// L-shape variants (4 rotation variants for variety)
    pub l_shapes: Vec<Handle<Mesh>>,

    /// Tower-on-base: podium + tower composite meshes
    /// Index 0: base only (for separate tower spawning)
    pub tower_bases: Vec<Handle<Mesh>>,

    /// Stepped building variants (2-3 tier configurations)
    pub stepped: Vec<Handle<Mesh>>,
}

/// Shared mesh pool for park/vegetation elements.
#[derive(Resource)]
pub struct VegetationMeshPool {
    /// Tree trunk cylinder
    pub trunk_mesh: Handle<Mesh>,

    /// Tree foliage sphere
    pub foliage_mesh: Handle<Mesh>,

    /// Unit ground plane for grass
    pub grass_plane: Handle<Mesh>,
}

/// Shared mesh pool for street furniture.
#[derive(Resource)]
pub struct FurnitureMeshPool {
    /// Street lamp pole cylinder
    pub lamp_pole: Handle<Mesh>,

    /// Street lamp fixture sphere
    pub lamp_fixture: Handle<Mesh>,

    /// Traffic light pole
    pub traffic_pole: Handle<Mesh>,

    /// Traffic light box
    pub traffic_box: Handle<Mesh>,

    /// Traffic light lens sphere
    pub traffic_lens: Handle<Mesh>,
}

fn setup_mesh_pools(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    // Building meshes
    let building_pool = BuildingMeshPool {
        // Unit cube for Box buildings - all scaling via instance transform
        box_mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),

        // L-shape variants with different wing ratios
        l_shapes: vec![
            // Wing ratio 0.4 - narrow side wing
            meshes.add(create_l_shape_mesh(0.4)),
            // Wing ratio 0.5 - balanced L
            meshes.add(create_l_shape_mesh(0.5)),
            // Wing ratio 0.6 - wide side wing
            meshes.add(create_l_shape_mesh(0.6)),
        ],

        // Tower base variants with different base height ratios
        tower_bases: vec![
            // Low podium (20% of total height)
            meshes.add(Cuboid::new(1.0, 0.2, 1.0)),
            // Medium podium (30% of total height)
            meshes.add(Cuboid::new(1.0, 0.3, 1.0)),
            // Tall podium (40% of total height)
            meshes.add(Cuboid::new(1.0, 0.4, 1.0)),
        ],

        // Stepped building variants
        stepped: vec![
            // 2-step building
            meshes.add(create_stepped_mesh(2)),
            // 3-step building
            meshes.add(create_stepped_mesh(3)),
        ],
    };

    // Vegetation meshes
    let vegetation_pool = VegetationMeshPool {
        trunk_mesh: meshes.add(Cylinder::new(0.3, 1.0)),
        foliage_mesh: meshes.add(Sphere::new(1.0)),
        grass_plane: meshes.add(Cuboid::new(1.0, 0.15, 1.0)),
    };

    // Street furniture meshes
    let furniture_pool = FurnitureMeshPool {
        lamp_pole: meshes.add(Cylinder::new(0.15, 1.0)),
        lamp_fixture: meshes.add(Sphere::new(0.4)),
        traffic_pole: meshes.add(Cylinder::new(0.1, 1.0)),
        traffic_box: meshes.add(Cuboid::new(0.4, 0.8, 0.3)),
        traffic_lens: meshes.add(Sphere::new(0.12)),
    };

    commands.insert_resource(building_pool);
    commands.insert_resource(vegetation_pool);
    commands.insert_resource(furniture_pool);

    info!("Mesh pools initialized");
}

/// Create an L-shaped mesh with the given wing ratio.
/// The mesh is normalized to fit in a 1x1x1 bounding box.
fn create_l_shape_mesh(wing_ratio: f32) -> Mesh {
    use bevy::render::mesh::{Indices, PrimitiveTopology};
    use bevy::render::render_asset::RenderAssetUsages;

    // L-shape consists of two overlapping boxes
    // Main wing: full width, partial depth
    // Side wing: partial width, remaining depth

    let main_depth = wing_ratio;
    let side_width = wing_ratio;
    let side_depth = 1.0 - main_depth + 0.1; // Slight overlap

    // We'll create vertices for both wings combined
    // Main wing: centered at z = (1 - main_depth) / 2
    // Side wing: centered at x = -(1 - side_width) / 2, z = -main_depth / 2

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    // Helper to add a box's vertices
    let add_box = |positions: &mut Vec<[f32; 3]>,
                   normals: &mut Vec<[f32; 3]>,
                   uvs: &mut Vec<[f32; 2]>,
                   indices: &mut Vec<u32>,
                   center: [f32; 3],
                   size: [f32; 3]| {
        let base_idx = positions.len() as u32;
        let hx = size[0] / 2.0;
        let hy = size[1] / 2.0;
        let hz = size[2] / 2.0;
        let cx = center[0];
        let cy = center[1];
        let cz = center[2];

        // 6 faces, 4 vertices each = 24 vertices
        // Front face (+Z)
        positions.extend([
            [cx - hx, cy - hy, cz + hz],
            [cx + hx, cy - hy, cz + hz],
            [cx + hx, cy + hy, cz + hz],
            [cx - hx, cy + hy, cz + hz],
        ]);
        normals.extend([[0.0, 0.0, 1.0]; 4]);
        uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);

        // Back face (-Z)
        positions.extend([
            [cx + hx, cy - hy, cz - hz],
            [cx - hx, cy - hy, cz - hz],
            [cx - hx, cy + hy, cz - hz],
            [cx + hx, cy + hy, cz - hz],
        ]);
        normals.extend([[0.0, 0.0, -1.0]; 4]);
        uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);

        // Right face (+X)
        positions.extend([
            [cx + hx, cy - hy, cz + hz],
            [cx + hx, cy - hy, cz - hz],
            [cx + hx, cy + hy, cz - hz],
            [cx + hx, cy + hy, cz + hz],
        ]);
        normals.extend([[1.0, 0.0, 0.0]; 4]);
        uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);

        // Left face (-X)
        positions.extend([
            [cx - hx, cy - hy, cz - hz],
            [cx - hx, cy - hy, cz + hz],
            [cx - hx, cy + hy, cz + hz],
            [cx - hx, cy + hy, cz - hz],
        ]);
        normals.extend([[-1.0, 0.0, 0.0]; 4]);
        uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);

        // Top face (+Y)
        positions.extend([
            [cx - hx, cy + hy, cz + hz],
            [cx + hx, cy + hy, cz + hz],
            [cx + hx, cy + hy, cz - hz],
            [cx - hx, cy + hy, cz - hz],
        ]);
        normals.extend([[0.0, 1.0, 0.0]; 4]);
        uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);

        // Bottom face (-Y)
        positions.extend([
            [cx - hx, cy - hy, cz - hz],
            [cx + hx, cy - hy, cz - hz],
            [cx + hx, cy - hy, cz + hz],
            [cx - hx, cy - hy, cz + hz],
        ]);
        normals.extend([[0.0, -1.0, 0.0]; 4]);
        uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);

        // Indices for 6 faces (2 triangles each)
        for face in 0..6 {
            let fi = base_idx + face * 4;
            indices.extend([fi, fi + 1, fi + 2, fi, fi + 2, fi + 3]);
        }
    };

    // Main wing
    let main_center = [0.0, 0.0, (1.0 - main_depth) / 2.0];
    let main_size = [1.0, 1.0, main_depth];
    add_box(
        &mut positions,
        &mut normals,
        &mut uvs,
        &mut indices,
        main_center,
        main_size,
    );

    // Side wing
    let side_center = [-(1.0 - side_width) / 2.0, 0.0, -(main_depth / 2.0)];
    let side_size = [side_width, 1.0, side_depth];
    add_box(
        &mut positions,
        &mut normals,
        &mut uvs,
        &mut indices,
        side_center,
        side_size,
    );

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

/// Create a stepped building mesh with the given number of steps.
/// Each step is 15% smaller than the previous.
fn create_stepped_mesh(num_steps: usize) -> Mesh {
    use bevy::render::mesh::{Indices, PrimitiveTopology};
    use bevy::render::render_asset::RenderAssetUsages;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    let step_height = 1.0 / num_steps as f32;

    for i in 0..num_steps {
        let shrink = 1.0 - (i as f32 * 0.15);
        let y_base = step_height * i as f32;
        let y_center = y_base + step_height / 2.0 - 0.5; // Center around origin

        let base_idx = positions.len() as u32;
        let hx = shrink / 2.0;
        let hy = step_height / 2.0;
        let hz = shrink / 2.0;

        // 6 faces per step
        // Front face (+Z)
        positions.extend([
            [-hx, y_center - hy, hz],
            [hx, y_center - hy, hz],
            [hx, y_center + hy, hz],
            [-hx, y_center + hy, hz],
        ]);
        normals.extend([[0.0, 0.0, 1.0]; 4]);
        uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);

        // Back face (-Z)
        positions.extend([
            [hx, y_center - hy, -hz],
            [-hx, y_center - hy, -hz],
            [-hx, y_center + hy, -hz],
            [hx, y_center + hy, -hz],
        ]);
        normals.extend([[0.0, 0.0, -1.0]; 4]);
        uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);

        // Right face (+X)
        positions.extend([
            [hx, y_center - hy, hz],
            [hx, y_center - hy, -hz],
            [hx, y_center + hy, -hz],
            [hx, y_center + hy, hz],
        ]);
        normals.extend([[1.0, 0.0, 0.0]; 4]);
        uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);

        // Left face (-X)
        positions.extend([
            [-hx, y_center - hy, -hz],
            [-hx, y_center - hy, hz],
            [-hx, y_center + hy, hz],
            [-hx, y_center + hy, -hz],
        ]);
        normals.extend([[-1.0, 0.0, 0.0]; 4]);
        uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);

        // Top face (+Y) - only for top step or visible ledges
        positions.extend([
            [-hx, y_center + hy, hz],
            [hx, y_center + hy, hz],
            [hx, y_center + hy, -hz],
            [-hx, y_center + hy, -hz],
        ]);
        normals.extend([[0.0, 1.0, 0.0]; 4]);
        uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);

        // Bottom face (-Y) - only for bottom step
        if i == 0 {
            positions.extend([
                [-hx, y_center - hy, -hz],
                [hx, y_center - hy, -hz],
                [hx, y_center - hy, hz],
                [-hx, y_center - hy, hz],
            ]);
            normals.extend([[0.0, -1.0, 0.0]; 4]);
            uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        }

        // Indices for faces
        let num_faces = if i == 0 { 6 } else { 5 };
        for face in 0..num_faces {
            let fi = base_idx + face * 4;
            indices.extend([fi, fi + 1, fi + 2, fi, fi + 2, fi + 3]);
        }
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
