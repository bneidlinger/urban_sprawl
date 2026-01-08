//! Object data management for GPU culling.
//!
//! Manages the buffer of cullable objects sent to the GPU compute shader.

use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};

/// Per-object data for GPU culling (32 bytes).
///
/// Stored in a GPU buffer and processed by the culling compute shader.
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
#[repr(C)]
pub struct ObjectData {
    /// Bounding sphere: xyz = center (world space), w = radius
    pub bounding_sphere: [f32; 4],
    /// Entity bits for identification (split into two u32 to avoid padding issues)
    pub entity_bits_low: u32,
    pub entity_bits_high: u32,
    /// Mesh ID for draw call batching
    pub mesh_id: u32,
    /// Flags: bit 0 = visible, bits 1-31 reserved
    pub flags: u32,
}

impl ObjectData {
    /// Create new object data with a bounding sphere.
    pub fn new(center: Vec3, radius: f32, entity: Entity, mesh_id: u32) -> Self {
        let bits = entity.to_bits();
        Self {
            bounding_sphere: [center.x, center.y, center.z, radius],
            entity_bits_low: bits as u32,
            entity_bits_high: (bits >> 32) as u32,
            mesh_id,
            flags: 1, // Visible by default
        }
    }

    /// Get the bounding sphere center.
    pub fn center(&self) -> Vec3 {
        Vec3::new(
            self.bounding_sphere[0],
            self.bounding_sphere[1],
            self.bounding_sphere[2],
        )
    }

    /// Get the bounding sphere radius.
    pub fn radius(&self) -> f32 {
        self.bounding_sphere[3]
    }

    /// Check if the object is marked as visible.
    pub fn is_visible(&self) -> bool {
        (self.flags & 1) != 0
    }

    /// Set the visibility flag.
    pub fn set_visible(&mut self, visible: bool) {
        if visible {
            self.flags |= 1;
        } else {
            self.flags &= !1;
        }
    }
}

/// Buffer containing all object data for GPU culling.
#[derive(Resource, Default)]
pub struct ObjectDataBuffer {
    /// CPU-side object data
    objects: Vec<ObjectData>,
    /// Dirty flag - set when buffer needs GPU upload
    dirty: bool,
}

impl ObjectDataBuffer {
    /// Create a new buffer with capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            objects: Vec::with_capacity(capacity),
            dirty: false,
        }
    }

    /// Add an object to the buffer.
    pub fn push(&mut self, object: ObjectData) {
        self.objects.push(object);
    }

    /// Clear all objects.
    pub fn clear(&mut self) {
        self.objects.clear();
    }

    /// Get the number of objects.
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Check if buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Get all objects as a slice.
    pub fn objects(&self) -> &[ObjectData] {
        &self.objects
    }

    /// Get mutable access to objects.
    pub fn objects_mut(&mut self) -> &mut [ObjectData] {
        &mut self.objects
    }

    /// Get raw bytes for GPU upload.
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.objects)
    }

    /// Mark buffer as needing GPU upload.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Check if buffer needs GPU upload.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Clear dirty flag after GPU upload.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Get object by index.
    pub fn get(&self, index: usize) -> Option<&ObjectData> {
        self.objects.get(index)
    }

    /// Get mutable object by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut ObjectData> {
        self.objects.get_mut(index)
    }

    /// Update visibility flags from GPU readback.
    pub fn update_visibility(&mut self, visibility_flags: &[u32]) {
        for (object, &flag) in self.objects.iter_mut().zip(visibility_flags.iter()) {
            object.set_visible(flag != 0);
        }
    }
}

/// Component marking an entity as cullable by the GPU culling system.
#[derive(Component)]
pub struct GpuCullable {
    /// Local-space bounding sphere radius
    pub local_radius: f32,
    /// Mesh ID for batching (entities with same mesh_id can be batched)
    pub mesh_id: u32,
    /// Current visibility state (updated by culling system)
    pub visible: bool,
    /// Index in the object data buffer (for GPU readback mapping)
    pub buffer_index: Option<usize>,
}

impl GpuCullable {
    /// Create a new cullable component with a bounding radius.
    pub fn new(radius: f32) -> Self {
        Self {
            local_radius: radius,
            mesh_id: 0,
            visible: true,
            buffer_index: None,
        }
    }

    /// Create with a specific mesh ID for batching.
    pub fn with_mesh_id(mut self, mesh_id: u32) -> Self {
        self.mesh_id = mesh_id;
        self
    }
}

impl Default for GpuCullable {
    fn default() -> Self {
        Self::new(1.0)
    }
}

/// Statistics about culling performance.
#[derive(Resource, Default, Debug)]
pub struct CullStats {
    /// Total number of cullable objects
    pub total_objects: usize,
    /// Number of visible objects after culling
    pub visible_objects: usize,
    /// Number of culled objects
    pub culled_objects: usize,
    /// Cull ratio (0.0 = nothing culled, 1.0 = everything culled)
    pub cull_ratio: f32,
}

impl CullStats {
    /// Get a human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            "Culling: {}/{} visible ({:.1}% culled)",
            self.visible_objects,
            self.total_objects,
            self.cull_ratio * 100.0
        )
    }
}

/// Indirect draw command for GPU-driven rendering.
///
/// Compatible with Vulkan/DX12 DrawIndexedIndirect command.
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
#[repr(C)]
pub struct DrawIndexedIndirect {
    /// Number of indices to draw
    pub index_count: u32,
    /// Number of instances (0 = culled, 1+ = visible)
    pub instance_count: u32,
    /// First index in the index buffer
    pub first_index: u32,
    /// Vertex offset added to each index
    pub vertex_offset: i32,
    /// First instance ID
    pub first_instance: u32,
}

impl DrawIndexedIndirect {
    /// Create a new draw command.
    pub fn new(index_count: u32, first_index: u32, vertex_offset: i32, first_instance: u32) -> Self {
        Self {
            index_count,
            instance_count: 1, // Visible by default
            first_index,
            vertex_offset,
            first_instance,
        }
    }

    /// Mark as culled (instance_count = 0).
    pub fn cull(&mut self) {
        self.instance_count = 0;
    }

    /// Mark as visible (instance_count = 1).
    pub fn show(&mut self) {
        self.instance_count = 1;
    }

    /// Check if this draw command is culled.
    pub fn is_culled(&self) -> bool {
        self.instance_count == 0
    }
}

/// Buffer of indirect draw commands for GPU-driven rendering.
#[derive(Resource, Default)]
pub struct IndirectDrawBuffer {
    /// Draw commands for each mesh batch
    commands: Vec<DrawIndexedIndirect>,
    /// Dirty flag for GPU upload
    dirty: bool,
}

impl IndirectDrawBuffer {
    /// Add a draw command.
    pub fn push(&mut self, command: DrawIndexedIndirect) {
        self.commands.push(command);
        self.dirty = true;
    }

    /// Clear all commands.
    pub fn clear(&mut self) {
        self.commands.clear();
    }

    /// Get the number of commands.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Get all commands as a slice.
    pub fn commands(&self) -> &[DrawIndexedIndirect] {
        &self.commands
    }

    /// Get mutable access to commands.
    pub fn commands_mut(&mut self) -> &mut [DrawIndexedIndirect] {
        &mut self.commands
    }

    /// Get raw bytes for GPU upload.
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.commands)
    }

    /// Count visible (non-culled) draw commands.
    pub fn visible_count(&self) -> usize {
        self.commands.iter().filter(|c| !c.is_culled()).count()
    }
}
