#define _GNU_SOURCE 1
#include <sys/syscall.h>
#include <unistd.h>

int renameat2(int oldfd, const char *old, int newfd, const char *new, unsigned int flags) {
    return syscall(SYS_renameat2, oldfd, old, newfd, new, flags);
}
