use self::paging::PhysicalAddress;
pub use self::area_frame_allocator::AreaFrameAllocator;
pub use self::paging::test_paging;

pub mod paging;
pub mod area_frame_allocator;

/// The size, in bytes, of a virtual memory page.
///
/// This is also the size of alocated physical frames.
pub const PAGE_SIZE: usize = 4096;

/// Represents a physical memory frame.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
	number: usize,
}

impl Frame {
	/// Retrieves the `Frame` which contains the given `address`.
	fn containing_address(address: usize) -> Frame {
		Frame{ number: address / PAGE_SIZE }
	}

	/// Retrieves the first physical address in the `Frame`.
	fn start_address(&self) -> PhysicalAddress {
		self.number * PAGE_SIZE
	}
}

/// The trait defining the interface for an object which allocates `Frame`s.
pub trait FrameAllocator {
    /// Allocates the next frame.
    ///
    /// # Returns
    ///
    /// The allocated `Frame`, or `None` if there are no frames available to
    /// alocate.
    fn allocate_frame(&mut self) -> Option<Frame>;

    /// Deallocates the given `Frame`.
	///
	/// # Parameters
	///
	/// * `frame` - The frame to deallocate.
    fn deallocate_frame(&mut self, frame: Frame);
}
