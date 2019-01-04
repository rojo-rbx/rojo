--[[
	This is a placeholder module waiting for Cryo to become available.
]]

local None = newproxy(true)
getmetatable(None).__tostring = function()
	return "None"
end

local function merge(...)
	local output = {}

	for i = 1, select("#", ...) do
		local source = select(i, ...)

		if source ~= nil then
			for key, value in pairs(source) do
				if value == None then
					output[key] = nil
				else
					output[key] = value
				end
			end
		end
	end

	return output
end

return {
	None = None,
	merge = merge,
}