
DAT=$(wildcard */*.dat)
OUT=$(addsuffix .out, $(basename $(DAT)))

ALIAS=$(notdir $(OUT))

.PHONY: all $(ALIAS)

all: $(OUT)
	

$(ALIAS):
	make $(wildcard */$@)

$(OUT): %.out: %.dat
	lua5.4 sim_perm.lua -o $(basename $<) $< > $(basename $<).out
