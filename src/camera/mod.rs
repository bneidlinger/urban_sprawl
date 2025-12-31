//! Orthographic camera system with zoom, pan, and rotate controls.

use bevy::{
    input::mouse::MouseMotion,
    pbr::{DistanceFog, FogFalloff},
    prelude::*,
};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(Update, (camera_zoom, camera_pan, camera_rotate));
    }
}

/// Marker component for the main isometric camera.
#[derive(Component)]
pub struct IsometricCamera {
    pub zoom: f32,
    pub rotation: f32,
}

impl Default for IsometricCamera {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            rotation: 0.0,
        }
    }
}

fn setup_camera(mut commands: Commands) {
    // Standard isometric angle: ~35.264 degrees (arctan(1/sqrt(2)))
    let iso_angle = 35.264_f32.to_radians();
    let distance = 500.0;

    commands.spawn((
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scale: 0.1,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(distance, distance * iso_angle.tan(), distance)
            .looking_at(Vec3::ZERO, Vec3::Y),
        DistanceFog {
            color: Color::srgba(0.6, 0.7, 0.8, 0.85),
            falloff: FogFalloff::Exponential { density: 0.0015 },
            directional_light_color: Color::srgba(1.0, 0.8, 0.6, 0.3),
            directional_light_exponent: 12.0,
        },
        IsometricCamera::default(),
    ));
}

fn camera_zoom(
    mut query: Query<(&mut Projection, &mut IsometricCamera)>,
    mut scroll_events: EventReader<bevy::input::mouse::MouseWheel>,
) {
    let scroll: f32 = scroll_events.read().map(|e| e.y).sum();
    if scroll == 0.0 {
        return;
    }

    for (mut projection, mut iso_cam) in &mut query {
        iso_cam.zoom = (iso_cam.zoom - scroll * 0.1).clamp(0.1, 10.0);
        if let Projection::Orthographic(ref mut ortho) = *projection {
            ortho.scale = iso_cam.zoom * 0.1;
        }
    }
}

fn camera_pan(
    mut query: Query<(&mut Transform, &IsometricCamera)>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    time: Res<Time>,
) {
    let mut direction = Vec3::ZERO;
    let speed = 100.0;

    // Keyboard panning
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        direction.z -= 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        direction.z += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }

    if direction != Vec3::ZERO {
        let delta = direction.normalize() * speed * time.delta_secs();
        for (mut transform, _) in &mut query {
            transform.translation += delta;
        }
    }

    // Mouse panning (middle button or right button drag)
    if mouse_buttons.pressed(MouseButton::Middle) || mouse_buttons.pressed(MouseButton::Right) {
        let mut mouse_delta = Vec2::ZERO;
        for event in mouse_motion.read() {
            mouse_delta += event.delta;
        }

        if mouse_delta != Vec2::ZERO {
            for (mut transform, iso_cam) in &mut query {
                // Scale pan speed based on zoom level
                let pan_speed = iso_cam.zoom * 0.5;
                // Map mouse X to world X/Z diagonal (isometric), mouse Y to opposite diagonal
                let world_delta = Vec3::new(
                    (-mouse_delta.x + mouse_delta.y) * pan_speed,
                    0.0,
                    (-mouse_delta.x - mouse_delta.y) * pan_speed,
                );
                transform.translation += world_delta;
            }
        }
    } else {
        // Clear any pending mouse motion events when not panning
        mouse_motion.clear();
    }
}

fn camera_rotate(
    mut query: Query<(&mut Transform, &mut IsometricCamera)>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let rotation_speed = 1.0;
    let mut rotation_delta = 0.0;

    if keys.pressed(KeyCode::KeyQ) {
        rotation_delta -= rotation_speed * time.delta_secs();
    }
    if keys.pressed(KeyCode::KeyE) {
        rotation_delta += rotation_speed * time.delta_secs();
    }

    if rotation_delta != 0.0 {
        for (mut transform, mut iso_cam) in &mut query {
            iso_cam.rotation += rotation_delta;
            let target = Vec3::ZERO; // TODO: Track actual look-at target
            transform.rotate_around(target, Quat::from_rotation_y(rotation_delta));
        }
    }
}
