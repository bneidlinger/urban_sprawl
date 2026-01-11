//! Construction sites for buildings under development.
//!
//! When zones develop, a construction site appears first with cranes,
//! scaffolding, and barriers. As construction progresses, the building
//! gradually takes shape until completion.

use bevy::prelude::*;
use rand::rngs::StdRng;
use std::f32::consts::PI;

use crate::game_state::GameState;
use crate::procgen::building_factory::{BuildingArchetype, FacadeStyle};
use crate::procgen::lot_engine::ZoneType;
use crate::render::building_spawner::Building;
use crate::simulation::zones::GrownBuilding;

pub struct ConstructionSitesPlugin;

impl Plugin for ConstructionSitesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConstructionConfig>()
            .add_systems(
                Update,
                (
                    update_construction_progress,
                    update_construction_visuals,
                    complete_construction,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Configuration for construction sites.
#[derive(Resource)]
pub struct ConstructionConfig {
    /// Base construction time in seconds for a 10m building.
    pub base_construction_time: f32,
    /// Additional time per meter of height.
    pub time_per_meter: f32,
    /// Random seed for construction variation (reserved for future use).
    pub _seed: u64,
}

impl Default for ConstructionConfig {
    fn default() -> Self {
        Self {
            base_construction_time: 10.0,  // 10 seconds base
            time_per_meter: 0.5,           // +0.5 seconds per meter
            _seed: 77777,
        }
    }
}

/// Construction phase enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructionPhase {
    /// 0-20%: Digging foundation
    Foundation,
    /// 20-60%: Building structure/frame
    Structure,
    /// 60-90%: Enclosing with walls
    Enclosure,
    /// 90-100%: Final finishing
    Finishing,
}

impl ConstructionPhase {
    pub fn from_progress(progress: f32) -> Self {
        match progress {
            p if p < 0.2 => Self::Foundation,
            p if p < 0.6 => Self::Structure,
            p if p < 0.9 => Self::Enclosure,
            _ => Self::Finishing,
        }
    }
}

/// Marker component for a construction site.
#[derive(Component)]
pub struct ConstructionSite {
    /// Progress from 0.0 (just started) to 1.0 (complete).
    pub progress: f32,
    /// Current construction phase.
    pub phase: ConstructionPhase,
    /// Target building height when complete.
    pub target_height: f32,
    /// Target building footprint size.
    pub footprint_size: f32,
    /// Zone type being built.
    pub zone_type: ZoneType,
    /// Reference to the zone cell entity.
    pub zone_cell: Entity,
    /// Total construction duration in seconds.
    pub duration: f32,
    /// Time elapsed since construction started.
    pub elapsed: f32,
}

impl ConstructionSite {
    pub fn new(
        target_height: f32,
        footprint_size: f32,
        zone_type: ZoneType,
        zone_cell: Entity,
        config: &ConstructionConfig,
    ) -> Self {
        let duration = config.base_construction_time + target_height * config.time_per_meter;

        Self {
            progress: 0.0,
            phase: ConstructionPhase::Foundation,
            target_height,
            footprint_size,
            zone_type,
            zone_cell,
            duration,
            elapsed: 0.0,
        }
    }
}

/// Marker for the crane entity (child of construction site).
#[derive(Component)]
pub struct Crane;

/// Marker for scaffolding entities.
#[derive(Component)]
pub struct Scaffolding;

/// Marker for construction barriers.
#[derive(Component)]
pub struct ConstructionBarrier;

/// Marker for the partial building mesh.
#[derive(Component)]
pub struct PartialBuilding;

/// Marker for the foundation pit.
#[derive(Component)]
pub struct FoundationPit;

/// Bundle for spawning a complete construction site.
#[derive(Bundle)]
pub struct ConstructionSiteBundle {
    pub site: ConstructionSite,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
}

/// System to update construction progress over time.
fn update_construction_progress(
    time: Res<Time>,
    mut sites: Query<&mut ConstructionSite>,
) {
    for mut site in &mut sites {
        if site.progress >= 1.0 {
            continue;
        }

        site.elapsed += time.delta_secs();
        site.progress = (site.elapsed / site.duration).min(1.0);

        let new_phase = ConstructionPhase::from_progress(site.progress);
        if new_phase != site.phase {
            site.phase = new_phase;
        }
    }
}

/// System to update construction site visuals based on progress.
fn update_construction_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    sites: Query<(Entity, &ConstructionSite, &Children), Changed<ConstructionSite>>,
    partial_buildings: Query<Entity, With<PartialBuilding>>,
) {
    for (_site_entity, site, children) in &sites {
        // Find and update the partial building mesh
        for &child in children.iter() {
            if let Ok(partial_entity) = partial_buildings.get(child) {
                // Update partial building height based on progress
                let current_height = match site.phase {
                    ConstructionPhase::Foundation => 0.0,
                    ConstructionPhase::Structure => {
                        let phase_progress = (site.progress - 0.2) / 0.4;
                        site.target_height * 0.3 * phase_progress
                    }
                    ConstructionPhase::Enclosure => {
                        let phase_progress = (site.progress - 0.6) / 0.3;
                        site.target_height * (0.3 + 0.5 * phase_progress)
                    }
                    ConstructionPhase::Finishing => {
                        let phase_progress = (site.progress - 0.9) / 0.1;
                        site.target_height * (0.8 + 0.2 * phase_progress)
                    }
                };

                if current_height > 0.1 {
                    // Update the mesh with new height
                    let new_mesh = meshes.add(Cuboid::new(
                        site.footprint_size,
                        current_height,
                        site.footprint_size,
                    ));

                    commands.entity(partial_entity).insert((
                        Mesh3d(new_mesh),
                        Transform::from_xyz(0.0, current_height / 2.0, 0.0),
                    ));
                }
            }
        }
    }
}

