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

### Tilt-Shift Effect (Miniature Look)
**Files:** `src/render/tilt_shift.rs`, `assets/shaders/tilt_shift.wgsl`

- Post-processing effect that blurs top and bottom of screen
- Sharp focus band in center creates "miniature city" aesthetic
- Configurable via `TiltShiftConfig` resource:
  - `enabled: true` - toggle effect on/off
  - `focus_center: 0.5` - vertical center of focus band (0-1)
  - `focus_width: 0.25` - width of sharp focus band
  - `blur_amount: 3.0` - maximum blur at screen edges
  - `blur_samples: 8` - quality (higher = smoother blur)
  - `saturation: 1.15` - color saturation boost for miniature feel
- Uses Gaussian blur with smooth falloff from focus band
- Runs as post-process after tonemapping

### River & Water System
**Files:** `src/procgen/river.rs`, `src/render/water.rs`, `src/render/bridges.rs`, `assets/shaders/water.wgsl`

- Procedural meandering river using cubic Bezier curves
- Custom water shader with animated waves, reflections, and foam
- Automatic bridge spawning where roads cross water (26 bridges)
- Road network respects water boundaries
- Configurable via `RiverConfig` resource

### Facade-Aware Windows
**File:** `src/render/window_lights.rs`

- Building windows vary by facade style (Glass, Brick, Concrete, Metal, Painted)
- Glass facades: Large floor-to-ceiling panels (2.4m×2.6m), blue-tinted
- Brick/Painted facades: Small windows (1.0m×1.4m) with white frames
- Window frames spawned for traditional styles (~1,700 frames)
- Different occupancy rates and night intensities per style

### Rooftop Details
**File:** `src/render/rooftop_details.rs`

- AC units on commercial/industrial buildings (60% probability)
- Water towers on traditional residential (Brick/Painted facades)
- Communication antennas on various buildings (20% probability)
- Helipads on tall commercial buildings
- ~460 rooftop elements spawned

### Street Trees
**File:** `src/render/street_trees.rs`

- Trees lining sidewalks on Major and Minor roads
- 25m spacing, alternating sides of street
- Varied heights (5-10m) and foliage sizes (2-3.5m)
- Three foliage color variations
- ~1,700 street trees spawned

### Moving Vehicles
**File:** `src/simulation/vehicle_traffic.rs`

- Cars driving on road network with waypoint navigation
- Traffic light awareness (stop on red/yellow)
- Speed varies by road type (Highway 1.5x, Major 1.0x, Minor 0.8x)
- ~25 vehicles active at a time

### Pedestrians
**File:** `src/simulation/pedestrians.rs`

- Citizens walking on sidewalks between intersections
- Varied clothing colors and skin tones
- ~50 pedestrians active

## In Progress

None currently.

## Planned

From `graphics.md` roadmap:

| Feature | Category | Status |
|---------|----------|--------|
| Rain/snow | Weather | Next priority |
| Fog/haze | Weather | Next priority |
| Industrial smoke | Animation | Planned |
| Bloom effect | Post-process | Planned |
| Car headlights | Vehicles | Planned |

## Configuration

All visual systems can be tuned via their respective `Resource` structs:

- `TimeOfDay` - day/night cycle speed, current time
- `DayNightConfig` - sun intensity, ambient colors, sky colors
- `WindowLightConfig` - window size, occupancy rate, facade settings
- `TerrainConfig` - noise parameters, mesh resolution
- `CloudShadowConfig` - wind, coverage, opacity, noise scale
- `BuildingShadowConfig` - offset direction, opacity, spread
- `TiltShiftConfig` - focus band, blur amount, saturation
- `RiverConfig` - river path, width, water level
- `RooftopDetailConfig` - AC/antenna/water tower probabilities
