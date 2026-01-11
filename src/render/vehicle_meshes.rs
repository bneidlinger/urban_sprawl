//! Procedural vehicle mesh generation with angular, box-based geometry.
//!
//! Creates distinct vehicle shapes using hard-edged boxes for body, cabin,
//! hood, and trunk sections rather than smooth interpolated surfaces.

use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};

/// Vehicle shape category for mesh generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VehicleShape {
    Sedan,
    SUV,
    Truck,
    Van,
    Bus,
    SportsCar,
    Hatchback,
}

/// Configuration for generating a vehicle mesh.
#[derive(Clone, Debug)]
pub struct VehicleMeshConfig {
    pub length: f32,
    pub width: f32,
    pub height: f32,
    pub shape: VehicleShape,
}

impl Default for VehicleMeshConfig {
    fn default() -> Self {
        Self {
            length: 4.5,
            width: 1.8,
            height: 1.4,
            shape: VehicleShape::Sedan,
        }
    }
}

/// Generate a complete vehicle mesh (body + cabin combined).
pub fn generate_vehicle_mesh(config: &VehicleMeshConfig) -> Mesh {
    match config.shape {
        VehicleShape::Sedan => generate_sedan_mesh(config),
        VehicleShape::SUV => generate_suv_mesh(config),
        VehicleShape::Truck => generate_truck_mesh(config),
        VehicleShape::Van => generate_van_mesh(config),
        VehicleShape::Bus => generate_bus_mesh(config),
        VehicleShape::SportsCar => generate_sports_car_mesh(config),
        VehicleShape::Hatchback => generate_hatchback_mesh(config),
    }
}

/// Helper to add a box (6 faces) to the mesh
fn add_box(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    min: Vec3,
    max: Vec3,
) {
    let base = positions.len() as u32;

    // 8 corners of the box
    let corners = [
        Vec3::new(min.x, min.y, min.z), // 0: left-bottom-front
        Vec3::new(max.x, min.y, min.z), // 1: right-bottom-front
        Vec3::new(max.x, max.y, min.z), // 2: right-top-front
        Vec3::new(min.x, max.y, min.z), // 3: left-top-front
        Vec3::new(min.x, min.y, max.z), // 4: left-bottom-back
        Vec3::new(max.x, min.y, max.z), // 5: right-bottom-back
        Vec3::new(max.x, max.y, max.z), // 6: right-top-back
        Vec3::new(min.x, max.y, max.z), // 7: left-top-back
    ];

    // Each face needs its own vertices for correct normals
    // Front face (z = min.z)
    positions.extend([[min.x, min.y, min.z], [max.x, min.y, min.z], [max.x, max.y, min.z], [min.x, max.y, min.z]]);
    normals.extend([[0.0, 0.0, -1.0]; 4]);
    indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);

    // Back face (z = max.z)
    let b = base + 4;
    positions.extend([[max.x, min.y, max.z], [min.x, min.y, max.z], [min.x, max.y, max.z], [max.x, max.y, max.z]]);
    normals.extend([[0.0, 0.0, 1.0]; 4]);
    indices.extend([b, b + 1, b + 2, b, b + 2, b + 3]);

    // Left face (x = min.x)
    let b = base + 8;
    positions.extend([[min.x, min.y, max.z], [min.x, min.y, min.z], [min.x, max.y, min.z], [min.x, max.y, max.z]]);
    normals.extend([[-1.0, 0.0, 0.0]; 4]);
    indices.extend([b, b + 1, b + 2, b, b + 2, b + 3]);

    // Right face (x = max.x)
    let b = base + 12;
    positions.extend([[max.x, min.y, min.z], [max.x, min.y, max.z], [max.x, max.y, max.z], [max.x, max.y, min.z]]);
    normals.extend([[1.0, 0.0, 0.0]; 4]);
    indices.extend([b, b + 1, b + 2, b, b + 2, b + 3]);

    // Top face (y = max.y)
    let b = base + 16;
    positions.extend([[min.x, max.y, min.z], [max.x, max.y, min.z], [max.x, max.y, max.z], [min.x, max.y, max.z]]);
    normals.extend([[0.0, 1.0, 0.0]; 4]);
    indices.extend([b, b + 1, b + 2, b, b + 2, b + 3]);

    // Bottom face (y = min.y)
    let b = base + 20;
    positions.extend([[min.x, min.y, max.z], [max.x, min.y, max.z], [max.x, min.y, min.z], [min.x, min.y, min.z]]);
    normals.extend([[0.0, -1.0, 0.0]; 4]);
    indices.extend([b, b + 1, b + 2, b, b + 2, b + 3]);
}

