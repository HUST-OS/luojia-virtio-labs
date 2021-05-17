use std::{
    env,
    path::{Path, PathBuf},
    process::{self, Command},
};

#[macro_use]
extern crate clap;

const DEFAULT_TARGET: &'static str = "riscv64imac-unknown-none-elf";

fn main() {    
    let matches = clap_app!(xtask =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
        (@subcommand build =>
            (about: "Build virtio test project")
        )
        (@subcommand asm =>
            (about: "View asm code for virtio test project")
        )
        (@subcommand qemu =>
            (about: "Run QEMU")
        )
    ).get_matches();
    if let Some(_matches) = matches.subcommand_matches("build") {
        xtask_build();
        xtask_binary();
    } else if let Some(_matches) = matches.subcommand_matches("qemu") {
        xtask_build();
        xtask_binary();
        xtask_qemu();
    } else if let Some(_matches) = matches.subcommand_matches("asm") {
        xtask_build();
        xtask_asm();
    } else {
        println!("Use `cargo qemu` to run, `cargo xtask --help` for help")
    }
}

fn xtask_build() {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .current_dir(project_root().join("virtio-test"))
        .args(&["build", "--release"])
        .args(&["--package", "virtio-test"])
        .args(&["--target", DEFAULT_TARGET])
        .status().unwrap();

    if !status.success() {
        println!("cargo build failed");
        process::exit(1);
    }
}

fn xtask_asm() {
    // @{{objdump}} -D {{test-kernel-elf}} | less
    let objdump = "riscv64-unknown-elf-objdump";
    Command::new(objdump)
        .current_dir(dist_dir())
        .arg("-d")
        .arg("virtio-test")
        .status().unwrap();
}

fn xtask_binary() {
    /*
    objdump := "riscv64-unknown-elf-objdump"
objcopy := "rust-objcopy --binary-architecture=riscv64"

build: firmware
    @{{objcopy}} {{test-kernel-elf}} --strip-all -O binary {{test-kernel-bin}}
 */
    let objcopy = "rust-objcopy";
    let status = Command::new(objcopy)
        .current_dir(dist_dir())
        .arg("virtio-test")
        .arg("--binary-architecture=riscv64")
        .arg("--strip-all")
        .args(&["-O", "binary", "virtio-test.bin"])
        .status().unwrap();

    if !status.success() {
        println!("objcopy binary failed");
        process::exit(1);
    }
}

fn xtask_qemu() {
    /*
    qemu: build
    @qemu-system-riscv64 \
            -machine virt \
            -nographic \
            -bios none \
            -device loader,file={{rustsbi-bin}},addr=0x80000000 \
            -device loader,file={{test-kernel-bin}},addr=0x80200000 \
            -smp threads={{threads}}
    */
    let status = Command::new("qemu-system-riscv64")
        .current_dir(dist_dir())
        .args(&["-machine", "virt"])
        .args(&["-bios", "none"])
        .arg("-nographic")
        .args(&["-device", "loader,file=../../../bootloader/rustsbi-qemu.bin,addr=0x80000000"])
        .args(&["-device", &format!("loader,file={},addr={:#x}", "virtio-test.bin", 0x80200000usize)])
        .status().unwrap();
    
    if !status.success() {
        println!("qemu failed");
        process::exit(1);
    }
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}

fn dist_dir() -> PathBuf {
    project_root().join("target").join(DEFAULT_TARGET).join("release")
}
