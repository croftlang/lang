
/* Bytes: a length and up to B_BYTES_CAPACITY values from 0 to 255. */

b_bool b_bytes_equal(const b_bytes *a, const b_bytes *b)
{
    b_u16 index;
    if (a->length != b->length) {
        return 0;
    }
    for (index = 0; index < a->length; index++) {
        if (a->items[index] != b->items[index]) {
            return 0;
        }
    }
    return 1;
}

b_bool b_bytes_get(const b_bytes *bytes, b_int index, b_int *out)
{
    if (index < 0 || index >= (b_int) bytes->length) {
        *out = 0;
        return 0;
    }
    *out = bytes->items[index];
    return 1;
}

void b_bytes_add(const b_bytes *bytes, b_int byte, b_bytes *out)
{
    *out = *bytes;
    B_CHECK(byte >= 0 && byte <= 255, "a byte is from 0 to 255");
    B_CHECK(out->length < B_BYTES_CAPACITY, "bytes are full");
    if (out->length < B_BYTES_CAPACITY) {
        out->items[out->length] = (unsigned char) byte;
        out->length++;
    }
}

void b_bytes_slice(const b_bytes *bytes, b_int start, b_int end, b_bytes *out)
{
    b_int index;
    if (start < 0) {
        start = 0;
    }
    if (end > (b_int) bytes->length) {
        end = bytes->length;
    }
    out->length = 0;
    for (index = start; index < end; index++) {
        out->items[out->length] = bytes->items[index];
        out->length++;
    }
}

/* Each byte becomes one character, up to the string's capacity. */
void b_bytes_to_text(const b_bytes *bytes, b_string *out)
{
    b_u16 index;
    out->length = 0;
    for (index = 0; index < bytes->length && out->length < B_STRING_CAPACITY; index++) {
        out->text[out->length] = (char) bytes->items[index];
        out->length++;
    }
    B_CHECK(index == bytes->length, "bytes are longer than a string can hold");
}

void b_string_to_bytes(const b_string *text, b_bytes *out)
{
    b_u16 index;
    out->length = 0;
    for (index = 0; index < text->length && out->length < B_BYTES_CAPACITY; index++) {
        out->items[out->length] = (unsigned char) text->text[index];
        out->length++;
    }
    B_CHECK(index == text->length, "text is longer than bytes can hold");
}
