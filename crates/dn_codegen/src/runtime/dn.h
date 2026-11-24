/* The dn runtime. Every generated program includes this header.
   Written in C89 so it builds with cc65, z88dk and old Atari ST compilers. */
#ifndef DN_H
#define DN_H

typedef long dn_int;
typedef unsigned char dn_bool;
typedef unsigned char dn_unit;
typedef unsigned char dn_u8;
typedef signed char dn_i8;
typedef unsigned short dn_u16;
typedef short dn_i16;
typedef long dn_i32;

#ifndef DN_STRING_CAPACITY
#define DN_STRING_CAPACITY 32
#endif

typedef struct {
    dn_u16 length;
    char text[DN_STRING_CAPACITY];
} dn_string;

#define DN_MAX(a, b) ((a) > (b) ? (a) : (b))
#define DN_MIN(a, b) ((a) < (b) ? (a) : (b))
#define DN_ABS(a) ((a) < 0 ? -(a) : (a))

/* Stops the program when a contract is broken. */
void dn_fail(const char *message);

#ifdef DN_DEBUG
#define DN_CHECK(condition, message) if (!(condition)) dn_fail(message)
#else
#define DN_CHECK(condition, message)
#endif

/* Sets every byte of a value to zero, so unused parts are never read unset. */
void dn_clear(void *value, unsigned int size);

void dn_string_set(dn_string *out, const char *text);
dn_bool dn_string_equal(const dn_string *a, const dn_string *b);
void dn_string_join(const dn_string *a, const dn_string *b, dn_string *out);
dn_int dn_string_find(const dn_string *text, const dn_string *part, dn_int from);
dn_bool dn_string_starts_with(const dn_string *text, const dn_string *part);
dn_bool dn_string_ends_with(const dn_string *text, const dn_string *part);
void dn_string_to_upper(const dn_string *text, dn_string *out);
void dn_string_to_lower(const dn_string *text, dn_string *out);
void dn_string_trim(const dn_string *text, dn_string *out);
void dn_string_slice(const dn_string *text, dn_int start, dn_int end, dn_string *out);
void dn_int_to_string(dn_int value, dn_string *out);
dn_bool dn_string_to_int(const dn_string *text, dn_int *out);

void dn_arguments_set(int count, char **values);
dn_int dn_argument_count(void);
void dn_argument(dn_int index, dn_string *out);

#endif
