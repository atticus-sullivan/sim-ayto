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

DAT_RUST := de01/de01.yaml de01r/de01r.yaml de02/de02.yaml de02r/de02r.yaml de03/de03.yaml de03r/de03r.yaml de04/de04.yaml de04r/de04r.yaml de05/de05.yaml
DAT_RUST += us08/us08.yaml
DATA_RUST := $(addprefix data/,$(DATA_RUST))
OUT_RUST := $(addsuffix .txt, $(basename $(DAT_RUST)))

OUT    := $(OUT_RUST)
ALIAS  := $(notdir $(OUT))
CALIAS := $(patsubst %.yaml,check_%,$(notdir $(DAT_RUST)))

.PHONY: all clean $(patsubst %/,clean_%,$(dir $(DAT_RUST))) check $(addprefix check_,$(DAT_RUST)) $(ALIAS) $(CALIAS) stats_de.html stats_us.html graph

GENARGS ?= --transpose -c

-include Makefile.conf
ANSITOIMG_PREFIX ?= 


all: $(OUT) graph
	


clean: $(patsubst %/,clean_%,$(dir $(DAT_RUST)))
	- $(RM) stats_us.html stats_de.html

$(patsubst %/,clean_%,$(dir $(DAT_RUST))): clean_%: data/%
	- $(RM) "$(<)/$(<)"*.{txt,col.out,pdf,png,dot}


check: $(patsubst %/,check_%,$(dir $(DAT_RUST)))
	

$(CALIAS):
	@make --no-print-directory $@/$(patsubst check_%,%.yaml,$@)

$(addprefix check_,$(DAT_RUST)): check_%: data/% rust/target/release/ayto
	./rust/target/release/ayto check $<


$(ALIAS):
	@make --no-print-directory $(patsubst %.txt,%,$@)/$@

$(OUT_RUST): %.txt: %.yaml rust/target/release/ayto
	@date
	./rust/target/release/ayto sim $(GENARGS) -o $(basename $<) $< > $(basename $<).col.out
	# strip ansi color stuff to get a plain text file
	sed 's/\x1b\[[0-9;]*m//g' $(basename $<).col.out > $(basename $<).txt
	# colored output
	$(ANSITOIMG_PREFIX) python3 generate_png.py "$(basename $<).col.out" "$(basename $<).col.png" "$(basename $<)_tab.png"
	@date


graph: stats_de.html stats_us.html

stats_us.html stats_de.html: rust/target/release/ayto
	./rust/target/release/ayto graph ./stats_de.html ./stats_us.html

rust/target/release/ayto: ./rust/src/*
	make -C rust buildRelease