/// Helper to add a sloped box (like a hood or windshield)
/// front_height and back_height allow for angled top surface
fn add_sloped_box(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    min_x: f32, max_x: f32,
    min_y: f32,
    front_top_y: f32, back_top_y: f32,
    front_z: f32, back_z: f32,
) {
    let base = positions.len() as u32;

    // Front face
    positions.extend([
        [min_x, min_y, front_z], [max_x, min_y, front_z],
        [max_x, front_top_y, front_z], [min_x, front_top_y, front_z]
    ]);
    normals.extend([[0.0, 0.0, -1.0]; 4]);
    indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);

    // Back face
    let b = base + 4;
    positions.extend([
        [max_x, min_y, back_z], [min_x, min_y, back_z],
        [min_x, back_top_y, back_z], [max_x, back_top_y, back_z]
    ]);
    normals.extend([[0.0, 0.0, 1.0]; 4]);
    indices.extend([b, b + 1, b + 2, b, b + 2, b + 3]);

    // Left face
    let b = base + 8;
    positions.extend([
        [min_x, min_y, back_z], [min_x, min_y, front_z],
        [min_x, front_top_y, front_z], [min_x, back_top_y, back_z]
    ]);
    normals.extend([[-1.0, 0.0, 0.0]; 4]);
    indices.extend([b, b + 1, b + 2, b, b + 2, b + 3]);

    // Right face
    let b = base + 12;
    positions.extend([
        [max_x, min_y, front_z], [max_x, min_y, back_z],
        [max_x, back_top_y, back_z], [max_x, front_top_y, front_z]
    ]);
    normals.extend([[1.0, 0.0, 0.0]; 4]);
    indices.extend([b, b + 1, b + 2, b, b + 2, b + 3]);

    // Top face (sloped)
    let b = base + 16;
    positions.extend([
        [min_x, front_top_y, front_z], [max_x, front_top_y, front_z],
        [max_x, back_top_y, back_z], [min_x, back_top_y, back_z]
    ]);
    // Calculate sloped normal
    let slope = (back_top_y - front_top_y) / (back_z - front_z);
    let normal = Vec3::new(0.0, 1.0, -slope).normalize();
    normals.extend([[normal.x, normal.y, normal.z]; 4]);
    indices.extend([b, b + 1, b + 2, b, b + 2, b + 3]);

    // Bottom face
    let b = base + 20;
    positions.extend([
        [min_x, min_y, back_z], [max_x, min_y, back_z],
        [max_x, min_y, front_z], [min_x, min_y, front_z]
    ]);
    normals.extend([[0.0, -1.0, 0.0]; 4]);
    indices.extend([b, b + 1, b + 2, b, b + 2, b + 3]);
}

