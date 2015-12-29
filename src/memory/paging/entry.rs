use memory::Frame;	// needed later

/// Represents an entry in the page table.
pub struct Entry(u64);

impl Entry {
	/// Determines whether the entry is currently being used.
	pub fn is_unused(&self) -> bool {
		self.0 == 0
	}

	/// Sets an entry to be unusued.
	pub fn set_unused(&mut self) {
		self.0 = 0;
	}

	/// Retrieves the entry's flags.
	pub fn flags(&self) -> EntryFlags {
		EntryFlags::from_bits_truncate(self.0)
	}

	/// Retrieves the `Frame` the entry points to.
	pub fn pointed_frame(&self) -> Option<Frame> {
		if self.flags().contains(PRESENT) {
			Some(Frame::containing_address(self.0 as usize & 0x000fffff_fffff000))
		} else {
			None
		}
	}

	/// Sets the entry's flags for a particular `Frame`.
	pub fn set(&mut self, frame: Frame, flags: EntryFlags) {
		assert!(frame.start_address() & !0x000fffff_fffff000 == 0);
		self.0 = (frame.start_address() as u64) | flags.bits();
	}
}

bitflags! {
	/// The flags used to indicate the entry's state and contents.
	flags EntryFlags: u64 {
		/// Indicates the page this entry maps to is in memory.
		const PRESENT 			= 1 << 0,
		/// Indicates that this page is writable.
		const WRITEABLE 		= 1 << 1,
		/// Indicates that user-mode processes can access this page.
		const USER_ACCESSIBLE	= 1 << 2,
		/// Indicates that writes should go directly to memory,
		/// bypassing the cache.
		const WRITE_THROUGH		= 1 << 3,
		/// Indicates that no cache (read or write) should be used for
		/// this page.
		const NO_CACHE 			= 1 << 4,
		/// Set by the CPU when the page is accessed.
		const ACCESSED 			= 1 << 5,
		/// Set by the CPU when a write to this page occurs.
		const DIRTY 			= 1 << 6,
		/// Indicates that this is a huge page.
		///
		/// * In P3 tables this means that the page is 1GiB.
		/// * In P2 tables this means that the page is 2MiB.
		/// * Must be 0 in P1 or P4 tables.
		const HUGE_PAGE 		= 1 << 7,
		/// Indicates that this page is not flushed from cache on
		/// address space switch.
		const GLOBAL 			= 1 << 8,
		/// Indicates that code cannot be executed from this page.
		const NO_EXECUTE 		= 1 << 63,
	}
}
