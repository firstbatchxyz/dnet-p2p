#!/usr/bin/make -f

# Defines the C compiler to use (gcc)
CC = gcc

# Compiler flags:
# -Wall: enable all warning messages
# -Wextra: enable extra warning messages
# -std=c11: use C11 standard
CFLAGS = -Wall -Wextra -std=c11

# Directory containing source files
SRC_DIR = src

# Directory for compiled objects and executable
BUILD_DIR = build

# Find all .c files in src directory
# wildcard function searches for all files matching the pattern
SRCS = $(wildcard $(SRC_DIR)/*.c)

# Convert .c filenames to .o filenames in build directory
# Pattern substitution: from 'src/%.c' to 'build/%.o'
OBJS = $(SRCS:$(SRC_DIR)/%.c=$(BUILD_DIR)/%.o)

# Default target (due to being the first rule) that builds the executable
all: $(BUILD_DIR)/main

# Rule to link object files into final executable
# $@ represents the target name (build/main)
# $^ represents all prerequisites (the object files)
$(BUILD_DIR)/main: $(OBJS)
		$(CC) $(CFLAGS) -o $@ $^

# Rule to compile .c files into .o files
# $< represents the first prerequisite (the .c file)
# Creates build directory if it doesn't exist
$(BUILD_DIR)/%.o: $(SRC_DIR)/%.c
		mkdir -p $(BUILD_DIR)
		$(CC) $(CFLAGS) -c $< -o $@

# Clean target removes the build directory
clean:
		rm -rf $(BUILD_DIR)

# Format target runs clang-format on all .c files
format:
		clang-format -i $(SRCS)

# Clean and make again
again:
		@make clean && make

.PHONY: all clean again
