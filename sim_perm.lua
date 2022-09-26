local perm      = require("perm")
local colors    = require"term.colors"
local prompt    = require"prompt"
prompt.colorize = true
prompt.name     = "sim"
prompt.history  = "sim.hist" -- otherwise no history

-- kinds of TODOs: TODOmoredup

----------------
-- EXIT-CODES --
----------------
-- +0 -> success
-- -1 -> error on argument parsing

local _M = {}

local function table_copy(t)
	if type(t) ~= "table" then return t end
	local r = {}
	for k,v in ipairs(t) do
		r[k] = table_copy(v)
	end
	return r
end

-----------------
--  CONSTRAINT --
-----------------
-- default values
local constraint = {table=nil,tablePR=nil, eliminated=0, map=nil,cnt=nil, right=nil, name=nil,flags=nil, noNight=nil,noBox=nil}
constraint.__index = constraint
-- constructor
function constraint:new(o)
	o = o or {}
	for _,v in ipairs{"map","cnt", "right"} do assert(o[v]) end
	setmetatable(o, self)
	return o
end
function constraint.gen_table(l1,l2, init)
	init = init or 0
	local tab = {}
	for i1=1,l1 do
		tab[i1] = {}
		for i2=1,l2 do
			tab[i1][i2] = init
		end
	end
	return tab
end
function constraint:eliminate(p, num, ds)
	num = num or 1
	ds  = ds or {}
	assert(self.table)
	for i1,t2 in pairs(p) do
		-- for _,i1 in pairs(t1) do
			for _,i2 in pairs(t2) do
				self.table[i1][i2] = self.table[i1][i2]+num
			end
		-- end
			for _,d in ipairs(ds) do
				self.table[i1][d] = self.table[i1][d]+1 --TODOmoredup
			end
	end
	self.eliminated    = self.eliminated + num
end
function constraint:apply_table(tab, total)
	assert(self.table)
	self.entro_real = -math.log(1-self.eliminated/total, 2)
	total = total - self.eliminated
	if self.cnt == 1 then
		for k,v in pairs(self.map) do
			assert(not self.entro) -- ensure loop is only run once
			local nomatch, match
			-- the if could be avoided (just added anyways)
			if self.right == 1 then
				nomatch = - self.table[k][v]/total * math.log(self.table[k][v]/total, 2)
				  match = - (total-self.table[k][v])/total * math.log((total-self.table[k][v])/total, 2)
			else
				  match = - self.table[k][v]/total * math.log(self.table[k][v]/total, 2)
				nomatch = - (total-self.table[k][v])/total * math.log((total-self.table[k][v])/total, 2)
			end
			self.entro = nomatch + match
		end
	end
	if self.cnt == 1 then
		local max = {H=0,i1=-1,i2=-1} -- percent values
		for i1,v1 in ipairs(tab) do
			for i2,c in ipairs(v1) do
				-- print(string.format("i1:%d i2:%d \t c:%f -> %f el:%d t:%d", i1,i2, c, c/(total+self.eliminated), self.eliminated, total))
				c = c/(total+self.eliminated) -- use old total
				assert(c < 1.1, string.format("c:%f el:%d t:%d", c, self.eliminated, total))
				local H = - c * math.log(c,2) - (1-c) * math.log(1-c,2)
				if H > max.H then
					max.H=H max.i1=i1 max.i2=i2 max.p=c
				end
			end
		end
		self.entro_max = max
	end
	self.tablePR = {}
	for k1,v1 in ipairs(self.table) do
		self.tablePR[k1] = {}
		for k2,v2 in ipairs(v1) do
			self.tablePR[k1][k2] = (tab[k1][k2] - v2)*100/total
			tab[k1][k2]          = (tab[k1][k2] - v2)
		end
	end
	return total
