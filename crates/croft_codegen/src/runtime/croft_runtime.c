/* The Croft runtime: strings, arguments, the console and contract failures. C89. */
#include <stdio.h>
#include <stdlib.h>
#include "croft.h"

static int argument_count = 0;
static char **argument_values = 0;

void croft_fail(const char *message)
{
    puts(message);
    exit(1);
}

void croft_clear(void *value, unsigned int size)
{
    unsigned char *bytes = (unsigned char *) value;
    unsigned int index;
    for (index = 0; index < size; index++) {
        bytes[index] = 0;
    }
}

void croft_string_set(const char *text, char *out, croft_u16 *length, croft_u16 capacity)
{
    croft_u16 count = 0;
    while (text[count] != '\0' && count < capacity) {
        out[count] = text[count];
        count++;
    }
    *length = count;
}

croft_string_view croft_string_view_from(const char *text, croft_u16 length)
{
    croft_string_view view;
    view.text = text;
    view.length = length;
    return view;
}

/* Copies as much of text as fits, forwards, so out may be where text is
   borrowed from. Gives back whether all of it fitted. */
croft_bool croft_string_copy(croft_string_view text, char *out, croft_u16 *length, croft_u16 capacity)
{
    croft_u16 index;
    croft_u16 count = text.length < capacity ? text.length : capacity;
    for (index = 0; index < count; index++) {
        out[index] = text.text[index];
    }
    *length = count;
    return text.length <= capacity;
}

croft_bool croft_string_equal(croft_string_view a, croft_string_view b)
{
    croft_u16 index;
    if (a.length != b.length) {
        return 0;
    }
    for (index = 0; index < a.length; index++) {
        if (a.text[index] != b.text[index]) {
            return 0;
        }
    }
    return 1;
}

/* Either part may be the characters of out itself, so b is copied first,
   from the end, when a is already in place. */
void croft_string_join(croft_string_view a, croft_string_view b, char *out, croft_u16 *length, croft_u16 capacity)
{
    croft_u16 index;
    croft_u16 kept = a.length < capacity ? a.length : capacity;
    croft_u16 added = b.length < capacity - kept ? b.length : (croft_u16) (capacity - kept);
    for (index = added; index > 0; index--) {
        out[kept + index - 1] = b.text[index - 1];
    }
    if (a.text != out) {
        for (index = 0; index < kept; index++) {
            out[index] = a.text[index];
        }
    }
    *length = (croft_u16) (kept + added);
    CROFT_CHECK(a.length + b.length <= capacity, "joined text is longer than a string can hold");
}

croft_int croft_string_find(croft_string_view text, croft_string_view part, croft_int from)
{
    croft_int start;
    croft_int index;
    for (start = from < 0 ? 0 : from; start + part.length <= text.length; start++) {
        for (index = 0; index < part.length; index++) {
            if (text.text[start + index] != part.text[index]) {
                break;
            }
        }
        if (index == part.length) {
            return start;
        }
    }
    return -1;
}

croft_bool croft_string_starts_with(croft_string_view text, croft_string_view part)
{
    return part.length <= text.length
        && croft_string_equal(croft_string_view_from(text.text, part.length), part);
}

croft_bool croft_string_ends_with(croft_string_view text, croft_string_view part)
{
    return part.length <= text.length
        && croft_string_equal(croft_string_view_from(text.text + text.length - part.length, part.length), part);
}

void croft_string_to_upper(croft_string_view text, char *out, croft_u16 *length, croft_u16 capacity)
{
    croft_u16 index;
    croft_string_copy(text, out, length, capacity);
    for (index = 0; index < *length; index++) {
        if (out[index] >= 'a' && out[index] <= 'z') {
            out[index] = (char) (out[index] - 'a' + 'A');
        }
    }
}

void croft_string_to_lower(croft_string_view text, char *out, croft_u16 *length, croft_u16 capacity)
{
    croft_u16 index;
    croft_string_copy(text, out, length, capacity);
    for (index = 0; index < *length; index++) {
        if (out[index] >= 'A' && out[index] <= 'Z') {
            out[index] = (char) (out[index] - 'A' + 'a');
        }
    }
}

/* The text without the spaces at either end, still borrowed from text.
   The view is kept before it is returned: zsdcc, z88dk's compiler, fails on
   returning another function's struct straight away. */
