CC = gcc
CFLAGS = -I.

# we load the shared library from the target/release directory
# of the Rust project, which is where Cargo places the compiled library
LDFLAGS = -L ../target/release -ldllmd

SRC_DIR = src
BUILD_DIR = build

SRCS = $(wildcard $(SRC_DIR)/*.c)
OBJS = $(SRCS:$(SRC_DIR)/%.c=$(BUILD_DIR)/%.o)
TARGET = $(BUILD_DIR)/main

.PHONY: all clean again

all: $(BUILD_DIR) $(TARGET)

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/%.o: $(SRC_DIR)/%.c
	$(CC) $(CFLAGS) -c $< -o $@

$(TARGET): $(OBJS)
	$(CC) $(OBJS) $(LDFLAGS) -o $(TARGET)

clean:
	rm -rf $(BUILD_DIR)

again:
	@echo "Rebuilding..."
	$(MAKE) clean
	$(MAKE) all
