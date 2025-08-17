#!/bin/bash

# Build script for userspace test program
# This creates a minimal ELF binary for testing Ring0→Ring3 transitions

set -e

echo "Building userspace test program..."

# Assemble the test program
as --64 -o userspace_test.o userspace_test.s

# Link into ELF executable
ld -o userspace_test userspace_test.o \
   --entry=_start \
   --section-start=.text=0x400000 \
   --section-start=.rodata=0x401000 \
   -static

# Verify it's a valid ELF
file userspace_test
readelf -h userspace_test

echo "Userspace test program built successfully!"
echo "Entry point: $(readelf -h userspace_test | grep 'Entry point' | awk '{print $4}')"
echo "Size: $(stat -c%s userspace_test) bytes"

# Clean up object file
rm -f userspace_test.o

echo "Building userspace thread_stress program..."

# Assemble the thread_stress program
as --64 -o userspace_thread_stress.o userspace_thread_stress.s

# Link into ELF executable at 0x400000 entry
ld -o userspace_thread_stress userspace_thread_stress.o \
   --entry=_start \
   --section-start=.text=0x400000 \
   --section-start=.rodata=0x401000 \
   -static

echo "Userspace thread_stress program built successfully!"
file userspace_thread_stress | cat
readelf -h userspace_thread_stress | cat

# Clean up object file
rm -f userspace_thread_stress.o