end
local epsilon = 0.00005
function constraint:print_table(s1,s2)
	assert(self.tablePR)

	local ml = 0
	for _,v in ipairs(s1) do if #v > ml then ml = #v end end
	for _,v in ipairs(s2) do if #v > ml then ml = #v end end

	if self.entro_max then
		io.write("opt: ", s1[self.entro_max.i1], " -> ", s2[self.entro_max.i2], " => H = ", self.entro_max.H)
		io.write("\n")
	end

	if self.map then
		io.write(self.right)
		if self.noNight then
			io.write("\tMN#", self.noNight)
		end
		if self.noBox then
			io.write("\tMB#", self.noBox)
		end
		io.write("\t", self.name and self.name or "")
		io.write("\n")
		_M.print_constr_map(self.map, s1, s2)
	end

	if self.entro then
		io.write(("H = %.4f bit/X "):format(self.entro))
	end
	io.write(("-> I = %.4f bit"):format(math.abs(self.entro_real)))
	io.write("\n")

	io.write(("%"..tostring(ml).."s|"):format(""))
	for _,v in ipairs(s2) do
		io.write(("%"..tostring(ml).."s|"):format(v))
	end
	io.write("\n")
	for k1,v1 in ipairs(self.tablePR) do
		io.write(("%"..tostring(ml).."s|"):format(s1[k1]))
		for _,v2 in ipairs(v1) do
			if 80-epsilon < v2 and v2 < 100+epsilon then
				io.write(tostring(colors.green))
			elseif 0-epsilon < v2 and v2 < 0+epsilon then
				io.write(tostring(colors.dim), tostring(colors.red))
			-- elseif xyz then
			-- 	io.write(tostring(colors.bright), tostring(colors.black))
			end
			io.write(("%"..tostring(ml)..".4f|"):format(v2), tostring(colors.reset))
		end
		io.write("\n")
	end
end
function constraint:write_dot_tab(s1,s2, fn)
	local of = io.open(fn..".dot", "w")
	assert(of, "opening '"..fn..".dot' failed")
	of:write("digraph structs { node[shape=plaintext] struct[label=<\n")
	of:write('<table cellspacing="2" border="0" rows="*" columns="*">\n')

	of:write("<tr>")
	-- pr_time("generate dot tab")
	of:write("<td></td>")
	for _,v in ipairs(s2) do
		of:write("<td><B>", v, "</B></td>")
	end
	of:write("</tr>\n")
	for k1,v1 in ipairs(self.tablePR) do
		of:write("<tr>")
		of:write("<td><B>", s1[k1], "</B></td>")
		for _,v2 in ipairs(v1) do
			of:write("<td>")
			if 80-epsilon < v2 and v2 < 100+epsilon then
				of:write('<font color="darkgreen">')
			elseif 0-epsilon < v2 and v2 < 0+epsilon then
				of:write('<font color="red">')
			else
				of:write('<font color="black">')
			end
			of:write(("%.4f"):format(v2), "</font></td>")
		end
		of:write("</tr>\n")
	end
	of:write("</table>")
	of:write(">];}")
	of:close()
	-- pr_time("generate pdf")
	os.execute(string.format("dot -Tpdf -o '%s.pdf' '%s.dot'", fn,fn))
	os.execute(string.format("dot -Tpng -o '%s.png' '%s.dot'", fn,fn))
end
-------------------
--  HELPER STUFF --
-------------------
function math.factorial(n)
	assert(n >= 0)
	local prod = 1
	for i=1,n do
		prod = prod * i
	end
	return prod
end
local function pr_time(s)
	-- print(os.date("%Y-%m-%d %H:%M:%S"), s)
end
pr_time("start")

local function dot_node(file, par, self, e1, e2s)
	file:write("{", '"',self,'"', '[shape="record" label=<<table border="0" cellborder="0" cellspacing="0"><tr><td>',e1,'</td></tr>')
	for _,v in ipairs(e2s) do
		file:write('<tr><td>',v,'</td></tr>')
	end
	file:write('</table>>]', "}\n")
	for p,_ in pairs(par) do
		file:write('"',p,'"', " -> {", '"',self,'"}\n')
	end
