/* Native functions for the 48K ZX Spectrum adapters. They are placeholders
   until the z88dk build is set up; the prototypes are in program.h. */
#include "program.h"

void zx_cls(dn_unit *out)
{
    *out = 0;
}

void sprite_draw(Dungeon_SpriteId sprite, dn_int x, dn_int y, dn_unit *out)
{
    (void) sprite;
    (void) x;
    (void) y;
    *out = 0;
}

void zx_print_at(dn_int row, dn_int column, const dn_string *text, dn_unit *out)
{
    (void) row;
    (void) column;
    (void) text;
    *out = 0;
}

void read_key(dn_option_Dungeon_Key *out)
{
    out->some = 0;
}

/* The DivMMC's files. Until esxDOS calls are written, reading finds nothing
   and writing succeeds without keeping anything. */
void divmmc_read(const dn_string *path, dn_result_bytes_error *out)
{
    (void) path;
    dn_clear(out, sizeof *out);
    out->error.code = DN_ERROR_Std_Files_NotFound;
}

void divmmc_write(const dn_string *path, const dn_bytes *data, dn_result_unit_error *out)
{
    (void) path;
    (void) data;
    dn_clear(out, sizeof *out);
    out->ok = 1;
}

void divmmc_append(const dn_string *path, const dn_bytes *data, dn_result_unit_error *out)
{
    divmmc_write(path, data, out);
}

void divmmc_exists(const dn_string *path, dn_bool *out)
{
    (void) path;
    *out = 0;
}

void divmmc_remove(const dn_string *path, dn_result_unit_error *out)
{
    (void) path;
    dn_clear(out, sizeof *out);
    out->error.code = DN_ERROR_Std_Files_NotFound;
}
