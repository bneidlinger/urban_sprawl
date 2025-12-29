# Urban Sprawl Development Plan

## Current State Summary

### Completed Features
- **Procedural Road Network**: Tensor field-based generation with organic layouts
- **Building Generation**: 4 shapes (Box, L-Shape, Tower, Stepped) × 3 zones
- **Parks & Trees**: Green spaces with procedural tree placement
- **Road Infrastructure**: Sidewalks, lane markings, crosswalks, intersections
- **Street Furniture**: Lamps, traffic lights, fire hydrants, benches, parked cars
- **Camera System**: Isometric view with pan/zoom/rotate

### Scaffolded (Not Active)
- Traffic simulation (Nagel-Schreckenberg CA)
- Citizen agents (needs, schedules, states)
- Flow field pathfinding
- Terrain height maps
- Wave Function Collapse zoning

---

## Phase 1: Visual Polish & Atmosphere

### 1.1 Day/Night Cycle
- [ ] Sun position animation (directional light rotation)
- [ ] Sky color gradient (dawn → day → dusk → night)
- [ ] Street lamp activation at night (emissive intensity ramp)
- [ ] Building window lights at night (random pattern)
- [ ] Moon and stars for night sky

### 1.2 Weather System
- [ ] Fog/haze for depth and atmosphere
- [ ] Rain particles with wet road reflections
- [ ] Puddle generation on roads during rain
- [ ] Cloud shadows moving across city

### 1.3 Enhanced Lighting
- [ ] Ambient occlusion for building bases
- [ ] Bloom effect for lights at night
- [ ] Colored accent lighting (neon signs, storefronts)
- [ ] Car headlights/taillights

### 1.4 Water Features
- [ ] Rivers cutting through city
- [ ] Bridges over water
- [ ] Fountains in parks
- [ ] Reflective water surfaces
- [ ] Waterfront promenades

---

## Phase 2: Building & Architecture Detail

### 2.1 Facade Details
- [ ] Window grid patterns on buildings
- [ ] Balconies on residential buildings
- [ ] Storefronts with awnings on commercial ground floors
- [ ] Rooftop details (AC units, water towers, helipads)
- [ ] Antenna/spire variations on tall buildings

### 2.2 Landmark Buildings
- [ ] Unique procedural landmarks (clock towers, monuments)
- [ ] Churches/temples with distinct silhouettes
- [ ] Stadium/arena structures
- [ ] Train stations with platforms
- [ ] Iconic skyscrapers with distinctive tops

### 2.3 Building Textures
- [ ] Procedural facade textures (brick, glass, concrete)
- [ ] Wear/weathering variation
- [ ] Graffiti on industrial buildings
- [ ] Billboards and advertisements

### 2.4 Construction Sites
- [ ] Crane meshes
- [ ] Scaffolding around "under construction" buildings
- [ ] Foundation pits
- [ ] Construction barriers

---

## Phase 3: Transportation & Movement

### 3.1 Moving Vehicles
- [ ] Cars driving on roads (flow field pathfinding)
- [ ] Traffic light response (stop on red)
- [ ] Lane changing behavior
- [ ] Vehicle variety (sedans, SUVs, trucks, buses)
- [ ] Emergency vehicles with sirens

### 3.2 Public Transit
- [ ] Bus stops with shelters
- [ ] Buses following routes
- [ ] Elevated train tracks
- [ ] Train cars moving on tracks
- [ ] Subway station entrances

### 3.3 Pedestrians
- [ ] Walking citizens on sidewalks
- [ ] Crosswalk usage (wait for signal)
- [ ] Crowd density near commercial areas
- [ ] Simple low-poly character models

### 3.4 Parking Infrastructure
- [ ] Parking lots (surface lots in suburbs)
- [ ] Parking garages (multi-story structures)
- [ ] Cars entering/exiting parking

---

## Phase 4: Urban Life Details

### 4.1 Street-Level Details
- [ ] Newspaper stands
- [ ] Food carts/vendors
- [ ] Bus shelters
- [ ] Phone booths / charging stations
- [ ] Bike racks with bikes
- [ ] Outdoor café seating

