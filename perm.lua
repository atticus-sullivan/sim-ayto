-- sim_ayto
-- Copyright (C) 2024  Lukas
-- 
-- This program is free software: you can redistribute it and/or modify
-- it under the terms of the GNU General Public License as published by
-- the Free Software Foundation, either version 3 of the License, or
-- (at your option) any later version.
-- 
-- This program is distributed in the hope that it will be useful,
-- but WITHOUT ANY WARRANTY; without even the implied warranty of
-- MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
-- GNU General Public License for more details.
-- 
-- You should have received a copy of the GNU General Public License
-- along with this program.  If not, see <http://www.gnu.org/licenses/>.

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
