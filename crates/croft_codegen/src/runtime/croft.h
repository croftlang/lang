/* The Croft runtime. Every generated program includes this header.
   Written in C89 so it builds with cc65, z88dk and old Atari ST compilers. */
#ifndef CROFT_H
#define CROFT_H

typedef long croft_int;
typedef unsigned char croft_bool;
typedef unsigned char croft_unit;
typedef unsigned char croft_u8;
typedef signed char croft_i8;
typedef unsigned short croft_u16;
typedef short croft_i16;
typedef long croft_i32;

#ifndef CROFT_STRING_CAPACITY
#define CROFT_STRING_CAPACITY 32
#endif

typedef struct {
    croft_u16 length;
    char text[CROFT_STRING_CAPACITY];
} croft_string;

/* Text borrowed for the length of a call, without copying it: a string
   literal, a string parameter, or the characters of a string kept elsewhere.
   Every string passed into a function arrives as one of these. */
typedef struct {
    const char *text;
    croft_u16 length;
} croft_string_view;

/* Every string, a full croft_string or a croft_text_N of its own size, has
   a length and its characters. A string is read through CROFT_VIEW, and
   written by a runtime function given CROFT_OUT: where its characters go,
   where its length goes, and how many characters there is room for. */
#define CROFT_VIEW(s) croft_string_view_from((s).text, (s).length)
#define CROFT_OUT(s) (s).text, &(s).length, (croft_u16) sizeof (s).text

#ifndef CROFT_BYTES_CAPACITY
#define CROFT_BYTES_CAPACITY 64
#endif

typedef struct {
    croft_u16 length;
    unsigned char items[CROFT_BYTES_CAPACITY];
} croft_bytes;

/* Bytes borrowed for the length of a call, as croft_string_view borrows
   text: the values of a full croft_bytes, or of a croft_bytes_N kept in a
   size of its own. A Bytes is read through CROFT_BYTES_VIEW, and written by
   a runtime function given CROFT_BYTES_OUT: where its values go, where its
   length goes, and how many values there is room for. */
typedef struct {
    const unsigned char *items;
    croft_u16 length;
} croft_bytes_view;

#define CROFT_BYTES_VIEW(b) croft_bytes_view_from((b).items, (b).length)
#define CROFT_BYTES_OUT(b) (b).items, &(b).length, (croft_u16) sizeof (b).items

#define CROFT_MAX(a, b) ((a) > (b) ? (a) : (b))
#define CROFT_MIN(a, b) ((a) < (b) ? (a) : (b))
#define CROFT_ABS(a) ((a) < 0 ? -(a) : (a))
/* Through unsigned long, since C leaves shifting a negative number left undefined. */
#define CROFT_SHIFT_LEFT(value, amount) ((croft_int) ((unsigned long) (value) << (amount)))
/* C89 leaves shifting a negative number right up to the compiler, so the
   sign is kept by hand: ~ makes it positive, and ~ again after the shift. */
#define CROFT_SHIFT_RIGHT(value, amount) \
    ((croft_int) ((value) < 0 ? ~(~(value) >> (amount)) : (value) >> (amount)))

/* Stops the program when a contract is broken. */
void croft_fail(const char *message);

/* Whether the constraints the compiler could not work out are checked while
   the program runs. Croft writes this from the target's `checks` setting. */
#ifndef CROFT_CHECKS
#define CROFT_CHECKS
#endif

#ifdef CROFT_CHECKS
#define CROFT_CHECK(condition, message) if (!(condition)) croft_fail(message)
#else
/* Not worked out, but still a use of what it names, for the compiler. */
#define CROFT_CHECK(condition, message) (void) sizeof(condition)
#endif

/* A parameter whose Range fits a smaller type is passed in that type, but as
   a croft_int when checks are on, so its check sees the number as it was given. */
#ifdef CROFT_CHECKS
#define CROFT_RANGED(type) croft_int
#else
#define CROFT_RANGED(type) type
#endif

/* Sets every byte of a value to zero, so unused parts are never read unset. */
void croft_clear(void *value, unsigned int size);