end
local function serialize_i2s(i2s)
	local first = true
	local i2_string = ""
	for _,v in ipairs(i2s) do
		if first then
			i2_string = v
			first = false
		else
			i2_string = i2_string .. "," .. v
		end
	end
	return i2_string
end
local function group_cnt(l)
	local r = {}
	for _,v in ipairs(l) do
		for i1,i2s in ipairs(v) do
			local co = ("%d|%s"):format(i1, serialize_i2s(i2s))
			r[co] = (r[co] or 0) +1
		end
	end
	return r
end
local function tree_ordering(ps)
	local len = #ps[1]
	local tab = group_cnt(ps)
	local amounts = {}
	for i=1,len do amounts[i] = {idx=i, cnt=nil} end
	for co,cnt in pairs(tab) do
		if cnt > 0 then
			local i1,_ = co:match("(%d+)|(.+)")
			i1 = tonumber(i1)
			amounts[i1].cnt = (amounts[i1].cnt or 0) + 1
		end
	end
	table.sort(amounts, function(a,b) return a.cnt < b.cnt end)
	return amounts
end
function _M.poss_to_dot_tree(ps, s1,s2, file, collapse)
	local order = tree_ordering(ps)
	local nodes = {}
	for _,p in ipairs(ps) do
		local par = "root"
		for _,o in ipairs(order) do
			local i1 = o.idx
			local i2s = p[i1]
			local first,i2_string = true,""
			local e2 = {}
			for _,v in ipairs(i2s) do
				table.insert(e2, s2[v])
				if first then
					i2_string = v
					first = false
				else
					i2_string = i2_string .. "," .. v
				end
			end
			local co
			if collapse then
				co = string.format("%d|%s", i1,i2_string)
			else
				co = string.format("%s|%d|%s", par, i1,i2_string)
			end
			if not nodes[co] then
				nodes[co] = {{}, s1[i1],e2}
			end
			nodes[co][1][par] = true
			par = co
		end
	end
	file:write("digraph D {\n")
	for co,x in pairs(nodes) do
		local par,e1,e2 = table.unpack(x)
		dot_node(file, par, co, e1, e2)
	end
	file:write("}\n")
end
-- just a small wrapper 
local function write_dot_tree(fn, poss, s1,s2, bound, collapse)
	if #poss <= bound then
		local of = io.open(fn..".dot", "w")
		assert(of, "opening '"..fn..".dot' failed")
		pr_time("generate dot tree")
		_M.poss_to_dot_tree(poss, s1,s2, of, collapse)
		of:close()
		pr_time("generate pdf")
		os.execute(string.format("dot -Tpdf -o '%s.pdf' '%s.dot'", fn,fn))
		os.execute(string.format("dot -Tpng -o '%s.png' '%s.dot'", fn,fn))
	end
end

function _M.gen_lut(s)
	local r = {}
	for i,v in ipairs(s) do
		assert(r[v] == nil, "value "..v.." occures multiple times")
		r[v] = i
	end
	return r
end

function _M.print_constr_map(m, s1,s2)
	for k,v in pairs(m) do
		io.write(s1[k], " -> ", s2[v], "\n")
	end
end
function _M.print_map(m, s1,s2)
	for k,_v in pairs(m) do
		io.write(s1[k], " -> {")
		local first = true
		for _,v in ipairs(_v) do
			if not first then io.write(", ") else first = false end
			io.write(s2[v])
		end
		io.write("}\n")
	end
end
function _M.poss_print(ps, s1,s2)
	for _,p in ipairs(ps) do
		_M.print_map(p, s1,s2)
		print()
	end
end

