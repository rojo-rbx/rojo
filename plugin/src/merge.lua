local function merge(...)
	local newTable = {}

	for _, mergeTable in ipairs({ ... }) do
		for key, value in pairs(mergeTable) do
			newTable[key] = value
		end
	end

	return newTable
end

return merge