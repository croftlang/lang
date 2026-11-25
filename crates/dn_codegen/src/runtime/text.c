
/* Text kept in a string field of its own size: a length and its
   characters, copied to and from a full dn_string. */

void dn_text_load(const char *text, dn_u16 length, dn_string *out)
{
    dn_u16 index;
    out->length = length;
    for (index = 0; index < length; index++) {
        out->text[index] = text[index];
    }
}

/* Copies as much as fits, and gives back whether all of it did. */
dn_bool dn_text_store(const dn_string *in, char *text, dn_u16 *length, dn_u16 capacity)
{
    dn_u16 index;
    *length = in->length < capacity ? in->length : capacity;
    for (index = 0; index < *length; index++) {
        text[index] = in->text[index];
    }
    return in->length <= capacity;
}

dn_bool dn_text_equal(const char *a, dn_u16 a_length, const char *b, dn_u16 b_length)
{
    dn_u16 index;
    if (a_length != b_length) {
        return 0;
    }
    for (index = 0; index < a_length; index++) {
        if (a[index] != b[index]) {
            return 0;
        }
    }
    return 1;
}

dn_bool dn_bytes_put_text(dn_bytes *out, const char *text, dn_u16 length)
{
    dn_u16 index;
    if (!dn_bytes_put_length(out, length) || out->length + length > DN_BYTES_CAPACITY) {
        return 0;
    }
    for (index = 0; index < length; index++) {
        out->items[out->length] = (unsigned char) text[index];
        out->length++;
    }
    return 1;
}

dn_bool dn_bytes_take_text(const dn_bytes *in, dn_u16 *at, char *text, dn_u16 *length,
                           dn_u16 capacity)
{
    dn_u16 index;
    if (!dn_bytes_take_length(in, at, length) || *length > capacity || *at + *length > in->length) {
        return 0;
    }
    for (index = 0; index < *length; index++) {
        text[index] = (char) in->items[*at + index];
    }
    *at = (dn_u16) (*at + *length);
    return 1;
}
