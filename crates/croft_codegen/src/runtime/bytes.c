
/* Bytes: a length and up to CROFT_BYTES_CAPACITY values from 0 to 255. */

croft_bool croft_bytes_equal(const croft_bytes *a, const croft_bytes *b)
{
    croft_u16 index;
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

croft_bool croft_bytes_get(const croft_bytes *bytes, croft_int index, croft_int *out)
{
    if (index < 0 || index >= (croft_int) bytes->length) {
        *out = 0;
        return 0;
    }
    *out = bytes->items[index];
    return 1;
}

void croft_bytes_add(const croft_bytes *bytes, croft_int byte, croft_bytes *out)
{
    *out = *bytes;
    CROFT_CHECK(byte >= 0 && byte <= 255, "a byte is from 0 to 255");
    CROFT_CHECK(out->length < CROFT_BYTES_CAPACITY, "bytes are full");
    if (out->length < CROFT_BYTES_CAPACITY) {
        out->items[out->length] = (unsigned char) byte;
        out->length++;
    }
}

void croft_bytes_slice(const croft_bytes *bytes, croft_int start, croft_int end, croft_bytes *out)
{
    croft_int index;
    if (start < 0) {
        start = 0;
    }
    if (end > (croft_int) bytes->length) {
        end = bytes->length;
    }
    out->length = 0;
    for (index = start; index < end; index++) {
        out->items[out->length] = bytes->items[index];
        out->length++;
    }
}

/* Each byte becomes one character, up to the string's capacity. */
void croft_bytes_to_text(const croft_bytes *bytes, croft_string *out)
{
    croft_u16 index;
    out->length = 0;
    for (index = 0; index < bytes->length && out->length < CROFT_STRING_CAPACITY; index++) {
        out->text[out->length] = (char) bytes->items[index];
        out->length++;
    }
    CROFT_CHECK(index == bytes->length, "bytes are longer than a string can hold");
}

void croft_string_to_bytes(const croft_string *text, croft_bytes *out)
{
    croft_u16 index;
    out->length = 0;
    for (index = 0; index < text->length && out->length < CROFT_BYTES_CAPACITY; index++) {
        out->items[out->length] = (unsigned char) text->text[index];
        out->length++;
    }
    CROFT_CHECK(index == text->length, "text is longer than bytes can hold");
}
