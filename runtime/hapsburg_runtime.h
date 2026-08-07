#ifndef HAPSBURG_RUNTIME_H
#define HAPSBURG_RUNTIME_H

/*
 * The Hapsburg runtime.
 *
 * v1's memory model is deliberately the simplest one that can be honest:
 * every birth() is a heap allocation with process lifetime, nothing is
 * freed. The pitch's refcounted "cause_of_death" / Hemophilia model is
 * real future work, not implemented here — see README.
 */

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include <sys/types.h> /* ssize_t, for hb_correspondence_receive_line's getline() */

/* ---- lists ---- */

typedef struct {
    int64_t *data;
    int64_t len;
    int64_t cap;
} HbListInt;

typedef struct {
    char **data;
    int64_t len;
    int64_t cap;
} HbListStr;

typedef struct {
    HbListInt *data;
    int64_t len;
    int64_t cap;
} HbListListInt;

HbListInt hb_list_int_new(void);
void hb_list_int_push(HbListInt *l, int64_t v);
int64_t hb_list_int_get(HbListInt *l, int64_t idx);
int64_t hb_list_int_max(HbListInt *l);
int64_t hb_list_int_min(HbListInt *l);

HbListStr hb_list_str_new(void);
void hb_list_str_push(HbListStr *l, char *v);
char *hb_list_str_get(HbListStr *l, int64_t idx);

HbListListInt hb_list_list_int_new(void);
void hb_list_list_int_push(HbListListInt *l, HbListInt v);
HbListInt hb_list_list_int_get(HbListListInt *l, int64_t idx);

/* ---- strings / integers ---- */

int64_t hb_integer_parse(const char *s);
HbListStr hb_string_chars(const char *s);
HbListStr hb_string_lines(const char *s);
HbListStr hb_string_split_whitespace(const char *s);
int64_t hb_string_length(const char *s);

/* ---- marry(): Hapsburg's type-cast operator ---- */

char *hb_integer_to_string(int64_t x);
char *hb_bool_to_string(int b);

/* ---- Habsburg::Correspondence: reading stdin ---- */

/* One line, trailing newline stripped, "" at EOF. */
char *hb_correspondence_receive_line(void);
/* Everything until EOF, newlines preserved (for multi-line puzzle input). */
char *hb_correspondence_receive_all(void);

/* ---- misc builtins ---- */

int64_t hb_abs(int64_t x);
void hb_print_int(int64_t x);
void hb_print_str(const char *s);

/* Every unhandled Hapsburg exception is, thematically, an InbreedingError:
 * the hierarchy produced no viable heir. Prints a pedigree-flavored
 * message and terminates — there is no catch/recover yet (future work). */
void hb_assassinate(const char *exception_class, const char *reason) __attribute__((noreturn));

/* ---- Habsburg::Accumulator, a compiler-provided regency ---- */

typedef struct {
    int64_t value;
} HbAccumulator;

HbAccumulator *hb_accumulator_new(int64_t seed);
void hb_accumulator_absorb(HbAccumulator *a, int64_t x);

#endif
