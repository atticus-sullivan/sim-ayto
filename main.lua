local sim = require"sim"

local file = io.open("sim.dat")
local rs = sim.run(file)

-- rs = to_list(rs)
-- for _,r in ipairs(rs) do
-- 	require("sim-tree").print_map(r)
-- 	print()
-- end
sim.to_dot("root", rs)
