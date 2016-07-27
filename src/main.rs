extern crate libc;
extern crate winapi;
extern crate kernel32;

use std::mem;

struct Assembler {
    bytes: Vec<u8>
}

impl Assembler {
    fn new() -> Assembler {
        Assembler {
            bytes: Vec::new()
        }
    }

    fn mov_eax_abs_32(&mut self, value: u32) {
        self.bytes.push(0xb8);
        self.bytes.push(((value >>  0) & 0xff) as u8);
        self.bytes.push(((value >>  8) & 0xff) as u8);
        self.bytes.push(((value >> 16) & 0xff) as u8);
        self.bytes.push(((value >> 24) & 0xff) as u8);
    }

    fn call_eax(&mut self) {
        self.bytes.push(0xff);
        self.bytes.push(0xd0);
    }

    fn ret(&mut self) {
        self.bytes.push(0xc3);
    }

    fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(unix)]
mod jitter {
    use libc;

    use std::{mem, ptr};

    pub struct Jitter {
        size: usize,
        mem: *mut u8
    }

    impl Jitter {
        pub fn new(bytes: &[u8]) -> Jitter {
            unsafe {
                const PAGE_SIZE: usize = 4096;
                let size = {
                    let mut size = 0;
                    while size < bytes.len() {
                        size += PAGE_SIZE;
                    }
                    size
                };

                // TODO: OS might not give writable + executable memory. Best to ask for writable, then make executable afterwards.
                let mem: *mut u8 = mem::transmute(libc::mmap(
                    ptr::null_mut(),
                    size,
                    libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
                    libc::MAP_ANON | libc::MAP_SHARED,
                    -1,
                    0));

                for (i, x) in bytes.iter().enumerate() {
                    *mem.offset(i as isize) = *x;
                }

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
        pub fn new(bytes: &[u8]) -> Jitter {
            unsafe {
                const PAGE_SIZE: usize = 4096;
                let size = {
                    let mut size = 0;
                    while size < bytes.len() {
                        size += PAGE_SIZE;
                    }
                    size
                };

                // TODO: OS might not give writable + executable memory. Best to ask for writable, then make executable afterwards.
                let mem: *mut u8 = mem::transmute(kernel32::VirtualAlloc(
                    ptr::null_mut(),
                    size as u32,
                    winapi::MEM_COMMIT,
                    winapi::PAGE_EXECUTE_READWRITE));

                for (i, x) in bytes.iter().enumerate() {
                    *mem.offset(i as isize) = *x;
                }

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

extern "stdcall" fn hi() -> i32 {
    5000
}

fn main() {
    let mut asm = Assembler::new();

    asm.mov_eax_abs_32(unsafe { mem::transmute(hi) });
    asm.call_eax();
    asm.ret();

    let mut jitter = Jitter::new(asm.bytes());
    println!("Result: {}", jitter.run());
}
