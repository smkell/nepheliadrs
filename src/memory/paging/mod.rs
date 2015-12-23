//! Defines the interface and types for memory paging. 
//!
//! # Guarantees 
//! 
//! 1. A page table owns all of its subtables.
//! 2. The recursively mapped P4 table is owned by a RecusivePageTable struct.

pub use self::entry::*;

use memory::PAGE_SIZE;
use memory::Frame;
use memory::FrameAllocator;

use self::table::{Table, Level4};

use core::ptr::Unique;

mod entry;
mod table;

const ENTRY_COUNT: usize = 512;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

/// Represents a page of virtual memory.
///
/// A page is a fixed size chunk of memory. 
pub struct Page {
	number: usize,
}

impl Page {
	fn containing_address(address: VirtualAddress) -> Page {
		assert!(address < 0x0000_8000_0000_0000 || address >= 0xffff_8000_0000_0000,
			"invalid address: 0x{:x}", address);
		Page { number: address / PAGE_SIZE }		
	}

	fn start_address(&self) -> usize {
		self.number * PAGE_SIZE
	}

	fn p4_index(&self) -> usize {
		(self.number >> 27) & 0o777
	}

	fn p3_index(&self) -> usize {
		(self.number >> 18) & 0o777
	}

	fn p2_index(&self) -> usize {
		(self.number >> 9) & 0o777
	}

	fn p1_index(&self) -> usize {
		(self.number >> 0) & 0o777
	}
}

/// Represents a handle for the recursive page table hierarchy.
pub struct RecusivePageTable {
	p4: Unique<Table<Level4>>,
}

impl RecusivePageTable {

	/// Constructs a handle for the recursive page table hierarchy.
	pub unsafe fn new() -> RecusivePageTable {
		RecusivePageTable {
			p4: Unique::new(table::P4),
		}
	}

	/// Retrieves a reference to the P4 table.
	fn p4(&self) -> &Table<Level4> {
		unsafe { self.p4.get() }
	}

	/// Retrieves a mutable reference to the P4 table.
	fn p4_mut(&mut self) -> &mut Table<Level4> {
		unsafe { self.p4.get_mut() }
	}

	pub fn translate(&self, virtual_address: VirtualAddress) -> Option<PhysicalAddress> {
		let offset = virtual_address % PAGE_SIZE;
		self.translate_page(Page::containing_address(virtual_address))
			.map(|frame| frame.number * PAGE_SIZE + offset)
	}

	fn translate_page(&self, page: Page) -> Option<Frame> {
		use self::entry::HUGE_PAGE;

		let p3 = self.p4().next_table(page.p4_index());

		let huge_page = || {
			p3.and_then(|p3| {
				let p3_entry = &p3[page.p3_index()];
				// 1 GiB page?
				if let Some(start_frame) = p3_entry.pointed_frame() {
					if p3_entry.flags().contains(HUGE_PAGE) {
						// address must be 1 GiB aligned 
						assert!(start_frame.number % (ENTRY_COUNT * ENTRY_COUNT) == 0);
						return Some(Frame {
							number: start_frame.number + page.p2_index() * ENTRY_COUNT +
									page.p1_index(),
						});
					}
				}
				if let Some(p2) = p3.next_table(page.p3_index()) {
					let p2_entry = &p2[page.p2_index()];
					// 2 MiB page?
					if let Some(start_frame) = p2_entry.pointed_frame() {
						// address must be 2MiB aligned
						assert!(start_frame.number % ENTRY_COUNT == 0);
						return Some(Frame {
							number: start_frame.number + page.p1_index()
						});
					}
				}
				None
			})
		};

		p3.and_then(|p3| p3.next_table(page.p3_index()))
		  .and_then(|p2| p2.next_table(page.p2_index()))
		  .and_then(|p1| p1[page.p1_index()].pointed_frame())
		  .or_else(huge_page)
	}

