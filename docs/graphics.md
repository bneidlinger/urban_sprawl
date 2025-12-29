🎨 Graphics & Visual Realism
Improve the aesthetic from a basic grid to a living, breathing city.

[ ] Implement Bitmasking (Auto-Tiling)

[ ] Create logic to detect adjacent road/zone tiles.

[ ] Add specific sprites for T-junctions, corners, and dead-ends.

[ ] Dynamic Lighting System

[ ] Implement a global Day/Night cycle timer.

[ ] Create a "Light Map" overlay surface using multiplication blending.

[ ] Add point-light sources for streetlights and "window glow" in high-density areas.

[ ] Visual Depth & Shadows

[ ] Add drop-shadows to buildings (simple alpha-blended offsets).

[ ] Implement a "Cloud Shadow" layer that drifts across the map using Perlin noise.

[ ] Weather & Atmospheric Effects

[ ] Add a particle system for rain/snow.

[ ] Implement screen-space shaders (via ModernGL or similar) for Bloom and Tilt-shift effects.

🏗️ Procedural Generation & Assets
Move away from primitive shapes toward varied, interesting structures.

[ ] Sprite-Based Rendering

[ ] Replace pygame.draw.rect calls with screen.blit for textured assets.

[ ] Create a library of 32x32 textures for different density levels.

[ ] Layered Building Construction

[ ] Implement "Construction States" (dirt -> scaffolding -> finished building).

[ ] Randomize building heights and facade colors within the same zone type.

[ ] Animation

[ ] Add subtle animations for "Active" zones (e.g., smoke from industrial chimneys, flickering lights).

🧠 Simulation Logic & Urban Mechanics
Increase the realism of how the city expands and functions.

[ ] Topography & Terrain

[ ] Generate a Perlin noise height map for the world.

[ ] Add water bodies (rivers/lakes) and forests.

[ ] Implement building costs based on slope/terrain difficulty.

[ ] Desirability Heatmaps

[ ] Pollution: Industrial zones lower the desirability of nearby residential.

[ ] Value: Proximity to parks and water increases land value and density.

[ ] Noise: High-traffic roads decrease residential quality of life.

[ ] Agent-Based Traffic

[ ] Implement basic A* pathfinding for "Commuter" entities.

[ ] Visualize traffic density on roads to identify bottlenecks.

⚙️ Technical Scaling
Ensure the engine can handle larger maps and more complex logic.

[ ] Optimization

[ ] Implement "Dirty Rect" rendering to only update changed tiles.

[ ] Move simulation logic to a fixed timestep independent of frame rate.

[ ] Engine Exploration

[ ] Research ModernGL integration for hardware-accelerated 2D effects.

[ ] Evaluate a potential port to Panda3D or Godot if 3D realism becomes the primary goal.

🚀 Quick Wins (Prioritized)
Texture Mapping: Swap colors for pixel-art tiles.

Shadows: Add 50% alpha black rectangles under buildings for instant depth.

Terrain: Use noise to generate a "World" so the city isn't in a void.