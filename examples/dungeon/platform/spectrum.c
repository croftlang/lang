/* Native functions for the 48K ZX Spectrum adapters. They are placeholders
   until the z88dk build is set up; the prototypes are in program.h. */
#include "program.h"

void zx_cls(b_unit *out)
{
    *out = 0;
}

void sprite_draw(Dungeon_SpriteId sprite, b_int x, b_int y, b_unit *out)
{
    (void) sprite;
    (void) x;
    (void) y;
    *out = 0;
}

void zx_print_at(b_int row, b_int column, const b_string *text, b_unit *out)
{
    (void) row;
    (void) column;
    (void) text;
    *out = 0;
}

void read_key(b_option_Dungeon_Key *out)
{
    out->some = 0;
}

/* The DivMMC's files. Until esxDOS calls are written, reading finds nothing
   and writing succeeds without keeping anything. */
void divmmc_read(const b_string *path, b_result_bytes_error *out)
{
    (void) path;
    b_clear(out, sizeof *out);
    out->error.code = B_ERROR_Std_Files_NotFound;
}

void divmmc_write(const b_string *path, const b_bytes *data, b_result_unit_error *out)
{
    (void) path;
    (void) data;
    b_clear(out, sizeof *out);
    out->ok = 1;
}

void divmmc_append(const b_string *path, const b_bytes *data, b_result_unit_error *out)
{
    divmmc_write(path, data, out);
}

void divmmc_exists(const b_string *path, b_bool *out)
{
    (void) path;
    *out = 0;
}

void divmmc_remove(const b_string *path, b_result_unit_error *out)
{
    (void) path;
    b_clear(out, sizeof *out);
    out->error.code = B_ERROR_Std_Files_NotFound;
}
