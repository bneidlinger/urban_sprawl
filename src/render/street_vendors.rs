//! Street vendors: newspaper stands, food carts, and kiosks.
//!
//! Spawns vendor stalls along sidewalks near commercial areas.

use bevy::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::f32::consts::PI;

use crate::procgen::building_factory::BuildingArchetype;
use crate::render::building_spawner::{Building, BuildingsSpawned};

pub struct StreetVendorsPlugin;

impl Plugin for StreetVendorsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StreetVendorConfig>()
            .init_resource::<StreetVendorsSpawned>()
            .add_systems(Update, spawn_street_vendors.run_if(should_spawn_vendors));
    }
}

#[derive(Resource, Default)]
pub struct StreetVendorsSpawned(pub bool);

fn should_spawn_vendors(
    buildings_spawned: Res<BuildingsSpawned>,
    vendors_spawned: Res<StreetVendorsSpawned>,
) -> bool {
    buildings_spawned.0 && !vendors_spawned.0
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VendorType {
    NewspaperStand,
    FoodCart,
    CoffeeKiosk,
    FlowerStand,
}

/// Marker component for street vendors.
#[derive(Component)]
pub struct StreetVendor {
    pub vendor_type: VendorType,
}

#[derive(Resource)]
pub struct StreetVendorConfig {
    pub seed: u64,
    pub max_vendors: usize,
    pub min_spacing: f32,
    pub sidewalk_offset: f32,
}

impl Default for StreetVendorConfig {
    fn default() -> Self {
        Self {
            seed: 11111,
            max_vendors: 30,
            min_spacing: 25.0,
            sidewalk_offset: 5.0,
        }
    }
}

// Vendor type colors
const NEWSPAPER_COLOR: Color = Color::srgb(0.3, 0.5, 0.7);
const FOOD_CART_COLORS: &[(f32, f32, f32)] = &[
    (0.8, 0.3, 0.2),  // Red hot dog cart
    (0.2, 0.6, 0.3),  // Green produce cart
    (0.9, 0.7, 0.2),  // Yellow taco cart
    (0.6, 0.4, 0.2),  // Brown pretzel cart
];
const COFFEE_COLOR: Color = Color::srgb(0.4, 0.25, 0.15);
const FLOWER_COLORS: &[(f32, f32, f32)] = &[
    (0.9, 0.3, 0.4),  // Pink
    (0.9, 0.9, 0.3),  // Yellow
    (0.9, 0.5, 0.2),  // Orange
    (0.7, 0.3, 0.7),  // Purple
];

fn spawn_street_vendors(
    mut commands: Commands,
    config: Res<StreetVendorConfig>,
    buildings: Query<(&Building, &Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: ResMut<StreetVendorsSpawned>,
) {
    spawned.0 = true;

    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut vendor_count = 0;
    let mut placed_positions: Vec<Vec3> = Vec::new();

    // Collect commercial building positions
    let mut eligible_positions: Vec<Vec3> = buildings
        .iter()
        .filter(|(b, _)| b.building_type == BuildingArchetype::Commercial)
        .map(|(_, t)| t.translation)
        .collect();

    // Shuffle
    for i in (1..eligible_positions.len()).rev() {
        let j = rng.gen_range(0..=i);
        eligible_positions.swap(i, j);
    }

    // Common materials
    let metal_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.52),
        metallic: 0.7,
        perceptual_roughness: 0.4,
        ..default()
    });

    let canvas_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.85, 0.75),
        perceptual_roughness: 0.9,
        ..default()
    });

    let wood_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.35, 0.2),
        perceptual_roughness: 0.85,
        ..default()
    });

    for pos in eligible_positions {
        if vendor_count >= config.max_vendors {
            break;
        }

        // Offset to sidewalk
        let offset = Vec3::new(
            rng.gen_range(-8.0..8.0),
            0.0,
            rng.gen_range(-8.0..8.0),
        );
        let vendor_pos = Vec3::new(
            pos.x + offset.x,
            0.0,
            pos.z + offset.z,
        );

        // Check spacing
        let too_close = placed_positions
            .iter()
            .any(|p| p.distance(vendor_pos) < config.min_spacing);
        if too_close {
            continue;
        }

        let rotation = Quat::from_rotation_y(rng.gen::<f32>() * PI * 2.0);
        let vendor_type = match rng.gen_range(0..4) {
            0 => VendorType::NewspaperStand,
            1 => VendorType::FoodCart,
            2 => VendorType::CoffeeKiosk,
            _ => VendorType::FlowerStand,
        };

        commands
            .spawn((
                Transform::from_translation(vendor_pos).with_rotation(rotation),
                GlobalTransform::default(),
                Visibility::Visible,
                InheritedVisibility::default(),
                ViewVisibility::default(),
                StreetVendor { vendor_type },
            ))
            .with_children(|parent| {
                match vendor_type {
                    VendorType::NewspaperStand => {
                        spawn_newspaper_stand(parent, &mut meshes, &mut materials, &metal_material);
                    }
                    VendorType::FoodCart => {
                        let color_idx = rng.gen_range(0..FOOD_CART_COLORS.len());
                        let (r, g, b) = FOOD_CART_COLORS[color_idx];
                        let cart_material = materials.add(StandardMaterial {
                            base_color: Color::srgb(r, g, b),
                            perceptual_roughness: 0.6,
                            ..default()
                        });
                        spawn_food_cart(parent, &mut meshes, &cart_material, &metal_material, &canvas_material);
                    }
                    VendorType::CoffeeKiosk => {
                        let coffee_material = materials.add(StandardMaterial {
                            base_color: COFFEE_COLOR,
                            perceptual_roughness: 0.7,
                            ..default()
                        });
                        spawn_coffee_kiosk(parent, &mut meshes, &coffee_material, &metal_material);
                    }
                    VendorType::FlowerStand => {
                        spawn_flower_stand(parent, &mut meshes, &mut materials, &wood_material);
                    }
                }
            });

        placed_positions.push(vendor_pos);
        vendor_count += 1;
    }

    info!("Spawned {} street vendors", vendor_count);
}

