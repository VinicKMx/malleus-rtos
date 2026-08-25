INCLUDE memory.x

ENTRY(Reset);
EXTERN(Reset);
EXTERN(HardFault);
EXTERN(MemManage);
EXTERN(BusFault);
EXTERN(UsageFault);

SECTIONS
{
  /* STM32F767 has 110 peripheral IRQs. Reserving 512 bytes keeps the table
   * naturally aligned for its eventual 16 core + 110 peripheral entries. */
  .vector_table ORIGIN(FLASH) :
  {
    . = ALIGN(512);
    __vector_table = .;
    LONG(__stack_start);
    LONG(Reset);
    LONG(0);          /* NMI */
    LONG(HardFault);
    LONG(MemManage);
    LONG(BusFault);
    LONG(UsageFault);
    . = __vector_table + 64; /* Remaining architectural exceptions. */
    . = __vector_table + 512;
  } > FLASH

  .text :
  {
    . = ALIGN(4);
    KEEP(*(.text.Reset));
    *(.text .text.*);
    . = ALIGN(4);
  } > FLASH

  .rodata :
  {
    . = ALIGN(4);
    *(.rodata .rodata.*);
    . = ALIGN(4);
  } > FLASH

  .ARM.exidx :
  {
    *(.ARM.exidx .ARM.exidx.*);
  } > FLASH

  /* `.data` is loaded immediately after every read-only Flash section. */
  . = ALIGN(4);
  __etext = .;

  .data : AT(__etext)
  {
    . = ALIGN(4);
    __sdata = .;
    *(.data .data.*);
    . = ALIGN(4);
    __edata = .;
  } > DTCM
  __sidata = LOADADDR(.data);

  .bss (NOLOAD) :
  {
    . = ALIGN(4);
    __sbss = .;
    *(.bss .bss.*);
    *(COMMON);
    . = ALIGN(4);
    __ebss = .;
  } > DTCM

  .uninit (NOLOAD) :
  {
    . = ALIGN(4);
    __suninit = .;
    *(.uninit .uninit.*);
    . = ALIGN(4);
    __euninit = .;
  } > DTCM

  .got :
  {
    *(.got .got.*);
  } > FLASH

  /DISCARD/ :
  {
    *(.ARM.extab .ARM.extab.*);
    *(.eh_frame .eh_frame.*);
  }
}

ASSERT(SIZEOF(.vector_table) == 512, "vector table reservation changed");
ASSERT(SIZEOF(.got) == 0, "position-independent code is unsupported");
ASSERT(__sidata % 4 == 0, ".data load address must be word-aligned");
ASSERT(__sdata % 4 == 0, ".data start must be word-aligned");
ASSERT((__edata - __sdata) % 4 == 0, ".data size must be a whole number of words");
ASSERT(__sbss % 4 == 0, ".bss start must be word-aligned");
ASSERT((__ebss - __sbss) % 4 == 0, ".bss size must be a whole number of words");
ASSERT(__sidata >= ORIGIN(FLASH), ".data load image must start in Flash");
ASSERT(__sidata + SIZEOF(.data) <= ORIGIN(FLASH) + LENGTH(FLASH), ".data load image exceeds Flash");
ASSERT(__edata <= ORIGIN(DTCM) + LENGTH(DTCM), "DTCM data overflow");
ASSERT(__ebss <= ORIGIN(DTCM) + LENGTH(DTCM), "DTCM bss overflow");
ASSERT(__euninit <= __stack_start, "DTCM static data collides with the reset stack");
