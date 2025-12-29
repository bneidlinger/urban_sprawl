# Visual Improvements Progress

Tracking implementation of graphics enhancements from `graphics.md`.

## Completed

### Shadow Mapping
**File:** `src/render/day_night.rs`

- Enabled cascaded shadow maps on the sun's DirectionalLight
- 3 cascades covering 0.1 - 400 units (city scale)
- Shadow map resolution: 2048x2048
- Tuned bias settings (`shadow_depth_bias: 0.3`, `shadow_normal_bias: 1.8`)

```rust
DirectionalLight {
    shadows_enabled: true,
    shadow_depth_bias: 0.3,
    shadow_normal_bias: 1.8,
    ..
}
```

### Window Lights (Night Illumination)
**File:** `src/render/window_lights.rs`

- Windows on buildings glow at night with emissive materials
- Color variety: warm white, soft white, yellow, cool blue (TV glow)
- 60% occupancy rate (randomized per window)
- Smooth transitions at dawn (6-7 AM) and dusk (6-7 PM)
- Each window stores its own color for consistent appearance

### Terrain Height Variation
**File:** `src/render/instancing.rs`

- Replaced flat ground plane with procedural terrain mesh
- Fractal Perlin noise (4 octaves) for natural hills
- Configurable via `TerrainConfig` resource:
  - `size: 600.0` - terrain extent in world units
  - `resolution: 128` - mesh subdivisions
  - `height_scale: 8.0` - max height variation
  - `noise_scale: 0.008` - noise frequency
- Vertex normals recalculated for proper lighting
- **All objects follow terrain:** roads, sidewalks, intersections, road markings, crosswalks, parked cars, buildings, parks, and trees all sample terrain height

### Cloud Shadows
**Files:** `src/render/cloud_shadows.rs`, `assets/shaders/cloud_shadows.wgsl`

- Drifting cloud shadows using procedural fractal noise (5-octave FBM)
- Large plane mesh positioned above terrain with alpha-blended material
- Configurable via `CloudShadowConfig` resource:
  - `wind_direction` / `wind_speed` - shadow drift direction and speed
  - `noise_scale: 0.008` - cloud size (smaller = larger clouds)
  - `coverage: 0.45` - sky coverage (0 = clear, 1 = overcast)
  - `softness: 0.15` - edge falloff
  - `max_opacity: 0.35` - shadow darkness during day
- Automatically fades out at night (uses `TimeOfDay.transition_factor()`)

### Building Drop Shadows
**File:** `src/render/building_shadows.rs`

- Alpha-blended shadow planes beneath each building
- Shadow offset based on building height (simulates sun angle)
- Configurable via `BuildingShadowConfig` resource:
  - `offset_direction` - shadow direction (normalized Vec2)
  - `offset_scale: 0.15` - offset multiplier based on height
  - `opacity: 0.4` - shadow darkness
  - `spread: 1.2` - shadow size multiplier
- Shadows follow terrain height

## In Progress

None currently.

## Planned

From `graphics.md` roadmap:

| Feature | Category | Approach |
|---------|----------|----------|
| Rain/snow | Weather | Particle system (bevy_hanabi) |
| Water bodies | Terrain | Separate mesh with reflective material |
| Industrial smoke | Animation | Particle emitters on factories |
| Bloom/tilt-shift | Post-process | Bevy post-processing pipeline |

## Configuration

All visual systems can be tuned via their respective `Resource` structs:

- `TimeOfDay` - day/night cycle speed, current time
- `DayNightConfig` - sun intensity, ambient colors, sky colors
- `WindowLightConfig` - window size, occupancy rate
- `TerrainConfig` - noise parameters, mesh resolution
- `CloudShadowConfig` - wind, coverage, opacity, noise scale
- `BuildingShadowConfig` - offset direction, opacity, spread
