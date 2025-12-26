# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

IsoCitySim - A large-scale isometric city simulator built with Bevy (Rust ECS) targeting 100,000+ active entities with procedural generation.

## Build Commands

```bash
cargo build              # Build the project
cargo run                # Run the simulator
cargo check              # Fast type checking without full build
cargo test               # Run all tests
cargo test test_name     # Run a specific test
cargo clippy             # Run linter
```

**Windows Note:** The app forces Vulkan backend (`WGPU_BACKEND=vulkan`) because DX12 causes crashes on some systems.

## Architecture

### Plugin Structure

All game systems are organized as Bevy plugins registered in `main.rs`:

- **CameraPlugin** (`camera/`) - Orthographic camera with zoom/pan/rotate
- **RenderPlugin** (`render/`) - Mesh generation, instancing, visual elements
- **ProcgenPlugin** (`procgen/`) - Procedural city generation pipeline
- **SimulationPlugin** (`simulation/`) - Agent behavior, traffic (scaffolded)
- **WorldPlugin** (`world/`) - Spatial partitioning, terrain (scaffolded)
- **UiPlugin** (`ui/`) - Debug overlays and gizmos

### Procedural Generation Pipeline

The city generates in a specific order, controlled by marker resources and run conditions:

1. **TensorFieldPlugin** → Creates `TensorField` resource (grid + radial basis blending)
2. **RoadGeneratorPlugin** → Traces streamlines through tensor field → `RoadGraph` resource, emits `RoadsGenerated`
3. **BlockExtractorPlugin** → Grid-based lot extraction from road network → `CityBlocks` resource, emits `BlocksExtracted`
4. **RoadMeshPlugin** → Generates road/sidewalk/intersection meshes when `RoadsGenerated` detected
5. **BuildingSpawnerPlugin** → Spawns buildings and parks on lots when `BlocksExtracted` detected
6. **Street furniture** → Street lamps, traffic lights spawn after road meshes exist

### Key Resources

```rust
TensorField       // Blended basis fields for road direction sampling
RoadGraph         // petgraph-based graph with RoadNode/RoadEdge
CityBlocks        // Collection of buildable lots between roads
RoadsGenerated    // Marker resource signaling road generation complete
BlocksExtracted   // Marker resource signaling lots are ready
```

### Rendering Components

- `RoadMesh`, `SidewalkMesh`, `IntersectionMesh` - Road geometry markers
- `BuildingMesh` - Building entity with shape/zone info
- `ParkMarker`, `TreeMarker` - Green space entities
- `StreetLamp`, `TrafficLight` - Street furniture

## Architectural Constraints

### ECS Only
- All logic as Bevy Systems, NO OOP inheritance
- Data in Components, global state in Resources
- Use query filters for efficient iteration

### Rendering
- Hardware instancing for city geometry
- True 3D with orthographic projection (not SpriteBundle)
- CCW triangle winding for proper backface culling

### Performance Targets
- 100,000+ rendered instances at 60 FPS
- Simulation tick rate: 20 Hz (decoupled from render)

### Prohibited Patterns
- `Box<dyn Trait>` on hot paths (use enums)
- "God systems" that do everything
- Long-term Entity references without validation
- `String` where `&str` or interned strings suffice

## References

- Tensor Fields: [Chen et al. 2008 - Interactive Procedural Street Modeling](https://www.sci.utah.edu/~chengu/street_sig08/street_sig08.pdf)
- Shape Grammars: [Mueller et al. 2006 - Procedural Modeling of Buildings](http://peterwonka.net/Publications/pdfs/2006.SG.Mueller.ProceduralModelingOfBuildings.final.pdf)