fn spawn_newspaper_stand(
    parent: &mut ChildBuilder,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    metal_material: &Handle<StandardMaterial>,
) {
    // Main cabinet
    let cabinet_mesh = meshes.add(Cuboid::new(1.2, 1.5, 0.6));
    let cabinet_material = materials.add(StandardMaterial {
        base_color: NEWSPAPER_COLOR,
        perceptual_roughness: 0.5,
        ..default()
    });
    parent.spawn((
        Mesh3d(cabinet_mesh),
        MeshMaterial3d(cabinet_material),
        Transform::from_xyz(0.0, 0.75, 0.0),
    ));

    // Glass front
    let glass_mesh = meshes.add(Cuboid::new(1.1, 1.0, 0.05));
    let glass_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.8, 0.85, 0.9, 0.5),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.1,
        ..default()
    });
    parent.spawn((
        Mesh3d(glass_mesh),
        MeshMaterial3d(glass_material),
        Transform::from_xyz(0.0, 0.9, 0.33),
    ));

    // Newspaper stack on top
    let paper_mesh = meshes.add(Cuboid::new(0.4, 0.1, 0.3));
    let paper_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.93, 0.88),
        perceptual_roughness: 0.95,
        ..default()
    });
    for x in [-0.3, 0.0, 0.3] {
        parent.spawn((
            Mesh3d(paper_mesh.clone()),
            MeshMaterial3d(paper_material.clone()),
            Transform::from_xyz(x, 1.55, 0.0),
        ));
    }

    // Legs
    let leg_mesh = meshes.add(Cylinder::new(0.03, 0.3));
    for (x, z) in [(-0.5, -0.25), (0.5, -0.25), (-0.5, 0.25), (0.5, 0.25)] {
        parent.spawn((
            Mesh3d(leg_mesh.clone()),
            MeshMaterial3d(metal_material.clone()),
            Transform::from_xyz(x, 0.15, z),
        ));
    }
}

fn spawn_food_cart(
    parent: &mut ChildBuilder,
    meshes: &mut ResMut<Assets<Mesh>>,
    cart_material: &Handle<StandardMaterial>,
    metal_material: &Handle<StandardMaterial>,
    canvas_material: &Handle<StandardMaterial>,
) {
    // Cart body
    let body_mesh = meshes.add(Cuboid::new(1.8, 1.0, 0.9));
    parent.spawn((
        Mesh3d(body_mesh),
        MeshMaterial3d(cart_material.clone()),
        Transform::from_xyz(0.0, 0.9, 0.0),
    ));

    // Counter top
    let counter_mesh = meshes.add(Cuboid::new(1.9, 0.05, 1.0));
    parent.spawn((
        Mesh3d(counter_mesh),
        MeshMaterial3d(metal_material.clone()),
        Transform::from_xyz(0.0, 1.42, 0.0),
    ));

    // Wheels
    let wheel_mesh = meshes.add(Cylinder::new(0.2, 0.1));
    for x in [-0.7, 0.7] {
        parent.spawn((
            Mesh3d(wheel_mesh.clone()),
            MeshMaterial3d(metal_material.clone()),
            Transform::from_xyz(x, 0.2, 0.0)
                .with_rotation(Quat::from_rotation_z(PI / 2.0)),
        ));
    }

    // Umbrella/canopy
    let umbrella_mesh = meshes.add(Cuboid::new(2.2, 0.05, 1.4));
    parent.spawn((
        Mesh3d(umbrella_mesh),
        MeshMaterial3d(canvas_material.clone()),
        Transform::from_xyz(0.0, 2.5, 0.0),
    ));

    // Umbrella pole
    let pole_mesh = meshes.add(Cylinder::new(0.03, 1.0));
    parent.spawn((
        Mesh3d(pole_mesh),
        MeshMaterial3d(metal_material.clone()),
        Transform::from_xyz(0.0, 2.0, 0.0),
    ));

    // Handle
    let handle_mesh = meshes.add(Cylinder::new(0.02, 0.6));
    parent.spawn((
        Mesh3d(handle_mesh),
        MeshMaterial3d(metal_material.clone()),
        Transform::from_xyz(-1.1, 1.0, 0.0)
            .with_rotation(Quat::from_rotation_z(PI / 4.0)),
    ));
}

