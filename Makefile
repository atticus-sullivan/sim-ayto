DAT_RUST := s01/s01.yaml s01r/s01r.yaml s02/s02.yaml s02r/s02r.yaml s03/s03.yaml s03r/s03r.yaml s04/s04.yaml s04r/s04r.yaml s05/s05.yaml us08/us08.yaml
OUT_RUST := $(addsuffix .out, $(basename $(DAT_RUST)))

OUT    := $(OUT_RUST)
ALIAS  := $(notdir $(OUT))
CALIAS := $(patsubst %.yaml,check_%,$(notdir $(DAT_RUST)))

.PHONY: all clean_tex clean $(patsubst %/,clean_%,$(dir $(DAT_RUST))) check $(addprefix check_,$(DAT_RUST)) $(ALIAS) $(CALIAS)

include Makefile.conf

all: $(OUT)
	


clean: clean_tex $(patsubst %/,clean_%,$(dir $(DAT_RUST)))
	

$(patsubst %/,clean_%,$(dir $(DAT_RUST))): clean_%: %
	- $(RM) "$(<)/$(<)"*.{out,pdf,png,dot}

clean_tex:
	- $(RM) statsMN.tex{,sort}
	- $(RM) statsMB.tex{,sort}
	- $(RM) statsInfo.tex{,sort}
	- $(RM) -r tex-aux


check: $(patsubst %/,check_%,$(dir $(DAT_RUST)))
	

$(CALIAS):
	@make --no-print-directory $@/$(patsubst check_%,%.yaml,$@)

$(addprefix check_,$(DAT_RUST)): check_%: % rust/target/release/ayto
	./rust/target/release/ayto --only-check -o /tmp/wontBeUsed $<


$(ALIAS):
	@make --no-print-directory $(patsubst %.out,%,$@)/$@

$(OUT_RUST): %.out: %.yaml rust/target/release/ayto
	@date
	./rust/target/release/ayto -c -o $(basename $<) $< > $(basename $<).col.out
	# strip ansi color stuff to get a plain text file
	sed 's/\x1b\[[0-9;]*m//g' $(basename $<).col.out > $(basename $<).out
	# colored output
	$(ANSITOIMG_PREFIX) ./generate_png "$(basename $<).col.out" "$(basename $<).col.png" "$(basename $<)_tab.png"
	# generate files to generate latex plots
	echo "\\addplot table {$(basename $<)_statMN.out}; \\addlegendentry{$(basename $(notdir $<))}" >> "statsMN.tex"
	echo "\\addplot table {$(basename $<)_statMB.out}; \\addlegendentry{$(basename $(notdir $<))}" >> "statsMB.tex"
	echo "\\addplot table {$(basename $<)_statInfo.out}; \\addlegendentry{$(basename $(notdir $<))}" >> "statsInfo.tex"
	@date

rust/target/release/ayto: ./rust/src/*
	make -C rust buildRelease

stats.pdf: stats.tex statsMN.tex statsMB.tex statsInfo.tex
	sort -u statsInfo.tex > statsInfo.tex.sort
	sort -u statsMB.tex > statsMB.tex.sort
	sort -u statsMN.tex > statsMN.tex.sort
	mv statsInfo.tex.sort statsInfo.tex
	mv statsMB.tex.sort statsMB.tex
	mv statsMN.tex.sort statsMN.tex
	test -d tex-aux || mkdir tex-aux
	cluttealtex --output-directory=tex-aux --change-directory --shell-escape -e pdflatex "$<"
