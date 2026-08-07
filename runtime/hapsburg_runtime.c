#include "hapsburg_runtime.h"

/* ---- HbListInt ---- */

HbListInt hb_list_int_new(void) {
    HbListInt l;
    l.len = 0;
    l.cap = 8;
    l.data = (int64_t *)malloc(sizeof(int64_t) * l.cap);
    return l;
}

void hb_list_int_push(HbListInt *l, int64_t v) {
    if (l->len >= l->cap) {
        l->cap *= 2;
        l->data = (int64_t *)realloc(l->data, sizeof(int64_t) * l->cap);
    }
    l->data[l->len++] = v;
}

int64_t hb_list_int_get(HbListInt *l, int64_t idx) {
    if (idx < 0 || idx >= l->len) {
        hb_assassinate("InbreedingError", "list index out of range");
    }
    return l->data[idx];
}

int64_t hb_list_int_max(HbListInt *l) {
    if (l->len == 0) hb_assassinate("InbreedingError", "max() of an empty lineage");
    int64_t m = l->data[0];
    for (int64_t i = 1; i < l->len; i++) if (l->data[i] > m) m = l->data[i];
    return m;
}

int64_t hb_list_int_min(HbListInt *l) {
    if (l->len == 0) hb_assassinate("InbreedingError", "min() of an empty lineage");
    int64_t m = l->data[0];
    for (int64_t i = 1; i < l->len; i++) if (l->data[i] < m) m = l->data[i];
    return m;
}

/* ---- HbListStr ---- */

HbListStr hb_list_str_new(void) {
    HbListStr l;
    l.len = 0;
    l.cap = 8;
    l.data = (char **)malloc(sizeof(char *) * l.cap);
    return l;
}

void hb_list_str_push(HbListStr *l, char *v) {
    if (l->len >= l->cap) {
        l->cap *= 2;
        l->data = (char **)realloc(l->data, sizeof(char *) * l->cap);
    }
    l->data[l->len++] = v;
}

char *hb_list_str_get(HbListStr *l, int64_t idx) {
    if (idx < 0 || idx >= l->len) {
        hb_assassinate("InbreedingError", "list index out of range");
    }
    return l->data[idx];
}

/* ---- HbListListInt ---- */

HbListListInt hb_list_list_int_new(void) {
    HbListListInt l;
    l.len = 0;
    l.cap = 8;
    l.data = (HbListInt *)malloc(sizeof(HbListInt) * l.cap);
    return l;
}

void hb_list_list_int_push(HbListListInt *l, HbListInt v) {
    if (l->len >= l->cap) {
        l->cap *= 2;
        l->data = (HbListInt *)realloc(l->data, sizeof(HbListInt) * l->cap);
    }
    l->data[l->len++] = v;
}

HbListInt hb_list_list_int_get(HbListListInt *l, int64_t idx) {
    if (idx < 0 || idx >= l->len) {
        hb_assassinate("InbreedingError", "list index out of range");
    }
    return l->data[idx];
}

/* ---- strings / integers ---- */

int64_t hb_integer_parse(const char *s) {
    char *end;
    long long v = strtoll(s, &end, 10);
    if (end == s) {
        hb_assassinate("InbreedingError", "could not parse an Integer from this line of descent");
    }
    return (int64_t)v;
}

HbListStr hb_string_chars(const char *s) {
    HbListStr out = hb_list_str_new();
    for (const char *p = s; *p; p++) {
        char *one = (char *)malloc(2);
        one[0] = *p;
        one[1] = '\0';
        hb_list_str_push(&out, one);
    }
    return out;
}

HbListStr hb_string_lines(const char *s) {
    HbListStr out = hb_list_str_new();
    const char *start = s;
    for (const char *p = s;; p++) {
        if (*p == '\n' || *p == '\0') {
            int64_t n = p - start;
            if (n > 0 || *p == '\0') {
                if (n > 0) {
                    char *line = (char *)malloc(n + 1);
                    memcpy(line, start, n);
                    line[n] = '\0';
                    hb_list_str_push(&out, line);
                }
            }
            start = p + 1;
            if (*p == '\0') break;
        }
    }
    return out;
}

HbListStr hb_string_split_whitespace(const char *s) {
    HbListStr out = hb_list_str_new();
    const char *p = s;
    while (*p) {
        while (*p && isspace((unsigned char)*p)) p++;
        if (!*p) break;
        const char *start = p;
        while (*p && !isspace((unsigned char)*p)) p++;
        int64_t n = p - start;
        char *tok = (char *)malloc(n + 1);
        memcpy(tok, start, n);
        tok[n] = '\0';
        hb_list_str_push(&out, tok);
    }
    return out;
}

int64_t hb_string_length(const char *s) {
    return (int64_t)strlen(s);
}

/* ---- marry(): Hapsburg's type-cast operator ---- */

char *hb_integer_to_string(int64_t x) {
    /* -9223372036854775808 plus NUL is 21 bytes; 32 is comfortable. */
    char *buf = (char *)malloc(32);
    snprintf(buf, 32, "%lld", (long long)x);
    return buf;
}

char *hb_bool_to_string(int b) {
    /* Static storage is fine: the runtime never frees anything, so a
     * literal is exactly as safe as a fresh allocation here, minus the
     * allocation. */
    return b ? "true" : "false";
}

/* ---- Habsburg::Correspondence: reading stdin ---- */

char *hb_correspondence_receive_line(void) {
    size_t cap = 0;
    char *line = NULL;
    ssize_t n = getline(&line, &cap, stdin);
    if (n < 0) {
        free(line);
        char *empty = (char *)malloc(1);
        empty[0] = '\0';
        return empty;
    }
    if (n > 0 && line[n - 1] == '\n') {
        line[n - 1] = '\0';
    }
    return line;
}

char *hb_correspondence_receive_all(void) {
    size_t cap = 4096;
    size_t len = 0;
    char *buf = (char *)malloc(cap);
    size_t got;
    while ((got = fread(buf + len, 1, cap - len, stdin)) > 0) {
        len += got;
        if (len == cap) {
            cap *= 2;
            buf = (char *)realloc(buf, cap);
        }
    }
    buf[len] = '\0';
    /* Trim a single trailing newline, same convention as receive_line. */
    if (len > 0 && buf[len - 1] == '\n') {
        buf[len - 1] = '\0';
    }
    return buf;
}

/* ---- misc ---- */

int64_t hb_abs(int64_t x) {
    return x < 0 ? -x : x;
}

void hb_print_int(int64_t x) {
    printf("%lld\n", (long long)x);
}

void hb_print_str(const char *s) {
    printf("%s\n", s);
}

void hb_assassinate(const char *exception_class, const char *reason) {
    fprintf(stderr, "%s: %s\n", exception_class, reason);
    fprintf(stderr, "  (the line of succession produced no viable heir here — see the pedigree above)\n");
    exit(1);
}

/* ---- Habsburg::Accumulator ---- */

HbAccumulator *hb_accumulator_new(int64_t seed) {
    HbAccumulator *a = (HbAccumulator *)malloc(sizeof(HbAccumulator));
    a->value = seed;
    return a;
}

void hb_accumulator_absorb(HbAccumulator *a, int64_t x) {
    a->value += x;
}
