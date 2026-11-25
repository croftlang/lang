
/* Bytes: a length and up to DN_BYTES_CAPACITY values from 0 to 255. */

dn_bool dn_bytes_equal(const dn_bytes *a, const dn_bytes *b)
{
    dn_u16 index;
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

dn_bool dn_bytes_get(const dn_bytes *bytes, dn_int index, dn_int *out)
{
    if (index < 0 || index >= (dn_int) bytes->length) {
        *out = 0;
        return 0;
    }
    *out = bytes->items[index];
    return 1;
}

void dn_bytes_add(const dn_bytes *bytes, dn_int byte, dn_bytes *out)
{
    *out = *bytes;
    DN_CHECK(byte >= 0 && byte <= 255, "a byte is from 0 to 255");
    DN_CHECK(out->length < DN_BYTES_CAPACITY, "bytes are full");
    if (out->length < DN_BYTES_CAPACITY) {
        out->items[out->length] = (unsigned char) byte;
        out->length++;
    }
}

void dn_bytes_slice(const dn_bytes *bytes, dn_int start, dn_int end, dn_bytes *out)
{
    dn_int index;
    if (start < 0) {
        start = 0;
    }
    if (end > (dn_int) bytes->length) {
        end = bytes->length;
    }
    out->length = 0;
    for (index = start; index < end; index++) {
        out->items[out->length] = bytes->items[index];
        out->length++;
    }
}

/* Each byte becomes one character, up to the string's capacity. */
void dn_bytes_to_text(const dn_bytes *bytes, dn_string *out)
{
    dn_u16 index;
    out->length = 0;
    for (index = 0; index < bytes->length && out->length < DN_STRING_CAPACITY; index++) {
        out->text[out->length] = (char) bytes->items[index];
        out->length++;
    }
}

void dn_string_to_bytes(const dn_string *text, dn_bytes *out)
{
    dn_u16 index;
    out->length = 0;
    for (index = 0; index < text->length && out->length < DN_BYTES_CAPACITY; index++) {
        out->items[out->length] = (unsigned char) text->text[index];
        out->length++;
    }
}