/// Generate a sedan mesh using distinct box sections
fn generate_sedan_mesh(config: &VehicleMeshConfig) -> Mesh {
    let l = config.length;
    let w = config.width;
    let h = config.height;
    let hw = w / 2.0;
    let hl = l / 2.0;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    // Ground clearance
    let ground = h * 0.12;
    // Body height (below windows)
    let body_top = h * 0.45;
    // Roof height
    let roof_top = h * 0.95;

    // Longitudinal divisions
    let front_bumper = -hl;
    let hood_start = -hl + l * 0.08;
    let windshield_base = -hl + l * 0.28;
    let roof_start = -hl + l * 0.38;
    let roof_end = hl - l * 0.32;
    let rear_window_base = hl - l * 0.22;
    let trunk_end = hl - l * 0.08;
    let rear_bumper = hl;

    // Cabin width (narrower than body for window pillars effect)
    let cabin_w = hw * 0.88;

    // 1. Lower body box (full length, ground to body_top)
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw, ground, front_bumper),
        Vec3::new(hw, body_top, rear_bumper),
    );

    // 2. Hood (sloped up from front to windshield base)
    add_sloped_box(
        &mut positions, &mut normals, &mut indices,
        -hw * 0.95, hw * 0.95,
        body_top,
        body_top + h * 0.02, body_top + h * 0.12,
        hood_start, windshield_base,
    );

    // 3. Windshield (sloped from body to roof)
    add_sloped_box(
        &mut positions, &mut normals, &mut indices,
        -cabin_w, cabin_w,
        body_top,
        body_top + h * 0.12, roof_top,
        windshield_base, roof_start,
    );

    // 4. Roof box (flat top)
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-cabin_w, body_top, roof_start),
        Vec3::new(cabin_w, roof_top, roof_end),
    );

    // 5. Rear window (sloped down from roof to trunk)
    add_sloped_box(
        &mut positions, &mut normals, &mut indices,
        -cabin_w, cabin_w,
        body_top,
        roof_top, body_top + h * 0.15,
        roof_end, rear_window_base,
    );

    // 6. Trunk (sloped slightly down)
    add_sloped_box(
        &mut positions, &mut normals, &mut indices,
        -hw * 0.92, hw * 0.92,
        body_top,
        body_top + h * 0.15, body_top + h * 0.05,
        rear_window_base, trunk_end,
    );

    // 7. Front bumper and grille volume
    let bumper_height = body_top * 0.42;
    let bumper_depth = l * 0.035;
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw * 0.98, ground, front_bumper - bumper_depth),
        Vec3::new(hw * 0.98, bumper_height, front_bumper + l * 0.045),
    );

    // 8. Rear bumper
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw * 0.97, ground, trunk_end),
        Vec3::new(hw * 0.97, bumper_height * 0.92, rear_bumper + l * 0.02),
    );

    // 9. Side skirts
    let skirt_height = ground + h * 0.06;
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw, ground, hood_start),
        Vec3::new(-hw * 0.90, skirt_height, trunk_end),
    );
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(hw * 0.90, ground, hood_start),
        Vec3::new(hw, skirt_height, trunk_end),
    );

    build_mesh_with_uvs(positions, normals, indices)
}

/// Generate SUV mesh - taller, boxier than sedan
fn generate_suv_mesh(config: &VehicleMeshConfig) -> Mesh {
    let l = config.length;
    let w = config.width;
    let h = config.height;
    let hw = w / 2.0;
    let hl = l / 2.0;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    let ground = h * 0.15;
    let body_top = h * 0.48;
    let roof_top = h * 0.98;

    let front_bumper = -hl;
    let hood_start = -hl + l * 0.06;
    let windshield_base = -hl + l * 0.22;
    let roof_start = -hl + l * 0.30;
    let roof_end = hl - l * 0.12;
    let rear_bumper = hl;

    let cabin_w = hw * 0.92;

    // Lower body
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw, ground, front_bumper),
        Vec3::new(hw, body_top, rear_bumper),
    );

    // Hood (flatter than sedan)
    add_sloped_box(
        &mut positions, &mut normals, &mut indices,
        -hw * 0.95, hw * 0.95,
        body_top,
        body_top + h * 0.03, body_top + h * 0.08,
        hood_start, windshield_base,
    );

    // Windshield (steeper for SUV)
    add_sloped_box(
        &mut positions, &mut normals, &mut indices,
        -cabin_w, cabin_w,
        body_top,
        body_top + h * 0.08, roof_top,
        windshield_base, roof_start,
    );

    // Roof (extends further back)
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-cabin_w, body_top, roof_start),
        Vec3::new(cabin_w, roof_top, roof_end),
    );

    // Rear (nearly vertical for SUV)
    add_sloped_box(
        &mut positions, &mut normals, &mut indices,
        -cabin_w, cabin_w,
        body_top,
        roof_top, roof_top * 0.85,
        roof_end, rear_bumper - l * 0.03,
    );

    // Front bumper
    let bumper_height = body_top * 0.5;
    let bumper_depth = l * 0.04;
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw * 0.99, ground, front_bumper - bumper_depth),
        Vec3::new(hw * 0.99, bumper_height, front_bumper + l * 0.05),
    );

    // Rear bumper
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw * 0.98, ground, rear_bumper - l * 0.06),
        Vec3::new(hw * 0.98, bumper_height, rear_bumper + l * 0.02),
    );

    // Roof rails
    let rail_height = roof_top + h * 0.02;
    let rail_thickness = h * 0.03;
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-cabin_w, rail_height, roof_start + l * 0.04),
        Vec3::new(-cabin_w + hw * 0.12, rail_height + rail_thickness, roof_end - l * 0.02),
    );
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(cabin_w - hw * 0.12, rail_height, roof_start + l * 0.04),
        Vec3::new(cabin_w, rail_height + rail_thickness, roof_end - l * 0.02),
    );

    build_mesh_with_uvs(positions, normals, indices)
}