	/// Maps a virtual memory page to a particular physical memory frame.
	pub fn map_to<A>(&mut self, page: Page, frame: Frame, flags: EntryFlags, allocator: &mut A)
		where A : FrameAllocator
	{
		// Create the child tables.
		let mut p3 = self.p4_mut().next_table_create(page.p4_index(), allocator);
		let mut p2 = p3.next_table_create(page.p3_index(), allocator);
		let mut p1 = p2.next_table_create(page.p2_index(), allocator);

		assert!(p1[page.p1_index()].is_unused());	// The p1 entry should not be used by anything yet
		p1[page.p1_index()].set(frame, flags | PRESENT);	// Indicate that  
	}

	/// Maps a virtual memory page to a physical memory frame.
	///
	/// This function will map a virtual memory page to an available physical memory frame. If no 
	/// more frames are available it will panic.
	/// 
	/// # Parameters
	/// 
	/// * page - The page to map to a frame.
	/// * flags - The entry flags to apply to the new page entry.
	/// * allocator - The allocator to use for allocating the fame to the page.
	/// 
	/// # Panics 
	/// 
	/// * If there no available physical memory frames to allocate.
	pub fn map<A>(&mut self, page: Page, flags: EntryFlags, allocator: &mut A)
		where A : FrameAllocator
	{
		let frame =allocator.allocate_frame().expect("out of memory");
		self.map_to(page, frame, flags, allocator);
	}

	/// Remaps a particular frame to the associated page.
	pub fn identity_map<A>(&mut self,
						   frame: Frame,
						   flags: EntryFlags,
						   allocator: &mut A)
		where A : FrameAllocator
	{
		let page = Page::containing_address(frame.start_address());
		self.map_to(page, frame, flags, allocator)
	}

	/// Unmaps a page from the associated frame.
	///
	/// # Parameters
	///
	/// * page - The page to unmap.
	/// * allocator - The allocator to deallocate the mapping from.
	///
	/// # Panics 
	/// 
	/// * If the page is not currently mapped.
	/// * If the page is a huge page.
	fn unmap<A>(&mut self, page: Page, allocator: &mut A) 
		where A : FrameAllocator
	{
		// Assert that the page is mapped 
		assert!(self.translate(page.start_address()).is_some());

		let p1 = self.p4_mut()
					 .next_table_mut(page.p4_index())
					 .and_then(|p3| p3.next_table_mut(page.p3_index()))
					 .and_then(|p2| p2.next_table_mut(page.p2_index()))
					 .expect("mapping code does not support huge pages");

		let frame = p1[page.p1_index()].pointed_frame().unwrap();
		p1[page.p1_index()].set_unused();
		unsafe {
			::x86::tlb::flush(page.start_address());
		}
		// TODO free p(1,2,3) table if empty
		//allocator.deallocate_frame(frame);
	}
}

pub fn test_paging<A>(allocator: &mut A)
	where A : FrameAllocator
{
	let mut page_table = unsafe { RecusivePageTable::new() };

	// address 0 is mapped 
	println!("Some = {:?}", page_table.translate(0));
	// second P1 entry
	println!("Some = {:?}", page_table.translate(4096));
	// second P2 entry
	println!("Some = {:?}", page_table.translate(512 * 4096));
	// 300th P2 entry
	println!("Some = {:?}", page_table.translate(300 * 512 * 4096));
	// second P3 entry 
	println!("None = {:?}", page_table.translate(512 * 512 * 4096));
	// last mapped byte
	println!("Some = {:?}", page_table.translate(512 * 512 * 4096 - 1));

	let addr = 42 * 512 * 512 * 4096;	// 42th P3 entry
	let page = Page::containing_address(addr);
	let frame = allocator.allocate_frame().expect("no more frames");
	println!("None = {:?}, map to {:?}", page_table.translate(addr), frame);
	page_table.map_to(page, frame, EntryFlags::empty(), allocator);
	println!("Some = {:?}", page_table.translate(addr));
	println!("next free frame: {:?}", allocator.allocate_frame());

	println!("{:#x}",
             unsafe { *(Page::containing_address(addr).start_address() as *const u64) });
	page_table.unmap(Page::containing_address(addr), allocator);
	println!("None = {:?}", page_table.translate(addr));
}