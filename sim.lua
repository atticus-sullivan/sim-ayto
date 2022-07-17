local _M = {}

local function table_copy(t)
	local r = {}
	for k,v in pairs(t) do
		r[k] = v
	end
	return r
end
local function table_append(t1, t2)
	for _,v in ipairs(t2) do
		table.insert(t1, v)
	end
end
function _M.print_map(m)
	for k,v in pairs(m) do
		io.write(k, " -> ", v, "\n")
	end
end

local function backtrack(map, s1, s2, check, lvl)
	if not check(map) then
		return nil
	end
	if #s1 <= 0 then
		return true
	end
	local e1 = table.remove(s1)
	local node
	for i2 = 1,#s2 do
		local e2 = table.remove(s2, i2)
		map.m[e1] = e2
		map.cnt = map.cnt+1
		if lvl then
			io.write(string.rep(" ", lvl*4), "trying: ", e1, " -> ", e2, "\n")
		end
		local r = backtrack(map, s1, s2, check, lvl and lvl+1 or nil)
		if r then
			node = node or {name=e1, children={}}
			node.children[e2] = r
		end
		table.insert(s2, i2, e2)
		map.m[e1] = nil
		map.cnt = map.cnt-1
	end
	table.insert(s1, e1)
	return node
end

local function check_des(map, des)
	for _,d in ipairs(des) do
		local cnt,free = 0,0
		for _,e in ipairs(d.matches) do
			-- print(e[1], e[2])
			if not map.m[e[1]] then
				free = free+1
			elseif map.m[e[1]] == e[2] then
				cnt = cnt+1
			end
		end
		-- _M.print_map(map.m)
		-- if d.num == 1 then print(cnt) end
		if cnt < d.num-free or cnt > d.num then
			-- print("false")
			return false
		end
		-- print("keep on")
	end
	return true
end

local function table_contains(t, ele)
	for _,v in ipairs(t) do
		if v == ele then return true end
	end
	return false
end

local function parse_file(file)
	local s1,s2,des,mb,state = {},{},{},{},0

	local function next_line(line)
		line = file:read("l")
		while line:match("^#") do line = file:read("l") end
		return line
	end

	local function parse_set(line, s)
		table.insert(s, line)
	end

	local function parse_mb(line)
		local e1,e2 = line:match("^([^%->/]+)%->([^%->/]+)$")
		if not e1 or not e2 then
			e1,e2 = line:match("^([^%->/]+)%-/>([^%->/]+)$")
			assert(table_contains(s1, e1), "invalid mb mapping")
			assert(table_contains(s2, e2), "invalid mb mapping")
			assert(e1 and e2, "error")
			table.insert(mb, {e1, e2, false})
		else
			assert(table_contains(s1, e1), "invalid mb mapping")
			assert(table_contains(s2, e2), "invalid mb mapping")
			assert(e1 and e2, "error")
			table.insert(mb, {e1, e2, true})
		end
	end

	local function parse_mn(line)
		local d = {num=tonumber(line:match("^%d+")), matches={}}
		line = next_line(line)
		while line and line ~= "" do
			local e1,e2 = line:match("^([^%->,]+)%->([^%->,]+)$")
			assert(table_contains(s1, e1), "invalid mn mapping")
			assert(table_contains(s2, e2), "invalid mn mapping")
			table.insert(d.matches, {e1, e2})
			line = next_line(line)
		end
		table.insert(des, d)
	end

	local line = file:read("l")
	while line do
		if line:match("^#") then
			-- print("skip")
		elseif line == "" then
				state = state + 1
		else
			if state == 0 then
				parse_set(line, s1)
			elseif state == 1 then
				parse_set(line, s2)
			elseif state == 2 then
				parse_mb(line)
			elseif state >= 3 then
				parse_mn(line)
			else
				error("invalid state")
			end
		end
		line = file:read("l")
	end
	return s1,s2,des,mb
end

function _M.run(file)
	local s1,s2,des,mb = parse_file(file)
	local map={cnt=0, m={}}
	-- handle mb
	local rm1,rm2 = {},{}
	local pre_tree = {node={name=nil, children={}}}
	pre_tree.root = pre_tree.node
	for _,v in ipairs(mb) do
		if v[3] then
			map.m[v[1]] = v[2]
			local node = {name=nil, children={}}
			pre_tree.node.name = v[1]
			pre_tree.node.children[v[2]] = node
			pre_tree.node = node
			rm1[v[1]],rm2[v[2]] = true,true
		else
			table.insert(des, {num=0, matches={{v[1],v[2]}}})
		end
	end
	for i=#s1,1,-1 do if rm1[s1[i]] then table.remove(s1,i) end end
	for i=#s2,1,-1 do if rm2[s2[i]] then table.remove(s2,i) end end
	-- for _,v in ipairs(s1) do io.write(v, " ") end io.write("\n")
	-- for _,v in ipairs(s2) do io.write(v, " ") end io.write("\n")
	local r = backtrack(map, s1, s2, function(map) return check_des(map, des) end, nil)
	if r then
		pre_tree.node.name,pre_tree.node.children = r.name,r.children
		return pre_tree.root
	end
	return nil
end

function _M.to_dot(par, node, fn)
	assert(not fn:match("[/]"), "fn should contain no '/'")
	local file = io.open(fn, "w")
	local f, f_inc = 0, 1
	local function foo(par, node)
		if node == nil then return end
		if node == true then file:write(par, " -> ", "fin",tostring(f), "\n") f=f+f_inc return end
		for k,v in pairs(node.children) do
			local self = node.name..k..tostring(f)
			f = f+f_inc
			file:write(par, " -> ", self, "\n")
			foo(self, v)
		end
	end
	file:write("digraph D {")
	foo(par, node)
	file:write("}\n")
	file:close()
end
function _M.to_list(node, state)
	local r = {}
	if node == nil then return {} end
	if node == true then
		return {table_copy(state)}
	end
	state = state or {}
	for k,v in pairs(node.children) do
		state[node.name] = k
		local r_ = _M.to_list(v, state)
		if #r_ >= 1 then
			for _,v in ipairs(r_) do
				table.insert(r, v)
			end
		end
	end
	state[node.name] = nil
	return r
end

return _M
