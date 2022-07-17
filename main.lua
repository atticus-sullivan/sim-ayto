local sim = require"sim"

local file = io.open("sim.dat")
local rs = sim.run(file)

sim.to_dot("root", rs, "test.dot")
os.execute("dot -Tpdf -o 'test.pdf' 'test.dot'")

rs = sim.to_list(rs)
for _,r in ipairs(rs) do
	sim.print_map(r)
	print()
end
