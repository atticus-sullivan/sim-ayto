local sim = require"sim"

local file,rs

local runs = {
	{ifn=arg[1], rev=false, ofn="test.dot",     opdf="test.pdf",     f_inc=0},
	{ifn=arg[1], rev=true,  ofn="test.rev.dot", opdf="test.rev.pdf", f_inc=0},
}

for _,e in ipairs(runs) do
	file = io.open(e.ifn)
	rs = sim.run(file, e.rev)

	sim.to_dot("root", rs, e.ofn, e.f_inc)
	os.execute(string.format("dot -Tpdf -o '%s' '%s'", e.opdf, e.ofn))

	rs = sim.to_list(rs)
	for _,r in ipairs(rs) do
		sim.print_map(r)
		print()
	end
end
