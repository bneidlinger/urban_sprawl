# Urban Sprawl Development Plan

## Current State Summary

### Completed Features
- **Procedural Road Network**: Tensor field-based generation with organic layouts
- **Building Generation**: 4 shapes × 5 facade styles × 3 zones with facade-aware windows
- **Parks & Trees**: Green spaces with procedural tree placement + street trees
- **Road Infrastructure**: Sidewalks, lane markings, crosswalks, intersections
- **Street Furniture**: Lamps, traffic lights, fire hydrants, benches, parked cars
- **Camera System**: Isometric view with pan/zoom/rotate + mouse panning
- **Day/Night Cycle**: Dynamic sun lighting with window illumination
- **Water System**: Procedural river with animated shader and bridges
- **Moving Vehicles**: Cars with traffic light awareness and realistic angular meshes
- **Parked Vehicles**: Variety of parked cars with proper wheel placement
- **Pedestrians**: Citizens walking on sidewalks
- **Visual Effects**: Tilt-shift, cloud shadows, building shadows
- **Rooftop Details**: AC units, antennas, water towers
- **Weather System**: Dynamic fog, rain, wet surfaces with auto-cycling

### Gameplay Foundation ✅
- **Sandbox Mode**: Blank canvas start - player builds city from scratch
- **Procedural Mode**: Auto-generated city for testing/demo
- **Zone Painting**: R/C/I zone tools with visual overlays
- **Zone Growth**: Buildings spawn based on demand and land value
- **RCI Demand**: SimCity-style demand meters
- **Economy System**: Budget, taxes, service costs

### Scaffolded (Not Active)
- Full traffic simulation (Nagel-Schreckenberg CA)
- Citizen agents (needs, schedules, states)
- Wave Function Collapse zoning

---

## Phase 1: Visual Polish & Atmosphere

### 1.1 Day/Night Cycle ✅
- [x] Sun position animation (directional light rotation)
- [x] Sky color gradient (dawn → day → dusk → night)
- [x] Street lamp activation at night (emissive intensity ramp)
- [x] Building window lights at night (random pattern)
- [x] Moon and stars for night sky ✅

### 1.2 Weather System ✅
- [x] Fog/haze for depth and atmosphere ✅
- [x] Rain particles with wet road reflections ✅
- [x] Puddle generation on roads during rain ✅
- [x] Cloud shadows moving across city ✅
- [x] Weather state machine (Clear/Foggy/Rainy/Stormy) ✅
- [x] Automatic weather cycling (10-30 min) ✅
- [x] Keyboard controls (F, F5-F8, Shift+F) ✅

### 1.3 Enhanced Lighting ✅
- [ ] Ambient occlusion for building bases
- [x] Bloom effect for lights at night ✅
- [x] Colored accent lighting (neon signs, storefronts) ✅
- [x] Car headlights/taillights ✅

### 1.4 Water Features ✅
- [x] Rivers cutting through city
- [x] Bridges over water
- [ ] Fountains in parks
- [x] Reflective water surfaces
- [ ] Waterfront promenades

---

## Phase 2: Building & Architecture Detail

### 2.1 Facade Details ✅
- [x] Window grid patterns on buildings ✅
- [x] Balconies on residential buildings ✅
- [x] Storefronts with awnings on commercial ground floors ✅
- [x] Rooftop details (AC units, water towers, helipads) ✅
- [x] Antenna/spire variations on tall buildings ✅

### 2.2 Landmark Buildings
- [x] Unique procedural landmarks (clock towers, monuments) ✅
- [x] Churches/temples with distinct silhouettes ✅
- [ ] Stadium/arena structures
- [ ] Train stations with platforms
- [ ] Iconic skyscrapers with distinctive tops

### 2.3 Building Textures ✅
- [x] Procedural facade textures (brick, glass, concrete, metal, painted) ✅
- [x] PBR+ material system (roughness, metallic, height maps) ✅
- [x] Parallax Occlusion Mapping for surface depth ✅
- [x] Cook-Torrance BRDF lighting ✅
- [ ] Wear/weathering variation
- [x] Graffiti on industrial buildings ✅
- [x] Billboards and advertisements ✅

### 2.4 Construction Sites ✅
- [x] Crane meshes (mast, jib, counter-weight) ✅
- [x] Scaffolding around "under construction" buildings ✅
- [x] Foundation pits ✅
- [x] Construction barriers ✅
- [x] Integration with zone growth simulation ✅
- [x] Construction progress phases (Foundation → Structure → Enclosure → Finishing) ✅
- [x] Partial building mesh grows with progress ✅

