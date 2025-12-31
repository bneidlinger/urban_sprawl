# Changelog

All notable changes to IsoCitySim will be documented in this file.

## [Unreleased]

### Added
- **Pedestrians** - Walking pedestrians on sidewalks
  - 50 pedestrians spawned at road nodes, walking at ~1.4 m/s
  - Sidewalk offset calculation using perpendicular road direction
  - Varied clothing colors (8 options) and skin tones (5 options)
  - Random wandering between intersections on Major/Minor roads
- **Moving Vehicles** - Cars driving on the road network
  - 25 vehicles traversing road edges with waypoint-based navigation
  - Speed varies by road type (Highway 1.5x, Major 1.0x, Minor 0.8x)
  - Traffic light awareness - vehicles stop at red/yellow lights
  - Random edge selection at intersections
- **Street Trees** - Trees lining sidewalks on Major and Minor roads
  - 25m spacing, alternating sides of street
  - Varied heights (5-10m) and foliage sizes (2-3.5m)
  - Three foliage color variations
  - Terrain-following placement
- **Mouse Panning** - Camera pan with middle/right mouse button drag
- **Cloud Shadows** - Drifting cloud shadows using procedural 5-octave FBM noise
  - Custom WGSL shader (`assets/shaders/cloud_shadows.wgsl`)
  - Configurable wind direction, speed, coverage, and opacity
  - Automatically fades out at night
- **Building Drop Shadows** - Alpha-blended shadow planes beneath buildings
  - Shadow offset based on building height (simulates sun angle)
  - Configurable opacity and spread

### Fixed
- **Terrain-following for all objects** - All city elements now properly follow terrain height:
  - Roads, sidewalks, and intersections
  - Road markings (lane lines)
  - Crosswalks
  - Parked cars (body and wheels)
  - Buildings (all shapes: box, L-shape, tower, stepped)
  - Parks and trees
  - Building drop shadows
- Fixed `ResMut`/`Res` resource conflict in `window_lights.rs`

### Changed
- Road mesh generation now samples terrain height at each vertex
- Building spawner samples terrain at building center for proper placement

## [0.1.0] - Initial Release

### Added
- Procedural city generation using tensor field road network
- Day/night cycle with directional sun lighting
- Cascaded shadow maps (3 cascades, 2048x2048)
- Window lights that glow at night with color variety
- Terrain height variation using Perlin noise
- Multiple building shapes (Box, L-Shape, Tower on Base, Stepped)
- Street furniture (lamps, traffic lights, fire hydrants, benches)
- Parked cars with varied colors
- Crosswalks at intersections
- Road markings (center lines, edge lines)
- Parks with procedural trees
