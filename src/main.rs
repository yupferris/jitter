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

    fn mov_eax_imm_32(&mut self, value: u32) {
        self.bytes.push(0xb8);
        self.bytes.push(((value >>  0) & 0xff) as u8);
        self.bytes.push(((value >>  8) & 0xff) as u8);
        self.bytes.push(((value >> 16) & 0xff) as u8);
        self.bytes.push(((value >> 24) & 0xff) as u8);
    }

    fn push_eax(&mut self) {
        self.bytes.push(0x50);
    }

    fn call_eax(&mut self) {
        self.bytes.push(0xff);
        self.bytes.push(0xd0);
    }

    fn push_ebp(&mut self) {
        self.bytes.push(0x55);
    }

    fn pop_ebp(&mut self) {
        self.bytes.push(0x5d);
    }

    fn mov_ebp_esp(&mut self) {
        self.bytes.push(0x89);
        self.bytes.push(0xe5);
    }

    fn mov_esp_ebp(&mut self) {
        self.bytes.push(0x89);
        self.bytes.push(0xec);
    }

    fn add_esp_imm_u8(&mut self, value: u8) {
        self.bytes.push(0x83);
        self.bytes.push(0xc4);
        self.bytes.push(value);
    }

    fn sub_esp_imm_u8(&mut self, value: u8) {
        self.bytes.push(0x83);
        self.bytes.push(0xec);
        self.bytes.push(value);
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

extern "stdcall" fn print_the_thing() {
    println!("Well it printed the thing");
}

extern "stdcall" fn do_nothing() {
    println!("do nothing");
}

extern "stdcall" fn do_nothing_with_one_arg(_x: i32) {
    println!("do nothing with one arg");
}

extern "stdcall" fn do_nothing_with_two_args(_x: i32, _y: i32) {
    println!("do nothing with two args");
}

extern "stdcall" fn do_nothing_with_three_args(_x: i32, _y: i32, _z: i32) {
    println!("do nothing with three args");
}

extern "stdcall" fn hi(x: i32, y: i32) -> i32 {
    let ret = x + y;
    println!("hi called, result is {}", ret);
    ret
}

fn main() {
    let mut asm = Assembler::new();

    asm.push_ebp();
    asm.mov_ebp_esp();

    asm.sub_esp_imm_u8(8);
    asm.mov_eax_imm_32(unsafe { mem::transmute(print_the_thing as extern "stdcall" fn()) });
    asm.call_eax();
    asm.add_esp_imm_u8(8);

    asm.sub_esp_imm_u8(8);
    asm.mov_eax_imm_32(unsafe { mem::transmute(do_nothing as extern "stdcall" fn()) });
    asm.call_eax();
    asm.add_esp_imm_u8(8);

    asm.sub_esp_imm_u8(4);
    asm.mov_eax_imm_32(5);
    asm.push_eax();
    asm.mov_eax_imm_32(unsafe { mem::transmute(do_nothing_with_one_arg as extern "stdcall" fn(i32)) });
    asm.call_eax();
    asm.add_esp_imm_u8(4);

    asm.mov_eax_imm_32(5);
    asm.push_eax();
    asm.mov_eax_imm_32(6);
    asm.push_eax();
    asm.mov_eax_imm_32(unsafe { mem::transmute(do_nothing_with_two_args as extern "stdcall" fn(i32, i32)) });
    asm.call_eax();

    asm.sub_esp_imm_u8(12);
    asm.mov_eax_imm_32(1);
    asm.push_eax();
    asm.mov_eax_imm_32(2);
    asm.push_eax();
    asm.mov_eax_imm_32(3);
    asm.push_eax();
    asm.mov_eax_imm_32(unsafe { mem::transmute(do_nothing_with_three_args as extern "stdcall" fn(i32, i32, i32)) });
    asm.call_eax();
    asm.add_esp_imm_u8(12);

    asm.mov_eax_imm_32(32);
    asm.push_eax();
    asm.mov_eax_imm_32(16);
    asm.push_eax();
    asm.mov_eax_imm_32(unsafe { mem::transmute(hi as extern "stdcall" fn(i32, i32) -> i32) });
    asm.call_eax();

    asm.mov_esp_ebp();
    asm.pop_ebp();
    asm.ret();

    let mut jitter = Jitter::new(asm.bytes());
    let res = jitter.run();
    println!("Result: {}", res);
}
