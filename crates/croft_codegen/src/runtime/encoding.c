
/* Std.Encoding: hex, base64 and percent-encoding, each written up to the
   room the result has and checked like any other string or bytes, and the
   CRC-16/CCITT-FALSE and CRC-32 checksums, worked out a bit at a time so no
   table takes memory on an 8-bit machine. */

/* Each byte as two upper-case hex digits. */
void croft_bytes_to_hex(croft_bytes_view bytes, char *out, croft_u16 *length, croft_u16 capacity)
{
    static const char digits[] = "0123456789ABCDEF";
    croft_u16 index;
    *length = 0;
    for (index = 0; index < bytes.length && capacity - *length >= 2; index++) {
        out[*length] = digits[bytes.items[index] >> 4];
        out[*length + 1] = digits[bytes.items[index] & 0x0F];
        *length = (croft_u16) (*length + 2);
    }
    CROFT_CHECK(index == bytes.length, "hex text is longer than a string can hold");
}

/* The bytes two hex digits each stand for, or nothing for an odd number of
   digits or anything that is not a digit. */
croft_bool croft_bytes_from_hex(croft_string_view text, unsigned char *out, croft_u16 *length, croft_u16 capacity)
{
    croft_u16 index;
    int high;
    int low;
    *length = 0;
    if (text.length % 2 != 0) {
        return 0;
    }
    for (index = 0; index < text.length; index = (croft_u16) (index + 2)) {
        high = croft_hex_digit(text.text[index]);
        low = croft_hex_digit(text.text[index + 1]);
        if (high < 0 || low < 0) {
            *length = 0;
            return 0;
        }
        if (*length < capacity) {
            out[*length] = (unsigned char) (high * 16 + low);
            (*length)++;
        }
    }
    CROFT_CHECK(text.length / 2 <= capacity, "decoded bytes are longer than bytes can hold");
    return 1;
}

/* The value of a base64 digit in the standard alphabet, or -1 when c is not
   one. */
static int croft_base64_digit(char c)
{
    if (c >= 'A' && c <= 'Z') {
        return c - 'A';
    }
    if (c >= 'a' && c <= 'z') {
        return c - 'a' + 26;
    }
    if (c >= '0' && c <= '9') {
        return c - '0' + 52;
    }
    if (c == '+') {
        return 62;
    }
    if (c == '/') {
        return 63;
    }
    return -1;
}

/* Every three bytes as four characters of the standard base64 alphabet, the
   last group padded with = (RFC 4648). */