/// Generate truck mesh - cab + bed
fn generate_truck_mesh(config: &VehicleMeshConfig) -> Mesh {
    let l = config.length;
    let w = config.width;
    let h = config.height;
    let hw = w / 2.0;
    let hl = l / 2.0;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    let ground = h * 0.18;
    let body_top = h * 0.45;
    let cab_top = h * 0.95;
    let bed_top = h * 0.50;

    let front_bumper = -hl;
    let hood_start = -hl + l * 0.05;
    let windshield_base = -hl + l * 0.20;
    let cab_roof_start = -hl + l * 0.28;
    let cab_end = -hl + l * 0.45;
    let bed_start = -hl + l * 0.48;
    let rear_bumper = hl;

    let cabin_w = hw * 0.90;

    // Lower body (under cab)
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw, ground, front_bumper),
        Vec3::new(hw, body_top, cab_end),
    );

    // Hood
    add_sloped_box(
        &mut positions, &mut normals, &mut indices,
        -hw * 0.95, hw * 0.95,
        body_top,
        body_top + h * 0.02, body_top + h * 0.10,
        hood_start, windshield_base,
    );

    // Windshield
    add_sloped_box(
        &mut positions, &mut normals, &mut indices,
        -cabin_w, cabin_w,
        body_top,
        body_top + h * 0.10, cab_top,
        windshield_base, cab_roof_start,
    );

    // Cab roof
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-cabin_w, body_top, cab_roof_start),
        Vec3::new(cabin_w, cab_top, cab_end),
    );

    // Truck bed (open top box - sides only)
    // Bed floor
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw * 0.95, ground, bed_start),
        Vec3::new(hw * 0.95, body_top * 0.8, rear_bumper),
    );

    // Bed sides (left)
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw * 0.95, body_top * 0.8, bed_start),
        Vec3::new(-hw * 0.85, bed_top, rear_bumper - l * 0.02),
    );

    // Bed sides (right)
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(hw * 0.85, body_top * 0.8, bed_start),
        Vec3::new(hw * 0.95, bed_top, rear_bumper - l * 0.02),
    );

    // Bed tailgate
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw * 0.90, body_top * 0.8, rear_bumper - l * 0.04),
        Vec3::new(hw * 0.90, bed_top, rear_bumper - l * 0.02),
    );

    // Front bumper
    let bumper_height = body_top * 0.55;
    let bumper_depth = l * 0.04;
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw * 0.98, ground, front_bumper - bumper_depth),
        Vec3::new(hw * 0.98, bumper_height, front_bumper + l * 0.05),
    );

    // Side steps
    let step_height = ground + h * 0.06;
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw, ground, windshield_base),
        Vec3::new(-hw * 0.88, step_height, bed_start),
    );
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(hw * 0.88, ground, windshield_base),
        Vec3::new(hw, step_height, bed_start),
    );

    // Bed rails
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw * 0.95, bed_top, bed_start + l * 0.02),
        Vec3::new(-hw * 0.86, bed_top + h * 0.05, rear_bumper - l * 0.05),
    );
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(hw * 0.86, bed_top, bed_start + l * 0.02),
        Vec3::new(hw * 0.95, bed_top + h * 0.05, rear_bumper - l * 0.05),
    );

    build_mesh_with_uvs(positions, normals, indices)
}