croft_string_view croft_string_trimmed(croft_string_view text)
{
    croft_int start = 0;
    croft_int end = text.length;
    croft_string_view trimmed;
    while (start < end && text.text[start] == ' ') {
        start++;
    }
    while (end > start && text.text[end - 1] == ' ') {
        end--;
    }
    trimmed = croft_string_part(text, start, end);
    return trimmed;
}

/* The characters from start up to end, still borrowed from text. */
croft_string_view croft_string_part(croft_string_view text, croft_int start, croft_int end)
{
    croft_string_view part;
    if (start < 0) {
        start = 0;
    }
    if (end > text.length) {
        end = text.length;
    }
    if (end < start) {
        end = start;
    }
    part.text = text.text + start;
    part.length = (croft_u16) (end - start);
    return part;
}

/* Text with each place that holds from changed to to, looking on after each
   one changed. An empty from changes nothing. out is never where text is
   borrowed from. */
void croft_string_replace(croft_string_view text, croft_string_view from, croft_string_view to, char *out,
                          croft_u16 *length, croft_u16 capacity)
{
    croft_u16 at = 0;
    croft_u16 kept = 0;
    croft_u16 index;
    croft_bool fits = 1;
    croft_string_view rest;
    while (at < text.length) {
        rest = croft_string_part(text, at, text.length);
        if (from.length > 0 && croft_string_starts_with(rest, from)) {
            for (index = 0; index < to.length; index++) {
                if (kept < capacity) {
                    out[kept++] = to.text[index];
                } else {
                    fits = 0;
                }
            }
            at = (croft_u16) (at + from.length);
        } else {
            if (kept < capacity) {
                out[kept++] = text.text[at];
            } else {
                fits = 0;
            }
            at++;
        }
    }
    *length = kept;
    CROFT_CHECK(fits, "replaced text is longer than a string can hold");
}

/* Text written count times, one after another. */
void croft_string_repeat(croft_string_view text, croft_int count, char *out, croft_u16 *length, croft_u16 capacity)
{
    croft_u16 kept = 0;
    croft_u16 index;
    croft_int round;
    for (round = 0; round < count && text.length > 0; round++) {
        for (index = 0; index < text.length && kept < capacity; index++) {
            out[kept++] = text.text[index];
        }
    }
    *length = kept;
    CROFT_CHECK(count <= 0 || text.length == 0 || (croft_int) text.length * count <= (croft_int) capacity,
                "repeated text is longer than a string can hold");
}

/* Text made width characters long with the characters of fill, over and
   over, before it or after it. Text already that long is kept as it is, and
   an empty fill adds nothing. */
void croft_string_pad(croft_string_view text, croft_int width, croft_string_view fill, croft_bool before, char *out,
                      croft_u16 *length, croft_u16 capacity)
{
    croft_int missing = width - (croft_int) text.length;
    croft_int added;
    croft_u16 kept = 0;
    croft_u16 index;
    if (!before) {
        croft_string_copy(text, out, &kept, capacity);
    }
    for (added = 0; fill.length > 0 && added < missing && kept < capacity; added++) {
        out[kept++] = fill.text[added % fill.length];
    }
    if (before) {
        for (index = 0; index < text.length && kept < capacity; index++) {
            out[kept++] = text.text[index];
        }
    }
    *length = kept;
    CROFT_CHECK(CROFT_MAX(width, (croft_int) text.length) <= (croft_int) capacity,
                "padded text is longer than a string can hold");
}

/* The line of text that starts at *at, without its "\n" or "\r\n", still
   borrowed from text, and moves *at to the start of the next. */
croft_string_view croft_string_line(croft_string_view text, croft_u16 *at)
{
    croft_u16 start = *at;
    croft_u16 end = start;
    croft_string_view line;
    while (end < text.length && text.text[end] != '\n') {
        end++;
    }
    *at = (croft_u16) (end < text.length ? end + 1 : end);
    if (end > start && text.text[end - 1] == '\r') {
        end--;
    }
    line = croft_string_part(text, start, end);
    return line;
}

/* Below zero when a comes before b, byte by byte, the shorter first when one
   starts the other; zero when they are the same, and above zero otherwise. */