---

## Phase 3: Transportation & Movement

### 3.1 Moving Vehicles ✅
- [x] Cars driving on roads (waypoint navigation)
- [x] Traffic light response (stop on red)
- [x] Lane changing behavior ✅
- [x] Vehicle variety (sedans, SUVs, trucks, vans, buses) ✅
- [x] Emergency vehicles with sirens/lights (police, fire, ambulance) ✅
- [x] Realistic angular vehicle meshes (box-based geometry) ✅
- [x] Proper wheel meshes at vehicle corners ✅

### 3.2 Public Transit ✅
- [x] Bus stops with shelters ✅
- [x] Buses following routes ✅
- [x] Elevated train tracks ✅
- [x] Train cars moving on tracks ✅
- [x] Subway station entrances ✅

### 3.3 Pedestrians ✅
- [x] Walking citizens on sidewalks
- [x] Crosswalk usage (wait for signal) ✅
- [x] Crowd density near commercial areas ✅
- [x] Simple low-poly character models

### 3.4 Parking Infrastructure
- [x] Parking lots (surface lots in suburbs) ✅
- [x] Parking garages (multi-story structures) ✅
- [ ] Cars entering/exiting parking

---

## Phase 4: Urban Life Details ✅

### 4.1 Street-Level Details ✅
- [x] Newspaper stands ✅
- [x] Food carts/vendors ✅
- [x] Bus shelters ✅
- [x] Phone booths / charging stations ✅
- [x] Bike racks with bikes ✅
- [x] Outdoor café seating ✅

### 4.2 Signage & Wayfinding ✅
- [x] Street name signs at intersections ✅
- [x] One-way signs ✅
- [x] Speed limit signs ✅
- [x] Highway exit signs
- [x] Business signs on buildings

### 4.3 Utilities ✅
- [x] Power lines and poles (suburban areas) ✅
- [x] Manholes on roads ✅
- [x] Storm drains at curbs ✅
- [x] Utility boxes on sidewalks ✅

### 4.4 Nature Integration ✅
- [x] Street trees along sidewalks ✅
- [x] Planters and flower beds ✅
- [x] Community gardens
- [x] Green roofs on some buildings ✅
- [x] Bird flocks (simple particle system) ✅

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
- [x] Tilt-shift blur effect (miniature look) ✅
- [ ] Outline/cel-shading option
- [ ] Voxel-style building option
- [ ] Low-poly aesthetic mode

### 6.3 Camera Effects ✅
- [x] Vignette (cinematic polish plugin) ✅
- [x] Film grain (animated procedural noise) ✅
- [x] Chromatic aberration (edge color fringing) ✅
- [x] Tonemapping options (TonyMcMapface, AgX, ACES, Reinhard) ✅
- [ ] Depth of field
- [ ] Motion blur on pan
- [ ] Screen-space reflections

### 6.4 Audio (Future)
- [ ] Ambient city sounds
- [ ] Traffic noise based on density
- [ ] Birds in parks
- [ ] Rain/weather sounds

---

## Implementation Priority

### Immediate (High Visual Impact)
1. **Day/Night Cycle** - Dramatic atmosphere change ✅
2. **Window Lights** - Buildings come alive at night ✅
3. **Moving Vehicles** - City feels alive ✅
4. **Street Trees** - Softer, more natural look ✅
5. **Tilt-Shift Effect** - Instant "miniature city" aesthetic ✅

### Short Term
6. Water features (rivers, fountains) ✅
7. Pedestrians on sidewalks ✅
8. Building facade windows ✅
9. Rooftop details ✅
10. Weather (fog first, then rain) ✅

### Medium Term
11. Public transit system
12. Landmark buildings ✅
13. Full traffic simulation
14. Citizen agents
15. Signage and billboards ✅

### Long Term
16. Economic simulation
17. City growth over time
18. Audio system
19. Multiple art style options

---

## Technical Notes

### GPU-Driven Rendering Pipeline ✅

A comprehensive GPU-driven rendering infrastructure has been implemented to achieve 100,000+ entities at 60 FPS:

#### Phase 1: GPU Instancing ✅
- Extended InstanceData struct (112 bytes) with full 4x4 transform matrix
- Mesh pools for shared building geometry (Box, L-Shape, Tower-on-Base, Stepped)
- Building instance buffer with automatic resizing and dirty tracking
- Material parameters embedded in instance data (roughness, metallic, emissive, facade_type)

