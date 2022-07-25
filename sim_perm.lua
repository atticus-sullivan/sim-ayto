-- TODO how effectively insert:
-- insert to be later added persons at the total back even behind savely known ones (exclude them from permutations) -> translate constraints
-- constraints -> instructions (constraint, add_person)
-- perm -> conv ->> don't copy the last to be added elements (constrained table copy)
-- TODO merge constr and constr_app together to onw (addfield, apply) -> permgen applies all instructions until the first which shouldn't be applied or the first add instruction
local perm = require("perm")
local colors = require"term.colors"
local _M = {}

S1,S2 = {},{}

local function pr_time(s)
	print(os.date("%Y-%m-%d %H:%M:%S"), s)
end
pr_time("start")

local function dot_node(par, self, e1, e2, file)
	file:write('"',par,'"', " -> {", '"',self,'"', '[shape="record" label=<<table border="0" cellborder="0" cellspacing="0"><tr><td>',e1,'</td></tr><tr><td>',e2,'</td></tr></table>>]', "}\n")
end
local function table_copy(t)
	local r = {}
	for k,v in pairs(t) do
		r[k] = v
	end
	return r
end
local function table_contains(t, ele)
	for _,v in ipairs(t) do
		if v == ele then return true end
	end
	return false
end

-- TODO testing
local function explode(poss, i2)
	local len = #poss
	for i=1,len do
		local len2 = #poss[i]+1
		for j=1,len2 do
			local new = table_copy(poss[i])
			table.insert(new, j, i2)
			table.insert(poss, new)
		end
	end
	return poss
end

local function poss_to_dot(ps, s1,s2, file)
	local nodes = {}
	for _,p in ipairs(ps) do
		local par = "root"
		for i1,i2 in ipairs(p) do
			local co = string.format("%s|%d,%d", par, i1,i2)
			-- if nodes[co] then print(nodes[co][1], par) end -- TODO only debugging
			nodes[co] = {par, i1,i2}
			par = co
		end
	end
	file:write("digraph D {\n")
	for co,x in pairs(nodes) do
		local par,i1,i2 = table.unpack(x)
		dot_node(par, co, s1[i1], s2[i2], file)
	end
	file:write("}\n")
end

