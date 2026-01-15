# Makefile for CardArena C++ Client
# Supports Linux (g++) and Windows (MinGW/MSYS2)

CXX = g++
CXXFLAGS = -std=c++17 -Wall -Wextra -pthread
LDFLAGS = 

# Detect OS
ifeq ($(OS),Windows_NT)
    # Windows (MinGW)
    LDFLAGS += -lws2_32
    TARGET_EXT = .exe
else
    # Linux / WSL / macOS
    TARGET_EXT = 
endif

TARGET = client$(TARGET_EXT)
SRC_DIR = src
OBJ_DIR = obj

# Find all .cpp files
SRCS := $(shell find $(SRC_DIR) -name "*.cpp")
# Convert to .o files
OBJS := $(SRCS:$(SRC_DIR)/%.cpp=$(OBJ_DIR)/%.o)

# Include directories
INC_FLAGS := -I$(SRC_DIR)

all: $(TARGET)

$(TARGET): $(OBJS)
	@echo "Linking $(TARGET)..."
	@$(CXX) $(OBJS) -o $@ $(LDFLAGS)

$(OBJ_DIR)/%.o: $(SRC_DIR)/%.cpp
	@mkdir -p $(dir $@)
	@echo "Compiling $<..."
	@$(CXX) $(CXXFLAGS) $(INC_FLAGS) -c $< -o $@

clean:
	@echo "Cleaning..."
	@rm -rf $(OBJ_DIR) $(TARGET)

.PHONY: all clean
