#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(os_rust::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;
use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};
use os_rust::{println, task::{Task, keyboard, executor::Executor}};
use bootloader::{BootInfo, entry_point};

entry_point!(kernel_main);

fn kernel_main(bootinfo: &'static BootInfo) -> ! {
    use os_rust::memory;
    use os_rust::allocator;
    use x86_64::{structures::paging::Page, VirtAddr};

    println!("Hello World{}", "!");
    os_rust::init();

    let phys_mem_offset = VirtAddr::new(bootinfo.physical_memory_offset);
    let mut mapper = unsafe {
        memory::init(phys_mem_offset)
    };
    let mut frame_allocator = unsafe {
        memory::BootInfoFrameAllocator::init(&bootinfo.memory_map)
    };
    // map an unused page
    let page = Page::containing_address(VirtAddr::new(0xdeadbeaf000));
    memory::create_example_mapping(&mut mapper, page, &mut frame_allocator);

    // write to the mapped page
    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e) };

    // for heap
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    // allocate a number on the heap
    let heap_value = Box::new(41);
    println!("heap_value at {:p}", heap_value);

    // create a dynamically sized vector
    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    println!("vec at {:p}", vec.as_slice());

    // create a recursive counted vector -> will be freed when count reaches 0
    let reference_counted = Rc::new(vec!(1, 2, 3));
    let reference_counted_clone = reference_counted.clone();
    println!("cur reference cnt is {}", Rc::strong_count(&reference_counted_clone));
    core::mem::drop(reference_counted);
    println!("after reference cnt is {}", Rc::strong_count(&reference_counted_clone));

    // implement a simple executor for async tasks
    let mut executor = Executor::new();
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();

    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    os_rust::hlt_loop();
}

async fn async_num() -> u32 {
    54
}

async fn example_task() {
    let num = async_num().await;
    println!("async num: {}", num);
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    os_rust::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    os_rust::test_panic_handler(info)
}