int croft_string_compare(croft_string_view a, croft_string_view b)
{
    croft_u16 index;
    for (index = 0; index < a.length && index < b.length; index++) {
        if (a.text[index] != b.text[index]) {
            return (unsigned char) a.text[index] < (unsigned char) b.text[index] ? -1 : 1;
        }
    }
    return a.length == b.length ? 0 : (a.length < b.length ? -1 : 1);
}

void croft_int_to_string(croft_int value, char *out, croft_u16 *length, croft_u16 capacity)
{
    char digits[20];
    int count = 0;
    croft_bool negative = value < 0;
    unsigned long remaining = negative ? 0UL - (unsigned long) value : (unsigned long) value;
    do {
        digits[count] = (char) ('0' + remaining % 10);
        remaining /= 10;
        count++;
    } while (remaining > 0);
    *length = 0;
    if (negative && capacity > 0) {
        out[*length] = '-';
        (*length)++;
    }
    while (count > 0 && *length < capacity) {
        count--;
        out[*length] = digits[count];
        (*length)++;
    }
}

croft_bool croft_string_to_int(croft_string_view text, croft_int *out)
{
    unsigned long value = 0;
    unsigned long limit;
    unsigned long digit;
    croft_u16 index = 0;
    croft_bool negative = text.length > 0 && text.text[0] == '-';
    if (negative) {
        index = 1;
    }
    if (index == text.length) {
        return 0;
    }

    /* Int holds -2147483648 to 2147483647 on every target, so the number
       is read as an unsigned long, which has at least 32 bits, and refused
       as soon as it goes past what Int can hold. */
    limit = negative ? 2147483648UL : 2147483647UL;
    for (; index < text.length; index++) {
        if (text.text[index] < '0' || text.text[index] > '9') {
            return 0;
        }
        digit = (unsigned long) (text.text[index] - '0');
        if (value > (limit - digit) / 10) {
            return 0;
        }
        value = value * 10 + digit;
    }

    if (negative && value > 0) {
        *out = -(croft_int) (value - 1) - 1;
        return 1;
    }
    *out = (croft_int) value;
    return 1;
}

void croft_arguments_set(int count, char **values)
{
    argument_count = count;
    argument_values = values;
}

croft_int croft_argument_count(void)
{
    return argument_count > 0 ? argument_count - 1 : 0;
}

/* One command-line argument. With checks on, one longer than the string
   stops the program, rather than being cut short. */
void croft_argument(croft_int index, char *out, croft_u16 *length, croft_u16 capacity)
{
    const char *text = argument_values[index + 1];
    croft_string_set(text, out, length, capacity);
    CROFT_CHECK(text[*length] == '\0', "an argument is longer than a string can hold");
}

void croft_console_print(croft_string_view text, croft_unit *out)
{
    fwrite(text.text, 1, text.length, stdout);
    putchar('\n');
    *out = 0;
}

/* Prints a line on the error output, apart from what croft_console_print
   writes. */
void croft_console_print_error(croft_string_view text, croft_unit *out)
{
    fwrite(text.text, 1, text.length, stderr);
    fputc('\n', stderr);
    *out = 0;
}

/* The next line of input, without its "\n" or "\r\n". At the end of the
   input, with nothing read, *present is 0. With checks on, a line longer
   than the string stops the program rather than being cut short; with
   checks off, the rest of that line is skipped. */
void croft_console_read_line(croft_bool *present, char *out, croft_u16 *length, croft_u16 capacity)
{
    int character;
    int next;
    croft_bool read = 0;
    croft_bool full = 0;

    *length = 0;
    for (;;) {
        character = getchar();
        if (character == EOF) {
            break;
        }
        read = 1;
        if (character == '\n') {
            break;
        }
        if (character == '\r') {
            next = getchar();
            if (next == '\n') {
                break;
            }
            if (next != EOF) {
                ungetc(next, stdin);
            }
        }
        if (*length == capacity) {
            full = 1;
            continue;
        }
        out[*length] = (char) character;
        (*length)++;
    }

    CROFT_CHECK(!full, "a line is longer than a string can hold");
    *present = read;
}

/* Stops the program with an exit code. With checks off, a code outside 0
   to 255 keeps only its lowest 8 bits, as most systems do. */
void croft_process_exit(croft_int code, croft_unit *out)
{
    CROFT_CHECK(code >= 0 && code <= 255, "Process.exit: code must satisfy Range(0, 255)");
    *out = 0;
    exit((int) (code & 255));
}