local function parse_file(file, rev)
	local s1,s2,constr_app,constr,swap_idx,dup1,dup2 = {},{},{},{},-1,-1,-1
	-- real local
	local lut1,lut2,apply,state,use_additional = {},{},true,0,false

	local function next_line(line)
		line = file:read("l")
		while line and line:match("^#") do line = file:read("l") end
		return line
	end
	local function parse_set(line, s)
		table.insert(s, line:match("%s*([^%->,%s]+)%s*.*"))
	end
	local function translate(i1a, i1b, i2a,i2b)
		s1[i1a],s1[i1b] = s1[i1b],s1[i1a]
		s2[i2a],s2[i2b] = s2[i2b],s2[i2a]

		lut1[i1a],lut1[i1b] = lut1[i1b],lut1[i1a]
		lut2[i2a],lut2[i2b] = lut2[i2b],lut2[i2a]

		if i1a == dup1 then dup1 = i1b end
		if i2a == dup2 then dup2 = i2b end

		for _,v in ipairs(constr) do
			v.matches[i1a],v.matches[i1b] = v.matches[i1b],v.matches[i1a]
			local ia,ib
			for i,c in pairs(v.matches) do
				if c == i2a then ia = i end
				if c == i2b then ib = i end
			end
			if ia then v.matches[ia] = i2b end
			if ib then v.matches[ib] = i2a end
		end
		for _,v in ipairs(constr_app) do
			v.matches[i1a],v.matches[i1b] = v.matches[i1b],v.matches[i1a]
			local ia,ib
			for i,c in pairs(v.matches) do
				if c == i2a then ia = i end
				if c == i2b then ib = i end
			end
			if ia then v.matches[ia] = i2b end
			if ib then v.matches[ib] = i2a end
		end
	end
	local function parse_constraint(line)
		local d = {num=tonumber(line:match("^%s*(%d+)%s*")), matches={}, cnt=0, use_additional=use_additional}
		line = next_line(line)
		while line and line ~= "" do
			local e1,e2
			if rev then
				e2,e1 = line:match("^%s*([^%->, ]+)%s*%->%s*([^%->,%s]+)%s*.*$")
			else
				e1,e2 = line:match("^%s*([^%->, ]+)%s*%->%s*([^%->,%s]+)%s*.*$")
			end
			local i1,i2 = lut1[e1], lut2[e2]
			assert(i1, "invalid constraint "..line)
			assert(i2, "invalid constraint "..line)
			local found = false
			for _,v in ipairs(constr_app) do
				if v.save then
					if v[i1] == e2 then
						found = true
						-- if a save match is omitted, the number of matches decreases
						if v.num == 1 then d.num = d.num -1 end
					end
				end
			end
			for _,v in ipairs(constr) do
				if v.save then
					if v[i1] == e2 then
						found = true
						-- if a save match is omitted, the number of matches decreases
						if v.num == 1 then d.num = d.num -1 end
					end
				end
			end
			--omit already save known stuff
			if not found then
				d.matches[lut1[e1]] = lut2[e2]
				d.cnt = d.cnt+1
			end
			line = next_line(line)
		end
		-- assert(d.cnt == 10 or d.cnt == 1, "matching should always contain 1 or 10 pairs")
		if apply then
			table.insert(constr_app, d)
		else
			table.insert(constr, d)
		end
		assert(d.num >= 0 and d.num <= d.cnt, "invalid num "..tostring(d.num).." vs cnt "..tostring(d.cnt))
		if d.cnt == 1 and apply then
			d.save = true
			assert(d.num == 1 or d.num == 0)
			if d.num == 1 then
				for i1,i2 in pairs(d.matches) do
					assert(swap_idx >= 0)
					translate(i1, swap_idx, i2, swap_idx)
					swap_idx = swap_idx -1
					break -- only one element in d.matches
				end
			end
		end
	end
	local function gen_lut(s)
		local r = {}
		for i,v in ipairs(s) do
			r[v] = i
		end
		return r
	end

	local line = file:read("l")
	while line do
		if line:match("^#") then
			-- print("skip")
		elseif line == "" then
				state = state + 1
		elseif line == "apply to here" then
			apply = false
		elseif line == "additional persons" then
			use_additional = true
		else
			if state >= 4 and swap_idx < 0 then
				lut1,lut2 = gen_lut(s1), gen_lut(s2)
				swap_idx = #s1
			end
			if state == 0 then
				if rev then parse_set(line, s2) else parse_set(line, s1) end
			elseif state == 1 then
				if rev then
					assert(dup2 == -1) parse_set(line, s2) dup2 = #s2
				else
					assert(dup1 == -1) parse_set(line, s1) dup1 = #s1
				end
			elseif state == 2 then
				if rev then parse_set(line, s1) else parse_set(line, s2) end
			elseif state == 3 then
				if rev then
					assert(dup1 == -1) parse_set(line, s1) dup1 = #s1
				else
					assert(dup2 == -1) parse_set(line, s2) dup2 = #s2
				end
			elseif state >= 4 then
				parse_constraint(line)
			end
		end
		line = file:read("l")
	end
	if swap_idx < 0 then
		swap_idx = #s1
	end
	-- swap idx determines upt to which position should be permutated
	return s1,s2,constr_app,constr,swap_idx,dup1,dup2
end

local function hash_len(x)
	local r = 0
	for _,_ in pairs(x) do r=r+1 end
	return r
end

local function check_single(map, c, pr)
	if pr and pr > 2 then
		_M.print_map(map, S1, S2)
	end
	assert(c.cnt == 1)
	-- if c.num == 1 and c.cnt == 1 then pr=1 end
	local cnt = 0
	if pr and pr > 0 then
		print()
		_M.print_map(c.matches, S1, S2)
		print(#map, c.cnt)
	end
	-- go through information provided by d and count how may mappings are
	-- not set aka free and how many mappings are the same in d and the map
	for i1,i2 in pairs(c.matches) do
		-- print("checking if", s1[i1], "maps to", s2[i2])
		if not map[i1] then
			error("map is not fully defined")
			-- free = free+1
		elseif map[i1] == i2 then
			cnt = cnt+1
		end
	end
	-- if there are more matches between d and mapping, then d has right
	-- matches then map is no valid mapping (otherwise d.num would be
	-- higher)
	-- if there are fewer matches between d and the current mapping as are
	-- right in d, then the mapping isn't valid as well (at d.num has to be
	-- right, thus at least d.num have to match). Exceptions are mappings
	-- which are not fully decided yet, here there are free matchings which
	-- still can match with d
	if pr and pr > 1 then
		print(cnt, c.num)
	end
	-- local c_num = c.num <= 1 and c.num or c.num -1
	local c_num = c.num
	if cnt < c_num or cnt > c.num then
		if pr and pr > 1 then
			print("false")
		end
		return false
	end
	-- only if all constraints are met, the mapping can be valid
	if pr and pr > 1 then
		print("true")
	end
	return true
