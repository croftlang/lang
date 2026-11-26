/* Std.Files through C89's stdio, for the StdioFiles adapter. Written out
   only for a target that binds Files to StdioFiles. Listing a folder needs
   POSIX, so it is only built where there is one: Unix, macOS and MiNT. */
#if defined(__unix__) || defined(__unix) || defined(__APPLE__) || defined(__MINT__)
#define CROFT_HAS_FOLDERS
#ifndef _POSIX_C_SOURCE
#define _POSIX_C_SOURCE 1
#endif
#endif

#include <stdio.h>
#include "program.h"

#ifdef CROFT_HAS_FOLDERS
#include <dirent.h>
#include <string.h>
#include <sys/stat.h>
#endif

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

#ifdef CROFT_HAS_FOLDERS
/* Whether the entry called `name` in `folder`, a C string, is a folder. */
static croft_bool is_folder(const char *folder, const char *name)
{
    char path[2 * CROFT_PATH_CAPACITY + 2];
    struct stat info;
    size_t length = strlen(folder);
    strcpy(path, folder);
    if (length > 0 && folder[length - 1] != '/') {
        strcat(path, "/");
    }
    strcat(path, name);
    return stat(path, &info) == 0 && S_ISDIR(info.st_mode);
}

/* Whether a comes before b, byte by byte, the shorter first when one starts
   the other. */
static croft_bool comes_before(const croft_string *a, const croft_string *b)
{
    croft_u16 index;
    for (index = 0; index < a->length && index < b->length; index++) {
        if (a->text[index] != b->text[index]) {
            return (unsigned char) a->text[index] < (unsigned char) b->text[index];
        }
    }
    return a->length < b->length;
}
#endif

/* Each name is put in order as it is read, so the list is always sorted. */
void croft_files_list(croft_string_view folder, croft_result_list_string_error *out)
{
#ifdef CROFT_HAS_FOLDERS
    char name[CROFT_PATH_CAPACITY + 1];
    croft_u16 capacity = (croft_u16) (sizeof out->value.items / sizeof out->value.items[0]);
    croft_u16 room = (croft_u16) sizeof out->value.items[0].text;
    croft_string found;
    croft_u16 at;
    DIR *dir;
    struct dirent *entry;
    croft_clear(out, sizeof *out);
    to_path(folder, name);
    dir = opendir(name);
    if (dir == 0) {
        fail(&out->error, CROFT_ERROR_Std_Files_NotFound);
        return;
    }
    while ((entry = readdir(dir)) != 0) {
        size_t length = strlen(entry->d_name);
        croft_bool inner;
        if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) {
            continue;
        }
        inner = is_folder(name, entry->d_name);
        if (out->value.length >= capacity || length + inner > room) {
            closedir(dir);
            croft_clear(out, sizeof *out);
            fail(&out->error, CROFT_ERROR_Std_Files_NoRoom);
            return;
        }
        memcpy(found.text, entry->d_name, length);
        if (inner) {
            found.text[length] = '/';
        }
        found.length = (croft_u16) (length + inner);
        at = out->value.length;
        while (at > 0 && comes_before(&found, &out->value.items[at - 1])) {
            out->value.items[at] = out->value.items[at - 1];
            at--;
        }
        out->value.items[at] = found;
        out->value.length++;
    }
    closedir(dir);
    out->ok = 1;
#else
    (void) folder;
    croft_clear(out, sizeof *out);
    fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
#endif
}
