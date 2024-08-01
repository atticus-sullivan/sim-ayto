.PHONY: all run build buildAll buildRelease run check contCheck cont

SRC_FILES = $(shell find src/ -iname "*.rs")

all: run
	

target/release/ayto: $(SRC_FILES)
	RUSTC_WRAPPER=sccache cargo build --release

target/debug/ayto: $(SRC_FILES)
	RUSTC_WRAPPER=sccache cargo build

buildAll: buildRelease build
	

build: target/debug/ayto
	

buildRelease: target/release/ayto
	

run:
	RUSTC_WRAPPER=sccache cargo run

check:
	RUSTC_WRAPPER=sccache cargo check

contCheck:
	luamon -l rust -e rs -w . -t -x make -- check

cont:
	luamon -l rust -e rs -w . -t -x make