/// Generate van mesh - tall, boxy
fn generate_van_mesh(config: &VehicleMeshConfig) -> Mesh {
    let l = config.length;
    let w = config.width;
    let h = config.height;
    let hw = w / 2.0;
    let hl = l / 2.0;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    let ground = h * 0.12;
    let body_top = h * 0.35;
    let roof_top = h * 0.98;

    let front_bumper = -hl;
    let hood_end = -hl + l * 0.12;
    let windshield_top = -hl + l * 0.20;
    let rear_bumper = hl;

    // Lower body
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw, ground, front_bumper),
        Vec3::new(hw, body_top, rear_bumper),
    );

    // Short hood
    add_sloped_box(
        &mut positions, &mut normals, &mut indices,
        -hw * 0.95, hw * 0.95,
        body_top,
        body_top + h * 0.02, body_top + h * 0.08,
        front_bumper + l * 0.02, hood_end,
    );

    // Windshield (steep)
    add_sloped_box(
        &mut positions, &mut normals, &mut indices,
        -hw * 0.92, hw * 0.92,
        body_top,
        body_top + h * 0.08, roof_top,
        hood_end, windshield_top,
    );

    // Main cargo box (nearly full height)
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw * 0.95, body_top, windshield_top),
        Vec3::new(hw * 0.95, roof_top, rear_bumper - l * 0.02),
    );

    // Front bumper
    let bumper_height = body_top * 0.55;
    let bumper_depth = l * 0.035;
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw * 0.98, ground, front_bumper - bumper_depth),
        Vec3::new(hw * 0.98, bumper_height, front_bumper + l * 0.05),
    );

    // Side trim
    let trim_height = ground + h * 0.07;
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw, ground, hood_end),
        Vec3::new(-hw * 0.90, trim_height, rear_bumper - l * 0.05),
    );
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(hw * 0.90, ground, hood_end),
        Vec3::new(hw, trim_height, rear_bumper - l * 0.05),
    );

    build_mesh_with_uvs(positions, normals, indices)
}

/// Generate bus mesh - long, rectangular
fn generate_bus_mesh(config: &VehicleMeshConfig) -> Mesh {
    let l = config.length;
    let w = config.width;
    let h = config.height;
    let hw = w / 2.0;
    let hl = l / 2.0;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    let ground = h * 0.10;
    let body_top = h * 0.30;
    let roof_top = h * 0.98;

    let front_bumper = -hl;
    let windshield_base = -hl + l * 0.04;
    let windshield_top = -hl + l * 0.10;
    let rear_bumper = hl;

    // Lower body (full length)
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw, ground, front_bumper),
        Vec3::new(hw, body_top, rear_bumper),
    );

    // Front face / windshield area (sloped)
    add_sloped_box(
        &mut positions, &mut normals, &mut indices,
        -hw * 0.95, hw * 0.95,
        body_top,
        body_top + h * 0.05, roof_top,
        windshield_base, windshield_top,
    );

    // Main passenger compartment (rectangular)
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw * 0.98, body_top, windshield_top),
        Vec3::new(hw * 0.98, roof_top, rear_bumper - l * 0.03),
    );

    // Rear cap
    add_sloped_box(
        &mut positions, &mut normals, &mut indices,
        -hw * 0.95, hw * 0.95,
        body_top,
        roof_top, roof_top * 0.92,
        rear_bumper - l * 0.05, rear_bumper,
    );

    // Front bumper
    let bumper_height = body_top * 0.7;
    let bumper_depth = l * 0.035;
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw * 0.98, ground, front_bumper - bumper_depth),
        Vec3::new(hw * 0.98, bumper_height, front_bumper + l * 0.06),
    );

    // Side skirt trim
    let trim_height = ground + h * 0.06;
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw, ground, windshield_top),
        Vec3::new(-hw * 0.92, trim_height, rear_bumper - l * 0.06),
    );
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(hw * 0.92, ground, windshield_top),
        Vec3::new(hw, trim_height, rear_bumper - l * 0.06),
    );

    build_mesh_with_uvs(positions, normals, indices)
}

