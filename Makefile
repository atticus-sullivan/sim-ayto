DAT_RUST := s01/s01.yaml s01r/s01r.yaml s02/s02.yaml s02r/s02r.yaml s03/s03.yaml s03r/s03r.yaml s04/s04.yaml s04r/s04r.yaml s05/s05.yaml us08/us08.yaml
OUT_RUST := $(addsuffix .out, $(basename $(DAT_RUST)))

OUT    := $(OUT_RUST)
ALIAS  := $(notdir $(OUT))
CALIAS := $(patsubst %.yaml,check_%,$(notdir $(DAT_RUST)))

.PHONY: all clean $(patsubst %/,clean_%,$(dir $(DAT_RUST))) check $(addprefix check_,$(DAT_RUST)) $(ALIAS) $(CALIAS)

include Makefile.conf

all: $(OUT)
	


clean: $(patsubst %/,clean_%,$(dir $(DAT_RUST)))
	

$(patsubst %/,clean_%,$(dir $(DAT_RUST))): clean_%: %
	- $(RM) "$(<)/$(<)"*.{out,pdf,png,dot}


check: $(patsubst %/,check_%,$(dir $(DAT_RUST)))
	

$(CALIAS):
	@make --no-print-directory $@/$(patsubst check_%,%.yaml,$@)

$(addprefix check_,$(DAT_RUST)): check_%: % rust/target/release/ayto
	./rust/target/release/ayto --only-check -o /tmp/wontBeUsed $<


$(ALIAS):
	@make --no-print-directory $(patsubst %.out,%,$@)/$@

$(OUT_RUST): %.out: %.yaml rust/target/release/ayto
	@date
	./rust/target/release/ayto --transpose -c -o $(basename $<) $< > $(basename $<).col.out
	# strip ansi color stuff to get a plain text file
	sed 's/\x1b\[[0-9;]*m//g' $(basename $<).col.out > $(basename $<).out
	# colored output
	$(ANSITOIMG_PREFIX) python3 generate_png.py "$(basename $<).col.out" "$(basename $<).col.png" "$(basename $<)_tab.png"
	@date

rust/target/release/ayto: ./rust/src/*
	make -C rust buildRelease
