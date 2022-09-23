
DAT=$(wildcard */*.dat)
OUT=$(addsuffix .out, $(basename $(DAT)))

ALIAS=$(notdir $(OUT))

.PHONY: all $(ALIAS)

include Makefile.conf

all: $(OUT)
	

$(ALIAS):
	make $(wildcard */$@)

# $(OUT): perm.lua sim_perm.lua %.out: %.dat
# 	@date
# 	# lua5.4 sim_perm.lua -o $(basename $<) $< > $(basename $<).out
# 	lua sim_perm.lua -o $(basename $<) $< > $(basename $<).out
# 	@date

$(OUT): perm.lua sim_perm.lua %.out: %.dat
	@date
	# lua5.4 sim_perm.lua -o $(basename $<) $< > $(basename $<).out
	lua sim_perm.lua -c -o $(basename $<) $< > $(basename $<).out
	@date
