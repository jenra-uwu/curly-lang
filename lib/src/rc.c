#include <sys/mman.h>

#include "rc.h"

#define PAGE_SIZE 4096

struct s_rcalloc_header {
  void* next;
  size_t size;
  size_t rc;
};

struct s_rcalloc_header* start = NULL;

// Allocates something on the heap with a reference count of 1
void* rcalloc(size_t size) {
    // NULL if size is 0
    if (size == 0) {
        return NULL;
    }

    // Create initial part of heap
    if (start == NULL) {
        // Get mmapped pointer
        start = mmap(NULL, PAGE_SIZE, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_ANON, -1, 0);

        // Error
        if (start == (void*) -1)
            return NULL;

        // Set metadata
        start->next = NULL;
        start->size = PAGE_SIZE - sizeof(struct s_rcalloc_header);
        start->rc = 0;
    }

    // Get next free pointer with enough space if available
    struct s_rcalloc_header* p = start;
    struct s_rcalloc_header* last = start;
    while (p != NULL) {
        // Free pointer of appropriate size found
        if (!p->rc && p->size >= size) {
            // Shrink the pointer if it's sufficiently big
            if (p->size >= size * 2 + sizeof(struct s_rcalloc_header)) {
                struct s_rcalloc_header* q = (struct s_rcalloc_header*) (((void*) (p + 1)) + size);
                q->next = p->next;
                q->size = p->size - size - sizeof(struct s_rcalloc_header);
                q->rc = 0;
                p->next = q;
                p->size = size;
            }

            // Mark as used and return
            p->rc = 1;
            return (void*) (p + 1);
        }

        // Get next pointer
        last = p;
        p = p->next;
    }

    // Get new mmapped pointer
    p = mmap(NULL, size > PAGE_SIZE - sizeof(struct s_rcalloc_header) ? size + sizeof(struct s_rcalloc_header) : PAGE_SIZE, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_ANON, -1, 0);
    if (p == (void*) -1)
        return NULL;
    last->next = p;

    // Shrink if too big
    if (p->size >= size * 2 + sizeof(struct s_rcalloc_header)) {
        struct s_rcalloc_header* q = (struct s_rcalloc_header*) (((void*) (p + 1)) + size);
        q->next = p->next;
        q->size = p->size - size - sizeof(struct s_rcalloc_header);
        q->rc = 0;
        p->next = q;
        p->size = size;
    }

    return (void*) (p + 1);
}

// Copies a pointer with a given size onto the heap with a reference count of 1.
void* rccopy(void* ptr, size_t len, size_t size) {
    if (ptr == ((void*) 0))
        return ptr;
    void* alloced = rcalloc(size);
    if (alloced == ((void*) 0))
        return alloced;

    for (size_t i = 0; i < len; i++) {
        ((char*) alloced)[i] = ((char*) ptr)[i];
    }

    for (size_t i = len; i < size; i++) {
        ((char*) alloced)[i] = ((char*) ptr)[i];
    }

    return alloced;
}

// Increments the reference count.
inline void rcinc(void* ptr) {
    struct s_rcalloc_header* header = ptr;
    header--;
    header->rc++;
}

// Returns true if there is only one reference to the pointer.
bool has_one_reference(void* ptr) {
    struct s_rcalloc_header* header = ptr;
    header--;
    return header->rc == 1;
}

// Decrement the reference count.
void rcfree(void* ptr) {
    struct s_rcalloc_header* header = ptr;
    header--;

    if (header->rc)
        header->rc--;
}

// Frees a reference counted closure structure.
void rcfuncfree(void* ptr) {
    if (((unsigned long long) ptr) & 1)
        return;

    struct s_rcalloc_header* header = ptr;
    header--;

    if (header->rc) {
        if (header->rc == 1) {
            unsigned long long* closure = ptr;
            unsigned int* func = (unsigned int*) closure[0];
            unsigned int argc = *func;
            for (unsigned int i = 1; i < argc + 1; i++) {
                if (closure[i] == 0)
                    break;
                rcfuncfree((void*) closure[i]);
            }
        }

        header->rc--;
    } else {
        *((volatile char*) 0) = 69;
    }
}
