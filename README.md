# Urban Sprawl

A large-scale procedural city simulator built with [Bevy](https://bevyengine.org/) (Rust).

![Rust](https://img.shields.io/badge/Rust-1.75+-orange?logo=rust)
![Bevy](https://img.shields.io/badge/Bevy-0.15-232326?logo=bevy)
![License](https://img.shields.io/badge/License-MIT-blue)

## Overview

Urban Sprawl is a city-building simulation game built with Bevy. Start with a blank canvas or procedurally generated roads, then zone areas, place services, and watch your city grow. The simulation includes RCI demand, population growth, economy, land values, and service coverage - all affecting how your city develops.

The project targets 100,000+ rendered entities at 60 FPS using Bevy's ECS architecture and hardware instancing.

![City Screenshot](docs/screenshot.png)

## Features

### City Building
- **Two Game Modes** - Sandbox (blank canvas) or Procedural (generated road network)
- **Zone Painting** - Paint Residential, Commercial, and Industrial zones
- **Road Drawing** - Click to place road nodes, auto-connects and snaps to existing network
- **Service Buildings** - Place Police, Fire, Hospital, School, and Parks
- **Demolish Tool** - Remove buildings and clear zones

### City Simulation
- **RCI Demand System** - SimCity-style demand meters for each zone type
- **Building Growth** - Zones develop into buildings based on demand and land value
- **Population** - Citizens move in based on housing, jobs, services, and commute quality
- **Economy** - Tax income from buildings, maintenance costs, service expenses
- **Land Value** - Composite score from pollution, crime, services, parks, commute
- **Service Coverage** - Police reduce crime, hospitals boost health, schools boost education
- **Traffic** - Commute calculation affects population happiness

### City Generation
- **Tensor Field Roads** - Organic road networks using grid and radial basis field blending
- **Procedural Buildings** - Multiple shapes (box, L-shape, tower-on-base, stepped) with 5 facade styles
- **Water System** - Procedural rivers with animated water shader and automatic bridges
- **Green Spaces** - Parks with procedurally placed trees and street trees

### Visual Details
- **Day/Night Cycle** - Dynamic sun lighting with smooth transitions
- **Facade-Aware Windows** - Building windows vary by style with night-time illumination
- **Rooftop Details** - AC units, water towers, antennas
- **Tilt-Shift Effect** - Post-processing for miniature/diorama aesthetic
- **Cloud Shadows** - Drifting shadows using procedural noise

### City Life
- **Moving Vehicles** - Cars driving on roads with traffic light awareness
- **Pedestrians** - Citizens walking on sidewalks between intersections
- **Street Furniture** - Lamps, traffic lights, fire hydrants, benches, parked cars

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) 1.75 or later
- Vulkan-compatible GPU (DirectX 12 on Windows has known issues)

### Installation

```bash
git clone https://github.com/bneidlinger/urban_sprawl.git
cd urban_sprawl
cargo run
```

First build will take several minutes as Bevy compiles. Subsequent builds are fast due to dynamic linking.

### Controls

| Input | Action |
|-------|--------|
| WASD / Arrow Keys | Pan camera |
| Middle/Right Mouse + Drag | Pan camera |
| Mouse Wheel | Zoom in/out |
| Q / E | Rotate camera |
| R / C / I | Zone tool (Residential/Commercial/Industrial) |
| D | Road drawing tool |
| X | Demolish tool |
| V | Query/inspect tool |
| Escape | Deselect tool |
| P | Pause simulation |
| [ / ] | Slow down / Speed up time |
| 1-4 | Time presets (Dawn/Day/Dusk/Night) |

## Architecture

The project uses Bevy's plugin system with a staged generation pipeline:

```
TensorField → RoadGraph → CityBlocks → Buildings/Parks
     ↓            ↓            ↓
  Basis      Road Meshes   Street Furniture
  Fields     + Sidewalks   (Lamps, Lights)
```

See [CLAUDE.md](CLAUDE.md) for detailed architectural documentation.

## Roadmap

### Completed
- [x] Tensor field road generation
- [x] Building spawning with shape variety and facade styles
- [x] Parks and green spaces with street trees
- [x] Street furniture (lamps, traffic lights, hydrants, benches)
- [x] Day/night cycle with dynamic lighting
- [x] Moving vehicles with traffic light awareness
- [x] Pedestrians walking on sidewalks
- [x] Water system with rivers and bridges
- [x] Facade-aware windows with night illumination
- [x] Rooftop details (AC units, antennas, water towers)
- [x] Tilt-shift post-processing effect
- [x] Cloud shadows
- [x] Game modes (Sandbox / Procedural)
- [x] Zone painting tool (R/C/I zones)
- [x] Road drawing tool
- [x] Demolish tool
- [x] Service placement (Police, Fire, Hospital, School, Park)
- [x] RCI demand system
- [x] Building growth from zones
- [x] Population tracking with growth factors
- [x] City economy (budget, taxes, costs)
- [x] Land value system (pollution, crime, services, commute)
- [x] Service coverage effects
- [x] Commute/traffic calculation

### In Progress
- [ ] Save/Load system with JSON persistence
- [ ] Autosave functionality

### Planned
- [ ] Weather system (fog, rain)
- [ ] Public transit (buses, trains)
- [ ] Landmark buildings
- [ ] Heat map overlays (crime, pollution, land value)
- [ ] Citizen agents with daily schedules
- [ ] Advanced traffic simulation with pathfinding

## References

This project implements techniques from:

- [Interactive Procedural Street Modeling](https://www.sci.utah.edu/~chengu/street_sig08/street_sig08.pdf) - Chen et al. 2008
- [Procedural Modeling of Buildings](http://peterwonka.net/Publications/pdfs/2006.SG.Mueller.ProceduralModelingOfBuildings.final.pdf) - Mueller et al. 2006

## License

This project is licensed under the MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting PRs.
