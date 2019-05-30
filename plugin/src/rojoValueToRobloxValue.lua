local RbxDom = require(script:FindFirstAncestor("Rojo").RbxDom)

local function rojoValueToRobloxValue(value)
	-- TODO: Manually decode this value by looking up its GUID The Rojo server
	-- doesn't give us valid ref values yet, so this isn't important yet.
	if value.Type == "Ref" then
		return nil
	end

	local success, decodedValue = RbxDom.EncodedValue.decode(value)

	if not success then
		error(decodedValue, 2)
	end

	return decodedValue
end

return rojoValueToRobloxValue