#![feature(naked_functions, asm)]
#![feature(alloc_error_handler)]
#![no_std]
#![no_main]

mod mmio;

use linked_list_allocator::LockedHeap;

use core::mem::MaybeUninit;
const KERNEL_HEAP_SIZE: usize = 64 * 1024;
static mut HEAP_SPACE: MaybeUninit<[u8; KERNEL_HEAP_SIZE]> = MaybeUninit::uninit();

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

unsafe fn init_heap() {
    ALLOCATOR.lock().init(
        HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE
    )
}

#[allow(unused)]
#[cfg_attr(not(test), alloc_error_handler)]
fn oom(layout: core::alloc::Layout) -> ! {
    println!("!! Out of memory: {:?}", layout);
    println!("!! Kernel: Test failed due to out of memory");
    sbi::shutdown()
}

use riscv::register::stvec::{self, TrapMode};

pub extern "C" fn rust_main(hartid: usize, dtb_pa: usize) -> ! {
    extern "C" { fn sbss(); fn ebss();/* fn ekernel(); */}
    unsafe { r0::zero_bss(&mut sbss as *mut _ as *mut u64, &mut ebss as *mut _ as *mut u64) };
    println!("<< Kernel: Hart id = {}, DTB physical address = {:#x}", hartid, dtb_pa);
    unsafe { init_heap() };
    unsafe { stvec::write(start_trap as usize, TrapMode::Direct) };
    
    // unsafe { dump_dtb(dtb_pa) };

    
    let header = unsafe {
        &mut *(0x10001000 as *mut mmio::VirtIoHeader)
    };
    println!("Magic = {:#x}, Version = {}", header.magic.read(), header.version.read());

    println!("<< Kernel: test SUCCESS, shutdown");
    sbi::shutdown()
}

// unsafe fn dump_dtb(dtb_pa: usize) {
//     const DEVICE_TREE_MAGIC: u32 = 0xD00DFEED;
//     #[repr(C)]
//     struct DtbHeader { magic: u32, size: u32 }
//     let header = &*(dtb_pa as *const DtbHeader);
//     let magic = u32::from_be(header.magic);
//     if magic == DEVICE_TREE_MAGIC {
//         let size = u32::from_be(header.size);
//         // 拷贝数据，加载并遍历
//         let data = core::slice::from_raw_parts(dtb_pa as *const u8, size as usize);
//         if let Ok(dt) = device_tree::DeviceTree::load(data) {
//             println!("{:?}", dt);
//         }
//     }
// }

pub extern "C" fn rust_trap_exception() {

}

use core::panic::PanicInfo;

#[cfg_attr(not(test), panic_handler)]
#[allow(unused)]
fn panic(info: &PanicInfo) -> ! {
    println!("!! Kernel: {}", info);
    println!("!! Kernel: Test failed due to panic");
    sbi::shutdown()
}

const BOOT_STACK_SIZE: usize = 4096 * 4 * 8;

static mut BOOT_STACK: [u8; BOOT_STACK_SIZE] = [0; BOOT_STACK_SIZE];

#[naked]
#[link_section = ".text.entry"] 
#[export_name = "_start"]
unsafe extern "C" fn entry() -> ! {
    asm!("
    # 1. set sp
    # sp = bootstack + (hartid + 1) * 0x10000
    add     t0, a0, 1
    slli    t0, t0, 14
1:  auipc   sp, %pcrel_hi({boot_stack})
    addi    sp, sp, %pcrel_lo(1b)
    add     sp, sp, t0
    # 2. jump to rust_main (absolute address)
1:  auipc   t0, %pcrel_hi({rust_main})
    addi    t0, t0, %pcrel_lo(1b)
    jr      t0
    ", 
    boot_stack = sym BOOT_STACK, 
    rust_main = sym rust_main,
    options(noreturn))
}

