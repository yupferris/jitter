// TODO: Windows support with VirtualAlloc/VirtualFree
//  https://msdn.microsoft.com/en-us/library/windows/desktop/aa366887(v=vs.85).aspx

extern crate libc;

use std::mem;
use std::ptr;

const PAGE_SIZE: usize = 4096;

struct Jitter {
    size: usize,
    mem: *mut u8
}

impl Jitter {
    fn new(num_pages: usize) -> Jitter {
        unsafe {
            let size = num_pages * PAGE_SIZE;

            let mem: *mut u8 = mem::transmute(libc::mmap(
                ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
                libc::MAP_ANON | libc::MAP_SHARED,
                -1,
                0));

            *mem.offset(0x00) = 0xb8;
            *mem.offset(0x01) = 0x2a;
            *mem.offset(0x02) = 0x00;
            *mem.offset(0x03) = 0x00;
            *mem.offset(0x04) = 0x00;
            *mem.offset(0x05) = 0xc3;

            Jitter {
                size: size,
                mem: mem
            }
        }
    }

    fn run(&mut self) -> i32 {
        unsafe {
            let fn_ptr: extern fn() -> i32 = mem::transmute(self.mem);
            fn_ptr()
        }
    }
}

impl Drop for Jitter {
    fn drop(&mut self) {
        unsafe { libc::munmap(self.mem as *mut _, self.size); }
    }
}

fn main() {
    let mut jitter = Jitter::new(1);
    println!("Result: {}", jitter.run());
}
