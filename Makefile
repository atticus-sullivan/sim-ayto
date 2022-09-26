
DAT=$(wildcard s*/*.dat)
OUT=$(addsuffix .out, $(basename $(DAT)))

ALIAS=$(notdir $(OUT))

.PHONY: all clean_tex $(ALIAS)

include Makefile.conf

all: $(OUT)
	make stats.pdf

clean_tex:
	- rm statsMN.tex{,sort}
	- rm statsMB.tex{,sort}
	- rm statsInfo.tex{,sort}
	- rm -r tex-aux

$(ALIAS):
	make $(wildcard */$@)

$(OUT): %.out: %.dat perm.lua sim_perm.lua
	@date
	# lua5.4 sim_perm.lua -o $(basename $<) $< > $(basename $<).out
	lua sim_perm.lua -c -o $(basename $<) $< > $(basename $<).out
	echo "\\addplot table {$(basename $<)_statMN.out}; \\addlegendentry{$(basename $(notdir $<))}" >> "statsMN.tex"
	echo "\\addplot table {$(basename $<)_statMB.out}; \\addlegendentry{$(basename $(notdir $<))}" >> "statsMB.tex"
	echo "\\addplot table {$(basename $<)_statInfo.out}; \\addlegendentry{$(basename $(notdir $<))}" >> "statsInfo.tex"
	@date

stats.pdf: stats.tex statsMN.tex statsMB.tex statsInfo.tex
	sort -u statsInfo.tex > statsInfo.tex.sort
	sort -u statsMB.tex > statsMB.tex.sort
	sort -u statsMN.tex > statsMN.tex.sort
	test -d tex-aux || mkdir tex-aux
	~/programme/cluttex_fork/bin/cluttex --output-directory=tex-aux --change-directory --shell-escape -e pdflatex "$<"
