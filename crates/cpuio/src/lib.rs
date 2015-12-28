//! CPU-level input/output instructions, including `inb`, `outb`, etc., and 
//! a high level Rust wrapper.

#![feature(asm)]
#![feature(const_fn)]
#![no_std]

use core::marker::PhantomData;

#[cfg(any(target_arch="x86", target_arch="x86_64"))]
pub use x86::{inb, outb, inw, outw, inl, outl};

#[cfg(any(target_arch="x86", target_arch="x86_64"))]
mod x86;

/// This trait is defined for any type which can be read or written over a
/// port.
///
/// The processor suppors I/O with `u8`, `u16`, and `u32`. The functions in
/// this trait are all unsafe because they can write to arbitrary ports, which
/// is an inherently unsafe operation.
pub trait InOut {
    /// Read a value from the specified port.
    unsafe fn port_in(port: u16) -> Self;

    /// Write a value to the specified port.
    unsafe fn port_out(port: u16, value: Self);
}

impl InOut for u8 {
    unsafe fn port_in(port: u16) -> u8       { inb(port) }
    unsafe fn port_out(port: u16, value: u8) { outb(port, value) }
}

impl InOut for u16 {
    unsafe fn port_in(port: u16) -> u16       { inw(port) }
    unsafe fn port_out(port: u16, value: u16) { outw(port, value) }
}

impl InOut for u32 {
    unsafe fn port_in(port: u16) -> u32       { inl(port) }
    unsafe fn port_out(port: u16, value: u32) { outl(port, value) }
}

pub struct Port<T: InOut> {
    port: u16,
    phantom: PhantomData<T>,
}

impl<T: InOut> Port<T> {
    /// Constructs a new I/O port.
    pub const unsafe fn new(port: u16) -> Port<T> {
        Port { port: port, phantom: PhantomData }
    }

    /// Read data from the port.
    pub unsafe fn read(&mut self) -> T {
        T::port_in(self.port)
    }

    /// Write data to the port.
    pub unsafe fn write(&mut self, value: T) {
        T::port_out(self.port, value);
    }
}
