INCLUDE memory.x

ENTRY(Reset);
EXTERN(Reset);

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
    . = __vector_table + 512;
  } > FLASH

  .text :
  {
    . = ALIGN(4);
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
    *(.uninit .uninit.*);
    . = ALIGN(4);
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
ASSERT(__edata <= ORIGIN(DTCM) + LENGTH(DTCM), "DTCM data overflow");
ASSERT(__ebss <= ORIGIN(DTCM) + LENGTH(DTCM), "DTCM bss overflow");
