OUTPUT_FORMAT(elf64-x86-64)
ENTRY(_start)

SECTIONS
{
  /* Higher-half kernel base to avoid low-memory overlap */
  . = 0xFFFF800000000000;

  .text : ALIGN(2M) {
    *(.text .text.*)
  }

  .rodata : ALIGN(2M) {
    *(.rodata .rodata.*)
  }

  .data : ALIGN(2M) {
    *(.data .data.*)
  }

  .bss : ALIGN(2M) {
    *(COMMON)
    *(.bss .bss.*)
  }
}


