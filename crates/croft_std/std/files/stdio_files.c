/* Std.Files through C89's stdio, for the StdioFiles adapter. Written out
   only for a target that binds Files to StdioFiles. Listing and making
   folders needs POSIX, so they are only built where there is one: Unix,
   macOS and MiNT. */
#if defined(__unix__) || defined(__unix) || defined(__APPLE__) || defined(__MINT__)
#define CROFT_HAS_FOLDERS
#ifndef _POSIX_C_SOURCE
#define _POSIX_C_SOURCE 1
#endif
#endif

#include <errno.h>
#include <stdio.h>
#include "program.h"

#ifdef CROFT_HAS_FOLDERS
#include <dirent.h>
#include <string.h>
#include <sys/stat.h>
#endif

/* The longest path fopen is given. */
#define CROFT_PATH_CAPACITY 255

/* A Croft string as a C string, for fopen. Gives 0 for a path longer than
   CROFT_PATH_CAPACITY, or with a NUL byte in it, which would end it early,
   so a path is never cut short into the name of another file. */
static croft_bool to_path(croft_string_view path, char *out)
{
    croft_u16 index;
    out[0] = '\0';
    if (path.length > CROFT_PATH_CAPACITY) {
        return 0;
    }
    for (index = 0; index < path.length; index++) {
        if (path.text[index] == '\0') {
            out[0] = '\0';
            return 0;
        }
        out[index] = path.text[index];
    }
    out[path.length] = '\0';
    return 1;
}

static void fail(croft_error *error, croft_u16 code)
{
    croft_clear(error, sizeof *error);
    error->code = code;
}

/* Whether the call that just failed, with errno cleared before it, failed
   because there is nothing at the path. ENOENT and ENOTDIR are POSIX, not
   C89, so a machine without them can't tell, and counts every failure as a
   missing file. */
static croft_bool is_missing(void)
{
#ifdef ENOENT
    if (errno == ENOENT) {
        return 1;
    }
#ifdef ENOTDIR
    if (errno == ENOTDIR) {
        return 1;
    }
#endif
    return 0;
#else
    return 1;
#endif
}

/* The error for a path that couldn't be opened: NotFound when there is
   nothing there, Unreadable when it is there but can't be opened, such as
   one the program has no permission for. */
static croft_u16 open_error(void)
{
    return is_missing() ? CROFT_ERROR_Std_Files_NotFound : CROFT_ERROR_Std_Files_Unreadable;
}

/* Reads the whole file into the Bytes `into` writes, failing with NoRoom
   when it holds more than `most` bytes, or more than there is room for. */
void croft_files_read(croft_string_view path, croft_int most, croft_bytes_writer *into, croft_result_unit_error *out)
{
    char name[CROFT_PATH_CAPACITY + 1];
    FILE *file;
    croft_u16 room = into->capacity;
    croft_clear(out, sizeof *out);
    *into->length = 0;
    if (most < (croft_int) room) {
        room = most > 0 ? (croft_u16) most : 0;
    }
    if (!to_path(path, name)) {
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
        return;
    }
    errno = 0;
    file = fopen(name, "rb");
    if (file == 0) {
        fail(&out->error, open_error());
        return;
    }
    *into->length = (croft_u16) fread(into->items, 1, room, file);
    if (ferror(file)) {
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
    } else if (*into->length == room && fgetc(file) != EOF) {
        fail(&out->error, CROFT_ERROR_Std_Files_NoRoom);
    } else {
        out->ok = 1;
    }
    fclose(file);
}

static void put(croft_string_view path, croft_bytes_view data, const char *mode,
                croft_result_unit_error *out)
{
    char name[CROFT_PATH_CAPACITY + 1];
    FILE *file;
    size_t written;
    croft_clear(out, sizeof *out);
    if (!to_path(path, name)) {
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
        return;
    }
    errno = 0;
    file = fopen(name, mode);
    if (file == 0) {
        /* A folder on the way that isn't there makes it NotFound. */
        fail(&out->error, open_error());
        return;
    }
    written = fwrite(data.items, 1, data.length, file);
    if (fclose(file) != 0 || written != data.length) {
        fail(&out->error, CROFT_ERROR_Std_Files_NoRoom);
        return;
    }
    out->ok = 1;
}

void croft_files_write(croft_string_view path, croft_bytes_view data, croft_result_unit_error *out)
{
    put(path, data, "wb", out);
}

void croft_files_append(croft_string_view path, croft_bytes_view data, croft_result_unit_error *out)
{
    put(path, data, "ab", out);
}

void croft_files_exists(croft_string_view path, croft_bool *out)
{
    char name[CROFT_PATH_CAPACITY + 1];
    FILE *file;
    if (!to_path(path, name)) {
        *out = 0;
        return;
    }
    errno = 0;
    file = fopen(name, "rb");
    if (file == 0) {
        /* A file the program may not read is still there. */
        *out = !is_missing();
        return;
    }
    *out = 1;
    fclose(file);
}

void croft_files_remove(croft_string_view path, croft_result_unit_error *out)
{
    char name[CROFT_PATH_CAPACITY + 1];
    croft_bool exists;
    croft_clear(out, sizeof *out);
    if (!to_path(path, name)) {
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
        return;
    }
    croft_files_exists(path, &exists);
    if (!exists) {
        fail(&out->error, CROFT_ERROR_Std_Files_NotFound);
        return;
    }
    if (remove(name) != 0) {
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
        return;
    }
    out->ok = 1;
}

/* How much of a file is copied at a time: little, for 8-bit machines. */
#define CROFT_COPY_CHUNK 128