/// System to complete construction and spawn the final building.
fn complete_construction(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    sites: Query<(Entity, &ConstructionSite, &Transform)>,
    mut zone_cells: Query<&mut crate::tools::zone_paint::ZoneCell>,
) {
    for (site_entity, site, transform) in &sites {
        if site.progress < 1.0 {
            continue;
        }

        // Despawn the construction site and all children
        commands.entity(site_entity).despawn_recursive();

        // Spawn the completed building
        let color = building_color(site.zone_type);
        let mesh = meshes.add(Cuboid::new(
            site.footprint_size,
            site.target_height,
            site.footprint_size,
        ));
        let material = materials.add(StandardMaterial {
            base_color: color,
            ..default()
        });

        let building_pos = Vec3::new(
            transform.translation.x,
            site.target_height / 2.0,
            transform.translation.z,
        );

        let building_entity = commands
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_translation(building_pos),
                GrownBuilding {
                    zone_cell: site.zone_cell,
                    growth_time: time.elapsed_secs(),
                },
                Building {
                    lot_index: 0,
                    building_type: zone_to_archetype(site.zone_type),
                    facade_style: FacadeStyle::Concrete,
                },
            ))
            .id();

        // Update the zone cell
        if let Ok(mut cell) = zone_cells.get_mut(site.zone_cell) {
            cell.building = Some(building_entity);
            // Keep development_level at 1 (set when construction started)
        }

        info!(
            "Construction complete: {:?} building at ({:.1}, {:.1})",
            site.zone_type, transform.translation.x, transform.translation.z
        );
    }
}