local function parse_file(file, rev)
	local s1,s2,instructions,instrs,dup = {},{},{},{},{{},{}}
	-- real local
	local lut1,lut2,state,final,noNight,noBox = {},{},0,false,1,1

	local function next_line(line)
		line = file:read("l")
		while line and line:match("^#") do line = file:read("l") end
		return line
	end
	local function parse_set(line, s)
		table.insert(s, line:match("%s*([^%->,%s]+)%s*.*"))
	end
	local function parse_instruction(line)
		if line == "add" then
			assert(#dup[1] == 0 and #dup[2] == 0, "currently only one can be dupped")
			-- add instruction
			-- elements will be added at the end -> obmit from permutations
			line = next_line(line)
			if line ~= "" and line ~= "Dummy" then
				local work = not rev and s1 or s2
				assert(work ~= s1, "adding is only allowed to the second set")
				parse_set(line, work)
				table.insert(dup[1], #work)
			end
			line = next_line(line)
			if line ~= "" and line ~= "Dummy" then
				local work = not rev and s2 or s1
				assert(work ~= s1, "adding is only allowed to the second set")
				parse_set(line, work)
				table.insert(dup[2], #work)
			end
			lut1,lut2 = _M.gen_lut(s1), _M.gen_lut(s2)
			table.insert(instructions, instrs)
			instrs = {}
			return
		end
		-- constraint
		local tmp = {}
		for ele in (line:gsub("#.*", "") .. " "):gmatch("[^%s]* ") do table.insert(tmp, ele) end
		assert(1 <= #tmp and #tmp <= 3, "Error on parsing '" .. line .. "'")
		local d = {right=tonumber(tmp[1]), name=tmp[2], flags=tmp[3], map={}, cnt=0}
		if d.flags and d.flags:match("f") then
			if not final then
				final = true
			else
				d.flags = d.flags:gsub("f", "")
			end
		end
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
			d.map[i1] = i2 -- ATTENTION: d.map might be hash-map
			d.cnt     = d.cnt+1
			line      = next_line(line)
		end
		if not d.flags or not d.flags:match("c") then
			if d.cnt == 1 then
				d.noBox = noBox
				noBox = noBox + 1
			else
				d.noNight = noNight
				noNight = noNight + 1
			end
		end
		assert(d.right >= 0 and d.right <= d.cnt, "invalid right "..tostring(d.right).." vs cnt "..tostring(d.cnt))
		table.insert(instrs, constraint:new(d))
	end

	local line = file:read("l")
	while line do
		if line:match("^#") then
			-- print("skip")
		elseif line == "" then
				state = state + 1
		else
			if state >= 2 then
				lut1,lut2 = _M.gen_lut(s1), _M.gen_lut(s2)
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
	assert(#s1 - #dup[1] == #s2 - #dup[2], "unbalanced starting point")
	if #instrs > 0 then table.insert(instructions, instrs) end
	return s1,s2,instructions,dup
end

local function any_map(l, bar)
	for k,v in ipairs(l) do
		if bar(k,v) then return true end
	end
	return false
end

function _M.count_lights(matching, c)
		local cnt = 0
		for i1,i2 in pairs(c.map) do
			if not matching[i1] then
				error("map is not fully defined")
			elseif any_map(matching[i1], function(_,x) return x == i2 end) then
			-- elseif map[i1] == i2 then
				cnt = cnt+1
			end
		end
		return cnt
	end

function _M.hist(instructions, s1, s2, rev, mno, mbo)
	local len = 0
	for _,v in ipairs(s1) do len = math.max(len, #v) end
	for _,v in ipairs(s2) do len = math.max(len, #v) end
	if rev then s1,s2 = s2,s1 end
	local i_ = 0
	io.write("  #  |")
	io.write("R |")
	for i1=1,#s1 do
		if not s1[i1]:match("^[dD]ummy$") then
			io.write(string.format("%"..tostring(len).."s| ", s1[i1]))
			i_ = i_ + 1
		end
	end
	io.write("  I   ")
	io.write("\n", string.rep("=",6 + 3 + 5 + 3 + i_*(len+2)), "\n")
	for _,instrs in ipairs(instructions) do
		for _,e in ipairs(instrs) do
			local m = {}
			for k,v in pairs(e.map) do m[not rev and k or v] = not rev and v or k end
			if e.noNight and e.cnt ~= 1 then
				io.write(string.format("MN#%02d|", e.noNight))
			elseif e.noBox and e.cnt == 1 then
				io.write(string.format("MB#%02d|", e.noBox))
			else
				io.write("  -  |")
			end
			io.write(string.format("%02d|", e.right))
			for i1=1,#s1 do
				if not s1[i1]:match("^[dD]ummy$") then
					local i2 = m[i1]
					io.write(string.format("%"..tostring(len).."s| ", s2[i2 or -1] or ""))
				end
			end
			if e.noNight and mno then
				mno:write(("%d %.4f\n"):format(e.noNight, math.abs(e.entro_real)))
			end
			if e.noBox and mbo then
				mbo:write(("%d %.4f\n"):format(e.noBox, math.abs(e.entro_real)))
			end
			io.write(("|%.4f|"):format(math.abs(e.entro_real)))
			io.write("\n")
		end
	end
	io.write("\n")
end

local function arguments()
	local cli   = require 'cliargs'
	local function isNumber(key, value)
		local v = tonumber(value)
		if not v then
			print("Option " .. key .. " must be a number (was "..value..")")
			os.exit(-1)
		end
	end
	local function has_ext(ext, key, value)
		if not value:match("%."..ext.."$") then
			print("Option " .. key .. " has to end with '."..ext.."' (was '"..value.."')")
			os.exit(-1)
		end
	end
	-- local function isChoice(choices, key, value)
	-- 	for _,v in ipairs(choices) do
	-- 		if v == value then return end
	-- 	end
	-- 	io.write("Option ",key," has to be one of ")
	-- 	for _,v in ipairs(choices) do
	-- 		io.write(v," ")
	-- 	end
	-- 	io.write("(was '",value,"')", "\n")
	-- 	os.exit(-1)
	-- end
	local function file_exists(key,value)
		local f = io.open(value,"r")
		if f == nil then
			io.write("Given file '",value,"' (for",key,") has to exist.")
			os.exit(-1)
		end
		io.close(f)
	end
	local function file_not_exists(key,value)
		local f = io.open(value,"r")
		if f ~= nil then
			io.close(f)
			io.write("Given file '",value,"' (for ",key,") shall not exist.")
			os.exit(-1)
		end
	end

	cli
		:set_name(arg[0])
		:set_description("Runs a simulation of finding one of the right mappings from one set to another by using some hints")
		:argument("INPUT", "path to the input .dat file", function(k,v) return has_ext("dat", k,v) and file_exists(k,v) end)
		-- :option("-i, --interactive=LEVEL", "sets the level of interaction (x to skip first x runs, -x to start with xth run counted from last)", nil, isNumber)
		:flag("-i, --[no-]interactive", "interactive shell in the end", false)
		:flag("-c, --[no-]color", "use color in shell output", false)
		-- :option("-b, --bound=BOUND", "entropy bound", 5000, isNumber)
		:option("-d, --dot-bound=BOUND", "dot bound", 200, isNumber)
		-- :option("-f, --fast=LEVEL", "Fast run (0->no fast, 1->omit entropy, 2->omit prob table)", 0, function(k,v) isChoice({"0","1","2"}, k,v) end)
		:flag("-r, --[no-]reverse", "switch/reverse sets", false)
		-- :flag("-s, --[no-]stats", "collect stats", true)
		:option("-o, --output=OUTPUT", "Output STEM for .dot and .pdf", "test", function(k,v) return file_not_exists(k..".pdf",v) and file_not_exists(k..".dot",v) end)

	local arg,err = cli:parse()
	if not arg then
		print(err)
		os.exit(-1)
	end
	-- tonumber
	for _,v in ipairs{"d"} do arg[v] = tonumber(arg[v]) end
	for _,k in ipairs{"INPUT", "d", "r", "o", "c"} do assert(arg[k] ~= nil) end
	return arg
end
-- create a {} -> {} mapping and insert dup elements
local function conv(p)
	local ret = {}
	for k,v in pairs(p) do
		ret[k] = {v}
	end
	coroutine.yield(ret)
end
local function _filter(iter, foo)
	for k,v in table.unpack(iter) do
		if foo(k,v) then
			coroutine.yield(k,v)
		end
	end
end
local function filter(iter, foo)
	return coroutine.wrap(function() _filter(iter, foo) end)
end
local function _explode(iter, dup)
	-- TODOmoredup
	assert(#dup[1] == 0 and #dup[2] == 1, "not suported -> try reverse")
	local d = dup[2][1]
	for v1 in table.unpack(iter) do
		for k_ref,_ in pairs(v1) do
			local ret = {}
			for k,v in pairs(v1) do
				if k_ref == k then v=table_copy(v) table.insert(v, d) end
				ret[k] = v
			end
			coroutine.yield(ret)
		end
	end
end
local function explode(iter, dup)
	return coroutine.wrap(function() _explode(iter, dup) end)
end

local arg = arguments()

if not arg.c then
	colors = setmetatable({}, {
		__index = function(_,_) return "" end
	})
end

-- parsing
local file = io.open(arg["INPUT"])
local s1,s2,instructions,dup = parse_file(file, arg["r"])
file:close()
for _,instrs in ipairs(instructions) do for _,c in ipairs(instrs) do c.table = constraint.gen_table(#s1,#s2) end end
pr_time("parsing done")

-- calculate stuff
local total = math.factorial(10)*10
local left = {}
-- basic map
local a = {}
-- (#s1-#dup[1]) == (#s2-#dup[2])
for i=1,(#s1-#dup[1]) do a[i] = i end

-- TODOmoredup
local iter = {perm.permgen(a, #a, function(m) return conv(m) end)}
iter = {filter(iter, function(p)
	for _,c in ipairs(instructions[1]) do
		if _M.count_lights(p, c) ~= c.right then
			c:eliminate(p, 10, dup[2]) -- TODOmoredup
			return false
		end
	end
	return true
end)}
if instructions[2] then
	iter = {explode(iter, dup)}
	iter = {filter(iter, function(p)
		for _,c in ipairs(instructions[2]) do
			if _M.count_lights(p, c) ~= c.right then
				c:eliminate(p)
				return false
			end
		end
		return true
	end)}
end
for p in table.unpack(iter) do
	table.insert(left, p)
end

-- evaluate/print stuff
print(("total %d -> %f bit left"):format(total, -math.log(1/total, 2)))

local tab = constraint.gen_table(#s1, #s2, 0) -- TODOmoredup

local tmp = {table=tab, eliminated=0}
constraint.apply_table(tmp, constraint.gen_table(#s1,#s2, math.factorial(#s1-#dup[1]-1)*(#s1-#dup[1])), total)
constraint.print_table(tmp,s1,s2)

tab = constraint.gen_table(#s1, #s2, math.factorial(#s1-#dup[1]-1)*(#s1-#dup[1])) -- TODOmoredup
print()
local info,info_ctr = io.open(arg.o.."_statInfo.out", "w"),1
for _,instrs in ipairs(instructions) do
	for _,instr in ipairs(instrs) do
		total = instr:apply_table(tab, total)
		if instr.flags and instr.flags:match("f") then
			print(("="):rep(80))
		end
		instr:print_table(s1,s2)
		print(("%d left -> %f bit left"):format(total, -math.log(1/total, 2)))
		if info and (instr.noNight or instr.noBox) then
			info:write(("%d %.4f\n"):format(info_ctr, -math.log(1/total, 2)))
			info_ctr = info_ctr + 1
		end
		print()
	end
end
info:close()

_M.hist(instructions, s1, s2, false)
print()
local mbo = io.open(arg.o.."_statMB.out", "w")
local mno = io.open(arg.o.."_statMN.out", "w")
_M.hist(instructions, s1, s2, true, mbo, mno)
mno:close()
mbo:close()

write_dot_tree(arg.o, left, s1, s2, arg.d)
instructions[#instructions][#instructions[#instructions]]:write_dot_tab(s1, s2, arg.o.."_tab")
-- _M.poss_print(left, s1, s2)

pr_time("end")
return _M
