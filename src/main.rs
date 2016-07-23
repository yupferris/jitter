// TODO: Windows support with VirtualAlloc/VirtualFree
//  https://msdn.microsoft.com/en-us/library/windows/desktop/aa366887(v=vs.85).aspx

extern crate libc;
extern crate winapi;
extern crate kernel32;

#[cfg(unix)]
mod jitter {
    use libc;

    use std::{mem, ptr};

    pub struct Jitter {
        size: usize,
        mem: *mut u8
    }

    impl Jitter {
        pub fn new(num_pages: usize) -> Jitter {
            unsafe {
                const PAGE_SIZE: usize = 4096;
                let size = num_pages * PAGE_SIZE;

                // TODO: OS might not give writable + executable memory. Best to ask for writable, then make executable afterwards.
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

        pub fn run(&mut self) -> i32 {
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
}

#[cfg(windows)]
mod jitter {
    use winapi;
    use kernel32;

    use std::{mem, ptr};

    pub struct Jitter {
        mem: *mut u8
    }

    impl Jitter {
        pub fn new(num_pages: usize) -> Jitter {
            unsafe {
                const PAGE_SIZE: usize = 4096;
                let size = num_pages * PAGE_SIZE;

                // TODO: OS might not give writable + executable memory. Best to ask for writable, then make executable afterwards.
                let mem: *mut u8 = mem::transmute(kernel32::VirtualAlloc(
                    ptr::null_mut(),
                    size as u32,
                    winapi::MEM_COMMIT,
                    winapi::PAGE_EXECUTE_READWRITE));

                *mem.offset(0x00) = 0xb8;
                *mem.offset(0x01) = 0x2a;
                *mem.offset(0x02) = 0x00;
                *mem.offset(0x03) = 0x00;
                *mem.offset(0x04) = 0x00;
                *mem.offset(0x05) = 0xc3;

                Jitter {
                    mem: mem
                }
            }
        }

        pub fn run(&mut self) -> i32 {
            unsafe {
                let fn_ptr: extern fn() -> i32 = mem::transmute(self.mem);
                fn_ptr()
            }
        }
    }

    impl Drop for Jitter {
        fn drop(&mut self) {
            unsafe { kernel32::VirtualFree(self.mem as *mut _, 0, winapi::MEM_RELEASE); }
        }
    }
}

use jitter::*;

fn main() {
    let mut jitter = Jitter::new(1);
    println!("Result: {}", jitter.run());
}