void croft_files_copy(croft_string_view from, croft_string_view to, croft_result_unit_error *out)
{
    char name[CROFT_PATH_CAPACITY + 1];
    unsigned char chunk[CROFT_COPY_CHUNK];
    FILE *source;
    FILE *copy;
    size_t count;
    croft_bool written = 1;
    croft_clear(out, sizeof *out);
    if (!to_path(from, name)) {
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
        return;
    }
    errno = 0;
    source = fopen(name, "rb");
    if (source == 0) {
        fail(&out->error, open_error());
        return;
    }
    if (!to_path(to, name)) {
        fclose(source);
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
        return;
    }
    errno = 0;
    copy = fopen(name, "wb");
    if (copy == 0) {
        /* Worked out before fclose, which may change errno. */
        fail(&out->error, open_error());
        fclose(source);
        return;
    }
    while (written && (count = fread(chunk, 1, CROFT_COPY_CHUNK, source)) > 0) {
        written = fwrite(chunk, 1, count, copy) == count;
    }
    if (ferror(source)) {
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
    } else if (!written) {
        fail(&out->error, CROFT_ERROR_Std_Files_NoRoom);
    } else {
        out->ok = 1;
    }
    fclose(source);
    if (fclose(copy) != 0 && out->ok) {
        out->ok = 0;
        fail(&out->error, CROFT_ERROR_Std_Files_NoRoom);
    }
}

#ifdef CROFT_HAS_FOLDERS
/* Whether `path`, a C string, names a folder. */
static croft_bool is_folder_path(const char *path)
{
    struct stat info;
    return stat(path, &info) == 0 && S_ISDIR(info.st_mode);
}
#endif

/* Up to `length` bytes from `offset` bytes into the file, into the Bytes
   `into` writes, or as many as there is room for. A stream may seek past
   its end, and then reads nothing, so an offset at or past the end gives
   no bytes. */
void croft_files_read_part(croft_string_view path, croft_int offset, croft_int length,
                           croft_bytes_writer *into, croft_result_unit_error *out)
{
    char name[CROFT_PATH_CAPACITY + 1];
    FILE *file;
    croft_u16 room = into->capacity;
    croft_clear(out, sizeof *out);
    *into->length = 0;
    if (length < (croft_int) room) {
        room = length > 0 ? (croft_u16) length : 0;
    }
    if (!to_path(path, name)) {
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
        return;
    }
    errno = 0;
    file = fopen(name, "rb");
    if (file == 0) {
        fail(&out->error, open_error());
        return;
    }
    if (offset < 0 || fseek(file, (long) offset, SEEK_SET) != 0) {
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
        fclose(file);
        return;
    }
    *into->length = (croft_u16) fread(into->items, 1, room, file);
    if (ferror(file)) {
        *into->length = 0;
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
    } else {
        out->ok = 1;
    }
    fclose(file);
}

/* How many bytes the file holds, by seeking to its end: C89 has no other
   way. A folder, which fopen may open on some machines, is Unreadable. */
void croft_files_size(croft_string_view path, croft_result_int_error *out)
{
    char name[CROFT_PATH_CAPACITY + 1];
    FILE *file;
    long end;
    croft_clear(out, sizeof *out);
    if (!to_path(path, name)) {
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
        return;
    }
    errno = 0;
    file = fopen(name, "rb");
    if (file == 0) {
        fail(&out->error, open_error());
        return;
    }
#ifdef CROFT_HAS_FOLDERS
    if (is_folder_path(name)) {
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
        fclose(file);
        return;
    }
#endif
    end = fseek(file, 0L, SEEK_END) == 0 ? ftell(file) : -1L;
    fclose(file);
    if (end < 0) {
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
        return;
    }
    if (end > (long) CROFT_INT_MAX) {
        fail(&out->error, CROFT_ERROR_Std_Files_NoRoom);
        return;
    }
    out->value = (croft_int) end;
    out->ok = 1;
}

/* Moves the file with C89's rename. Nothing at `from`, or no folder at
   `to`, is NotFound; anything else, such as a file at `to` on a machine
   that won't replace it, or another disk, is Unreadable. */
void croft_files_rename(croft_string_view from, croft_string_view to, croft_result_unit_error *out)
{
    char source[CROFT_PATH_CAPACITY + 1];
    char target[CROFT_PATH_CAPACITY + 1];
    croft_clear(out, sizeof *out);
    if (!to_path(from, source) || !to_path(to, target)) {
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
        return;
    }
    errno = 0;
    if (rename(source, target) != 0) {
        fail(&out->error, open_error());
        return;
    }
    out->ok = 1;
}

/* Makes each folder on the way, from the first, so `a/b/c` makes `a`, then
   `a/b`, then `a/b/c`. */
void croft_files_make_folder(croft_string_view path, croft_result_unit_error *out)
{
#ifdef CROFT_HAS_FOLDERS
    char name[CROFT_PATH_CAPACITY + 1];
    size_t at;
    size_t length;
    char kept;
    croft_clear(out, sizeof *out);
    if (!to_path(path, name)) {
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
        return;
    }
    length = strlen(name);
    for (at = 1; at <= length; at++) {
        if (name[at] != '/' && name[at] != '\0') {
            continue;
        }
        kept = name[at];
        name[at] = '\0';
        if (!is_folder_path(name) && mkdir(name, 0777) != 0 && !is_folder_path(name)) {
            fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
            return;
        }
        name[at] = kept;
    }
    out->ok = 1;
#else
    (void) path;
    croft_clear(out, sizeof *out);
    fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
#endif
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
    if (!to_path(folder, name)) {
        fail(&out->error, CROFT_ERROR_Std_Files_Unreadable);
        return;
    }
    errno = 0;
    dir = opendir(name);
    if (dir == 0) {
        fail(&out->error, open_error());
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
