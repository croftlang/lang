
/* Writing and reading saved values, for FileStorage. A number is four
   bytes, least significant first; a length is two. Each gives back 0 when
   the bytes are full, or have run out. */

croft_bool croft_bytes_put_int(croft_bytes *out, croft_int value)
{
    unsigned long bits = (unsigned long) value;
    int index;
    if (out->length + 4 > CROFT_BYTES_CAPACITY) {
        return 0;
    }
    for (index = 0; index < 4; index++) {
        out->items[out->length] = (unsigned char) (bits & 0xFFUL);
        out->length++;
        bits >>= 8;
    }
    return 1;
}

croft_bool croft_bytes_take_int(const croft_bytes *in, croft_u16 *at, croft_int *out)
{
    unsigned long bits = 0;
    int index;
    if (*at + 4 > in->length) {
        return 0;
    }
    for (index = 3; index >= 0; index--) {
        bits = (bits << 8) | in->items[*at + index];
    }
    *at = (croft_u16) (*at + 4);
    /* Back from unsigned without relying on the compiler to wrap. */
    *out = bits > 0x7FFFFFFFUL ? -(croft_int) (0xFFFFFFFFUL - bits) - 1 : (croft_int) bits;
    return 1;
}

croft_bool croft_bytes_put_byte(croft_bytes *out, croft_u8 value)
{
    if (out->length + 1 > CROFT_BYTES_CAPACITY) {
        return 0;
    }
    out->items[out->length] = value;
    out->length++;
    return 1;
}

croft_bool croft_bytes_take_byte(const croft_bytes *in, croft_u16 *at, croft_u8 *out)
{
    if (*at + 1 > in->length) {
        return 0;
    }
    *out = in->items[*at];
    *at = (croft_u16) (*at + 1);
    return 1;
}

croft_bool croft_bytes_put_length(croft_bytes *out, croft_u16 length)
{
    if (out->length + 2 > CROFT_BYTES_CAPACITY) {
        return 0;
    }
    out->items[out->length] = (unsigned char) (length & 0xFF);
    out->items[out->length + 1] = (unsigned char) (length >> 8);
    out->length = (croft_u16) (out->length + 2);
    return 1;
}

croft_bool croft_bytes_take_length(const croft_bytes *in, croft_u16 *at, croft_u16 *out)
{
    if (*at + 2 > in->length) {
        return 0;
    }
    *out = (croft_u16) (in->items[*at] | (in->items[*at + 1] << 8));
    *at = (croft_u16) (*at + 2);
    return 1;
}

croft_bool croft_bytes_put_string(croft_bytes *out, croft_string_view text)
{
    croft_u16 index;
    if (!croft_bytes_put_length(out, text.length) || out->length + text.length > CROFT_BYTES_CAPACITY) {
        return 0;
    }
    for (index = 0; index < text.length; index++) {
        out->items[out->length] = (unsigned char) text.text[index];
        out->length++;
    }
    return 1;
}

/* Fails when the saved text is longer than there is room for. */
croft_bool croft_bytes_take_string(const croft_bytes *in, croft_u16 *at, char *text, croft_u16 *length,
                                   croft_u16 capacity)
{
    croft_u16 index;
    if (!croft_bytes_take_length(in, at, length) || *length > capacity || *at + *length > in->length) {
        return 0;
    }
    for (index = 0; index < *length; index++) {
        text[index] = (char) in->items[*at + index];
    }
    *at = (croft_u16) (*at + *length);
    return 1;
}

croft_bool croft_bytes_put_bytes(croft_bytes *out, croft_bytes_view bytes)
{
    croft_u16 index;
    if (!croft_bytes_put_length(out, bytes.length) || out->length + bytes.length > CROFT_BYTES_CAPACITY) {
        return 0;
    }
    for (index = 0; index < bytes.length; index++) {
        out->items[out->length] = bytes.items[index];
        out->length++;
    }
    return 1;
}

croft_bool croft_bytes_take_bytes(const croft_bytes *in, croft_u16 *at, unsigned char *items, croft_u16 *length,
                                  croft_u16 capacity)
{
    croft_u16 index;
    if (!croft_bytes_take_length(in, at, length) || *length > capacity || *at + *length > in->length) {
        return 0;
    }
    for (index = 0; index < *length; index++) {
        items[index] = in->items[*at + index];
    }
    *at = (croft_u16) (*at + *length);
    return 1;
}

/* The key an aggregate is kept under: the repository's prefix and its
   identity, short enough for the 8.3 names of FAT disks. */
void croft_storage_key(const char *prefix, croft_int identity, croft_string *out)
{
    croft_string number;
    croft_string_set(prefix, CROFT_OUT(*out));
    croft_int_to_string(identity, CROFT_OUT(number));
    croft_string_join(CROFT_VIEW(*out), CROFT_VIEW(number), CROFT_OUT(*out));
}
