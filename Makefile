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
DAT_RUST += us01 us02 us03 us04 us05 us06 us08

DAT_RUST := $(foreach var,$(DAT_RUST),data/$(var)/$(var).yaml)
OUT_RUST := $(addsuffix .txt, $(basename $(DAT_RUST)))

OUT    := $(OUT_RUST)
ALIAS  := $(basename $(notdir $(OUT)))
CHALIAS := $(patsubst %.yaml,check_%,$(notdir $(DAT_RUST)))
CAALIAS := $(patsubst %.yaml,cat_%,$(notdir $(DAT_RUST)))
CLALIAS := $(patsubst %.yaml,clean_%,$(notdir $(DAT_RUST)))
INITDIR := $(patsubst %.yaml,data/%,$(notdir $(DAT_RUST)))
MOALIAS := $(patsubst %.yaml,mon_%,$(notdir $(DAT_RUST)))

.PHONY: all clean check $(ALIAS) $(CLALIAS) $(CHALIAS) $(CAALIAS) $(MOALIAS) stats_de.html stats_us.html graph

GENARGS ?= --transpose -c

-include Makefile.conf
# eg in case ansitoimg is installed in a venv which needs to be sourced before
# executing the python script
ANSITOIMG_PREFIX ?= 
# eg if you want to send the image generation into the background you can set
# this to '&'
ANSITOIMG_SUFFIX ?= 
# which tool shall be used to output the log file
CAT ?= cat
# is executed after the output file was generated (in cat_* targets). Usually
# used to play/display some sort of notification
NOTIF ?= 
# should only be defined if the tree pdf shall be displayed (in case it was
# generated) when running cat_* target
# Usually defined via commandline. Must be set to the tool used to display the
# pdf file
# ZATHURA ?=


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
	$(NOTIF) &
ifdef ZATHURA
	-test -f $(f:.txt=.pdf) && $(ZATHURA) "$(f:.txt=.pdf)" & disown
endif
	$(CAT) $(f:.txt=.col.out)


$(INITDIR):
	$(eval show := $(let i,$@,$(patsubst data/%,%,$i)))
	mkdir ./data/$(show)
	cp ./data/.template.yaml "./data/$(show)/$(show).yaml"


$(MOALIAS):
	# https://github.com/edubart/luamon
	$(eval f := $(let i,$@,data/$(patsubst mon_%,%,$i)/$(patsubst mon_%,%,$i).txt))
	-test -f $(f:.txt=.pdf) && zathura "$(f:.txt=.pdf)" & disown
	luamon -w data/$(patsubst mon_%,%,$@) -e yaml -x make -- --no-print-directory cat_$(patsubst mon_%,%,$@)


$(ALIAS):
	@make --no-print-directory $(let i,$@,data/$i/$i.txt)

$(OUT_RUST): data/%.txt: data/%.yaml rust/target/release/ayto
	@date
	./rust/target/release/ayto sim $(GENARGS) -o $(basename $<) $< > $(basename $<).col.out
	# strip ansi color stuff to get a plain text file
	sed 's/\x1b\[[0-9;]*m//g' $(basename $<).col.out > $(basename $<).txt
	# colored output
	$(ANSITOIMG_PREFIX) python3 generate_png.py "$(basename $<).col.out" "$(basename $<).col.png" "$(basename $<)_tab.png" "$(basename $<)_sum.png" $(ANSITOIMG_SUFFIX)
	@date


graph: stats_de.html stats_us.html

stats_us.html stats_de.html: rust/target/release/ayto $(wildcard data/*/*.csv)
	./rust/target/release/ayto graph ./stats_de.html ./stats_us.html

rust/target/release/ayto: ./rust/src/*
	make -C rust buildRelease
