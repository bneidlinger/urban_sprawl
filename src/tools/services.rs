//! Service placement tool - place police, fire, hospital, and school buildings.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::ActiveTool;
use crate::game_state::GameState;

pub struct ServicesPlugin;

impl Plugin for ServicesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ServicesConfig>()
            .add_systems(
                Update,
                (handle_service_placement, update_service_preview)
                    .chain()
                    .run_if(in_state(GameState::Playing))
                    .run_if(is_service_tool_active),
            );
    }
}

/// Run condition: check if a service tool is active.
fn is_service_tool_active(tool: Res<State<ActiveTool>>) -> bool {
    matches!(tool.get(), ActiveTool::PlaceService(_))
}

/// Types of city services.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ServiceType {
    Police,
    Fire,
    Hospital,
    School,
    Park,
}

impl ServiceType {
    /// Get the display name for this service.
    pub fn name(&self) -> &'static str {
        match self {
            ServiceType::Police => "Police Station",
            ServiceType::Fire => "Fire Station",
            ServiceType::Hospital => "Hospital",
            ServiceType::School => "School",
            ServiceType::Park => "Park",
        }
    }

    /// Get the effect radius for this service.
    pub fn radius(&self) -> f32 {
        match self {
            ServiceType::Police => 100.0,
            ServiceType::Fire => 80.0,
            ServiceType::Hospital => 120.0,
            ServiceType::School => 60.0,
            ServiceType::Park => 40.0,
        }
    }

    /// Get the cost to build this service.
    pub fn cost(&self) -> i64 {
        match self {
            ServiceType::Police => 5000,
            ServiceType::Fire => 4000,
            ServiceType::Hospital => 10000,
            ServiceType::School => 6000,
            ServiceType::Park => 1000,
        }
    }

    /// Get the display color for this service.
    pub fn color(&self) -> Color {
        match self {
            ServiceType::Police => Color::srgb(0.3, 0.4, 0.8),  // Blue
            ServiceType::Fire => Color::srgb(0.9, 0.3, 0.2),    // Red
            ServiceType::Hospital => Color::srgb(0.9, 0.9, 0.9), // White
            ServiceType::School => Color::srgb(0.9, 0.7, 0.2),  // Yellow
            ServiceType::Park => Color::srgb(0.2, 0.7, 0.3),    // Green
        }
    }
}

/// Configuration for service placement.
#[derive(Resource)]
pub struct ServicesConfig {
    /// Size of service buildings.
    pub building_size: f32,
    /// Height of service buildings.
    pub building_height: f32,
}

impl Default for ServicesConfig {
    fn default() -> Self {
        Self {
            building_size: 15.0,
            building_height: 12.0,
        }
    }
}

/// Component for service buildings.
#[derive(Component)]
pub struct ServiceBuilding {
    pub service_type: ServiceType,
    pub radius: f32,
}

/// Marker for service preview.
#[derive(Component)]
struct ServicePreview;

/// Marker for radius preview.
#[derive(Component)]
struct RadiusPreview;

fn handle_service_placement(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    tool: Res<State<ActiveTool>>,
    config: Res<ServicesConfig>,
    mut budget: ResMut<crate::simulation::economy::CityBudget>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let service_type = match tool.get() {
        ActiveTool::PlaceService(st) => *st,
        _ => return,
    };

    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.get_single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_q.get_single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    // Check if we can afford it
    let cost = service_type.cost();
    if budget.funds < cost {
        info!(
            "Cannot afford {} (${} needed, ${} available)",
            service_type.name(),
            cost,
            budget.funds
        );
        return;
    }

    // Deduct cost
    budget.funds -= cost;

    // Create service building mesh
    let height = match service_type {
        ServiceType::Park => 1.0, // Parks are flat
        _ => config.building_height,
    };

    let mesh = if matches!(service_type, ServiceType::Park) {
        meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(config.building_size)))
    } else {
        meshes.add(Cuboid::new(config.building_size, height, config.building_size))
    };

    let material = materials.add(StandardMaterial {
        base_color: service_type.color(),
        ..default()
    });

    let y_pos = if matches!(service_type, ServiceType::Park) {
        0.1
    } else {
        height / 2.0
    };

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(Vec3::new(world_pos.x, y_pos, world_pos.y)),
        ServiceBuilding {
            service_type,
            radius: service_type.radius(),
        },
    ));

    info!(
        "Placed {} at ({:.1}, {:.1}) for ${}",
        service_type.name(),
        world_pos.x,
        world_pos.y,
        cost
    );
}

fn update_service_preview(
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    tool: Res<State<ActiveTool>>,
    config: Res<ServicesConfig>,
    mut preview_q: Query<(Entity, &mut Transform, &mut Sprite), (With<ServicePreview>, Without<RadiusPreview>)>,
    mut radius_q: Query<(Entity, &mut Transform, &mut Sprite), With<RadiusPreview>>,
) {
    let service_type = match tool.get() {
        ActiveTool::PlaceService(st) => *st,
        _ => {
            // Clean up previews
            for (entity, _, _) in &preview_q {
                commands.entity(entity).despawn();
            }
            for (entity, _, _) in &radius_q {
                commands.entity(entity).despawn();
            }
            return;
        }
    };

    let Ok(window) = windows.get_single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_q.get_single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        // Clean up previews when cursor leaves window
        for (entity, _, _) in &preview_q {
            commands.entity(entity).despawn();
        }
        for (entity, _, _) in &radius_q {
            commands.entity(entity).despawn();
        }
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    let color = service_type.color().with_alpha(0.6);
    let radius = service_type.radius();

    // Building preview
    if let Some((_, mut transform, mut sprite)) = preview_q.iter_mut().next() {
        transform.translation = Vec3::new(world_pos.x, 0.5, world_pos.y);
        sprite.color = color;
    } else {
        commands.spawn((
            Sprite {
                color,
                custom_size: Some(Vec2::splat(config.building_size)),
                ..default()
            },
            Transform::from_translation(Vec3::new(world_pos.x, 0.5, world_pos.y))
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            ServicePreview,
        ));
    }

    // Radius preview
    let radius_color = color.with_alpha(0.15);
    if let Some((_, mut transform, mut sprite)) = radius_q.iter_mut().next() {
        transform.translation = Vec3::new(world_pos.x, 0.2, world_pos.y);
        sprite.custom_size = Some(Vec2::splat(radius * 2.0));
        sprite.color = radius_color;
    } else {
        commands.spawn((
            Sprite {
                color: radius_color,
                custom_size: Some(Vec2::splat(radius * 2.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(world_pos.x, 0.2, world_pos.y))
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            RadiusPreview,
        ));
    }
}