### 4.2 Signage & Wayfinding
- [ ] Street name signs at intersections
- [ ] One-way signs
- [ ] Speed limit signs
- [ ] Highway exit signs
- [ ] Business signs on buildings

### 4.3 Utilities
- [ ] Power lines and poles (suburban areas)
- [ ] Manholes on roads
- [ ] Storm drains at curbs
- [ ] Utility boxes on sidewalks

### 4.4 Nature Integration
- [ ] Street trees along sidewalks
- [ ] Planters and flower beds
- [ ] Community gardens
- [ ] Green roofs on some buildings
- [ ] Bird flocks (simple particle system)

---

## Phase 5: Simulation Systems

### 5.1 Traffic Simulation
- [ ] Activate Nagel-Schreckenberg CA model
- [ ] Vehicle spawning at city edges
- [ ] Intersection priority logic
- [ ] Traffic congestion visualization
- [ ] Accident events

### 5.2 Citizen Simulation
- [ ] Spawn citizens with home/work locations
- [ ] Daily schedule (wake → commute → work → home)
- [ ] Needs system affecting behavior
- [ ] Population growth/decline

### 5.3 Economic Simulation
- [ ] Zone demand (R/C/I)
- [ ] Property values by location
- [ ] Business spawning/closing
- [ ] Employment tracking

### 5.4 City Growth
- [ ] Procedural expansion over time
- [ ] Road network extension
- [ ] Building construction animation
- [ ] Demolition of old structures

---

## Phase 6: Unique Art Style

### 6.1 Color Palette
- [ ] Cohesive color scheme (pick: warm sunset / cool twilight / vibrant day)
- [ ] Color grading post-process
- [ ] Distinct zone color identities
- [ ] Seasonal color variations

### 6.2 Stylization Options
- [ ] Tilt-shift blur effect (miniature look)
- [ ] Outline/cel-shading option
- [ ] Voxel-style building option
- [ ] Low-poly aesthetic mode

### 6.3 Camera Effects
- [ ] Depth of field
- [ ] Motion blur on pan
- [ ] Screen-space reflections
- [ ] Vignette

### 6.4 Audio (Future)
- [ ] Ambient city sounds
- [ ] Traffic noise based on density
- [ ] Birds in parks
- [ ] Rain/weather sounds

---

## Implementation Priority

### Immediate (High Visual Impact)
1. **Day/Night Cycle** - Dramatic atmosphere change
2. **Window Lights** - Buildings come alive at night
3. **Moving Vehicles** - City feels alive
4. **Street Trees** - Softer, more natural look
5. **Tilt-Shift Effect** - Instant "miniature city" aesthetic

### Short Term
6. Water features (rivers, fountains)
7. Pedestrians on sidewalks
8. Building facade windows
9. Rooftop details
10. Weather (fog first, then rain)

### Medium Term
11. Public transit system
12. Landmark buildings
13. Full traffic simulation
14. Citizen agents
15. Signage and billboards

### Long Term
16. Economic simulation
17. City growth over time
18. Audio system
19. Multiple art style options

---

## Technical Notes

### Performance Considerations
- Use GPU instancing for repeated elements (windows, trees, vehicles)
- LOD system for distant buildings
- Culling for off-screen elements
- Batch similar materials

### Bevy 0.15 Features to Leverage
- PBR materials with emissive for lights
- Gizmos for debug visualization
- Compute shaders for traffic CA
- Post-processing pipeline for effects

---

## Art Direction Goals

**Target Aesthetic**: A blend of:
- **SimCity 4** - Dense, detailed urbanism
- **Cities: Skylines** - Modern polish
- **Townscaper** - Cozy, stylized feel
- **Minecraft** - Procedural charm

**Unique Identity**:
- Warm, inviting color palette
- Miniature/diorama feel via tilt-shift
- Organic road layouts (not grid-only)
- Dense urban core fading to leafy suburbs
- Living city that breathes (day/night, weather, movement)
