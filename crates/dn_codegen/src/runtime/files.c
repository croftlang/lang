/* Std.Files through C89's stdio, for the StdioFiles adapter. Written out
   only for a target that binds Files to StdioFiles. */
#include <stdio.h>
#include "program.h"

/* A dn string as a C string, for fopen. */
static void to_path(const dn_string *path, char *out)
{
    dn_u16 index;
    for (index = 0; index < path->length; index++) {
        out[index] = path->text[index];
    }
    out[path->length] = '\0';
}

static void fail(dn_error *error, dn_u16 code)
{
    dn_clear(error, sizeof *error);
    error->code = code;
}

void dn_files_read(const dn_string *path, dn_result_bytes_error *out)
{
    char name[DN_STRING_CAPACITY + 1];
    FILE *file;
    dn_clear(out, sizeof *out);
    to_path(path, name);
    file = fopen(name, "rb");
    if (file == 0) {
        fail(&out->error, DN_ERROR_Std_Files_NotFound);
        return;
    }
    out->value.length = (dn_u16) fread(out->value.items, 1, DN_BYTES_CAPACITY, file);
    if (ferror(file)) {
        fail(&out->error, DN_ERROR_Std_Files_Unreadable);
    } else if (out->value.length == DN_BYTES_CAPACITY && fgetc(file) != EOF) {
        fail(&out->error, DN_ERROR_Std_Files_NoRoom);
    } else {
        out->ok = 1;
    }
    fclose(file);
}

static void put(const dn_string *path, const dn_bytes *data, const char *mode,
                dn_result_unit_error *out)
{
    char name[DN_STRING_CAPACITY + 1];
    FILE *file;
    size_t written;
    dn_clear(out, sizeof *out);
    to_path(path, name);
    file = fopen(name, mode);
    if (file == 0) {
        fail(&out->error, DN_ERROR_Std_Files_Unreadable);
        return;
    }
    written = fwrite(data->items, 1, data->length, file);
    if (fclose(file) != 0 || written != data->length) {
        fail(&out->error, DN_ERROR_Std_Files_NoRoom);
        return;
    }
    out->ok = 1;
}

void dn_files_write(const dn_string *path, const dn_bytes *data, dn_result_unit_error *out)
{
    put(path, data, "wb", out);
}

void dn_files_append(const dn_string *path, const dn_bytes *data, dn_result_unit_error *out)
{
    put(path, data, "ab", out);
}

void dn_files_exists(const dn_string *path, dn_bool *out)
{
    char name[DN_STRING_CAPACITY + 1];
    FILE *file;
    to_path(path, name);
    file = fopen(name, "rb");
    *out = file != 0;
    if (file != 0) {
        fclose(file);
    }
}

void dn_files_remove(const dn_string *path, dn_result_unit_error *out)
{
    char name[DN_STRING_CAPACITY + 1];
    dn_bool exists;
    dn_clear(out, sizeof *out);
    dn_files_exists(path, &exists);
    if (!exists) {
        fail(&out->error, DN_ERROR_Std_Files_NotFound);
        return;
    }
    to_path(path, name);
    if (remove(name) != 0) {
        fail(&out->error, DN_ERROR_Std_Files_Unreadable);
        return;
    }
    out->ok = 1;
}