end
local function check_all(map, c, dup1, dup2, pr)
	if pr and pr > 2 then
		print("map")
		_M.print_map(map, S1, S2)
	end
	-- assert(c.cnt == #map or c.cnt == #map-1)
		-- if c.num == 1 and c.cnt == 1 then pr=1 end
		local cnt = 0
		if pr and pr > 0 then
			print("matches")
			_M.print_map(c.matches, S1, S2)
			print(#map, c.cnt)
		end
		-- go through information provided by d and count how may mappings are
		-- not set aka free and how many mappings are the same in d and the map
		local dup2_idx
		for i1,i2 in pairs(map) do
			if i2 == dup2 then dup2_idx = i1 end
		end
		for i1,i2 in pairs(c.matches) do
			-- print("checking if", s1[i1], "maps to", s2[i2])
			if not map[i1] then
				error("map is not fully defined")
				-- free = free+1
			elseif map[i1] == i2 then
				cnt = cnt+1
			end
		end
		-- handle overhang
		local r
		if pr and pr > 1 then
			print(dup1, dup2, dup2_idx)
		end
		if not c.use_additional and dup2_idx and map[dup1] ~= dup2 then
			map[dup1],map[dup2_idx] = map[dup2_idx],map[dup1]
			assert(map[dup1] == dup2)
			if not check_all(map, c, dup1, dup2, pr) then
				return false
			else
				r = true
			end
			map[dup1],map[dup2_idx] = map[dup2_idx],map[dup1]
		end
		-- if there are more matches between d and mapping, then d has right
		-- matches then map is no valid mapping (otherwise d.num would be
		-- higher)
		-- if there are fewer matches between d and the current mapping as are
		-- right in d, then the mapping isn't valid as well (at d.num has to be
		-- right, thus at least d.num have to match). Exceptions are mappings
		-- which are not fully decided yet, here there are free matchings which
		-- still can match with d
		if pr and pr > 1 then
			print(cnt, c.num)
		end
		local c_num = c.num
		if not r and (cnt < c_num or cnt > c.num) then
			if pr and pr > 1 then
				print("false")
			end
			return false
		end
	-- only if all constraints are met, the mapping can be valid
	if pr and pr > 1 then
		print("true")
	end
	return true
end
local function check(map, constr, dup1, dup2, pr)
	for _,c in ipairs(constr) do
		if c.cnt == 1 then
			return check_single(map, c, pr)
		else
			return check_all(map, c, dup1, dup2, pr)
		end
	end
end

-- local function entropy_single(l, e1,e2, total)
-- 	local t = perm.count_pred(l, function(map) return map[e1] == e2 end)
-- 	local f = perm.count_pred(l, function(map) return map[e1] ~= e2 end)
-- 	return - f/total * math.log(f/total,2) - t/total*math.log(t/total,2)
-- end
-- local function entropy_single_max(l, total)
-- 	local r,e1,e2 = -1,-1
-- 	for i1=1,#l[1] do
-- 		for i2=1,#l[1] do
-- 			local e = entropy_single(l, i1,i2, total)
-- 			if e > r then
-- 				r,e1,e2 = e,i1,i2
-- 			end
-- 		end
-- 	end
-- 	return r,e1,e2
-- end
-- local function entropy_all(l, constr, total, lights)
-- 	-- local function filter_map(_map)
-- 	-- 	local map = {}
-- 	-- 	for k,v in ipairs(_map) do
-- 	-- 		if not s1[k]:match("Dummy.*") and not s2[v]:match("Dummy.*") then
-- 	-- 			map[k] = v
-- 	-- 			print(#map)
-- 	-- 		else
-- 	-- 			print("filtered")
-- 	-- 		end
-- 	-- 		print(k, s1[k],s2[v], s2[map[k]])
-- 	-- 	end
-- 	-- 	print(#_map, #map)
-- 	-- 	return map
-- 	-- end
-- 	local e = 0
-- 	local lights_real = constr.num
-- 	for i=0,lights do
-- 		constr.num = i
-- 		local c = perm.count_pred(l, function(map2) return check_des(map2,{constr},nil) end)
-- 		if c ~= 0 then
-- 			e = e - c/total * math.log(c/total, 2)
-- 		end
-- 	end
-- 	constr.num = lights_real
-- 	return e
-- end
-- local function entropy_all_max(l, total, lights)
-- 	local r,mi = -1,-1
-- 	for ei,map in ipairs(l) do
-- 		-- TODO map shouldn't contain dummy elements
-- 		local e = entropy_all(l, map, total, lights)
-- 		if e > r then
-- 			r,mi = e,ei
-- 		end
-- 	end
-- 	return r,l[mi]
-- end
--
-- local function write_entro_guess(h,g,s1,s2)
-- 	io.write(h)
-- 	for e1,e2 in pairs(g) do
-- 		io.write(" ", s1[e1], "->", s2[e2])
-- 	end
-- 	io.write("\n")
-- end

local function write_matches(g,s1,s2)
	for e1,e2 in pairs(g) do
		io.write(" ", s1[e1], "->", s2[e2])
	end
	io.write("\n")
end

function _M.print_map(m, s1,s2)
	for k,v in pairs(m) do
		io.write(s1[k], " -> ", s2[v], "\n")
	end
end

local function poss_print(p, s1,s2)
	for _,map in ipairs(p) do
		_M.print_map(map, s1,s2)
		print()
	end
end

local function prob_tab(p, s1, s2, t)
	local tab = {}
	local ml = 0
	for _,v in ipairs(s1) do if #v > ml then ml = #v end end
	for _,v in ipairs(s2) do if #v > ml then ml = #v end end
	io.write(string.rep(" ", ml+0+1))
	for j=1,#s2 do
		io.write(string.format("%"..tostring(ml).."s|", s2[j]))
	end
	io.write("\n")
	for i=1,#s1 do
		io.write(string.format("%"..tostring(ml).."s|", s1[i]))
		for j=1,#s2 do
			local co = string.format("%d|%d", i,j)
			tab[co] = perm.count_pred(p, function(map) return map[i] == j end)/(#p/100)
			-- print(t, t and t[co] or "")
			if t and t[co] and tab[co] ~= 0 then
				if tab[co] > t[co] then
					io.write(tostring(colors.green), string.format("%"..tostring(ml)..".4f", tab[co]), tostring(colors.reset), "|")
				elseif tab[co] < t[co] then
					io.write(tostring(colors.red), string.format("%"..tostring(ml)..".4f", tab[co]), tostring(colors.reset), "|")
				else
					io.write(string.format("%"..tostring(ml)..".4f|", tab[co]))
				end
			else
				io.write(string.format("%"..tostring(ml)..".4f|", tab[co]))
			end
		end
		io.write("\n")
	end
	return tab
end

local function dbg(map)
	local valid = {
		{
			["Andre"]="Isabelle",
			["Antonino"]="Dana",
			["Dustin"]="Marie",
			["Jordi"]="Estelle",
			["Leon"]="Jessica",
			["Marius"]="Zaira",
			["Max"]="Monami",
			["Mike"]="Joelina",
			["Tim"]="Kerstin",
			["William"]="Raphaela",
		},
		{
			["Andre"]="Dana",
			["Antonino"]="Joelina",
			["Dustin"]="Zaira",
			["Jordi"]="Desiree",
			["Leon"]="Jessica",
			["Marius"]="Isabelle",
			["Max"]="Marie",
			["Mike"]="Estelle",
			["Tim"]="Kerstin",
			["William"]="Raphaela",
		},
		{
			["Andre"]="Desiree",
			["Antonino"]="Dana",
			["Dustin"]="Marie",
			["Jordi"]="Estelle",
			["Leon"]="Jessica",
			["Marius"]="Zaira",
			["Max"]="Monami",
			["Mike"]="Joelina",
			["Tim"]="Kerstin",
			["William"]="Raphaela",
		},
		{
			["Andre"]="Isabelle",
			["Antonino"]="Dana",
			["Dustin"]="Desiree",
			["Jordi"]="Estelle",
			["Leon"]="Jessica",
			["Marius"]="Zaira",
			["Max"]="Monami",
			["Mike"]="Joelina",
			["Tim"]="Kerstin",
			["William"]="Raphaela",
		},
		{
			["Andre"]="Isabelle",
			["Antonino"]="Dana",
			["Dustin"]="Marie",
			["Jordi"]="Estelle",
			["Leon"]="Jessica",
			["Marius"]="Zaira",
			["Max"]="Monami",
			["Mike"]="Desiree",
			["Tim"]="Kerstin",
			["William"]="Raphaela",
		},
		{
			["Andre"]="Isabelle",
			["Antonino"]="Dana",
			["Dustin"]="Zaira",
			["Jordi"]="Monami",
			["Leon"]="Joelina",
			["Marius"]="Jessica",
			["Max"]="Kerstin",
			["Mike"]="Estelle",
			["Tim"]="Desiree",
			["William"]="Raphaela",
		},
		{
			["Andre"]="Isabelle",
			["Antonino"]="Jessica",
			["Dustin"]="Zaira",
			["Jordi"]="Desiree",
			["Leon"]="Raphaela",
			["Marius"]="Marie",
			["Max"]="Kerstin",
			["Mike"]="Estelle",
			["Tim"]="Joelina",
			["William"]="Dana",
		},
		{
			["Andre"]="Isabelle",
			["Antonino"]="Kerstin",
			["Dustin"]="Dana",
			["Jordi"]="Desiree",
			["Leon"]="Jessica",
			["Marius"]="Zaira",
			["Max"]="Marie",
			["Mike"]="Joelina",
			["Tim"]="Monami",
			["William"]="Raphaela",
		},
		{
			["Andre"]="Isabelle",
			["Antonino"]="Kerstin",
			["Dustin"]="Monami",
			["Jordi"]="Desiree",
			["Leon"]="Jessica",
			["Marius"]="Zaira",
			["Max"]="Marie",
			["Mike"]="Joelina",
			["Tim"]="Dana",
			["William"]="Raphaela",
		},
		{
			["Andre"]="Isabelle",
			["Antonino"]="Marie",
			["Dustin"]="Dana",
			["Jordi"]="Desiree",
			["Leon"]="Jessica",
			["Marius"]="Zaira",
			["Max"]="Monami",
			["Mike"]="Joelina",
			["Tim"]="Kerstin",
			["William"]="Raphaela",
		},
		{
			["Andre"]="Isabelle",
			["Antonino"]="Desiree",
			["Dustin"]="Marie",
			["Jordi"]="Jessica",
			["Leon"]="Raphaela",
			["Marius"]="Zaira",
			["Max"]="Kerstin",
			["Mike"]="Estelle",
			["Tim"]="Joelina",
			["William"]="Dana",
		},
		{
			["Andre"]="Isabelle",
			["Antonino"]="Desiree",
			["Dustin"]="Raphaela",
			["Jordi"]="Jessica",
			["Leon"]="Marie",
			["Marius"]="Zaira",
			["Max"]="Kerstin",
			["Mike"]="Estelle",
			["Tim"]="Joelina",
			["William"]="Dana",
		},
		{
			["Andre"]="Marie",
			["Antonino"]="Dana",
			["Dustin"]="Zaira",
			["Jordi"]="Isabelle",
			["Leon"]="Jessica",
			["Marius"]="Monami",
			["Max"]="Estelle",
			["Mike"]="Joelina",
			["Tim"]="Desiree",
			["William"]="Raphaela",
		},
		{
			["Andre"]="Marie",
			["Antonino"]="Dana",
			["Dustin"]="Zaira",
			["Jordi"]="Monami",
			["Leon"]="Jessica",
			["Marius"]="Estelle",
			["Max"]="Isabelle",
			["Mike"]="Joelina",
			["Tim"]="Desiree",
			["William"]="Raphaela",
		},
		{
			["Andre"]="Marie",
			["Antonino"]="Estelle",
			["Dustin"]="Isabelle",
			["Jordi"]="Monami",
			["Leon"]="Jessica",
			["Marius"]="Zaira",
			["Max"]="Raphaela",
			["Mike"]="Joelina",
			["Tim"]="Kerstin",
			["William"]="Desiree",
		},
		{
			["Andre"]="Marie",
			["Antonino"]="Isabelle",
			["Dustin"]="Zaira",
			["Jordi"]="Desiree",
			["Leon"]="Joelina",
			["Marius"]="Monami",
			["Max"]="Kerstin",
			["Mike"]="Estelle",
			["Tim"]="Jessica",
			["William"]="Dana",
		},
		{
			["Andre"]="Marie",
			["Antonino"]="Joelina",
			["Dustin"]="Zaira",
			["Jordi"]="Desiree",
			["Leon"]="Isabelle",
			["Marius"]="Monami",
			["Max"]="Kerstin",
			["Mike"]="Estelle",
			["Tim"]="Jessica",
			["William"]="Dana",
		},
		{
			["Andre"]="Marie",
			["Antonino"]="Raphaela",
			["Dustin"]="Isabelle",
			["Jordi"]="Monami",
			["Leon"]="Jessica",
			["Marius"]="Zaira",
			["Max"]="Estelle",
			["Mike"]="Joelina",
			["Tim"]="Kerstin",
			["William"]="Desiree",
		},
		{
			["Andre"]="Monami",
			["Antonino"]="Jessica",
			["Dustin"]="Zaira",
			["Jordi"]="Desiree",
			["Leon"]="Isabelle",
			["Marius"]="Marie",
			["Max"]="Kerstin",
			["Mike"]="Estelle",
			["Tim"]="Joelina",
			["William"]="Dana",
		}
	}
	for _,x in ipairs(valid) do
		local match = true
		for i1,i2 in pairs(map) do
			if S1[i1] ~= x[S2[i2]] then
				match = false
				break
			end
		end
		if match then return true end
	end
	return false
end

-- parsing
local file = io.open(arg[1])
local s1,s2,constr_app,constr,perm_num,dup1,dup2 = parse_file(file, false)
file:close()
S1,S2 = s1,s2 -- TODO only used for debug
-- for _,v in ipairs(s1) do io.write(v, " ") end io.write("\n")
-- for _,v in ipairs(s2) do io.write(v, " ") end io.write("\n")

-- for _,c in ipairs(constr_app) do _M.print_map(c.matches,s1,s2) end
-- print()

pr_time("parsing done")
-- permgen
local a = {}
for i=1,#s1 do a[i] = i end
local poss = perm.permgen(a, perm_num, {}, function(map) return true end)
print("total", #poss)
print(os.date("%Y-%m-%d %H:%M:%S"))
pr_time("permgen done")
print()

-- local lut1,lut2 = {},{}
-- for i,e in ipairs(s1) do lut1[e] = i end
-- for i,e in ipairs(s2) do lut2[e] = i end

local tabSingle,tabAll
for _,c in ipairs(constr) do
	-- local g,h,h_real
	if c.cnt > 1 then
		-- all
		-- calculating the max entropy is very expensive (10(lights) *
		-- #poss(possible matchings) * #poss(count_pred) constraints checken)
		-- -> only do this if the possibilities narrow down a bit
		-- if #poss <= 0 then
		-- 	h,g = entropy_all_max(poss, #poss, 10)
		-- else
		-- 	h,g = 0,{}
		-- end
		-- pr_time("entropy all start")
		-- h_real = entropy_all(poss, c, #poss, 10)
		-- pr_time("entropy all end")
	else
		-- single
		-- local g1,g2
		-- if #poss <= 3628800 then
		-- 	h,g1,g2 = entropy_single_max(poss, #poss)
		-- else
		-- 	h,g1,g2 = 0,1,1
		-- end
		-- for e1,e2 in pairs(c.matches) do
		-- 	pr_time("entropy single start")
		-- 	print("non parallel")
		-- 	h_real = entropy_single(poss, e1,e2, #poss)
		-- 	pr_time("entropy single end")
			-- os.exit()
			-- break -- is only one
		-- end
		-- g = {[g1]=g2}
	end
	poss = perm.filter_pred(poss, function(map)
		if dbg(map) then
			return check_all(map, c, dup1, dup2, 3)
		end
		return check_all(map, c, dup1, dup2)
	end)
	-- write_entro_guess(h,g, s1,s2)
	write_matches(c.matches, s1,s2)
	print(#poss)
	if #poss <= 3628800 then
		if c.cnt > 1 then
			tabSingle = prob_tab(poss, s1, s2, tabSingle)
		else
			tabAll = prob_tab(poss, s1, s2, tabAll)
		end
	end
	-- poss_print(poss, s1, s2)
	pr_time("iter")
	print()
end

if #poss <= 40 then
	local of = io.open("test.dot", "w")
	poss_to_dot(poss, s1,s2, of)
	of:close()
end

return
