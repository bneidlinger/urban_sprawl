# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Urban Sprawl - A large-scale procedural city simulator built with Bevy (Rust ECS) targeting 100,000+ rendered entities at 60 FPS with GPU-driven rendering.

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

- **GameStatePlugin** (`game_state.rs`) - Game state machine (MainMenu/Loading/Playing/Paused) and game modes (Sandbox/Procedural)
- **ToolsPlugin** (`tools/`) - Player interaction tools: zone painting, road drawing, demolish, service placement
- **CameraPlugin** (`camera/`) - Orthographic camera with zoom/pan/rotate
- **RenderPlugin** (`render/`) - Mesh generation, instancing, GPU-driven rendering
- **ProcgenPlugin** (`procgen/`) - Procedural city generation pipeline
- **SimulationPlugin** (`simulation/`) - Economy, demand, population, traffic, services
- **WorldPlugin** (`world/`) - Spatial partitioning, terrain
- **UiPlugin** (`ui/`) - Debug overlays, toolbox, stats bar

### Procedural Generation Pipeline

The city generates in a specific order, controlled by marker resources and run conditions:

1. **TensorFieldPlugin** → Creates `TensorField` resource (grid + radial basis blending)
2. **RoadGeneratorPlugin** → Traces streamlines through tensor field → `RoadGraph` resource, emits `RoadsGenerated`
3. **BlockExtractorPlugin** → Grid-based lot extraction from road network → `CityLots` resource
4. **RoadMeshPlugin** → Generates road/sidewalk/intersection meshes when `RoadsGenerated` detected
5. **BuildingSpawnerPlugin** → Spawns buildings and parks on lots when `CityLots` populated
6. **Street furniture** → Street lamps, traffic lights spawn after road meshes exist

### Generation Flow Control

The pipeline uses marker resources and `run_if` conditions to sequence generation:

```rust
// Event triggers initial generation
GenerateRoadsEvent → generate_roads_on_event → sets RoadsGenerated(true)

// Run conditions check markers
extract_blocks.run_if(should_extract_blocks)  // runs when RoadsGenerated.0 && !CityBlocks.extracted
spawn_buildings.run_if(should_spawn_buildings) // runs when !CityLots.lots.is_empty() && !BuildingsSpawned.0
```

### Key Resources

```rust
TensorField       // Blended basis fields for road direction sampling
RoadGraph         // petgraph-based graph with RoadNode/RoadEdge
CityLots          // Buildable lots extracted from grid (Vec<Lot>)
RoadsGenerated    // Marker resource (bool) signaling roads complete
BuildingsSpawned  // Marker resource (bool) signaling buildings spawned
```

### GPU-Driven Rendering Pipeline

The rendering system uses a multi-stage GPU-driven approach:

1. **MeshPoolsPlugin** - Shared mesh/material pools to reduce draw calls
2. **BuildingInstancesPlugin** - Hardware instancing with 112-byte instance data
3. **FacadeTexturesPlugin** - Procedural 5-layer texture arrays (Brick/Concrete/Glass/Metal/Painted)
4. **GpuCullingPlugin** - Compute shader frustum culling with CPU fallback
5. **HzbPlugin** - Hierarchical depth buffer for occlusion culling
6. **ClusteredShadingPlugin** - 16x9x24 cluster grid for 5,000+ dynamic lights

Culling pipeline: Frustum cull (~50% rejected) → HZB occlusion cull → Indirect draw

### Rendering Sub-Plugins

The `RenderPlugin` composes sub-plugins in dependency order:
- **DayNightPlugin** - Time-of-day lighting with sun direction
- **WeatherPlugin** - Fog, rain, wet surfaces, weather state cycling
- **RoadMeshPlugin** - Road/sidewalk/intersection geometry
- **BuildingSpawnerPlugin** - Building meshes with zone-based styling
- **WindowInstancesPlugin** - Batched window rendering (320,000+ potential entities)
- **ClusteredShadingPlugin** - Many-light management for street lamps, traffic lights, windows
- **TiltShiftPlugin** - Post-processing for miniature/diorama effect

### Simulation Systems

The `SimulationPlugin` runs city simulation at 20 Hz (decoupled from render):
- **DemandPlugin** - SimCity-style RCI demand meters
- **ZoneGrowthPlugin** - Buildings grow from zones based on demand/land value
- **PopulationPlugin** - Citizens move based on housing, jobs, services
- **EconomyPlugin** - Tax income, maintenance costs, budgets
- **LandValuePlugin** - Composite score (pollution, crime, services, commute)
- **ServiceCoveragePlugin** - Police/Fire/Hospital/School coverage effects
- **CommutePlugin** - Traffic calculation affecting happiness

### Player Tools

`ActiveTool` state enum controls current interaction mode:
- `ZonePaint(ZoneType)` - Paint Residential/Commercial/Industrial zones
- `RoadDraw` - Click to place road nodes with auto-connect
- `Demolish` - Remove buildings and clear zones
- `PlaceService(ServiceType)` - Place Police/Fire/Hospital/School/Park
- `Query` - Inspect objects

### Entity Components

Key marker components:
- `Building` - Building with lot_index, BuildingType, FacadeStyle
- `GpuCullable` - Registers entity for GPU frustum culling
- `DynamicCityLight` - Time-of-day controlled light intensity
- `StreetLamp`, `TrafficLight` - Street furniture markers

### Building Shapes

Buildings spawn with varied shapes based on zone type:
- **Box** - Simple rectangular prism (all zones)
- **LShape** - Two overlapping wings (larger lots)
- **TowerOnBase** - Podium with setback tower (commercial)
- **Stepped** - Multiple tiers with decreasing footprint (commercial/residential)

## Architectural Constraints

### ECS Patterns
- All logic as Bevy Systems, NO OOP inheritance
- Data in Components, global state in Resources
- Use marker resources + `run_if` conditions to sequence generation stages
- Use query filters for efficient iteration

### Rendering
- Hardware instancing for city geometry (112-byte instance data)
- True 3D with orthographic projection (not SpriteBundle)
- CCW triangle winding for proper backface culling
- Register cullable objects with `GpuCullable` component

### Performance Targets
- 100,000+ rendered instances at 60 FPS
- 5,000+ dynamic point lights via clustered shading
- Simulation tick rate: 20 Hz (decoupled from render)

### Prohibited Patterns
- `Box<dyn Trait>` on hot paths (use enums)
- "God systems" that do everything
- Long-term Entity references without validation
- `String` where `&str` or interned strings suffice

## References

- Tensor Fields: [Chen et al. 2008 - Interactive Procedural Street Modeling](https://www.sci.utah.edu/~chengu/street_sig08/street_sig08.pdf)
- Shape Grammars: [Mueller et al. 2006 - Procedural Modeling of Buildings](http://peterwonka.net/Publications/pdfs/2006.SG.Mueller.ProceduralModelingOfBuildings.final.pdf)
