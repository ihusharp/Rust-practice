#![no_std]
#![no_main]

mod vga_buffer;

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello World");
    panic!("Some panic message");
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> !{
    println!("woops!!! {}", info);
    loop {}
}
