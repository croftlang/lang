
/* Bytes: a length and values from 0 to 255, read through a croft_bytes_view
   and written through CROFT_BYTES_OUT, so any size can be read or written. */

croft_bytes_view croft_bytes_view_from(const unsigned char *items, croft_u16 length)
{
    croft_bytes_view view;
    view.items = items;
    view.length = length;
    return view;
}

/* Copies as many values as fit, forwards, so out may be where bytes is
   borrowed from. Gives back whether all of them fitted. */
croft_bool croft_bytes_copy(croft_bytes_view bytes, unsigned char *out, croft_u16 *length, croft_u16 capacity)
{
    croft_u16 index;
    croft_u16 count = bytes.length < capacity ? bytes.length : capacity;
    for (index = 0; index < count; index++) {
        out[index] = bytes.items[index];
    }
    *length = count;
    return bytes.length <= capacity;
}

croft_bool croft_bytes_equal(croft_bytes_view a, croft_bytes_view b)
{
    croft_u16 index;
    if (a.length != b.length) {
        return 0;
    }
    for (index = 0; index < a.length; index++) {
        if (a.items[index] != b.items[index]) {
            return 0;
        }
    }
    return 1;
}

/* a and then b. The values of b are copied last to first, and a only when
   it is somewhere else, so out may be where a is borrowed from. */
void croft_bytes_join(croft_bytes_view a, croft_bytes_view b, unsigned char *out, croft_u16 *length, croft_u16 capacity)
{
    croft_u16 index;
    croft_u16 kept = a.length < capacity ? a.length : capacity;
    croft_u16 added = b.length < capacity - kept ? b.length : (croft_u16) (capacity - kept);
    for (index = added; index > 0; index--) {
        out[kept + index - 1] = b.items[index - 1];
    }
    if (a.items != out) {
        for (index = 0; index < kept; index++) {
            out[index] = a.items[index];
        }
    }
    *length = (croft_u16) (kept + added);
    CROFT_CHECK(a.length + b.length <= capacity, "joined bytes are longer than bytes can hold");
}

croft_bool croft_bytes_get(croft_bytes_view bytes, croft_int index, croft_int *out)
{
    if (index < 0 || index >= (croft_int) bytes.length) {
        *out = 0;
        return 0;
    }
    *out = bytes.items[index];
    return 1;
}

void croft_bytes_add(croft_bytes_view bytes, croft_int byte, unsigned char *out, croft_u16 *length, croft_u16 capacity)
{
    croft_bytes_copy(bytes, out, length, capacity);
    CROFT_CHECK(byte >= 0 && byte <= 255, "a byte is from 0 to 255");
    CROFT_CHECK(bytes.length < capacity, "bytes are full");
    if (bytes.length < capacity) {
        out[*length] = (unsigned char) byte;
        (*length)++;
    }
}

/* Copies forwards, so out may be where bytes is borrowed from. */
void croft_bytes_slice(croft_bytes_view bytes, croft_int start, croft_int end, unsigned char *out, croft_u16 *length,
                       croft_u16 capacity)
{
    croft_int index;
    if (start < 0) {
        start = 0;
    }
    if (end > (croft_int) bytes.length) {
        end = bytes.length;
    }
    *length = 0;
    for (index = start; index < end && *length < capacity; index++) {
        out[*length] = bytes.items[index];
        (*length)++;
    }
}

/* Each byte becomes one character, up to the string's capacity. */
void croft_bytes_to_text(croft_bytes_view bytes, char *out, croft_u16 *length, croft_u16 capacity)
{
    croft_u16 index;
    *length = 0;
    for (index = 0; index < bytes.length && *length < capacity; index++) {
        out[*length] = (char) bytes.items[index];
        (*length)++;
    }
    CROFT_CHECK(index == bytes.length, "bytes are longer than a string can hold");
}

void croft_string_to_bytes(croft_string_view text, unsigned char *out, croft_u16 *length, croft_u16 capacity)
{
    croft_u16 index;
    *length = 0;
    for (index = 0; index < text.length && *length < capacity; index++) {
        out[*length] = (unsigned char) text.text[index];
        (*length)++;
    }
    CROFT_CHECK(index == text.length, "text is longer than bytes can hold");
}
