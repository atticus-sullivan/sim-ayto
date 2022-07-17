local sim = require"sim"

local file,rs,cnt

local runs = {
	{ifn=arg[1], rev=false, ofn="test1.dot", opdf="test1.pdf", id_inc=0, cnt=true,  dot=false, tree=false, pdf=false, title="Current"},
	-- {ifn=arg[2], rev=false, ofn="test2.dot", opdf="test2.pdf", id_inc=0, cnt=true, dot=false, tree=false, pdf=false, title="MB"},
	-- {ifn=arg[3], rev=false, ofn="test3.dot", opdf="test3.pdf", id_inc=0, cnt=true, dot=false, tree=false, pdf=false, title="MN"},
}

-- cfg.id_inc
-- cfg.fn
-- cfg.rev
-- cfg.cnt
-- cfg.dot
-- cfg.tree
for _,e in ipairs(runs) do
	if e.ifn then
		print(e.title)
		file = io.open(e.ifn)
		rs,cnt = sim.run(file, e)
		print(cnt)
		if e.pdf then
			os.execute(string.format("dot -Tpdf -o '%s' '%s'", e.opdf, e.ofn))
			print("pdf created")
		end

		rs = sim.to_list(rs)
		for _,r in ipairs(rs) do
			sim.print_map(r)
			print()
		end
	end
end
