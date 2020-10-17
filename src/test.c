#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <inttypes.h>

#define SZ_A     (64 * 1024)   // A
#define SZ_B    (204 * 1024)   // B
#define SZ_C      (4 * 1024)   // C
#define SZ_ABC  (272 * 1024)   // ABC
#define SZ_AB   (268 * 1024)   // AB
#define SZ_AC    (68 * 1024)   // AC

const char shm_name[] = "/shm_name1";
unsigned char* p_abc = NULL;
unsigned char* p_ac = NULL;

#define MMAP_PROT  (PROT_READ | PROT_WRITE)

#define USE_MAP_ANONYMOUSE  0
#if USE_MAP_ANONYMOUSE
#define MMAP_FLAG        (MAP_SHARED | MAP_ANONYMOUS)
#define MMAP_FLAG_FIXED  (MAP_SHARED | MAP_ANONYMOUS | MAP_FIXED)
#else
#define MMAP_FLAG        (MAP_SHARED)
#define MMAP_FLAG_FIXED  (MAP_SHARED | MAP_FIXED)
#endif

int main(void)
{
    int fd = - 1;
    void* p = NULL;
    fd = shm_open(shm_name, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    if (fd == - 1) {
        printf("shm_open() fail\n");
        return - 1;
    }

    if (0 != ftruncate(fd, SZ_ABC)) {
        printf("ftruncate() fail\n");
        return - 1;
    }

    if (0 != shm_unlink(shm_name)) {
        printf("shm_unlink() fail\n");
        return - 1;
    }

    p_abc = (unsigned char*)mmap(NULL, SZ_ABC, MMAP_PROT, MMAP_FLAG, fd, 0);
    if (p_abc == (unsigned char*) -1) {
        printf("p_abc = mmap(NULL) fail\n");
        p_abc = NULL;
        goto EXIT;
    }

    p_ac = (unsigned char*)mmap(NULL, SZ_AC, MMAP_PROT, MMAP_FLAG, fd, 0);
    if (p_ac == (unsigned char*) -1) {
        printf("p_ac = mmap(NULL) fail\n");
        p_ac = NULL;
        goto EXIT;
    }

    p = mmap(p_ac + SZ_A, SZ_C, MMAP_PROT, MMAP_FLAG_FIXED, fd, SZ_AB);
    if (p == MAP_FAILED || p != (void*)(p_ac + SZ_A)) {
        printf("mmap(MAP_FIXED) fail\n");
        p = NULL;
        goto EXIT;
    }
    close(fd);

    printf("mmap() ok:"
            "\np_abc=0x%" PRIxPTR
            "\n p_ac=0x%" PRIxPTR
            "\n    p=0x%" PRIxPTR "\n",
            (uintptr_t)p_abc, (uintptr_t)p_ac, (uintptr_t)p);

    // test
    memset(p_abc, 0xab, SZ_AB);
    memset(p_abc + SZ_AB, 0x0c, SZ_C);
    // should be: ab, 0c
    printf("sm4: %02x, %02x\n", p_ac[0], p_ac[SZ_A]);

    memset(p_ac, 0x0c, SZ_AC);
    // should be: 0c, 0c
    printf("sm0: %02x, %02x\n", p_abc[0], p_abc[SZ_AB]);

EXIT:
    if (p) munmap(p, SZ_C);
    if (p_ac) munmap(p_ac, SZ_AC);
    if (p_abc) munmap(p_abc, SZ_ABC);

    return 0;
}

