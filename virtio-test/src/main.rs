#![feature(naked_functions, asm)]
#![no_std]
#![no_main]

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

use riscv::register::{sepc, stvec::{self, TrapMode}, scause::{self, Trap, Exception}};

pub extern "C" fn rust_main(hartid: usize, dtb_pa: usize) -> ! {
    println!("<< Test-kernel: Hart id = {}, DTB physical address = {:#x}", hartid, dtb_pa);
    test_base_extension();
    test_sbi_ins_emulation();
    unsafe { stvec::write(start_trap as usize, TrapMode::Direct) };
    println!(">> Test-kernel: Trigger illegal exception");
    unsafe { asm!("csrw mcycle, x0") }; // mcycle cannot be written, this is always a 4-byte illegal instruction
    println!("<< Test-kernel: SBI test SUCCESS, shutdown");
    sbi::shutdown()
}

fn test_base_extension() {
    println!(">> Test-kernel: Testing base extension");
    let base_version = sbi::probe_extension(sbi::EXTENSION_BASE);
    if base_version == 0 {
        println!("!! Test-kernel: no base extension probed; SBI call returned value '0'");
        println!("!! Test-kernel: This SBI implementation may only have legacy extension implemented");
        println!("!! Test-kernel: SBI test FAILED due to no base extension found");
        sbi::shutdown()
    }
    println!("<< Test-kernel: Base extension version: {:x}", base_version);
    println!("<< Test-kernel: SBI specification version: {:x}", sbi::get_spec_version());
    println!("<< Test-kernel: SBI implementation Id: {:x}", sbi::get_sbi_impl_id());
    println!("<< Test-kernel: SBI implementation version: {:x}", sbi::get_sbi_impl_version());
    println!("<< Test-kernel: Device mvendorid: {:x}", sbi::get_mvendorid());
    println!("<< Test-kernel: Device marchid: {:x}", sbi::get_marchid());
    println!("<< Test-kernel: Device mimpid: {:x}", sbi::get_mimpid());
}

fn test_sbi_ins_emulation() {
    println!(">> Test-kernel: Testing SBI instruction emulation");
    let time = riscv::register::time::read64();
    println!("<< Test-kernel: Current time: {:x}", time);
}

pub extern "C" fn rust_trap_exception() {
    let cause = scause::read().cause();
    println!("<< Test-kernel: Value of scause: {:?}", cause);
    if cause != Trap::Exception(Exception::IllegalInstruction) {
        println!("!! Test-kernel: Wrong cause associated to illegal instruction");
        sbi::shutdown()
    }
    println!("<< Test-kernel: Illegal exception delegate success");
    sepc::write(sepc::read().wrapping_add(4));
}

use core::panic::PanicInfo;

#[cfg_attr(not(test), panic_handler)]
#[allow(unused)]
fn panic(info: &PanicInfo) -> ! {
    println!("!! Test-kernel: {}", info);
    println!("!! Test-kernel: SBI test FAILED due to panic");
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


#[cfg(target_pointer_width = "128")]
macro_rules! define_store_load {
    () => {
        ".altmacro
        .macro STORE reg, offset
            sq  \\reg, \\offset* {REGBYTES} (sp)
        .endm
        .macro LOAD reg, offset
            lq  \\reg, \\offset* {REGBYTES} (sp)
        .endm"
    };
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
