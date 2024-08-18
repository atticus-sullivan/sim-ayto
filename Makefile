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

DAT_RUST := s01/s01.yaml s01r/s01r.yaml s02/s02.yaml s02r/s02r.yaml s03/s03.yaml s03r/s03r.yaml s04/s04.yaml s04r/s04r.yaml s05/s05.yaml us08/us08.yaml
OUT_RUST := $(addsuffix .txt, $(basename $(DAT_RUST)))

OUT    := $(OUT_RUST)
ALIAS  := $(notdir $(OUT))
CALIAS := $(patsubst %.yaml,check_%,$(notdir $(DAT_RUST)))

.PHONY: all clean $(patsubst %/,clean_%,$(dir $(DAT_RUST))) check $(addprefix check_,$(DAT_RUST)) $(ALIAS) $(CALIAS) stats.html graph

GENARGS ?= --transpose -c

-include Makefile.conf
ANSITOIMG_PREFIX ?= 


all: $(OUT) graph
	


clean: $(patsubst %/,clean_%,$(dir $(DAT_RUST)))
	- $(RM) stats.html

$(patsubst %/,clean_%,$(dir $(DAT_RUST))): clean_%: %
	- $(RM) "$(<)/$(<)"*.{txt,col.out,pdf,png,dot}


check: $(patsubst %/,check_%,$(dir $(DAT_RUST)))
	

$(CALIAS):
	@make --no-print-directory $@/$(patsubst check_%,%.yaml,$@)

$(addprefix check_,$(DAT_RUST)): check_%: % rust/target/release/ayto
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


graph: stats.html

stats.html: rust/target/release/ayto
	./rust/target/release/ayto graph ./stats.html

rust/target/release/ayto: ./rust/src/*
	make -C rust buildRelease