/// Generate sports car mesh - low, sleek
fn generate_sports_car_mesh(config: &VehicleMeshConfig) -> Mesh {
    let l = config.length;
    let w = config.width;
    let h = config.height;
    let hw = w / 2.0;
    let hl = l / 2.0;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    let ground = h * 0.10;
    let body_top = h * 0.38;
    let roof_top = h * 0.85;

    let front_bumper = -hl;
    let hood_start = -hl + l * 0.05;
    let windshield_base = -hl + l * 0.35;
    let roof_start = -hl + l * 0.42;
    let roof_end = hl - l * 0.30;
    let rear_end = hl;

    let cabin_w = hw * 0.85;

    // Lower body (wedge shape - lower at front)
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw, ground, front_bumper),
        Vec3::new(hw, body_top, rear_end),
    );

    // Long hood (very low slope)
    add_sloped_box(
        &mut positions, &mut normals, &mut indices,
        -hw * 0.92, hw * 0.92,
        body_top,
        body_top - h * 0.05, body_top + h * 0.05,
        hood_start, windshield_base,
    );

    // Windshield (very raked)
    add_sloped_box(
        &mut positions, &mut normals, &mut indices,
        -cabin_w, cabin_w,
        body_top,
        body_top + h * 0.05, roof_top,
        windshield_base, roof_start,
    );

    // Low roof
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-cabin_w, body_top, roof_start),
        Vec3::new(cabin_w, roof_top, roof_end),
    );

    // Fastback rear
    add_sloped_box(
        &mut positions, &mut normals, &mut indices,
        -cabin_w * 0.95, cabin_w * 0.95,
        body_top,
        roof_top, body_top + h * 0.08,
        roof_end, rear_end - l * 0.05,
    );

    // Front splitter
    let splitter_height = ground + h * 0.05;
    let splitter_depth = l * 0.04;
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw * 0.98, ground, front_bumper - splitter_depth),
        Vec3::new(hw * 0.98, splitter_height, front_bumper + l * 0.05),
    );

    // Rear diffuser/spoiler
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw * 0.92, ground, rear_end - l * 0.06),
        Vec3::new(hw * 0.92, ground + h * 0.08, rear_end + l * 0.02),
    );

    build_mesh_with_uvs(positions, normals, indices)
}

