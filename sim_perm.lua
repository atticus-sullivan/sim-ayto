local perm      = require("perm")
local colors    = require"term.colors"
local prompt    = require"prompt"
prompt.colorize = true
prompt.name     = "sim"
prompt.history  = "sim.hist" -- otherwise no history

local _M = {}

-------------------
--  HELPER STUFF --
-------------------
local function pr_time(s)
	-- print(os.date("%Y-%m-%d %H:%M:%S"), s)
end
pr_time("start")

local function dot_node(par, self, e1, e2, file)
	file:write('"',par,'"', " -> {", '"',self,'"', '[shape="record" label=<<table border="0" cellborder="0" cellspacing="0"><tr><td>',e1,'</td></tr><tr><td>',e2,'</td></tr></table>>]', "}\n")
end
function _M.poss_to_dot(ps, s1,s2, file)
	local nodes = {}
	for _,p in ipairs(ps) do
		local par = "root"
		for i1,i2 in ipairs(p) do
			local co = string.format("%s|%d,%d", par, i1,i2)
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
local function table_copy(t)
	local r = {}
	for k,v in pairs(t) do
		r[k] = v
	end
	return r
end
function _M.gen_lut(s)
	local r = {}
	for i,v in ipairs(s) do
		r[v] = i
	end
	return r
end