#[cfg(target_pointer_width = "64")]
macro_rules! define_store_load {
    () => {
        ".altmacro
        .macro STORE reg, offset
            sd  \\reg, \\offset* {REGBYTES} (sp)
        .endm
        .macro LOAD reg, offset
            ld  \\reg, \\offset* {REGBYTES} (sp)
        .endm"
    };
}

#[cfg(target_pointer_width = "32")]
macro_rules! define_store_load {
    () => {
        ".altmacro
        .macro STORE reg, offset
            sw  \\reg, \\offset* {REGBYTES} (sp)
        .endm
        .macro LOAD reg, offset
            lw  \\reg, \\offset* {REGBYTES} (sp)
        .endm"
    };
}

#[naked]
#[link_section = ".text"]
unsafe extern "C" fn start_trap() {
    asm!(define_store_load!(), "
    .p2align 2
    addi    sp, sp, -16 * {REGBYTES}
    STORE   ra, 0
    STORE   t0, 1
    STORE   t1, 2
    STORE   t2, 3
    STORE   t3, 4
    STORE   t4, 5
    STORE   t5, 6
    STORE   t6, 7
    STORE   a0, 8
    STORE   a1, 9
    STORE   a2, 10
    STORE   a3, 11
    STORE   a4, 12
    STORE   a5, 13
    STORE   a6, 14
    STORE   a7, 15
    mv      a0, sp
    call    {rust_trap_exception}
    LOAD    ra, 0
    LOAD    t0, 1
    LOAD    t1, 2
    LOAD    t2, 3
    LOAD    t3, 4
    LOAD    t4, 5
    LOAD    t5, 6
    LOAD    t6, 7
    LOAD    a0, 8
    LOAD    a1, 9
    LOAD    a2, 10
    LOAD    a3, 11
    LOAD    a4, 12
    LOAD    a5, 13
    LOAD    a6, 14
    LOAD    a7, 15
    addi    sp, sp, 16 * {REGBYTES}
    sret
    ",
    REGBYTES = const core::mem::size_of::<usize>(),
    rust_trap_exception = sym rust_trap_exception,
    options(noreturn))
}

#[macro_use]
mod console {
    use super::sbi::*;
    use core::fmt::{self, Write};
    use spin::Mutex;

    struct Stdout;

    impl Write for Stdout {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let mut buffer = [0u8; 4];
            for c in s.chars() {
                for code_point in c.encode_utf8(&mut buffer).as_bytes().iter() {
                    console_putchar(*code_point as usize);
                }
            }
            Ok(())
        }
    }

    #[allow(unused)]
    pub fn print(args: fmt::Arguments) {
        STDOUT.lock().write_fmt(args).unwrap();
    }

    lazy_static::lazy_static! {
        static ref STDOUT: Mutex<Stdout> = Mutex::new(Stdout);
    }

    #[macro_export]
    macro_rules! print {
        ($fmt: literal $(, $($arg: tt)+)?) => {
            $crate::console::print(format_args!($fmt $(, $($arg)+)?));
        }
    }

    #[macro_export]
    macro_rules! println {
        ($fmt: literal $(, $($arg: tt)+)?) => {
            $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
        }
    }
}
#[allow(unused)]
mod sbi {
    pub const EXTENSION_BASE: usize = 0x10;
    pub const EXTENSION_TIMER: usize = 0x54494D45;
    pub const EXTENSION_IPI: usize = 0x735049;
    pub const EXTENSION_RFENCE: usize = 0x52464E43;
    pub const EXTENSION_HSM: usize = 0x48534D;
    pub const EXTENSION_SRST: usize = 0x53525354;

    const FUNCTION_BASE_GET_SPEC_VERSION: usize = 0x0;
    const FUNCTION_BASE_GET_SBI_IMPL_ID: usize = 0x1;
    const FUNCTION_BASE_GET_SBI_IMPL_VERSION: usize = 0x2;
    const FUNCTION_BASE_PROBE_EXTENSION: usize = 0x3;
    const FUNCTION_BASE_GET_MVENDORID: usize = 0x4;
    const FUNCTION_BASE_GET_MARCHID: usize = 0x5;
    const FUNCTION_BASE_GET_MIMPID: usize = 0x6;

