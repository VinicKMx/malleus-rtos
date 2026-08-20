/*
 * STM32F767ZI memory map.
 *
 * Sources:
 * - DS11532 Rev 9, STM32F765xx/767xx/768Ax/769xx datasheet.
 * - RM0410 Rev 5, sections 2.2-2.4.
 * - ES0334 Rev 9, device errata for STM32F76xxx/77xxx revisions A and Z/1.
 *
 * Keep physically distinct memories separate. In particular, combining DTCM,
 * SRAM1, and SRAM2 into one synthetic RAM region would hide bus-access and
 * future MPU/DMA constraints from the linker contract.
 */
MEMORY
{
  ITCM   (rwx) : ORIGIN = 0x00000000, LENGTH = 16K
  FLASH  (rx)  : ORIGIN = 0x08000000, LENGTH = 2048K
  DTCM   (rwx) : ORIGIN = 0x20000000, LENGTH = 128K
  SRAM1  (rwx) : ORIGIN = 0x20020000, LENGTH = 368K
  SRAM2  (rwx) : ORIGIN = 0x2007C000, LENGTH = 16K
}

/* The reset stack starts at the top of DTCM and grows down. */
__stack_start = ORIGIN(DTCM) + LENGTH(DTCM);

ASSERT(__stack_start % 8 == 0, "initial stack pointer must be 8-byte aligned");
