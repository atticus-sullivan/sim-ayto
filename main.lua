local file = io.open("sim.dat")

local rs = require("sim").run(file)
print(#rs)
for _,r in ipairs(rs) do
	require("sim").print_map(r)
	print()
end
