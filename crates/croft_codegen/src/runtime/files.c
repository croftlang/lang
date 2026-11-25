/* Std.Files through C89's stdio, for the StdioFiles adapter. Written out
   only for a target that binds Files to StdioFiles. */
#include <stdio.h>
#include "program.h"

/* The longest path fopen is given. */
#define CROFT_PATH_CAPACITY 255

/* A Croft string as a C string, for fopen. A longer path is cut short. */
static void to_path(croft_string_view path, char *out)
{
    croft_u16 index;
    croft_u16 length = path.length < CROFT_PATH_CAPACITY ? path.length : CROFT_PATH_CAPACITY;
    for (index = 0; index < length; index++) {
        out[index] = path.text[index];
    }
    out[length] = '\0';
}

static void fail(croft_error *error, croft_u16 code)
{
    croft_clear(error, sizeof *error);
    error->code = code;
}

void croft_files_read(croft_string_view path, croft_result_bytes_error *out)
{
    char name[CROFT_PATH_CAPACITY + 1];
    FILE *file;
    croft_clear(out, sizeof *out);
    to_path(path, name);
    file = fopen(name, "rb");
    if (file == 0) {
        fail(&out->error, CROFT_ERROR_Std_Files_NotFound);
        return;
    }
    out->value.length = (croft_u16) fread(out->value.items, 1, CROFT_BYTES_CAPACITY, file);
    if (ferror(file)) {
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
    } else if (out->value.length == CROFT_BYTES_CAPACITY && fgetc(file) != EOF) {
        fail(&out->error, CROFT_ERROR_Std_Files_NoRoom);
    } else {
        out->ok = 1;
    }
    fclose(file);
}

static void put(croft_string_view path, const croft_bytes *data, const char *mode,
                croft_result_unit_error *out)
{
    char name[CROFT_PATH_CAPACITY + 1];
    FILE *file;
    size_t written;
    croft_clear(out, sizeof *out);
    to_path(path, name);
    file = fopen(name, mode);
    if (file == 0) {
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
        return;
    }
    written = fwrite(data->items, 1, data->length, file);
    if (fclose(file) != 0 || written != data->length) {
        fail(&out->error, CROFT_ERROR_Std_Files_NoRoom);
        return;
    }
    out->ok = 1;
}

void croft_files_write(croft_string_view path, const croft_bytes *data, croft_result_unit_error *out)
{
    put(path, data, "wb", out);
}

void croft_files_append(croft_string_view path, const croft_bytes *data, croft_result_unit_error *out)
{
    put(path, data, "ab", out);
}

void croft_files_exists(croft_string_view path, croft_bool *out)
{
    char name[CROFT_PATH_CAPACITY + 1];
    FILE *file;
    to_path(path, name);
    file = fopen(name, "rb");
    *out = file != 0;
    if (file != 0) {
        fclose(file);
    }
}

void croft_files_remove(croft_string_view path, croft_result_unit_error *out)
{
    char name[CROFT_PATH_CAPACITY + 1];
    croft_bool exists;
    croft_clear(out, sizeof *out);
    croft_files_exists(path, &exists);
    if (!exists) {
        fail(&out->error, CROFT_ERROR_Std_Files_NotFound);
        return;
    }
    to_path(path, name);
    if (remove(name) != 0) {
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
        return;
    }
    out->ok = 1;
}
