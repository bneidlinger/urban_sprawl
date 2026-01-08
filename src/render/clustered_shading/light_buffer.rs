//! Light buffer management for city lights.
//!
//! Tracks all dynamic lights in the city for statistics and bulk operations.

use bevy::prelude::*;

/// Type of city light for categorization.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LightType {
    #[default]
    StreetLamp,
    TrafficLight,
    Window,
    Vehicle,
    Entrance,
}

/// Data for a city light (used for tracking and statistics).
#[derive(Clone, Debug)]
pub struct CityLight {
    pub position: Vec3,
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
    pub light_type: LightType,
    pub entity: Option<Entity>,
}

impl CityLight {
    pub fn new(position: Vec3, color: Color, intensity: f32, radius: f32, light_type: LightType) -> Self {
        Self {
            position,
            color,
            intensity,
            radius,
            light_type,
            entity: None,
        }
    }

    pub fn with_entity(mut self, entity: Entity) -> Self {
        self.entity = Some(entity);
        self
    }
}

/// Buffer containing all city lights for tracking.
#[derive(Resource, Default)]
pub struct CityLightBuffer {
    /// All registered city lights
    lights: Vec<CityLight>,

    /// Dirty flag - set when lights need to be re-processed
    pub dirty: bool,
}

impl CityLightBuffer {
    /// Add a new light to the buffer.
    pub fn add(&mut self, light: CityLight) -> usize {
        let index = self.lights.len();
        self.lights.push(light);
        self.dirty = true;
        index
    }

    /// Get a light by index.
    pub fn get(&self, index: usize) -> Option<&CityLight> {
        self.lights.get(index)
    }

    /// Get mutable reference to a light by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut CityLight> {
        self.dirty = true;
        self.lights.get_mut(index)
    }

    /// Update a light's position.
    pub fn update_position(&mut self, index: usize, position: Vec3) {
        if let Some(light) = self.lights.get_mut(index) {
            light.position = position;
            self.dirty = true;
        }
    }

    /// Update a light's intensity.
    pub fn update_intensity(&mut self, index: usize, intensity: f32) {
        if let Some(light) = self.lights.get_mut(index) {
            light.intensity = intensity;
            self.dirty = true;
        }
    }

    /// Clear all lights.
    pub fn clear(&mut self) {
        self.lights.clear();
        self.dirty = true;
    }

    /// Get total number of lights.
    pub fn len(&self) -> usize {
        self.lights.len()
    }

    /// Check if buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.lights.is_empty()
    }

    /// Iterate over all lights.
    pub fn iter(&self) -> impl Iterator<Item = &CityLight> {
        self.lights.iter()
    }

    /// Count lights by type.
    pub fn count_by_type(&self, light_type: LightType) -> usize {
        self.lights.iter().filter(|l| l.light_type == light_type).count()
    }

    /// Get all lights within a radius of a point.
    pub fn lights_in_radius(&self, center: Vec3, radius: f32) -> Vec<&CityLight> {
        let radius_sq = radius * radius;
        self.lights
            .iter()
            .filter(|l| l.position.distance_squared(center) <= radius_sq)
            .collect()
    }

    /// Get lights of a specific type.
    pub fn lights_of_type(&self, light_type: LightType) -> impl Iterator<Item = &CityLight> {
        self.lights.iter().filter(move |l| l.light_type == light_type)
    }
}

/// Statistics about light distribution in the scene.
#[derive(Debug, Default)]
pub struct LightDistributionStats {
    pub total: usize,
    pub by_type: [(LightType, usize); 4],
    pub min_intensity: f32,
    pub max_intensity: f32,
    pub avg_intensity: f32,
}

impl CityLightBuffer {
    /// Calculate distribution statistics.
    pub fn calculate_stats(&self) -> LightDistributionStats {
        if self.lights.is_empty() {
            return LightDistributionStats::default();
        }

        let mut min_intensity = f32::MAX;
        let mut max_intensity = f32::MIN;
        let mut total_intensity = 0.0;

        for light in &self.lights {
            min_intensity = min_intensity.min(light.intensity);
            max_intensity = max_intensity.max(light.intensity);
            total_intensity += light.intensity;
        }

        LightDistributionStats {
            total: self.lights.len(),
            by_type: [
                (LightType::StreetLamp, self.count_by_type(LightType::StreetLamp)),
                (LightType::TrafficLight, self.count_by_type(LightType::TrafficLight)),
                (LightType::Window, self.count_by_type(LightType::Window)),
                (LightType::Vehicle, self.count_by_type(LightType::Vehicle)),
            ],
            min_intensity,
            max_intensity,
            avg_intensity: total_intensity / self.lights.len() as f32,
        }
    }
}
