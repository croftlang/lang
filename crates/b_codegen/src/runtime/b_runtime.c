/* The b runtime: strings, arguments, the console and contract failures. C89. */
#include <stdio.h>
#include <stdlib.h>
#include "b.h"

static int argument_count = 0;
static char **argument_values = 0;

void b_fail(const char *message)
{
    puts(message);
    exit(1);
}

void b_clear(void *value, unsigned int size)
{
    unsigned char *bytes = (unsigned char *) value;
    unsigned int index;
    for (index = 0; index < size; index++) {
        bytes[index] = 0;
    }
}

void b_string_set(b_string *out, const char *text)
{
    b_u16 length = 0;
    while (text[length] != '\0' && length < B_STRING_CAPACITY) {
        out->text[length] = text[length];
        length++;
    }
    out->length = length;
}

b_bool b_string_equal(const b_string *a, const b_string *b)
{
    b_u16 index;
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

void b_string_join(const b_string *a, const b_string *b, b_string *out)
{
    b_string joined;
    b_u16 index;
    joined = *a;
    for (index = 0; index < b->length && joined.length < B_STRING_CAPACITY; index++) {
        joined.text[joined.length] = b->text[index];
        joined.length++;
    }
    B_CHECK(index == b->length, "joined text is longer than a string can hold");
    *out = joined;
}

b_int b_string_find(const b_string *text, const b_string *part, b_int from)
{
    b_int start;
    b_int index;
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

b_bool b_string_starts_with(const b_string *text, const b_string *part)
{
    b_string start;
    b_string_slice(text, 0, part->length, &start);
    return part->length <= text->length && b_string_equal(&start, part);
}

b_bool b_string_ends_with(const b_string *text, const b_string *part)
{
    b_string end;
    b_string_slice(text, (b_int) text->length - part->length, text->length, &end);
    return part->length <= text->length && b_string_equal(&end, part);
}

void b_string_to_upper(const b_string *text, b_string *out)
{
    b_u16 index;
    *out = *text;
    for (index = 0; index < out->length; index++) {
        if (out->text[index] >= 'a' && out->text[index] <= 'z') {
            out->text[index] = (char) (out->text[index] - 'a' + 'A');
        }
    }
}

void b_string_to_lower(const b_string *text, b_string *out)
{
    b_u16 index;
    *out = *text;
    for (index = 0; index < out->length; index++) {
        if (out->text[index] >= 'A' && out->text[index] <= 'Z') {
            out->text[index] = (char) (out->text[index] - 'A' + 'a');
        }
    }
}

void b_string_trim(const b_string *text, b_string *out)
{
    b_int start = 0;
    b_int end = text->length;
    while (start < end && text->text[start] == ' ') {
        start++;
    }
    while (end > start && text->text[end - 1] == ' ') {
        end--;
    }
    b_string_slice(text, start, end, out);
}

void b_string_slice(const b_string *text, b_int start, b_int end, b_string *out)
{
    b_string slice;
    b_int index;
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

void b_int_to_string(b_int value, b_string *out)
{
    char digits[12];
    int count = 0;
    b_bool negative = value < 0;
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
    while (count > 0 && out->length < B_STRING_CAPACITY) {
        count--;
        out->text[out->length] = digits[count];
        out->length++;
    }
}

b_bool b_string_to_int(const b_string *text, b_int *out)
{
    b_int value = 0;
    b_u16 index = 0;
    b_bool negative = text->length > 0 && text->text[0] == '-';
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

void b_arguments_set(int count, char **values)
{
    argument_count = count;
    argument_values = values;
}

b_int b_argument_count(void)
{
    return argument_count > 0 ? argument_count - 1 : 0;
}

void b_argument(b_int index, b_string *out)
{
    b_string_set(out, argument_values[index + 1]);
}

void b_console_print(const b_string *text, b_unit *out)
{
    fwrite(text->text, 1, text->length, stdout);
    putchar('\n');
    *out = 0;
}
