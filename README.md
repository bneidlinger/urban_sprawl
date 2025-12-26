# Urban Sprawl

A large-scale procedural city simulator built with [Bevy](https://bevyengine.org/) (Rust).

![Rust](https://img.shields.io/badge/Rust-1.75+-orange?logo=rust)
![Bevy](https://img.shields.io/badge/Bevy-0.15-232326?logo=bevy)
![License](https://img.shields.io/badge/License-MIT-blue)

## Overview

Urban Sprawl generates infinite, organic city layouts using tensor field-based road networks and procedural building placement. The project targets 100,000+ rendered entities at 60 FPS using Bevy's ECS architecture and hardware instancing.

![City Screenshot](docs/screenshot.png)

## Features

- **Tensor Field Road Generation** - Organic road networks using grid and radial basis field blending, traced via streamline integration
- **Procedural Buildings** - Multiple building shapes (box, L-shape, tower-on-base, stepped) with zone-based placement (commercial, residential, industrial)
- **City Infrastructure** - Sidewalks, lane markings, intersections, street lamps, and traffic lights
- **Green Spaces** - Parks with procedurally placed trees scattered throughout the city
- **Orthographic Camera** - Zoom, pan, and rotate controls for city exploration

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
| Mouse Wheel | Zoom in/out |
| Middle Mouse + Drag | Rotate camera |

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

- [x] Tensor field road generation
- [x] Building spawning with shape variety
- [x] Parks and green spaces
- [x] Street furniture (lamps, traffic lights)
- [ ] Vehicle traffic simulation
- [ ] Citizen agents with daily schedules
- [ ] Day/night cycle with dynamic lighting
- [ ] Zoning with Wave Function Collapse
- [ ] Building interiors

## References

This project implements techniques from:

- [Interactive Procedural Street Modeling](https://www.sci.utah.edu/~chengu/street_sig08/street_sig08.pdf) - Chen et al. 2008
- [Procedural Modeling of Buildings](http://peterwonka.net/Publications/pdfs/2006.SG.Mueller.ProceduralModelingOfBuildings.final.pdf) - Mueller et al. 2006

## License

This project is licensed under the MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting PRs.
