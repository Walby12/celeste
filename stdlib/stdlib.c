#include <string.h>
#include <stdint.h>

int64_t c_strlen(const char* s) {
    if (!s) return 0;
    return (int64_t)strlen(s);
}

int64_t c_strcmp(const char* s1, const char* s2) {
    return (int64_t)strcmp(s1, s2);
}

int64_t c_strequal(const char* s1, const char* s2) {
    if (strcmp(s1, s2) == 0) return 1;
    return 0;
}

void c_strcpy(char* dest, const char* src) {
    strcpy(dest, src);
}

void c_strcat(char* dest, const char* src) {
    strcat(dest, src);
}
