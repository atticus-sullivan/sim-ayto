DAT = $(wildcard s*/*.dat)

DAT_LUA = s01r/s01r.dat s02r/s02r.dat s02/s02.dat s03/s03.dat
OUT_LUA = $(addsuffix .out, $(basename $(DAT_LUA)))

DAT_PY = s04/s04.yaml
OUT_PY = $(addsuffix .out, $(basename $(DAT_PY)))

OUT   = $(OUT_LUA) $(OUT_PY)
ALIAS = $(notdir $(OUT))

.PHONY: all clean_tex $(ALIAS)

include Makefile.conf

all: $(OUT)
	

clean_tex:
	- rm statsMN.tex{,sort}
	- rm statsMB.tex{,sort}
	- rm statsInfo.tex{,sort}
	- rm -r tex-aux

$(ALIAS):
	make $(wildcard */$@)

$(OUT_PY): %.out: %.yaml sim.py
	@date
	python3 sim.py -c -o $(basename $<) $< > $(basename $<).out
	echo "\\addplot table {$(basename $<)_statMN.out}; \\addlegendentry{$(basename $(notdir $<))}" >> "statsMN.tex"
	echo "\\addplot table {$(basename $<)_statMB.out}; \\addlegendentry{$(basename $(notdir $<))}" >> "statsMB.tex"
	echo "\\addplot table {$(basename $<)_statInfo.out}; \\addlegendentry{$(basename $(notdir $<))}" >> "statsInfo.tex"
	@date

$(OUT_LUA): %.out: %.dat perm.lua sim_perm.lua
	@date
	lua sim_perm.lua -c -o $(basename $<) $< > $(basename $<).out
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
	~/programme/cluttex_fork/bin/cluttex --output-directory=tex-aux --change-directory --shell-escape -e pdflatex "$<"