fn spawn_coffee_kiosk(
    parent: &mut ChildBuilder,
    meshes: &mut ResMut<Assets<Mesh>>,
    coffee_material: &Handle<StandardMaterial>,
    metal_material: &Handle<StandardMaterial>,
) {
    // Main kiosk body
    let body_mesh = meshes.add(Cuboid::new(1.5, 2.2, 1.5));
    parent.spawn((
        Mesh3d(body_mesh),
        MeshMaterial3d(coffee_material.clone()),
        Transform::from_xyz(0.0, 1.1, 0.0),
    ));

    // Service window
    let window_mesh = meshes.add(Cuboid::new(1.0, 0.8, 0.05));
    let window_material = metal_material.clone();
    parent.spawn((
        Mesh3d(window_mesh),
        MeshMaterial3d(window_material),
        Transform::from_xyz(0.0, 1.3, 0.78),
    ));

    // Counter shelf
    let shelf_mesh = meshes.add(Cuboid::new(1.2, 0.05, 0.4));
    parent.spawn((
        Mesh3d(shelf_mesh),
        MeshMaterial3d(metal_material.clone()),
        Transform::from_xyz(0.0, 0.9, 0.95),
    ));

    // Coffee sign on top
    let sign_mesh = meshes.add(Cuboid::new(0.8, 0.4, 0.1));
    let sign_material = metal_material.clone();
    parent.spawn((
        Mesh3d(sign_mesh),
        MeshMaterial3d(sign_material),
        Transform::from_xyz(0.0, 2.4, 0.7),
    ));

    // Chimney/vent
    let chimney_mesh = meshes.add(Cylinder::new(0.1, 0.4));
    parent.spawn((
        Mesh3d(chimney_mesh),
        MeshMaterial3d(metal_material.clone()),
        Transform::from_xyz(0.4, 2.4, 0.0),
    ));
}

fn spawn_flower_stand(
    parent: &mut ChildBuilder,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    wood_material: &Handle<StandardMaterial>,
) {
    // Display table
    let table_mesh = meshes.add(Cuboid::new(2.0, 0.8, 0.8));
    parent.spawn((
        Mesh3d(table_mesh),
        MeshMaterial3d(wood_material.clone()),
        Transform::from_xyz(0.0, 0.4, 0.0),
    ));

    // Flower buckets
    let bucket_mesh = meshes.add(Cylinder::new(0.12, 0.25));
    let bucket_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.4, 0.42),
        metallic: 0.6,
        ..default()
    });

    let mut rng = StdRng::seed_from_u64(12345);
    for x in [-0.6, -0.2, 0.2, 0.6] {
        for z in [-0.2, 0.2] {
            parent.spawn((
                Mesh3d(bucket_mesh.clone()),
                MeshMaterial3d(bucket_material.clone()),
                Transform::from_xyz(x, 0.92, z),
            ));

            // Flowers in bucket
            let flower_mesh = meshes.add(Sphere::new(0.08));
            let (r, g, b) = FLOWER_COLORS[rng.gen_range(0..FLOWER_COLORS.len())];
            let flower_material = materials.add(StandardMaterial {
                base_color: Color::srgb(r, g, b),
                perceptual_roughness: 0.9,
                ..default()
            });
            for _ in 0..3 {
                let fx = x + rng.gen_range(-0.08..0.08);
                let fz = z + rng.gen_range(-0.08..0.08);
                parent.spawn((
                    Mesh3d(flower_mesh.clone()),
                    MeshMaterial3d(flower_material.clone()),
                    Transform::from_xyz(fx, 1.1, fz),
                ));
            }
        }
    }

    // Awning
    let awning_mesh = meshes.add(Cuboid::new(2.2, 0.05, 1.0));
    let awning_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.5, 0.3),
        perceptual_roughness: 0.8,
        ..default()
    });
    parent.spawn((
        Mesh3d(awning_mesh),
        MeshMaterial3d(awning_material),
        Transform::from_xyz(0.0, 1.8, 0.0),
    ));

    // Awning poles
    let pole_mesh = meshes.add(Cylinder::new(0.03, 1.0));
    for x in [-0.9, 0.9] {
        parent.spawn((
            Mesh3d(pole_mesh.clone()),
            MeshMaterial3d(wood_material.clone()),
            Transform::from_xyz(x, 1.3, 0.4),
        ));
    }
}
