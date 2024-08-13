.PHONY: all run build buildAll buildRelease run check contCheck cont test

SRC_FILES = $(shell find src/ -iname "*.rs")

all: run
	

test:
	cargo test

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

coverage:
	$(RM) target/profraw/*
	$(RM) -r target/coverage/*
	RUSTFLAGS='-Cinstrument-coverage' LLVM_PROFILE_FILE='target/profraw/cargo-test-%p-%m.profraw' cargo test
	grcov target/profraw/ --binary-path ./target/debug/deps/ -s . -t html --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o target/coverage
