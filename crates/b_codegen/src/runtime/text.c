
/* Text kept in a string field of its own size: a length and its
   characters, copied to and from a full b_string. */

void b_text_load(const char *text, b_u16 length, b_string *out)
{
    b_u16 index;
    out->length = length;
    for (index = 0; index < length; index++) {
        out->text[index] = text[index];
    }
}

/* Copies as much as fits, and gives back whether all of it did. */
b_bool b_text_store(const b_string *in, char *text, b_u16 *length, b_u16 capacity)
{
    b_u16 index;
    *length = in->length < capacity ? in->length : capacity;
    for (index = 0; index < *length; index++) {
        text[index] = in->text[index];
    }
    return in->length <= capacity;
}

b_bool b_text_equal(const char *a, b_u16 a_length, const char *b, b_u16 b_length)
{
    b_u16 index;
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

b_bool b_bytes_put_text(b_bytes *out, const char *text, b_u16 length)
{
    b_u16 index;
    if (!b_bytes_put_length(out, length) || out->length + length > B_BYTES_CAPACITY) {
        return 0;
    }
    for (index = 0; index < length; index++) {
        out->items[out->length] = (unsigned char) text[index];
        out->length++;
    }
    return 1;
}

b_bool b_bytes_take_text(const b_bytes *in, b_u16 *at, char *text, b_u16 *length,
                           b_u16 capacity)
{
    b_u16 index;
    if (!b_bytes_take_length(in, at, length) || *length > capacity || *at + *length > in->length) {
        return 0;
    }
    for (index = 0; index < *length; index++) {
        text[index] = (char) in->items[*at + index];
    }
    *at = (b_u16) (*at + *length);
    return 1;
}
