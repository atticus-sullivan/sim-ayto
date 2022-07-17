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
		return {}
	end
	if #s1 <= 0 then
		return {table_copy(map.m)}
	end
	local r = {}
	local e1 = table.remove(s1)
	for i2 = 1,#s2 do
		local e2 = table.remove(s2, i2)
		map.m[e1] = e2
		map.cnt = map.cnt+1
		if lvl then
			io.write(string.rep(" ", lvl*4), "trying: ", e1, " -> ", e2, "\n")
		end
		table_append(r, backtrack(map, s1, s2, check, lvl and lvl+1 or nil))
		table.insert(s2, i2, e2)
		map.m[e1] = nil
		map.cnt = map.cnt-1
	end
	table.insert(s1, e1)
	return r
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

function _M.run(file)
	local s1,s2,des,mb,state = {},{},{},{},0
	for line in file:lines() do
		if not line:match("^#") then
			if line == "" then
				state = state + 1
			else
				if state == 0 then
					table.insert(s1, line)
				elseif state == 1 then
					table.insert(s2, line)

				elseif state == 2 then
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
				elseif state >= 3 then
					local d = {num=tonumber(line:match("^%d+")), matches={}}
					line = line:gsub("^%d+%s*", "")
					for e1,e2 in line:gmatch("([^%->,]+)%->([^%->,]+)") do
						assert(table_contains(s1, e1), "invalid mn mapping")
						assert(table_contains(s2, e2), "invalid mn mapping")
						table.insert(d.matches, {e1, e2})
					end
					table.insert(des, d)
				else
					error("invalid state")
				end
			end
		else
			-- print("skip")
		end
	end
	local map={cnt=0, m={}}
	-- handle mb
	local rm1 = {}
	local rm2 = {}
	for _,v in ipairs(mb) do
		if v[3] then
			map.m[v[1]] = v[2]
			rm1[v[1]],rm2[v[2]] = true,true
		else
			table.insert(des, {num=0, matches={{v[1],v[2]}}})
		end
	end
	for i=#s1,1,-1 do if rm1[s1[i]] then table.remove(s1,i) end end
	for i=#s2,1,-1 do if rm2[s2[i]] then table.remove(s2,i) end end
	-- for _,v in ipairs(s1) do io.write(v, " ") end io.write("\n")
	-- for _,v in ipairs(s2) do io.write(v, " ") end io.write("\n")
	return backtrack(map, s1, s2, function(map) return check_des(map, des) end, nil)
end

return _M