    #[repr(C)]
    pub struct SbiRet {
        /// Error number
        pub error: usize,
        /// Result value
        pub value: usize,
    }

    #[inline(always)]
    fn sbi_call(extension: usize, function: usize, arg0: usize, arg1: usize, arg2: usize) -> SbiRet {
        let (error, value);
        match () {
            #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
            () => unsafe { asm!(
                "ecall", 
                in("a0") arg0, in("a1") arg1, in("a2") arg2,
                in("a6") function, in("a7") extension,
                lateout("a0") error, lateout("a1") value,
            ) },
            #[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
            () => {
                drop((extension, function, arg0, arg1, arg2));
                unimplemented!("not RISC-V instruction set architecture")
            }
        };
        SbiRet { error, value }
    }

    #[inline]
    pub fn get_spec_version() -> usize {
        sbi_call(EXTENSION_BASE, FUNCTION_BASE_GET_SPEC_VERSION, 0, 0, 0).value
    }

    #[inline]
    pub fn get_sbi_impl_id() -> usize {
        sbi_call(EXTENSION_BASE, FUNCTION_BASE_GET_SBI_IMPL_ID, 0, 0, 0).value
    }

    #[inline]
    pub fn get_sbi_impl_version() -> usize {
        sbi_call(EXTENSION_BASE, FUNCTION_BASE_GET_SBI_IMPL_VERSION, 0, 0, 0).value
    }

    #[inline]
    pub fn probe_extension(extension_id: usize) -> usize {
        sbi_call(EXTENSION_BASE, FUNCTION_BASE_PROBE_EXTENSION, extension_id, 0, 0).value
    }

    #[inline]
    pub fn get_mvendorid() -> usize {
        sbi_call(EXTENSION_BASE, FUNCTION_BASE_GET_MVENDORID, 0, 0, 0).value
    }

    #[inline]
    pub fn get_marchid() -> usize {
        sbi_call(EXTENSION_BASE, FUNCTION_BASE_GET_MARCHID, 0, 0, 0).value
    }

    #[inline]
    pub fn get_mimpid() -> usize {
        sbi_call(EXTENSION_BASE, FUNCTION_BASE_GET_MIMPID, 0, 0, 0).value
    }

    #[inline(always)]
    fn sbi_call_legacy(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
        let ret;
        match () {
            #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
            () => unsafe { asm!(
                "ecall", 
                in("a0") arg0, in("a1") arg1, in("a2") arg2,
                in("a7") which,
                lateout("a0") ret,
            ) },
            #[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
            () => {
                drop((which, arg0, arg1, arg2));
                unimplemented!("not RISC-V instruction set architecture")
            }
        };
        ret
    }

    const SBI_SET_TIMER: usize = 0;
    const SBI_CONSOLE_PUTCHAR: usize = 1;
    const SBI_CONSOLE_GETCHAR: usize = 2;
    const SBI_CLEAR_IPI: usize = 3;
    const SBI_SEND_IPI: usize = 4;
    const SBI_REMOTE_FENCE_I: usize = 5;
    const SBI_REMOTE_SFENCE_VMA: usize = 6;
    const SBI_REMOTE_SFENCE_VMA_ASID: usize = 7;
    const SBI_SHUTDOWN: usize = 8;

    pub fn console_putchar(c: usize) {
        sbi_call_legacy(SBI_CONSOLE_PUTCHAR, c, 0, 0);
    }

    pub fn console_getchar() -> usize {
        sbi_call_legacy(SBI_CONSOLE_GETCHAR, 0, 0, 0)
    }

    pub fn shutdown() -> ! {
        sbi_call_legacy(SBI_SHUTDOWN, 0, 0, 0);
        unreachable!()
    }

    pub fn set_timer(time: usize) {
        sbi_call_legacy(SBI_SET_TIMER, time, 0, 0);
    }
}
