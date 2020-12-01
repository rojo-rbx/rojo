local function merge(...)
	local newTable = {}

	for i = 1, select("#", ...) do
		local mergeTable = select(i, ...)
		for key, value in pairs(mergeTable) do
			newTable[key] = value
		end
	end

	return newTable
end

return merge