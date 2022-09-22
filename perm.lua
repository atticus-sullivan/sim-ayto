local _M = {}
local function table_copy(t)
	if type(t) ~= "table" then return t end
	local r = {}
	for k,v in ipairs(t) do
		r[k] = table_copy(v)
	end
	return r
end

local function _permgen(a, n, conv)
	if n == 0 then
		conv(a)
	else
		for i=1,n do

			-- put i-th element as the last one
			a[n], a[i] = a[i], a[n]

			-- generate all permutations of the other elements
			_permgen(a, n - 1, conv)

			-- restore i-th element
			a[n], a[i] = a[i], a[n]

		end
	end
end
-- fixed values might be inserted at the end of a
function _M.permgen(a,n, conv)
	conv = conv or function(x) return coroutine.yield(x) end
	return coroutine.wrap(function() _permgen(a,n, conv) end)
end

return _M