/// Spawn a construction site with all visual elements.
pub fn spawn_construction_site(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    target_height: f32,
    footprint_size: f32,
    zone_type: ZoneType,
    zone_cell: Entity,
    config: &ConstructionConfig,
    _rng: &mut StdRng,
) -> Entity {
    let site = ConstructionSite::new(target_height, footprint_size, zone_type, zone_cell, config);

    // Materials
    let crane_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.7, 0.1), // Yellow crane
        ..default()
    });
    let barrier_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.5, 0.0), // Orange barriers
        emissive: LinearRgba::new(0.3, 0.15, 0.0, 1.0),
        ..default()
    });
    let scaffolding_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.6, 0.65), // Gray metal
        metallic: 0.8,
        ..default()
    });
    let foundation_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.3, 0.25), // Dirt brown
        ..default()
    });
    let partial_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.7, 0.7, 0.7), // Gray concrete
        perceptual_roughness: 0.9,
        ..default()
    });

    // Crane meshes
    let crane_base = meshes.add(Cuboid::new(1.5, 0.5, 1.5));
    let crane_mast = meshes.add(Cuboid::new(0.4, target_height * 1.2, 0.4));
    let crane_jib = meshes.add(Cuboid::new(target_height * 0.8, 0.3, 0.3));
    let crane_counter = meshes.add(Cuboid::new(3.0, 0.4, 0.4));

    // Barrier mesh
    let barrier_mesh = meshes.add(Cuboid::new(0.1, 1.0, 2.0));

    // Foundation pit mesh
    let foundation_mesh = meshes.add(Cuboid::new(footprint_size * 1.1, 0.5, footprint_size * 1.1));

    // Scaffolding poles
    let scaffold_pole = meshes.add(Cylinder::new(0.05, 4.0));
    let scaffold_plank = meshes.add(Cuboid::new(0.1, 0.05, 2.0));

    // Partial building (starts invisible)
    let partial_mesh = meshes.add(Cuboid::new(footprint_size, 0.1, footprint_size));

    // Spawn the construction site entity hierarchy
    let site_entity = commands
        .spawn((
            ConstructionSiteBundle {
                site,
                transform: Transform::from_translation(position),
                global_transform: GlobalTransform::default(),
                visibility: Visibility::Visible,
                inherited_visibility: InheritedVisibility::default(),
                view_visibility: ViewVisibility::default(),
            },
        ))
        .with_children(|parent| {
            // Foundation pit (slightly below ground)
            parent.spawn((
                Mesh3d(foundation_mesh),
                MeshMaterial3d(foundation_material.clone()),
                Transform::from_xyz(0.0, -0.25, 0.0),
                FoundationPit,
            ));

            // Crane - offset to corner
            let crane_offset = footprint_size * 0.4;
            parent.spawn((
                Mesh3d(crane_base.clone()),
                MeshMaterial3d(crane_material.clone()),
                Transform::from_xyz(crane_offset, 0.25, crane_offset),
            ));

            // Crane mast
            let mast_height = target_height * 1.2;
            parent.spawn((
                Mesh3d(crane_mast),
                MeshMaterial3d(crane_material.clone()),
                Transform::from_xyz(crane_offset, mast_height / 2.0, crane_offset),
                Crane,
            ));

            // Crane jib (horizontal arm)
            parent.spawn((
                Mesh3d(crane_jib),
                MeshMaterial3d(crane_material.clone()),
                Transform::from_xyz(
                    crane_offset - target_height * 0.3,
                    mast_height,
                    crane_offset,
                ),
            ));

            // Counter-weight
            parent.spawn((
                Mesh3d(crane_counter),
                MeshMaterial3d(crane_material.clone()),
                Transform::from_xyz(
                    crane_offset + 2.5,
                    mast_height,
                    crane_offset,
                ),
            ));

            // Construction barriers around perimeter
            let barrier_dist = footprint_size * 0.6;
            for i in 0..4 {
                let angle = i as f32 * PI / 2.0;
                let (bx, bz) = (angle.cos() * barrier_dist, angle.sin() * barrier_dist);
                let rotation = Quat::from_rotation_y(angle);

                parent.spawn((
                    Mesh3d(barrier_mesh.clone()),
                    MeshMaterial3d(barrier_material.clone()),
                    Transform::from_xyz(bx, 0.5, bz).with_rotation(rotation),
                    ConstructionBarrier,
                ));
            }

            // Scaffolding on two sides
            for side in 0..2 {
                let side_offset = if side == 0 {
                    Vec3::new(footprint_size * 0.5 + 0.3, 0.0, 0.0)
                } else {
                    Vec3::new(0.0, 0.0, footprint_size * 0.5 + 0.3)
                };

                // Vertical poles
                for j in 0..3 {
                    let pole_z = (j as f32 - 1.0) * footprint_size * 0.4;
                    let pole_pos = if side == 0 {
                        Vec3::new(side_offset.x, 2.0, pole_z)
                    } else {
                        Vec3::new(pole_z, 2.0, side_offset.z)
                    };

                    parent.spawn((
                        Mesh3d(scaffold_pole.clone()),
                        MeshMaterial3d(scaffolding_material.clone()),
                        Transform::from_translation(pole_pos),
                        Scaffolding,
                    ));
                }

                // Horizontal planks at different heights
                for level in 1..3 {
                    let plank_y = level as f32 * 1.5;
                    let plank_pos = if side == 0 {
                        Vec3::new(side_offset.x, plank_y, 0.0)
                    } else {
                        Vec3::new(0.0, plank_y, side_offset.z)
                    };
                    let plank_rot = if side == 0 {
                        Quat::from_rotation_y(PI / 2.0)
                    } else {
                        Quat::IDENTITY
                    };

                    parent.spawn((
                        Mesh3d(scaffold_plank.clone()),
                        MeshMaterial3d(scaffolding_material.clone()),
                        Transform::from_translation(plank_pos).with_rotation(plank_rot),
                        Scaffolding,
                    ));
                }
            }

            // Partial building (grows as construction progresses)
            parent.spawn((
                Mesh3d(partial_mesh),
                MeshMaterial3d(partial_material),
                Transform::from_xyz(0.0, 0.05, 0.0),
                PartialBuilding,
            ));
        })
        .id();

    site_entity
}

fn building_color(zone_type: ZoneType) -> Color {
    match zone_type {
        ZoneType::Residential => Color::srgb(0.85, 0.9, 0.85),
        ZoneType::Commercial => Color::srgb(0.7, 0.75, 0.9),
        ZoneType::Industrial => Color::srgb(0.8, 0.75, 0.6),
        ZoneType::Civic => Color::srgb(0.9, 0.85, 0.95),
        ZoneType::Green => Color::srgb(0.3, 0.7, 0.3),
    }
}

fn zone_to_archetype(zone_type: ZoneType) -> BuildingArchetype {
    match zone_type {
        ZoneType::Residential => BuildingArchetype::Residential,
        ZoneType::Commercial => BuildingArchetype::Commercial,
        ZoneType::Industrial => BuildingArchetype::Industrial,
        _ => BuildingArchetype::Commercial,
    }
}
