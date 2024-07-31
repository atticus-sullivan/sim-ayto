DAT_RUST = s01r/s01r.yaml s02/s02.yaml s02r/s02r.yaml s03/s03.yaml s03r/s03r.yaml s04/s04.yaml s04r/s04r.yaml s05/s05.yaml
OUT_RUST = $(addsuffix .out, $(basename $(DAT_RUST)))

OUT   = $(OUT_RUST)
ALIAS = $(notdir $(OUT))

.PHONY: all clean_tex clean $(patsubst %/,clean_%,$(dir $(DAT_RUST))) $(ALIAS)

include Makefile.conf

all: $(OUT)
	

clean: clean_tex $(patsubst %/,clean_%,$(dir $(DAT_RUST)))
	

$(patsubst %/,clean_%,$(dir $(DAT_RUST))): clean_%: %
	-rm "$(<)/$(<)"*.{out,pdf,png,dot}

clean_tex:
	- rm statsMN.tex{,sort}
	- rm statsMB.tex{,sort}
	- rm statsInfo.tex{,sort}
	- rm -r tex-aux

$(ALIAS):
	make $(patsubst %.out,%,$@)/$@

$(OUT_RUST): %.out: %.yaml ./rust/src/*
	@date
	./rust/target/release/ayto -c -o $(basename $<) $< > $(basename $<).col.out
	sed 's/\x1b\[[0-9;]*m//g' $(basename $<).col.out > $(basename $<).out
	echo "\\addplot table {$(basename $<)_statMN.out}; \\addlegendentry{$(basename $(notdir $<))}" >> "statsMN.tex"
	echo "\\addplot table {$(basename $<)_statMB.out}; \\addlegendentry{$(basename $(notdir $<))}" >> "statsMB.tex"
	echo "\\addplot table {$(basename $<)_statInfo.out}; \\addlegendentry{$(basename $(notdir $<))}" >> "statsInfo.tex"
	@date

stats.pdf: stats.tex statsMN.tex statsMB.tex statsInfo.tex
	sort -u statsInfo.tex > statsInfo.tex.sort
	sort -u statsMB.tex > statsMB.tex.sort
	sort -u statsMN.tex > statsMN.tex.sort
	mv statsInfo.tex.sort statsInfo.tex
	mv statsMB.tex.sort statsMB.tex
	mv statsMN.tex.sort statsMN.tex
	test -d tex-aux || mkdir tex-aux
	cluttealtex --output-directory=tex-aux --change-directory --shell-escape -e pdflatex "$<"
