
/* Writing and reading saved values, for FileStorage. A number is four
   bytes, least significant first; a length is two. Each gives back 0 when
   the bytes are full, or have run out. */

dn_bool dn_bytes_put_int(dn_bytes *out, dn_int value)
{
    unsigned long bits = (unsigned long) value;
    int index;
    if (out->length + 4 > DN_BYTES_CAPACITY) {
        return 0;
    }
    for (index = 0; index < 4; index++) {
        out->items[out->length] = (unsigned char) (bits & 0xFFUL);
        out->length++;
        bits >>= 8;
    }
    return 1;
}

dn_bool dn_bytes_take_int(const dn_bytes *in, dn_u16 *at, dn_int *out)
{
    unsigned long bits = 0;
    int index;
    if (*at + 4 > in->length) {
        return 0;
    }
    for (index = 3; index >= 0; index--) {
        bits = (bits << 8) | in->items[*at + index];
    }
    *at = (dn_u16) (*at + 4);
    /* Back from unsigned without relying on the compiler to wrap. */
    *out = bits > 0x7FFFFFFFUL ? -(dn_int) (0xFFFFFFFFUL - bits) - 1 : (dn_int) bits;
    return 1;
}

dn_bool dn_bytes_put_length(dn_bytes *out, dn_u16 length)
{
    if (out->length + 2 > DN_BYTES_CAPACITY) {
        return 0;
    }
    out->items[out->length] = (unsigned char) (length & 0xFF);
    out->items[out->length + 1] = (unsigned char) (length >> 8);
    out->length = (dn_u16) (out->length + 2);
    return 1;
}

dn_bool dn_bytes_take_length(const dn_bytes *in, dn_u16 *at, dn_u16 *out)
{
    if (*at + 2 > in->length) {
        return 0;
    }
    *out = (dn_u16) (in->items[*at] | (in->items[*at + 1] << 8));
    *at = (dn_u16) (*at + 2);
    return 1;
}

dn_bool dn_bytes_put_string(dn_bytes *out, const dn_string *text)
{
    dn_u16 index;
    if (!dn_bytes_put_length(out, text->length) || out->length + text->length > DN_BYTES_CAPACITY) {
        return 0;
    }
    for (index = 0; index < text->length; index++) {
        out->items[out->length] = (unsigned char) text->text[index];
        out->length++;
    }
    return 1;
}

dn_bool dn_bytes_take_string(const dn_bytes *in, dn_u16 *at, dn_string *out)
{
    dn_u16 index;
    if (!dn_bytes_take_length(in, at, &out->length) || out->length > DN_STRING_CAPACITY
        || *at + out->length > in->length) {
        return 0;
    }
    for (index = 0; index < out->length; index++) {
        out->text[index] = (char) in->items[*at + index];
    }
    *at = (dn_u16) (*at + out->length);
    return 1;
}

dn_bool dn_bytes_put_bytes(dn_bytes *out, const dn_bytes *bytes)
{
    dn_u16 index;
    if (!dn_bytes_put_length(out, bytes->length) || out->length + bytes->length > DN_BYTES_CAPACITY) {
        return 0;
    }
    for (index = 0; index < bytes->length; index++) {
        out->items[out->length] = bytes->items[index];
        out->length++;
    }
    return 1;
}

dn_bool dn_bytes_take_bytes(const dn_bytes *in, dn_u16 *at, dn_bytes *out)
{
    dn_u16 index;
    if (!dn_bytes_take_length(in, at, &out->length) || out->length > DN_BYTES_CAPACITY
        || *at + out->length > in->length) {
        return 0;
    }
    for (index = 0; index < out->length; index++) {
        out->items[index] = in->items[*at + index];
    }
    *at = (dn_u16) (*at + out->length);
    return 1;
}

/* The file an aggregate is saved in: the repository's prefix, its identity
   and ".dat", which fits the 8.3 names of FAT disks. */
void dn_storage_path(const char *prefix, dn_int identity, dn_string *out)
{
    dn_string number;
    dn_string_set(out, prefix);
    dn_int_to_string(identity, &number);
    dn_string_join(out, &number, out);
    dn_string_set(&number, ".dat");
    dn_string_join(out, &number, out);
}
