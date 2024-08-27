# sim_ayto
# Copyright (C) 2024  Lukas Heindl
# 
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
# 
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
# 
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <http://www.gnu.org/licenses/>.

DAT_RUST := de01 de01r de02 de02r de03 de03r de04 de04r de05
DAT_RUST += us01 us08

DAT_RUST := $(foreach var,$(DAT_RUST),data/$(var)/$(var).yaml)
OUT_RUST := $(addsuffix .txt, $(basename $(DAT_RUST)))

OUT    := $(OUT_RUST)
ALIAS  := $(basename $(notdir $(OUT)))
CHALIAS := $(patsubst %.yaml,check_%,$(notdir $(DAT_RUST)))
CAALIAS := $(patsubst %.yaml,cat_%,$(notdir $(DAT_RUST)))
CLALIAS := $(patsubst %.yaml,clean_%,$(notdir $(DAT_RUST)))

.PHONY: all clean check $(ALIAS) $(CLALIAS) $(CHALIAS) $(CAALIAS) stats_de.html stats_us.html graph

GENARGS ?= --transpose -c

-include Makefile.conf
ANSITOIMG_PREFIX ?= 


all: $(OUT) graph
	


clean: $(CLALIAS)
	- $(RM) stats_us.html stats_de.html

$(CLALIAS):
	- $(RM) $(let i,$(patsubst clean_%,%,$@),data/$i/$i{.txt,.col.out,.pdf,.col.png,_tab.png,_sum.png,.dot,.csv} )
	- $(RM) $(let i,$(patsubst clean_%,%,$@),data/$i/stat{Info,MB,MN}.csv)


# check all input files
check: $(CHALIAS)
	
$(CHALIAS):
	./rust/target/release/ayto check $(let i,$(patsubst check_%,%,$@),data/$i/$i.yaml)


$(CAALIAS):
	$(eval f := $(let i,$@,data/$(patsubst cat_%,%,$i)/$(patsubst cat_%,%,$i).txt))
	# ensure the output file is up to date
	@make --no-print-directory $(f)
	bat $(f:.txt=.col.out)


$(ALIAS):
	@make --no-print-directory $(let i,$@,data/$i/$i.txt)

$(OUT_RUST): data/%.txt: data/%.yaml rust/target/release/ayto
	@date
	./rust/target/release/ayto sim $(GENARGS) -o $(basename $<) $< > $(basename $<).col.out
	# strip ansi color stuff to get a plain text file
	sed 's/\x1b\[[0-9;]*m//g' $(basename $<).col.out > $(basename $<).txt
	# colored output
	$(ANSITOIMG_PREFIX) python3 generate_png.py "$(basename $<).col.out" "$(basename $<).col.png" "$(basename $<)_tab.png" "$(basename $<)_sum.png"
	@date


graph: stats_de.html stats_us.html

stats_us.html stats_de.html: rust/target/release/ayto $(wildcard data/*/*.csv)
	./rust/target/release/ayto graph ./stats_de.html ./stats_us.html

rust/target/release/ayto: ./rust/src/*
	make -C rust buildRelease
