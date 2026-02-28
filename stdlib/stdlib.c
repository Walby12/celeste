#include <stdio.h>

int get_int() {
    int val = 0;
    if (scanf("%d", &val) != 1) {
        return 0;
    }
    return val;
}
