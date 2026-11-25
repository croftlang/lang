
/* Text kept in a string field of its own size: a length and its
   characters, copied to and from a full croft_string. */

void croft_text_load(const char *text, croft_u16 length, croft_string *out)
{
    croft_u16 index;
    out->length = length;
    for (index = 0; index < length; index++) {
        out->text[index] = text[index];
    }
}

/* Copies as much as fits, and gives back whether all of it did. */
croft_bool croft_text_store(const croft_string *in, char *text, croft_u16 *length, croft_u16 capacity)
{
    croft_u16 index;
    *length = in->length < capacity ? in->length : capacity;
    for (index = 0; index < *length; index++) {
        text[index] = in->text[index];
    }
    return in->length <= capacity;
}

croft_bool croft_text_equal(const char *a, croft_u16 a_length, const char *b, croft_u16 b_length)
{
    croft_u16 index;
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

croft_bool croft_bytes_put_text(croft_bytes *out, const char *text, croft_u16 length)
{
    croft_u16 index;
    if (!croft_bytes_put_length(out, length) || out->length + length > CROFT_BYTES_CAPACITY) {
        return 0;
    }
    for (index = 0; index < length; index++) {
        out->items[out->length] = (unsigned char) text[index];
        out->length++;
    }
    return 1;
}

croft_bool croft_bytes_take_text(const croft_bytes *in, croft_u16 *at, char *text, croft_u16 *length,
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
