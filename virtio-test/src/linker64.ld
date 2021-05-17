OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = 0x80200000;

SECTIONS
{
    /* Load the kernel at this address: "." means the current address */
    . = BASE_ADDRESS;
    start = .;

    .text : ALIGN(4K) {
        stext = .;
        *(.text.entry)
        *(.text .text.*)
        etext = .;
    }

    .rodata : ALIGN(4K) {
        srodata = .;
        *(.rodata .rodata.*)
        erodata = .;
    }

    .data : ALIGN(4K) {
        sdata = .;
        *(.data .data.*)
        edata = .;
    }

    .bss (NOLOAD) : ALIGN(4K)  {
        sbss = .;
        *(.sbss .bss .bss.*)
        ebss = .;
    }

    PROVIDE(end = .);
}
