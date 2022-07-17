CNT = 0
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

local function parse_file(file, rev)
	local s1,s2,des,mb,state = {},{},{},{},0

	local function next_line(line)
		line = file:read("l")
		while line and line:match("^#") do line = file:read("l") end
		return line
	end

	local function parse_set(line, s)
		table.insert(s, line:match("%s*([^%->, ]+)%s*"))
	end

	local function parse_mb(line)
		local e1,e2
		if rev then
			e2,e1 = line:match("^%s*([^%->/ ]+)%s*%->%s*([^%->/ ]+)%s*$")
		else
			e1,e2 = line:match("^%s*([^%->/ ]+)%s*%->%s*([^%->/ ]+)%s*$")
		end
		if not e1 or not e2 then
			if rev then
				e2,e1 = line:match("^%s*([^%->/ ]+)%s*%-/>%s*([^%->/ ]+)%s*$")
			else
				e1,e2 = line:match("^%s*([^%->/ ]+)%s*%-/>%s*([^%->/ ]+)%s*$")
			end
			assert(table_contains(s1, e1), "invalid mb mapping "..line)
			assert(table_contains(s2, e2), "invalid mb mapping "..line)
			assert(e1 and e2, "error")
			table.insert(mb, {e1, e2, false})
		else
			assert(table_contains(s1, e1), "invalid mb mapping "..line)
			assert(table_contains(s2, e2), "invalid mb mapping "..line)
			assert(e1 and e2, "error")
			table.insert(mb, {e1, e2, true})
		end
	end

	local function parse_mn(line)
		local d = {num=tonumber(line:match("^%s*(%d+)%s*")), matches={}}
		line = next_line(line)
		while line and line ~= "" do
			local e1,e2
			if rev then
				e2,e1 = line:match("^%s*([^%->, ]+)%s*%->%s*([^%->, ]+)%s*$")
			else
				e1,e2 = line:match("^%s*([^%->, ]+)%s*%->%s*([^%->, ]+)%s*$")
			end
			assert(table_contains(s1, e1), "invalid mn mapping "..line)
			assert(table_contains(s2, e2), "invalid mn mapping "..line)
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
				if rev then
					parse_set(line, s2)
				else
					parse_set(line, s1)
				end
			elseif state == 1 then
				if rev then
					parse_set(line, s1)
				else
					parse_set(line, s2)
				end
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

local function check_des(map, des)
	for _,d in ipairs(des) do
		local cnt,free = 0,0
		-- go through information provided by d and count how may mappings are
		-- not set aka free and how many mappings are the same in d and the map
		for _,e in ipairs(d.matches) do
			-- print(e[1], e[2])
			if not map[e[1]] then
				free = free+1
			elseif map[e[1]] == e[2] then
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
		if cnt < d.num-free or cnt > d.num then
			return false
		end
	end
	-- only if all constraints are met, the mapping can be valid
	return true
end

local function backtrack(map, s1, s2, check, lvl, cfg,par)
	if not check(map) then
		return nil
	end
	if #s1 <= 0 then
		if cfg.cnt then
			CNT = CNT+1
		end
		if cfg.dot then
			cfg.of:write(par, " -> ", "fin_",tostring(ID), "\n")
			ID = ID+cfg.id_inc
		end
		return true
	end
	local e1 = table.remove(s1)
	local node
	for i2 = 1,#s2 do
		local e2 = table.remove(s2, i2)
		map[e1] = e2
		if lvl then
			io.write(string.rep(" ", lvl*4), "trying: ", e1, " -> ", e2, "\n")
		end
		local self
		if cfg.dot then
			self = e1..e2.."_"..tostring(ID)
			ID = ID+cfg.id_inc
		end
		local r = backtrack(map, s1, s2, check, lvl and lvl+1 or nil, cfg,self)
		if r then
			node = node or {name=e1, children={}}
			if cfg.tree then
				node.children[e2] = r
			end
			if cfg.dot then
				cfg.of:write(par, " -> ", self, "\n")
			end
		end
		table.insert(s2, i2, e2)
		map[e1] = nil
	end
	table.insert(s1, e1)
	return node
end

local function handle_fixed(s1,s2,map,tree,des, mb, par,cfg)
	local rm1,rm2 = {},{} -- colect which to remove
	local self = par
	-- node is the current leaf, root is the root of the tree
	tree.root = tree.node
	for _,v in ipairs(mb) do
		if v[3] then
			-- definite match found -> remove both elements and insert into the
			-- tree and map
			-- this is a performance enhancement (one could just add this as additional constraint as well)
			map[v[1]] = v[2]
			if cfg.tree then
				-- child of the current match
				local node = {name=nil, children={}}
				-- fill the current node
				tree.node.name = v[1]
				tree.node.children[v[2]] = node
				-- set the next node to work on
				tree.node = node
			end
			if cfg.dot then
				self = v[1]..v[2].."_"..tostring(ID)
				cfg.of:write(par, " -> ", self, "\n")
				ID,par = ID+cfg.id_inc,self
			end
			-- add elements to the remove set
			rm1[v[1]],rm2[v[2]] = true,true
		else
			-- definitely no match -> insert this as an additional constraint
			table.insert(des, {num=0, matches={{v[1],v[2]}}})
		end
	end
	-- do the actual removing
	-- go from top to low to preserve to be removed indices
	for i=#s1,1,-1 do if rm1[s1[i]] then table.remove(s1,i) end end
	for i=#s2,1,-1 do if rm2[s2[i]] then table.remove(s2,i) end end
	return self
end


local _M = {}

function _M.print_map(m)
	for k,v in pairs(m) do
		io.write(k, " -> ", v, "\n")
	end
end

-- cfg.id_inc
-- cfg.fn
-- cfg.rev
-- cfg.cnt
-- cfg.dot
-- cfg.tree
-- cfg.of
function _M.run(file, cfg)
	if cfg.cnt then
		CNT = 0
	end
	if cfg.dot then
		assert(cfg.ofn)
		ID = 0
		cfg.of = io.open(cfg.ofn, "w")
		cfg.of:write("digraph D {")
	end
	local s1,s2,des,mb = parse_file(file, cfg.rev)
	local map = {}
	local tree = {node={name=nil, children={}}}
	local par = handle_fixed(s1, s2, map, tree, des, mb, "root", cfg)
	-- for _,v in ipairs(s1) do io.write(v, " ") end io.write("\n")
	-- for _,v in ipairs(s2) do io.write(v, " ") end io.write("\n")
	local r = backtrack(map, s1, s2, function(map) return check_des(map, des) end, nil, cfg, par)
	if cfg.dot then
		cfg.of:write("}\n")
		cfg.of:close()
	end
	if r then
		-- add to tree as the current node
		tree.node.name,tree.node.children = r.name,r.children
		return cfg.tree and tree.root or nil, cfg.cnt and CNT or nil
	end
	return nil, cfg.cnt and CNT or nil
end

-- TODO is being resolved by creating dot in the fly
function _M.to_dot(par, node, fn, id_inc)
	assert(not fn:match("[/]"), "fn should contain no '/'")
	local file = io.open(fn, "w")
	local id = 0
	local function foo(par, node)
		if node == nil then return end
		if node == true then file:write(par, " -> ", "fin_",tostring(id), "\n") id=id+id_inc return end
		for k,v in pairs(node.children) do
			local self = node.name..k.."_"..tostring(id)
			id = id+id_inc
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
