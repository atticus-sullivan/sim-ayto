local _M = {}
local function table_copy(t)
	local r = {}
	for k,v in ipairs(t) do
		r[k] = v
	end
	return r
end

function _M.perm_copy(t)
	local r = {}
	for k1,v1 in ipairs(t) do
		r[k1] = {}
		for k2,v2 in ipairs(v1) do
			r[k1][k2] = v2
		end
	end
	return r
end

local function _permgen(a, n, l, pred, conv)
	if n == 0 then
		if pred(a) then
			table.insert(l, conv(a))
		-- else
		-- 	print("filtered")
		end
	else
		for i=1,n do

			-- put i-th element as the last one
			a[n], a[i] = a[i], a[n]

			-- generate all permutations of the other elements
			_permgen(a, n - 1, l, pred, conv)

			-- restore i-th element
			a[n], a[i] = a[i], a[n]

		end
	end
end
-- fixed values might be inserted at the end of a
function _M.permgen(a,n,l, pred, conv)
	conv = conv or function(a) return table_copy(a) end
	pred = pred or function(_) return true end
	l = l or {}
	_permgen(a,n,l, pred, conv)
	return l
end

function _M.count_pred(l, pred)
	local r = 0
	for _,v in ipairs(l) do
		if pred(v) then r = r+1 end
	end
	return r
end

-- ATTENTION: no deep copy is returned
function _M.filter_pred(l, pred)
	local r = {}
	for _,v in ipairs(l) do
		if pred(v) then
			table.insert(r, v)
		end
	end
	return r
end

return _M
