// Adapters for the 48K ZX Spectrum, in C from the platform library.
context Platform

use { Screen, Keyboard, Key } from Dungeon
use { Files } from Std.Files

adapter SpectrumScreen for Screen = {
    fn clear() = native zx_cls()
    fn draw_sprite(sprite, at) = native sprite_draw(sprite, at.x, at.y)
    fn draw_text(text, at) = native zx_print_at(at.y / 8, at.x / 8, text)
}

adapter SpectrumKeyboard for Keyboard = {
    fn pressed() = native read_key()
}

// Files on the DivMMC's SD card, through esxDOS.
adapter DivmmcFiles for Files = {
    fn read(path) = native divmmc_read(path)
    fn write(path, data) = native divmmc_write(path, data)
    fn append(path, data) = native divmmc_append(path, data)
    fn exists(path) = native divmmc_exists(path)
    fn remove(path) = native divmmc_remove(path)
}
