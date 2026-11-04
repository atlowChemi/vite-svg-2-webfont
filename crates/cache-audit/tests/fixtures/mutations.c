#define _GNU_SOURCE
#include <assert.h>
#include <fcntl.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>

int main(void) {
    int dir = open(".", O_RDONLY | O_DIRECTORY);
    assert(dir >= 0);
    int file = openat(dir, "source", O_CREAT | O_RDWR, 0600);
    assert(file >= 0);
    assert(write(file, "data", 4) == 4);
    assert(linkat(dir, "source", dir, "linked", 0) == 0);
    assert(syscall(SYS_linkat, dir, "source", dir, "direct-link", 0) == 0);
    assert(linkat(file, "", dir, "fd-link", AT_EMPTY_PATH) == 0);
    assert(renameat(dir, "linked", dir, "renamed") == 0);
    assert(syscall(SYS_renameat2, dir, "renamed", dir, "direct-link", RENAME_EXCHANGE) == 0);
    assert(syscall(SYS_renameat, dir, "renamed", dir, "direct-rename") == 0);
    assert(unlinkat(dir, "source", 0) == 0);
    assert(syscall(SYS_unlinkat, dir, "direct-link", 0) == 0);
    assert(close(file) == 0);
    assert(close(dir) == 0);
    return 0;
}
