/* The dn runtime: strings, arguments, the console and contract failures. C89. */
#include <stdio.h>
#include <stdlib.h>
#include "dn.h"

static int argument_count = 0;
static char **argument_values = 0;

void dn_fail(const char *message)
{
    puts(message);
    exit(1);
}

void dn_clear(void *value, unsigned int size)
{
    unsigned char *bytes = (unsigned char *) value;
    unsigned int index;
    for (index = 0; index < size; index++) {
        bytes[index] = 0;
    }
}

void dn_string_set(dn_string *out, const char *text)
{
    dn_u16 length = 0;
    while (text[length] != '\0' && length < DN_STRING_CAPACITY) {
        out->text[length] = text[length];
        length++;
    }
    out->length = length;
}

dn_bool dn_string_equal(const dn_string *a, const dn_string *b)
{
    dn_u16 index;
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

void dn_string_join(const dn_string *a, const dn_string *b, dn_string *out)
{
    dn_string joined;
    dn_u16 index;
    joined = *a;
    for (index = 0; index < b->length && joined.length < DN_STRING_CAPACITY; index++) {
        joined.text[joined.length] = b->text[index];
        joined.length++;
    }
    DN_CHECK(index == b->length, "joined text is longer than a string can hold");
    *out = joined;
}

dn_int dn_string_find(const dn_string *text, const dn_string *part, dn_int from)
{
    dn_int start;
    dn_int index;
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

dn_bool dn_string_starts_with(const dn_string *text, const dn_string *part)
{
    dn_string start;
    dn_string_slice(text, 0, part->length, &start);
    return part->length <= text->length && dn_string_equal(&start, part);
}

dn_bool dn_string_ends_with(const dn_string *text, const dn_string *part)
{
    dn_string end;
    dn_string_slice(text, (dn_int) text->length - part->length, text->length, &end);
    return part->length <= text->length && dn_string_equal(&end, part);
}

void dn_string_to_upper(const dn_string *text, dn_string *out)
{
    dn_u16 index;
    *out = *text;
    for (index = 0; index < out->length; index++) {
        if (out->text[index] >= 'a' && out->text[index] <= 'z') {
            out->text[index] = (char) (out->text[index] - 'a' + 'A');
        }
    }
}

void dn_string_to_lower(const dn_string *text, dn_string *out)
{
    dn_u16 index;
    *out = *text;
    for (index = 0; index < out->length; index++) {
        if (out->text[index] >= 'A' && out->text[index] <= 'Z') {
            out->text[index] = (char) (out->text[index] - 'A' + 'a');
        }
    }
}

void dn_string_trim(const dn_string *text, dn_string *out)
{
    dn_int start = 0;
    dn_int end = text->length;
    while (start < end && text->text[start] == ' ') {
        start++;
    }
    while (end > start && text->text[end - 1] == ' ') {
        end--;
    }
    dn_string_slice(text, start, end, out);
}

void dn_string_slice(const dn_string *text, dn_int start, dn_int end, dn_string *out)
{
    dn_string slice;
    dn_int index;
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

void dn_int_to_string(dn_int value, dn_string *out)
{
    char digits[12];
    int count = 0;
    dn_bool negative = value < 0;
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
    while (count > 0 && out->length < DN_STRING_CAPACITY) {
        count--;
        out->text[out->length] = digits[count];
        out->length++;
    }
}

dn_bool dn_string_to_int(const dn_string *text, dn_int *out)
{
    dn_int value = 0;
    dn_u16 index = 0;
    dn_bool negative = text->length > 0 && text->text[0] == '-';
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

void dn_arguments_set(int count, char **values)
{
    argument_count = count;
    argument_values = values;
}

dn_int dn_argument_count(void)
{
    return argument_count > 0 ? argument_count - 1 : 0;
}

void dn_argument(dn_int index, dn_string *out)
{
    dn_string_set(out, argument_values[index + 1]);
}

void dn_console_print(const dn_string *text, dn_unit *out)
{
    fwrite(text->text, 1, text->length, stdout);
    putchar('\n');
    *out = 0;
}