local function parse_file(file, rev)
	local s1,s2,instructions,swap_idx,dup,added = {},{},{},-1,nil,nil
	-- real local
	local lut1,lut2,apply,state = {},{},true,0

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

		if i1a == dup[1] then dup[1] = i1b end
		if i2a == dup[2] then dup[2] = i2b end

		for _,v in ipairs(instructions) do
			if v.constraint then
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
	end
	local function parse_instruction(line)
		if line == "add" then
			-- add instruction
			-- elements will be added at the end -> obmit from permutations
			added = (added or 0)+1
			line = next_line(line)
			if rev then parse_set(line, s2) else parse_set(line, s1) end
			line = next_line(line)
			if rev then parse_set(line, s1) else parse_set(line, s2) end
			lut1,lut2 = _M.gen_lut(s1), _M.gen_lut(s2)
			table.insert(instructions, {add=true, i1=#s1, i2=#s2})
			assert(dup == nil)
			dup = {#s1, #s2}
			return
		end

		-- constraint
		local d = {constraint=true, num=tonumber(line:match("^%s*(%d+)%s*")), matches={}, cnt=0, added=added~=nil, apply=apply}
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
			-- TODO micro optimization: obmit already known stuff (safe)
			d.matches[i1] = i2 -- ATTENTION: d.matches might be hash-map
			d.cnt = d.cnt+1
			line = next_line(line)
		end
		-- assert(d.cnt == 10 or d.cnt == 1, "matching should always contain 1 or 10 pairs")
		assert(d.num >= 0 and d.num <= d.cnt, "invalid num "..tostring(d.num).." vs cnt "..tostring(d.cnt))
		table.insert(instructions, d)
		-- TODO micro-optimization: obmit save known stuff from permutations
		-- if d.cnt == 1 and apply and not added then
		-- 	d.save = true
		-- 	assert(d.num == 1 or d.num == 0)
		-- 	if d.num == 1 then
		-- 		for i1,i2 in pairs(d.matches) do
		-- 			assert(swap_idx >= 0)
		-- 			translate(i1, swap_idx, i2, swap_idx)
		-- 			swap_idx = swap_idx -1
		-- 			break -- only one element in d.matches
		-- 		end
		-- 	end
		-- end
	end

	local line = file:read("l")
	while line do
		if line:match("^#") then
			-- print("skip")
		elseif line == "" then
				state = state + 1
		elseif line == "apply to here" then
			apply = false
		else
			if state >= 2 and swap_idx < 0 then
				lut1,lut2 = _M.gen_lut(s1), _M.gen_lut(s2)
				swap_idx = #s1
			end
			if state == 0 then
				if rev then parse_set(line, s2) else parse_set(line, s1) end
			elseif state == 1 then
				if rev then parse_set(line, s1) else parse_set(line, s2) end
			elseif state >= 2 then
				parse_instruction(line)
			end
		end
		line = file:read("l")
	end
	if swap_idx < 0 then
		lut1,lut2 = _M.gen_lut(s1), _M.gen_lut(s2)
		swap_idx = #s1
	end
	-- swap idx determines upt to which position should be permutated
	return s1,s2,instructions,swap_idx,dup,added
end

function _M.check_all(map, c, dup, pr)
	if pr and pr > 1 then print(dup) end
	local function count_lights()
		local cnt = 0
		for i1,i2 in pairs(c.matches) do
			if not map[i1] then
				error("map is not fully defined")
			elseif map[i1] == i2 then
				cnt = cnt+1
			end
		end
		return cnt
	end
	local function count_pendant()
		-- handle dup (1)
		-- find the index which maps to dup[2]
		local dup2_idx
		if pr and pr > 1 then print(dup) end
		for i1,i2 in pairs(map) do
			if i2 == dup[2] then dup2_idx = i1 end
		end
		local cnt
		if pr and pr > 1 then print(dup[1], dup[2], dup2_idx) end
		-- if the additional person is mapped to another person, we know who the second match is and who has two matches -> use this Information
		if dup2_idx and map[dup[1]] ~= dup[2] then
			-- swap
			map[dup[1]],map[dup2_idx] = map[dup2_idx],map[dup[1]]
			assert(map[dup[1]] == dup[2])
			cnt = count_lights()
			-- restore map
			map[dup[1]],map[dup2_idx] = map[dup2_idx],map[dup[1]]
		end
		return cnt
	end

	-- count lights if the given map would be the solution
	local cnt = count_lights()

	if pr and pr > 1 then print(c.added) end
	if c.cnt > 1 and c.added then
		cnt = math.max(cnt, count_pendant() or 0)
	end

	local c_num = c.num
	if pr and pr > 1 then print(cnt, c_num) end

	if cnt ~= c_num then
		if pr and pr > 1 then print("false") end
		return false
	end

	if pr and pr > 1 then print("true") end
	return true
end
function _M.check(map, constr, dup, pr)
	local r = true
	for _,c in ipairs(constr) do
		r = r and _M.check_all(map, c, dup, pr) -- if one of both is false, r will stay false forever
	end
	return r
end

function _M.entropy_single(l, i1,i2, is_match, total)
	local info
	local t = perm.count_pred(l, function(map) return map[i1] == i2 end)
	local f = perm.count_pred(l, function(map) return map[i1] ~= i2 end)
	if is_match then info = -math.log(t/total,2) else info = -math.log(f/total,2) end
	if f == 0 then
		assert(t ~= 0)
		return - t/total*math.log(t/total,2), info
	elseif t == 0 then
		assert(f ~= 0)
		return - f/total * math.log(f/total,2), info
	else
		return - f/total * math.log(f/total,2) - t/total*math.log(t/total,2), info
	end
end
function _M.entropy_single_max(l, total)
	local r,e1,e2 = -1,-1,-1
	for i1=1,#l[1] do
		for i2=1,#l[1] do
			local e = _M.entropy_single(l, i1,i2, false, total)
			if e > r then
				r,e1,e2 = e,i1,i2
			end
		end
	end
	return r,e1,e2
end
function _M.entropy_all(l, constr, total, dup, lights)
	local e,info = 0,-1
	local lights_real = constr.num
	for i=0,lights do
		constr.num = i
		local c = perm.count_pred(l, function(map2) return _M.check_all(map2,constr,dup) end)
		if i == lights_real then info = -math.log(c/total, 2) end
		if c ~= 0 then
			e = e - c/total * math.log(c/total, 2)
		end
	end
	constr.num = lights_real
	return e,info
end
function _M.entropy_all_max(l, total, dup, lights)
	local r,mi = -1,-1
	for ei,map in ipairs(l) do
		local e = _M.entropy_all(l, {matching=map}, total, dup, lights)
		if e > r then
			r,mi = e,ei
		end
	end
	return r,l[mi]
end

function _M.entropy(poss, c, dup)
	local g,h,h_real,info
	if c.cnt > 1 then
		-- all
		-- calculating the max entropy is very expensive (10(lights) *
		-- #poss(possible matchings) * #poss(count_pred) constraints checken)
		-- -> only do this if the possibilities narrow down a bit
		if #poss <= 0 then
			h,g = _M.entropy_all_max(poss, #poss, dup, 10)
		else
			h,g = 0,{}
		end
		pr_time("entropy all start")
		h_real,info = _M.entropy_all(poss, c, #poss, dup, 10)
		pr_time("entropy all end")
	else
		-- single
		local g1,g2
		if #poss <= 3628800 then
			h,g1,g2 = _M.entropy_single_max(poss, #poss)
		else
			h,g1,g2 = 0,1,1
		end
		for i1,i2 in pairs(c.matches) do
			pr_time("entropy single start")
			h_real,info = _M.entropy_single(poss, i1,i2, c.num==1, #poss)
			pr_time("entropy single end")
			break -- is only one
		end
		g = {[g1]=g2}
	end
	return g,h,h_real,info
end

function _M.write_entro_guess(h,g,s1,s2, num, info)
	io.write(h)
	for e1,e2 in pairs(g) do
		io.write(" ", s1[e1], "->", s2[e2])
	end
	if num and info then
		io.write(" |-> ", num, " -> ", info)
	end
	io.write("\n")
end

function _M.write_matches(g,s1,s2)
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

function _M.poss_print(p, s1,s2)
	for _,map in ipairs(p) do
		_M.print_map(map, s1,s2)
		print()
	end
end

local epsilon = 0.00005
function _M.prob_tab(p, s1, s2, t)
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
			tab[co] = perm.count_pred(p, function(map) return map[i] == j end)
			-- print("\n", tab[co])
			tab[co] = tab[co]/(#p/100)
			if 100-epsilon < tab[co] and tab[co] < 100+epsilon then
				io.write(tostring(colors.green), string.format("%"..tostring(ml)..".4f", tab[co]), tostring(colors.reset), "|")
			elseif 0-epsilon < tab[co] and tab[co] < 0+epsilon then
				io.write(tostring(colors.dim),tostring(colors.red), string.format("%"..tostring(ml)..".4f", tab[co]), tostring(colors.reset), "|")
			elseif t and t[co] and tab[co] > t[co]-epsilon and tab[co] < t[co]+epsilon then
				io.write(tostring(colors.bright),tostring(colors.black), string.format("%"..tostring(ml)..".4f", tab[co]), tostring(colors.reset), "|")
			else
				io.write(string.format("%"..tostring(ml)..".4f|", tab[co]))
			end
		end
		io.write("\n")
	end
	return tab
end

-- local cli   = require 'cliargs'
-- local function isNumber(key, value)
-- 	if not tonumber(value) then print("Option " .. key .. " must be a number (was "..value..")") return nil, "" end
-- 	return value
-- end
--
-- cli:set_name("aoc_cli.lua")
-- cli:splat("date", "day and year to read from (default: today)", nil, 2)
-- cli:option("-s, --solution=SOLUTION", "send a solution to the server")
-- cli:option("-p, --part=PART", "set the part of the exercise", "1")
-- cli:flag("-f, --fetch", "fetch the problem statement")
-- cli:flag("-i, --input", "get the input file")
-- cli:flag("--print-problem", "print the problem statement after fetching")
-- cli:option("-k, --key=KEY", "the line in the .sess file to use", "1", isNumber)
-- TODO -x should be interactive on the xth but least run, nil should disable this completely
local interactive = -1
if arg[2] and arg[2] == "-i" then
	if arg[3] then
		interactive = tonumber(arg[3])
	else
		interactive = 0
	end
end

-- parsing
local file = io.open(arg[1])
local s1,s2,instructions,perm_num,dup,added = parse_file(file, false)
file:close()
pr_time("parsing done")

-- permgen
local a = {}
for i=1,#s1 do a[i] = i end
local poss = perm.permgen(
	a,
	perm_num,
	{},
	function(map)
		return true
	end,
	function(map)
		local r = {}
		local a = added or 0
		-- obmit later added stuff
		for i=1,#map-a do
			r[i] = map[i]
		end
		return r
	end)
print("total", #poss)
pr_time("permgen done")
print()

local tabSingle,tabAll
for _,c in ipairs(instructions) do
	if c.add then
		assert(#poss[1]+1 == dup[1], string.format("%d %d %d", #poss[1], dup[1], dup[2]))
		poss = perm.poss_append(poss, dup[2])
		print("add "..s1[dup[1]].." "..s2[dup[2]].." to the possibilities")
		print(#poss, "poss left")
	elseif c.constraint then
		-- entropy stuff
		local g,h,h_real,info = _M.entropy(poss, c, dup)
		_M.write_entro_guess(h,g, s1,s2)
		_M.write_entro_guess(h_real,c.matches, s1,s2, c.num, info)
		poss = perm.filter_pred(poss, function(map)
			return _M.check_all(map, c, dup)
		end)
		print(#poss, "poss left")
		if #poss < 3265920 then
			if c.cnt > 1 then
				tabSingle = _M.prob_tab(poss, s1, s2, tabSingle)
			else
				tabAll = _M.prob_tab(poss, s1, s2, tabAll)
			end
		end
		print()
	else
		error("invalid instruction")
	end
	if interactive == 0 then
		local lut1,lut2 = _M.gen_lut(s1), _M.gen_lut(s2)
		M = {
			entropy_single = function(e1,e2, is_match)
				return _M.entropy_single(poss, lut1[e1], lut2[e2], is_match, #poss)
			end,
			entropy_all    = function(c, rev)
				local c = {apply=false, added=1}
				for e1,e2 in pairs(co.matches) do
					assert(lut1[e1]) assert(lut2[e2])
					if not rev then
						c[lut1[e1]] = lut2[e2]
					else
						c[lut2[e2]] = lut1[e1]
					end
				end
				return _M.entropy_app(poss, c, #poss, dup, 10)
			end,
			count_applied  = function(co,rev)
				local c = {apply=false, added=1}
				for e1,e2 in pairs(co.matches) do
					assert(lut1[e1]) assert(lut2[e2])
					if not rev then
						c[lut1[e1]] = lut2[e2]
					else
						c[lut2[e2]] = lut1[e1]
					end
				end
				return perm.count_pred(poss, function(map) return _M.check(map, c, dup) end)
			end
		}
		print("M.entropy_single(e1,e2, is_match) -> entropy,info", "M.entropy_all(constr,rev) -> entropy,info", "count_applied(constr,rev) -> count", "constr={num=%d, matches={...}, cnt=%d}")
		prompt.enter()
		M = nil
	else
		interactive = interactive > 0 and interactive-1 or -1
	end
end

if #poss <= 40 then
	local of = io.open("test.dot", "w")
	pr_time("generate dot")
	_M.poss_to_dot(poss, s1,s2, of)
	of:close()
	pr_time("generate pdf")
	os.execute(string.format("dot -Tpdf -o 'test.pdf' 'test.dot'"))
end

pr_time("end")
return _M