void croft_string_set(const char *text, char *out, croft_u16 *length, croft_u16 capacity);
croft_string_view croft_string_view_from(const char *text, croft_u16 length);
croft_bool croft_string_copy(croft_string_view text, char *out, croft_u16 *length, croft_u16 capacity);
croft_bool croft_string_equal(croft_string_view a, croft_string_view b);
void croft_string_join(croft_string_view a, croft_string_view b, char *out, croft_u16 *length, croft_u16 capacity);
croft_int croft_string_find(croft_string_view text, croft_string_view part, croft_int from);
croft_bool croft_string_starts_with(croft_string_view text, croft_string_view part);
croft_bool croft_string_ends_with(croft_string_view text, croft_string_view part);
void croft_string_to_upper(croft_string_view text, char *out, croft_u16 *length, croft_u16 capacity);
void croft_string_to_lower(croft_string_view text, char *out, croft_u16 *length, croft_u16 capacity);
croft_string_view croft_string_trimmed(croft_string_view text);
croft_string_view croft_string_part(croft_string_view text, croft_int start, croft_int end);
void croft_string_replace(croft_string_view text, croft_string_view from, croft_string_view to, char *out,
                          croft_u16 *length, croft_u16 capacity);
void croft_string_repeat(croft_string_view text, croft_int count, char *out, croft_u16 *length, croft_u16 capacity);
void croft_string_pad(croft_string_view text, croft_int width, croft_string_view fill, croft_bool before, char *out,
                      croft_u16 *length, croft_u16 capacity);
croft_string_view croft_string_line(croft_string_view text, croft_u16 *at);
int croft_string_compare(croft_string_view a, croft_string_view b);
void croft_int_to_string(croft_int value, char *out, croft_u16 *length, croft_u16 capacity);

croft_int croft_divide_even(croft_int numerator, croft_int denominator);
croft_int croft_decimal_multiply(croft_int a, croft_int b, croft_int scale);
croft_int croft_decimal_divide(croft_int a, croft_int b, croft_int scale);
void croft_decimal_to_string(croft_int value, croft_int places, char *out, croft_u16 *length, croft_u16 capacity);

croft_bytes_view croft_bytes_view_from(const unsigned char *items, croft_u16 length);
croft_bool croft_bytes_copy(croft_bytes_view bytes, unsigned char *out, croft_u16 *length, croft_u16 capacity);
croft_bool croft_bytes_equal(croft_bytes_view a, croft_bytes_view b);
void croft_bytes_join(croft_bytes_view a, croft_bytes_view b, unsigned char *out, croft_u16 *length, croft_u16 capacity);
croft_bool croft_bytes_get(croft_bytes_view bytes, croft_int index, croft_int *out);
void croft_bytes_add(croft_bytes_view bytes, croft_int byte, unsigned char *out, croft_u16 *length, croft_u16 capacity);
void croft_bytes_set(croft_bytes_view bytes, croft_int index, croft_int byte, unsigned char *out, croft_u16 *length,
                     croft_u16 capacity);
croft_bool croft_bytes_get_u16(croft_bytes_view bytes, croft_int index, croft_bool big, croft_int *out);
void croft_bytes_add_u16(croft_bytes_view bytes, croft_int value, croft_bool big, unsigned char *out,
                         croft_u16 *length, croft_u16 capacity);
void croft_bytes_slice(croft_bytes_view bytes, croft_int start, croft_int end, unsigned char *out, croft_u16 *length,
                       croft_u16 capacity);
void croft_bytes_to_text(croft_bytes_view bytes, char *out, croft_u16 *length, croft_u16 capacity);
void croft_string_to_bytes(croft_string_view text, unsigned char *out, croft_u16 *length, croft_u16 capacity);
croft_bool croft_bytes_put_int(croft_bytes *out, croft_int value);
croft_bool croft_bytes_take_int(const croft_bytes *in, croft_u16 *at, croft_int *out);
croft_bool croft_bytes_put_byte(croft_bytes *out, croft_u8 value);
croft_bool croft_bytes_take_byte(const croft_bytes *in, croft_u16 *at, croft_u8 *out);
croft_bool croft_bytes_put_length(croft_bytes *out, croft_u16 length);
croft_bool croft_bytes_take_length(const croft_bytes *in, croft_u16 *at, croft_u16 *out);
croft_bool croft_bytes_put_string(croft_bytes *out, croft_string_view text);
croft_bool croft_bytes_take_string(const croft_bytes *in, croft_u16 *at, char *text, croft_u16 *length,
                                   croft_u16 capacity);
croft_bool croft_bytes_put_bytes(croft_bytes *out, croft_bytes_view bytes);
croft_bool croft_bytes_take_bytes(const croft_bytes *in, croft_u16 *at, unsigned char *items, croft_u16 *length,
                                  croft_u16 capacity);
void croft_storage_key(const char *prefix, croft_int identity, croft_string *out);

croft_bool croft_string_to_int(croft_string_view text, croft_int *out);

void croft_arguments_set(int count, char **values);
croft_int croft_argument_count(void);
void croft_argument(croft_int index, char *out, croft_u16 *length, croft_u16 capacity);

/* The built-in Console port: prints a line. */
void croft_console_print(croft_string_view text, croft_unit *out);

#endif