/// Generate hatchback mesh - compact, tall rear
fn generate_hatchback_mesh(config: &VehicleMeshConfig) -> Mesh {
    let l = config.length;
    let w = config.width;
    let h = config.height;
    let hw = w / 2.0;
    let hl = l / 2.0;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    let ground = h * 0.12;
    let body_top = h * 0.42;
    let roof_top = h * 0.95;

    let front_bumper = -hl;
    let hood_start = -hl + l * 0.06;
    let windshield_base = -hl + l * 0.25;
    let roof_start = -hl + l * 0.35;
    let roof_end = hl - l * 0.18;
    let rear_bumper = hl;

    let cabin_w = hw * 0.88;

    // Lower body
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw, ground, front_bumper),
        Vec3::new(hw, body_top, rear_bumper),
    );

    // Hood
    add_sloped_box(
        &mut positions, &mut normals, &mut indices,
        -hw * 0.95, hw * 0.95,
        body_top,
        body_top + h * 0.02, body_top + h * 0.10,
        hood_start, windshield_base,
    );

    // Windshield
    add_sloped_box(
        &mut positions, &mut normals, &mut indices,
        -cabin_w, cabin_w,
        body_top,
        body_top + h * 0.10, roof_top,
        windshield_base, roof_start,
    );

    // Roof
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-cabin_w, body_top, roof_start),
        Vec3::new(cabin_w, roof_top, roof_end),
    );

    // Hatch (steep angle)
    add_sloped_box(
        &mut positions, &mut normals, &mut indices,
        -cabin_w * 0.95, cabin_w * 0.95,
        body_top,
        roof_top, body_top + h * 0.05,
        roof_end, rear_bumper - l * 0.03,
    );

    // Front bumper
    let bumper_height = body_top * 0.45;
    let bumper_depth = l * 0.035;
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw * 0.98, ground, front_bumper - bumper_depth),
        Vec3::new(hw * 0.98, bumper_height, front_bumper + l * 0.05),
    );

    // Rear bumper
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-hw * 0.97, ground, rear_bumper - l * 0.06),
        Vec3::new(hw * 0.97, bumper_height * 0.95, rear_bumper + l * 0.02),
    );

    // Roof spoiler
    add_box(
        &mut positions, &mut normals, &mut indices,
        Vec3::new(-cabin_w * 0.8, roof_top, roof_end - l * 0.02),
        Vec3::new(cabin_w * 0.8, roof_top + h * 0.05, rear_bumper - l * 0.01),
    );

    build_mesh_with_uvs(positions, normals, indices)
}

/// Generate a wheel mesh (solid cylinder - tire with filled faces)
pub fn generate_wheel_mesh(radius: f32, width: f32) -> Mesh {
    let segments = 16;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    // Outer tire surface (cylinder around the edge)
    for i in 0..segments {
        let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let next_angle = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;

        let x1 = angle.cos() * radius;
        let y1 = angle.sin() * radius;
        let x2 = next_angle.cos() * radius;
        let y2 = next_angle.sin() * radius;

        let base = positions.len() as u32;

        // Quad on tire tread surface
        positions.extend([
            [x1, y1, -width / 2.0],
            [x2, y2, -width / 2.0],
            [x2, y2, width / 2.0],
            [x1, y1, width / 2.0],
        ]);

        let n1 = Vec3::new(x1, y1, 0.0).normalize();
        let n2 = Vec3::new(x2, y2, 0.0).normalize();
        normals.extend([
            [n1.x, n1.y, n1.z],
            [n2.x, n2.y, n2.z],
            [n2.x, n2.y, n2.z],
            [n1.x, n1.y, n1.z],
        ]);

        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    // Solid side faces (left and right) - filled circles
    for side in [-1.0_f32, 1.0] {
        let z = side * width / 2.0;
        let normal = [0.0, 0.0, side];

        // Center point of the wheel face
        let center_idx = positions.len() as u32;
        positions.push([0.0, 0.0, z]);
        normals.push(normal);

        // Edge vertices around the wheel
        for i in 0..segments {
            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let x = angle.cos() * radius;
            let y = angle.sin() * radius;
            positions.push([x, y, z]);
            normals.push(normal);
        }

        // Triangle fan from center to edge (solid filled circle)
        for i in 0..segments {
            let i1 = center_idx + 1 + i as u32;
            let i2 = center_idx + 1 + ((i + 1) % segments) as u32;
            if side > 0.0 {
                indices.extend([center_idx, i1, i2]);
            } else {
                indices.extend([center_idx, i2, i1]);
            }
        }
    }

    build_mesh_with_uvs(positions, normals, indices)
}

/// Build final mesh with UVs
fn build_mesh_with_uvs(
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
) -> Mesh {
    // Generate simple UVs based on position
    let uvs: Vec<[f32; 2]> = positions.iter().map(|p| {
        [p[0] * 0.5 + 0.5, p[2] * 0.5 + 0.5]
    }).collect();

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