#### Phase 2: Clustered Shading ✅
- 16x9x24 cluster grid (3,456 clusters) with exponential depth slicing
- Supports 5,000+ dynamic point lights at 60 FPS
- Light-to-cluster assignment compute shader (`cluster_assign.wgsl`)
- Fragment shader lighting loop (`cluster_lighting.wgsl`)
- O(1) cluster lookup per fragment

#### Phase 3: Window Entity Batching ✅
- WindowInstanceData struct with position, size, normal, color, intensity
- Facade-aware window properties per architectural style
- Instanced window shader (`window_instanced.wgsl`)
- Reduces potential 320,000 window entities to batched buffer

#### Phase 4: Texture Arrays ✅
- 5-layer procedural facade texture array (Brick, Concrete, Glass, Metal, Painted)
- 256x256 textures generated at runtime with varied patterns
- Single-draw-call material switching via facade_type index

#### Phase 5: GPU Frustum Culling ✅
- ObjectData buffer with bounding spheres (32 bytes per object)
- FrustumPlanes extracted using Gribb-Hartmann method from view-projection matrix
- WGSL compute shader (`frustum_cull.wgsl`) with 64-thread workgroups
- CPU fallback culling when compute shaders unavailable
- GpuCullable component for per-entity culling participation
- Real-time statistics tracking (visible/culled ratio)

#### Phase 6: HZB Occlusion Culling ✅
- HzbPyramid resource for depth mip chain management
- CpuHzbPyramid for software fallback testing
- MAX reduction depth pyramid generation (`hzb_generate.wgsl`)
- Screen-space projection for bounding sphere size calculation
- Previous-frame depth usage to avoid render dependencies
- Integration with frustum culling shader (`main_with_hzb` entry point)

#### Phase 7: Indirect Draw Integration ✅
- GpuCullingPipeline with compute shader binding and bind group layout
- GpuCullingBuffers managing all GPU resources (uniforms, frustum, objects, visibility, indirect)
- DrawIndexedIndirect buffer populated by `main_indirect` compute shader entry point
- GpuCullingNode render graph node for compute pass execution
- ExtractedCullData for main-to-render world data extraction
- Zero CPU involvement in visibility determination

#### Phase 8: PBR+ Materials ✅
- Extended `FacadeTextureArray` with roughness, metallic, and height maps (R8Unorm)
- `BuildingInstancedMaterial` with 5 texture array bindings (albedo, normal, roughness, metallic, height)
- `BuildingMaterialUniforms` struct with POM parameters (scale, layers)
- Parallax Occlusion Mapping in `building_instanced.wgsl`:
  - Ray marching through height field with layer interpolation
  - Per-facade depth scales (brick 0.06, metal 0.04, concrete 0.02)
  - Distance-based LOD (64→8 layers, disabled beyond 150m)
  - POM self-shadowing for recessed areas
- Cook-Torrance BRDF lighting:
  - GGX/Trowbridge-Reitz normal distribution
  - Smith's Schlick-GGX geometry function
  - Fresnel-Schlick approximation
- `BuildingPbrMaterialHandle` resource for material initialization

#### Phase 9: Cinematic Post-Processing ✅
- `CinematicPolishPlugin` with combined single-pass shader
- Film grain: Animated hash-based noise, luminance-aware intensity
- Vignette: Radial falloff with configurable radius/softness
- Chromatic aberration: Distance-scaled RGB channel offset
- Time-of-day modulation (effects intensify at night)
- Render graph integration after tilt-shift, before EndMainPassPostProcessing
- `TonemappingConfig` with multiple modes (TonyMcMapface, AgX, ACES, Reinhard)
- `TaaConfig` for Temporal Anti-Aliasing (requires perspective camera)

### Performance Targets
| Component | Metric | Target |
|-----------|--------|--------|
| Building draw calls | < 50 | (from thousands) |
| Dynamic point lights | 5,000+ | at 60 FPS |
| Window entities | < 5,000 | (from 320,000) |
| Material state changes | 0 | during main pass |
| CPU cull time | < 0.1ms | per frame |
| Overdraw reduction | 40-60% | in dense views |
| Total CPU render time | < 1ms | per frame |

### Legacy Performance Considerations
- Use GPU instancing for repeated elements (windows, trees, vehicles) ✅
- LOD system for distant buildings
- Culling for off-screen elements ✅
- Batch similar materials ✅

### Bevy 0.15 Features to Leverage
- PBR materials with emissive for lights ✅
- Gizmos for debug visualization ✅
- Compute shaders for traffic CA / GPU culling ✅
- Post-processing pipeline for effects ✅

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
