/* Std.Files through C89's stdio, for the StdioFiles adapter. Written out
   only for a target that binds Files to StdioFiles. */
#include <stdio.h>
#include "program.h"

/* A b string as a C string, for fopen. */
static void to_path(const b_string *path, char *out)
{
    b_u16 index;
    for (index = 0; index < path->length; index++) {
        out[index] = path->text[index];
    }
    out[path->length] = '\0';
}

static void fail(b_error *error, b_u16 code)
{
    b_clear(error, sizeof *error);
    error->code = code;
}

void b_files_read(const b_string *path, b_result_bytes_error *out)
{
    char name[B_STRING_CAPACITY + 1];
    FILE *file;
    b_clear(out, sizeof *out);
    to_path(path, name);
    file = fopen(name, "rb");
    if (file == 0) {
        fail(&out->error, B_ERROR_Std_Files_NotFound);
        return;
    }
    out->value.length = (b_u16) fread(out->value.items, 1, B_BYTES_CAPACITY, file);
    if (ferror(file)) {
        fail(&out->error, B_ERROR_Std_Files_Unreadable);
    } else if (out->value.length == B_BYTES_CAPACITY && fgetc(file) != EOF) {
        fail(&out->error, B_ERROR_Std_Files_NoRoom);
    } else {
        out->ok = 1;
    }
    fclose(file);
}

static void put(const b_string *path, const b_bytes *data, const char *mode,
                b_result_unit_error *out)
{
    char name[B_STRING_CAPACITY + 1];
    FILE *file;
    size_t written;
    b_clear(out, sizeof *out);
    to_path(path, name);
    file = fopen(name, mode);
    if (file == 0) {
        fail(&out->error, B_ERROR_Std_Files_Unreadable);
        return;
    }
    written = fwrite(data->items, 1, data->length, file);
    if (fclose(file) != 0 || written != data->length) {
        fail(&out->error, B_ERROR_Std_Files_NoRoom);
        return;
    }
    out->ok = 1;
}

void b_files_write(const b_string *path, const b_bytes *data, b_result_unit_error *out)
{
    put(path, data, "wb", out);
}

void b_files_append(const b_string *path, const b_bytes *data, b_result_unit_error *out)
{
    put(path, data, "ab", out);
}

void b_files_exists(const b_string *path, b_bool *out)
{
    char name[B_STRING_CAPACITY + 1];
    FILE *file;
    to_path(path, name);
    file = fopen(name, "rb");
    *out = file != 0;
    if (file != 0) {
        fclose(file);
    }
}

void b_files_remove(const b_string *path, b_result_unit_error *out)
{
    char name[B_STRING_CAPACITY + 1];
    b_bool exists;
    b_clear(out, sizeof *out);
    b_files_exists(path, &exists);
    if (!exists) {
        fail(&out->error, B_ERROR_Std_Files_NotFound);
        return;
    }
    to_path(path, name);
    if (remove(name) != 0) {
        fail(&out->error, B_ERROR_Std_Files_Unreadable);
        return;
    }
    out->ok = 1;
}
