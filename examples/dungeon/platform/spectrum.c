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

void divmmc_load(const dn_string *name, dn_result_list_int_error *out)
{
    (void) name;
    dn_clear(out, sizeof *out);
    out->ok = 1;
}

void divmmc_store(const dn_string *name, const dn_list_int *data, dn_result_unit_error *out)
{
    (void) name;
    (void) data;
    dn_clear(out, sizeof *out);
    out->ok = 1;
}
