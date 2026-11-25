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

void croft_string_set(croft_string *out, const char *text)
{
    croft_u16 length = 0;
    while (text[length] != '\0' && length < CROFT_STRING_CAPACITY) {
        out->text[length] = text[length];
        length++;
    }
    out->length = length;
}

croft_bool croft_string_equal(const croft_string *a, const croft_string *b)
{
    croft_u16 index;
    if (a->length != b->length) {
        return 0;
    }
    for (index = 0; index < a->length; index++) {
        if (a->text[index] != b->text[index]) {
            return 0;
        }
    }
    return 1;
}

void croft_string_join(const croft_string *a, const croft_string *b, croft_string *out)
{
    croft_string joined;
    croft_u16 index;
    joined = *a;
    for (index = 0; index < b->length && joined.length < CROFT_STRING_CAPACITY; index++) {
        joined.text[joined.length] = b->text[index];
        joined.length++;
    }
    CROFT_CHECK(index == b->length, "joined text is longer than a string can hold");
    *out = joined;
}

croft_int croft_string_find(const croft_string *text, const croft_string *part, croft_int from)
{
    croft_int start;
    croft_int index;
    for (start = from; start + part->length <= text->length; start++) {
        for (index = 0; index < part->length; index++) {
            if (text->text[start + index] != part->text[index]) {
                break;
            }
        }
        if (index == part->length) {
            return start;
        }
    }
    return -1;
}

croft_bool croft_string_starts_with(const croft_string *text, const croft_string *part)
{
    croft_string start;
    croft_string_slice(text, 0, part->length, &start);
    return part->length <= text->length && croft_string_equal(&start, part);
}

croft_bool croft_string_ends_with(const croft_string *text, const croft_string *part)
{
    croft_string end;
    croft_string_slice(text, (croft_int) text->length - part->length, text->length, &end);
    return part->length <= text->length && croft_string_equal(&end, part);
}

void croft_string_to_upper(const croft_string *text, croft_string *out)
{
    croft_u16 index;
    *out = *text;
    for (index = 0; index < out->length; index++) {
        if (out->text[index] >= 'a' && out->text[index] <= 'z') {
            out->text[index] = (char) (out->text[index] - 'a' + 'A');
        }
    }
}

void croft_string_to_lower(const croft_string *text, croft_string *out)
{
    croft_u16 index;
    *out = *text;
    for (index = 0; index < out->length; index++) {
        if (out->text[index] >= 'A' && out->text[index] <= 'Z') {
            out->text[index] = (char) (out->text[index] - 'A' + 'a');
        }
    }
}

void croft_string_trim(const croft_string *text, croft_string *out)
{
    croft_int start = 0;
    croft_int end = text->length;
    while (start < end && text->text[start] == ' ') {
        start++;
    }
    while (end > start && text->text[end - 1] == ' ') {
        end--;
    }
    croft_string_slice(text, start, end, out);
}

void croft_string_slice(const croft_string *text, croft_int start, croft_int end, croft_string *out)
{
    croft_string slice;
    croft_int index;
    if (start < 0) {
        start = 0;
    }
    if (end > text->length) {
        end = text->length;
    }
    slice.length = 0;
    for (index = start; index < end; index++) {
        slice.text[slice.length] = text->text[index];
        slice.length++;
    }
    *out = slice;
}

void croft_int_to_string(croft_int value, croft_string *out)
{
    char digits[12];
    int count = 0;
    croft_bool negative = value < 0;
    unsigned long remaining = negative ? 0UL - (unsigned long) value : (unsigned long) value;
    do {
        digits[count] = (char) ('0' + remaining % 10);
        remaining /= 10;
        count++;
    } while (remaining > 0);
    out->length = 0;
    if (negative) {
        out->text[out->length] = '-';
        out->length++;
    }
    while (count > 0 && out->length < CROFT_STRING_CAPACITY) {
        count--;
        out->text[out->length] = digits[count];
        out->length++;
    }
}

croft_bool croft_string_to_int(const croft_string *text, croft_int *out)
{
    croft_int value = 0;
    croft_u16 index = 0;
    croft_bool negative = text->length > 0 && text->text[0] == '-';
    if (negative) {
        index = 1;
    }
    if (index == text->length) {
        return 0;
    }
    for (; index < text->length; index++) {
        if (text->text[index] < '0' || text->text[index] > '9') {
            return 0;
        }
        value = value * 10 + (text->text[index] - '0');
    }
    *out = negative ? -value : value;
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

void croft_argument(croft_int index, croft_string *out)
{
    croft_string_set(out, argument_values[index + 1]);
}

void croft_console_print(const croft_string *text, croft_unit *out)
{
    fwrite(text->text, 1, text->length, stdout);
    putchar('\n');
    *out = 0;
}
