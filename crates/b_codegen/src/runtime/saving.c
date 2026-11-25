
/* Writing and reading saved values, for FileStorage. A number is four
   bytes, least significant first; a length is two. Each gives back 0 when
   the bytes are full, or have run out. */

b_bool b_bytes_put_int(b_bytes *out, b_int value)
{
    unsigned long bits = (unsigned long) value;
    int index;
    if (out->length + 4 > B_BYTES_CAPACITY) {
        return 0;
    }
    for (index = 0; index < 4; index++) {
        out->items[out->length] = (unsigned char) (bits & 0xFFUL);
        out->length++;
        bits >>= 8;
    }
    return 1;
}

b_bool b_bytes_take_int(const b_bytes *in, b_u16 *at, b_int *out)
{
    unsigned long bits = 0;
    int index;
    if (*at + 4 > in->length) {
        return 0;
    }
    for (index = 3; index >= 0; index--) {
        bits = (bits << 8) | in->items[*at + index];
    }
    *at = (b_u16) (*at + 4);
    /* Back from unsigned without relying on the compiler to wrap. */
    *out = bits > 0x7FFFFFFFUL ? -(b_int) (0xFFFFFFFFUL - bits) - 1 : (b_int) bits;
    return 1;
}

b_bool b_bytes_put_length(b_bytes *out, b_u16 length)
{
    if (out->length + 2 > B_BYTES_CAPACITY) {
        return 0;
    }
    out->items[out->length] = (unsigned char) (length & 0xFF);
    out->items[out->length + 1] = (unsigned char) (length >> 8);
    out->length = (b_u16) (out->length + 2);
    return 1;
}

b_bool b_bytes_take_length(const b_bytes *in, b_u16 *at, b_u16 *out)
{
    if (*at + 2 > in->length) {
        return 0;
    }
    *out = (b_u16) (in->items[*at] | (in->items[*at + 1] << 8));
    *at = (b_u16) (*at + 2);
    return 1;
}

b_bool b_bytes_put_string(b_bytes *out, const b_string *text)
{
    b_u16 index;
    if (!b_bytes_put_length(out, text->length) || out->length + text->length > B_BYTES_CAPACITY) {
        return 0;
    }
    for (index = 0; index < text->length; index++) {
        out->items[out->length] = (unsigned char) text->text[index];
        out->length++;
    }
    return 1;
}

b_bool b_bytes_take_string(const b_bytes *in, b_u16 *at, b_string *out)
{
    b_u16 index;
    if (!b_bytes_take_length(in, at, &out->length) || out->length > B_STRING_CAPACITY
        || *at + out->length > in->length) {
        return 0;
    }
    for (index = 0; index < out->length; index++) {
        out->text[index] = (char) in->items[*at + index];
    }
    *at = (b_u16) (*at + out->length);
    return 1;
}

b_bool b_bytes_put_bytes(b_bytes *out, const b_bytes *bytes)
{
    b_u16 index;
    if (!b_bytes_put_length(out, bytes->length) || out->length + bytes->length > B_BYTES_CAPACITY) {
        return 0;
    }
    for (index = 0; index < bytes->length; index++) {
        out->items[out->length] = bytes->items[index];
        out->length++;
    }
    return 1;
}

b_bool b_bytes_take_bytes(const b_bytes *in, b_u16 *at, b_bytes *out)
{
    b_u16 index;
    if (!b_bytes_take_length(in, at, &out->length) || out->length > B_BYTES_CAPACITY
        || *at + out->length > in->length) {
        return 0;
    }
    for (index = 0; index < out->length; index++) {
        out->items[index] = in->items[*at + index];
    }
    *at = (b_u16) (*at + out->length);
    return 1;
}

/* The key an aggregate is kept under: the repository's prefix and its
   identity, short enough for the 8.3 names of FAT disks. */
void b_storage_key(const char *prefix, b_int identity, b_string *out)
{
    b_string number;
    b_string_set(out, prefix);
    b_int_to_string(identity, &number);
    b_string_join(out, &number, out);
}
