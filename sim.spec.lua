local lester = require 'lester'
local describe, it, expect = lester.describe, lester.it, lester.expect
local testee = require 'sim'

local function mock_file(l)
	local mock = {}
	local function lines(l)
		for _,v in ipairs(l) do
			coroutine.yield(v)
		end
	end
	function mock:lines()
		return coroutine.wrap(function() lines(l) end)
	end
	return mock
end

local function map_eq(s1, s2)
	for k,v in pairs(s1) do
		if s2[k] ~= v then
			return false
		end
	end
	for k,v in pairs(s2) do
		if s1[k] ~= v then
			return false
		end
	end
	return true
end
local function res_eq(r1, r2)
	for _,a in ipairs(r1) do
		local found = false
		for k,v in pairs(r2) do
			if map_eq(a,v) then
				r2[k] = nil
				found = true
				break
			end
		end
		if not found then return false end
	end
	return true
end

local tests = {
	{
		file={"A", "B", "C", "", "1", "2", "3", "", "A->2", "", "0 A->1,B->2,C->3"},
		real={
			{["A"] = "2", ["B"] = "3", ["C"] = "1",},
		}
	},
	{
		file={"A", "B", "C", "", "1", "2", "3", "", "A-/>2", "", "0 A->1,B->2,C->3"},
		real={
			{["A"] = "3", ["B"] = "1", ["C"] = "2",},
		}
	},
	{
		file={"A", "B", "C", "", "1", "2", "3", "", "", "3 A->1,B->2,C->3"},
		real={
			{["A"] = "1", ["B"] = "2", ["C"] = "3",},
		}
	},
	{
		file={"A", "B", "C", "", "1", "2", "3", "", "", "0 A->1"},
		real={
			{["A"] = "2", ["B"] = "1", ["C"] = "3",},
			{["A"] = "2", ["B"] = "3", ["C"] = "1",},
			{["A"] = "3", ["B"] = "2", ["C"] = "1",},
			{["A"] = "3", ["B"] = "1", ["C"] = "2",},
		}
	},
}

local rs
for _,t in ipairs(tests) do
	rs = testee.to_list(testee.run(mock_file(t.file)))
	for _,r in pairs(rs) do
		testee.print_map(r)
		print()
	end
	print(res_eq(rs, t.real))
	print("===============================")
end