void croft_bytes_to_base64(croft_bytes_view bytes, char *out, croft_u16 *length, croft_u16 capacity)
{
    static const char alphabet[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    croft_u16 index = 0;
    croft_u16 left;
    unsigned char first;
    unsigned char second;
    unsigned char third;
    *length = 0;
    while (index < bytes.length && capacity - *length >= 4) {
        left = (croft_u16) (bytes.length - index);
        first = bytes.items[index];
        second = left > 1 ? bytes.items[index + 1] : 0;
        third = left > 2 ? bytes.items[index + 2] : 0;
        out[*length] = alphabet[first >> 2];
        out[*length + 1] = alphabet[((first & 0x03) << 4) | (second >> 4)];
        out[*length + 2] = left > 1 ? alphabet[((second & 0x0F) << 2) | (third >> 6)] : '=';
        out[*length + 3] = left > 2 ? alphabet[third & 0x3F] : '=';
        *length = (croft_u16) (*length + 4);
        index = (croft_u16) (index + (left < 3 ? left : 3));
    }
    CROFT_CHECK(index == bytes.length, "base64 text is longer than a string can hold");
}

/* The bytes base64 text in the standard alphabet stands for, or nothing when
   it is not that: a length that is not a multiple of 4, a character outside
   the alphabet, = anywhere but the end, or bits after the last byte that are
   not 0. */
croft_bool croft_bytes_from_base64(croft_string_view text, unsigned char *out, croft_u16 *length, croft_u16 capacity)
{
    croft_u16 index;
    int part[4];
    unsigned char decoded[3];
    int place;
    int padding;
    croft_bool fits = 1;
    *length = 0;
    if (text.length % 4 != 0) {
        return 0;
    }
    for (index = 0; index < text.length; index = (croft_u16) (index + 4)) {
        padding = 0;
        if (index + 4 == text.length && text.text[index + 3] == '=') {
            padding = text.text[index + 2] == '=' ? 2 : 1;
        }
        for (place = 0; place < 4; place++) {
            part[place] = place < 4 - padding ? croft_base64_digit(text.text[index + place]) : 0;
            if (part[place] < 0) {
                *length = 0;
                return 0;
            }
        }
        if ((padding == 2 && (part[1] & 0x0F) != 0) || (padding == 1 && (part[2] & 0x03) != 0)) {
            *length = 0;
            return 0;
        }
        decoded[0] = (unsigned char) ((part[0] << 2) | (part[1] >> 4));
        decoded[1] = (unsigned char) (((part[1] & 0x0F) << 4) | (part[2] >> 2));
        decoded[2] = (unsigned char) (((part[2] & 0x03) << 6) | part[3]);
        for (place = 0; place < 3 - padding; place++) {
            if (*length < capacity) {
                out[*length] = decoded[place];
                (*length)++;
            } else {
                fits = 0;
            }
        }
    }
    CROFT_CHECK(fits, "decoded bytes are longer than bytes can hold");
    return 1;
}

/* Whether c is unreserved in a URL, kept as it is by percent-encoding:
   a letter, a digit, -, ., _ or ~ (RFC 3986). */
static croft_bool croft_is_unreserved(char c)
{
    return (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == '-' || c == '.' ||
           c == '_' || c == '~';
}

/* The text with every character but the unreserved ones written as % and
   two upper-case hex digits. */
void croft_string_percent_encode(croft_string_view text, char *out, croft_u16 *length, croft_u16 capacity)
{
    static const char digits[] = "0123456789ABCDEF";
    croft_u16 index;
    unsigned char c;
    *length = 0;
    for (index = 0; index < text.length; index++) {
        c = (unsigned char) text.text[index];
        if (croft_is_unreserved(text.text[index])) {
            if (capacity - *length < 1) {
                break;
            }
            out[*length] = text.text[index];
            (*length)++;
        } else {
            if (capacity - *length < 3) {
                break;
            }
            out[*length] = '%';
            out[*length + 1] = digits[c >> 4];
            out[*length + 2] = digits[c & 0x0F];
            *length = (croft_u16) (*length + 3);
        }
    }
    CROFT_CHECK(index == text.length, "percent-encoded text is longer than a string can hold");
}

/* The text with each % and two hex digits, in either case, made the
   character they stand for, or nothing when a % is not followed by two hex
   digits. A + stays a +. */
croft_bool croft_string_percent_decode(croft_string_view text, char *out, croft_u16 *length, croft_u16 capacity)
{
    croft_u16 index = 0;
    int high;
    int low;
    char c;
    *length = 0;
    while (index < text.length) {
        c = text.text[index];
        if (c == '%') {
            if (text.length - index < 3) {
                *length = 0;
                return 0;
            }
            high = croft_hex_digit(text.text[index + 1]);
            low = croft_hex_digit(text.text[index + 2]);
            if (high < 0 || low < 0) {
                *length = 0;
                return 0;
            }
            c = (char) (high * 16 + low);
            index = (croft_u16) (index + 3);
        } else {
            index++;
        }
        if (*length < capacity) {
            out[*length] = c;
            (*length)++;
        }
    }
    return 1;
}

/* CRC-16/CCITT-FALSE: polynomial 0x1021, starting from 0xFFFF, high bit
   first, so "123456789" gives 0x29B1. */
croft_int croft_bytes_crc16(croft_bytes_view bytes)
{
    unsigned int crc = 0xFFFFu;
    croft_u16 index;
    int bit;
    for (index = 0; index < bytes.length; index++) {
        crc ^= (unsigned int) bytes.items[index] << 8;
        for (bit = 0; bit < 8; bit++) {
            crc = (crc & 0x8000u) ? (crc << 1) ^ 0x1021u : crc << 1;
            crc &= 0xFFFFu;
        }
    }
    return (croft_int) crc;
}

/* CRC-32 as zip, PNG and Ethernet use it: polynomial 0x04C11DB7 read low
   bit first, starting from and finishing with 0xFFFFFFFF, so "123456789"
   gives 0xCBF43926. Its 32 bits are given as an Int, below zero when the
   top bit is set. */
croft_int croft_bytes_crc32(croft_bytes_view bytes)
{
    unsigned long crc = 0xFFFFFFFFUL;
    croft_u16 index;
    int bit;
    for (index = 0; index < bytes.length; index++) {
        crc ^= (unsigned long) bytes.items[index];
        for (bit = 0; bit < 8; bit++) {
            crc = (crc & 1UL) ? (crc >> 1) ^ 0xEDB88320UL : crc >> 1;
        }
    }
    return croft_int_from_bits(crc ^ 0xFFFFFFFFUL);
}
