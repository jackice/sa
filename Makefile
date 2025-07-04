# 定义变量
CARGO = cargo
CROSS = cross
PROJECT_NAME = sa
TARGET_DIR = target
RELEASE_DIR = $(TARGET_DIR)/release
BIN_NAME = $(PROJECT_NAME)
BIN_EXT = 
WIN_TARGET = x86_64-pc-windows-gnu

# 检测操作系统
ifeq ($(OS),Windows_NT)
    BIN_EXT = .exe
    RM = del /Q
    MKDIR = mkdir
    RMDIR = rmdir /S /Q
else
    RM = rm -f
    MKDIR = mkdir -p
    RMDIR = rm -rf
endif

# 默认目标
.PHONY: all
all: build

# 构建debug版本
.PHONY: build
build:
	$(CARGO) build

# 构建release版本
.PHONY: release
release:
	$(CARGO) build --release

# 构建Windows release版本 (在macOS上)
.PHONY: release-win
release-win:
	$(CROSS) build --release --target $(WIN_TARGET)

# 运行程序
.PHONY: run
run:
	$(CARGO) run

# 运行测试
.PHONY: test
test:
	$(CARGO) test

# 清理构建文件
.PHONY: clean
clean:
	$(CARGO) clean

# 安装release版本到系统路径 (需要管理员权限)
.PHONY: install
install: release
ifeq ($(OS),Windows_NT)
	copy "$(RELEASE_DIR)\$(BIN_NAME)$(BIN_EXT)" "%ProgramFiles%\$(PROJECT_NAME)"
else
	cp "$(RELEASE_DIR)/$(BIN_NAME)" "/usr/local/bin"
endif

# 卸载程序
.PHONY: uninstall
uninstall:
ifeq ($(OS),Windows_NT)
	del "%ProgramFiles%\$(PROJECT_NAME)\$(BIN_NAME)$(BIN_EXT)"
else
	rm -f "/usr/local/bin/$(BIN_NAME)"
endif

# 检查代码格式
.PHONY: fmt
fmt:
	$(CARGO) fmt

# 检查代码lint
.PHONY: lint
lint:
	$(CARGO) clippy

# 显示帮助信息
.PHONY: help
help:
	@echo "可用命令:"
	@echo "  make build      - 构建debug版本"
	@echo "  make release    - 构建release版本"
	@echo "  make release-win - 构建Windows release版本(需cross)"
	@echo "  make run        - 运行程序"
	@echo "  make test       - 运行测试"
	@echo "  make clean      - 清理构建文件"
	@echo "  make install    - 安装release版本到系统路径"
	@echo "  make uninstall  - 卸载程序"
	@echo "  make fmt        - 格式化代码"
	@echo "  make lint       - 运行clippy检查"
