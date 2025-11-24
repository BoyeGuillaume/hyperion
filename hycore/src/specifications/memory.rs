use bitflags::bitflags;
use hyinstr::{modules::operand::Operand, types::Typeref};

bitflags! {
    /// Flags that describe the access properties of a memory region.
    #[derive(Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub struct MemoryAccessFlags: u8 {
        /// The provided memory region might be read by the current function
        const READ = 1 << 0;

        /// The provided memory region might be written to by the current function
        const WRITE = 1 << 1;

        /// Provided memory region might be externally read
        ///
        /// External reads provide a strong guarantee that the sequence of writes performed by
        /// the current function should remain consistent. This can be used when interacting with
        /// memory-mapped I/O regions or shared memory on boundary between this framework and
        /// other systems.
        ///
        /// This should be used to provide a consistent behavior but it shouldn't be used internally
        /// as it constraints the explorable space significantly.
        const EXTERNAL_READ = 1 << 2;

        /// Provided memory region might be externally written to
        ///
        /// External writes indicate that the memory region might be modified by external agents
        /// outside the control of the current function. This is useful for modeling interactions
        /// with hardware devices or shared memory where the content can change independently.
        ///
        /// This flag should be used sparingly as it can complicate reasoning about memory state.
        const EXTERNAL_WRITE = 1 << 3;
        const EXTERNAL = Self::EXTERNAL_READ.bits() | Self::EXTERNAL_WRITE.bits();
    }
}

impl MemoryAccessFlags {
    /// Check if the memory region is accessed in any way (read or write)
    pub fn is_accessed(&self) -> bool {
        self.contains(MemoryAccessFlags::READ) || self.contains(MemoryAccessFlags::WRITE)
    }

    /// Check if the memory region has external side effects (external read or write)
    pub fn has_external_side_effects(&self) -> bool {
        self.intersects(MemoryAccessFlags::EXTERNAL)
    }
}

bitflags! {
    /// Flags that describe the aliasing properties between two memory regions.
    ///
    /// These flags help in understanding how different memory regions might overlap or interact.
    #[derive(Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub struct MemoryAliasingFlags: u8 {
        /// The two memory regions might alias (i.e., overlap in memory)
        ///
        /// If this flag is not set, it indicates that the two memory regions are guaranteed
        /// to be disjoint and do not overlap in memory.
        const ALIASING = 1 << 0;

        /// When both regions are accessed, they are accessed in a consistent order
        ///
        /// This flag requires that if both memory regions have [`MemoryAccessFlags::has_external_side_effects`]
        /// set, the accesses to these regions (loads/stores depending on the access type) must occur in a
        /// consistent order.
        const ORDERED_ACCESS = 1 << 1;
    }
}

#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// Starting address of the memory region (inclusive)
    pub start: Operand,

    /// Size of the memory region in bytes (if known)
    pub size: Option<Operand>,

    /// Access flags for the memory region
    pub access_flags: MemoryAccessFlags,

    /// Type of data stored in the memory region (if known)
    pub ty: Option<Typeref>,

    /// Alignment in bytes (required for correctness of certain operations)
    pub alignment: u32,

    /// Memory space identifier (if applicable)
    ///
    /// Some architectures support multiple memory spaces (e.g., data, code, stack). This
    /// field also helps distinguish between host and device memory spaces in GPU contexts.
    pub memory_space: Option<u32>,

    /// Additional constraints (properties that MUST hold for this memory region)
    ///
    /// For example, this can be used to specify that this memory region is such that
    /// %start + %index % 8 == 0 (i.e., the start address plus some index is aligned to 8 bytes)
    pub additional_aliasing_constraints: Vec<Operand>,
}

#[derive(Debug, Clone)]
pub struct MemoryAliasing {
    /// List of memory regions involved in the aliasing analysis
    pub regions: Vec<MemoryRegion>,

    /// Matrix describing the aliasing relationships between memory regions
    pub exclusions: nalgebra::DMatrix<MemoryAliasingFlags>,
}
