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

MODE ?= release

DAT_RUST := de01 de01r de02 de02r de03 de03r de04 de04r de05 de05r de06 de07
DAT_RUST += us01 us02 us03 us04 us05 us06 us07 us08 us09
DAT_RUST += uk01

DAT_RUST := $(foreach var,$(DAT_RUST),data/$(var)/$(var).yaml)
OUT_RUST := $(addsuffix .txt, $(basename $(DAT_RUST)))

OUT    := $(OUT_RUST)
ALIAS  := $(basename $(notdir $(OUT)))
CAALIAS := $(patsubst %.yaml,cat_%,$(notdir $(DAT_RUST)))
CEALIAS := $(patsubst %.yaml,cache_%,$(notdir $(DAT_RUST)))
CHALIAS := $(patsubst %.yaml,check_%,$(notdir $(DAT_RUST)))
CLALIAS := $(patsubst %.yaml,clean_%,$(notdir $(DAT_RUST)))
EDALIAS := $(patsubst %.yaml,edit_%,$(notdir $(DAT_RUST)))
INITDIR := $(patsubst %.yaml,data/%,$(notdir $(DAT_RUST)))
MOALIAS := $(patsubst %.yaml,mon_%,$(notdir $(DAT_RUST)))

.PHONY: all clean check $(ALIAS) $(CLALIAS) $(CHALIAS) $(CAALIAS) $(CEALIAS) $(MOALIAS) comparison hugo cache

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

# what is the file to figure out when to rebuild output files (for local run,
# probably this should be set to rust/target/<MODE>/ayto (but just the
# following should also work)
RUST_DEP ?= $(wildcard rust/src/*.rs)

# options to pass to the EDITOR
EDITOR_OPTS ?= ""


all: $(OUT) comparison hugo
	


clean: $(CLALIAS)
	- $(RM) stats_us.html stats_de.html

$(CLALIAS):
	- $(RM) $(let i,$(patsubst clean_%,%,$@),data/$i/$i{.txt,.col.out,.pdf,.col.png,_tab.png,_sum.png,*.dot,.csv,.md} )
	- $(RM) $(let i,$(patsubst clean_%,%,$@),data/$i/stat{Info,MB,MN,Sum}.{csv,json})


# check all input files
check: $(CHALIAS)
	
$(CHALIAS):
	./rust/target/$(MODE)/ayto check $(let i,$(patsubst check_%,%,$@),data/$i/$i.yaml)

cat: cat_$(CUR)

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

mon: mon_$(CUR)

$(MOALIAS):
ifdef SLEEP
	sleep $(SLEEP)
endif
	# https://github.com/edubart/luamon
	$(eval f := $(let i,$@,data/$(patsubst mon_%,%,$i)/$(patsubst mon_%,%,$i).txt))
	-test -f $(f:.txt=.pdf) && zathura "$(f:.txt=.pdf)" & disown
	luamon -w data/$(patsubst mon_%,%,$@) -e yaml -x make -- --no-print-directory cat_$(patsubst mon_%,%,$@)

edit: edit_$(CUR)

$(EDALIAS):
	$(eval f := $(let i,$@,data/$(patsubst edit_%,%,$i)/$(patsubst edit_%,%,$i).yaml))
	$${EDITOR} $(EDITOR_OPTS) $(f)


$(ALIAS):
	@make --no-print-directory $(let i,$@,data/$i/$i.txt)

$(OUT_RUST): data/%.txt: data/%.yaml $(RUST_DEP)
	@date
	test $$(git rev-parse --abbrev-ref HEAD) = "build" || ./rust/target/$(MODE)/ayto sim $(GENARGS) -o $(basename $<) $< > $(basename $<).col.out
	# strip ansi color stuff to get a plain text file
	sed 's/\x1b\[[0-9;]*m//g' $(basename $<).col.out > $(basename $<).txt
	# colored output
ifndef SKIP_PNG_TABS
	$(ANSITOIMG_PREFIX) python3 generate_png.py "$(basename $<).col.out" "./gh-pages/static/$$(basename "$<" .yaml)/$$(basename "$<" .yaml)" $(ANSITOIMG_SUFFIX)
endif
	# tree if available
	for dot_file in "$(basename $<)"*.dot ; do \
		test -e "$${dot_file}" && \
			name="$$(echo $${dot_file} | sed -E 's/^.*\/(.*)\.dot$$/\1/')" && \
			dot -Tpng -o "./gh-pages/static/$$(basename "$<" .yaml)/$${name}.png" "$${dot_file}" && \
			dot -Tpdf -o "./gh-pages/static/$$(basename "$<" .yaml)/$${name}.pdf" "$${dot_file}" || continue ; \
	done
	@date


cache: cache_$(CUR)
$(CEALIAS): $(RUST_DEP)
	$(eval f := $(let i,$@,data/$(patsubst cache_%,%,$i)/$(patsubst cache_%,%,$i).yaml))
	test $$(git rev-parse --abbrev-ref HEAD) = "build" || ./rust/target/$(MODE)/ayto cache $(f)

comparison: gh-pages/content/comparison/de.md gh-pages/content/comparison/us.md

hugo: comparison
	cd ./gh-pages && hugo build
	echo "$(pwd)/gh-pages/public/ayto"

gh-pages/content/comparison/de.md gh-pages/content/comparison/us.md: rust/target/$(MODE)/ayto $(wildcard data/*/*.json)
	./rust/target/$(MODE)/ayto comparison gh-pages/content/comparison/de.md gh-pages/content/comparison/us.md

rust/target/$(MODE)/ayto: ./rust/src/*
	make -C rust target/$(MODE)/ayto
