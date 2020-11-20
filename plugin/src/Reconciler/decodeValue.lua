--[[
	Transforms a value encoded by rbx_dom_weak on the server side into a value
	usable by Rojo's reconciler, potentially using RbxDom.
]]

local RbxDom = require(script.Parent.Parent.Parent.RbxDom)
local Error = require(script.Parent.Error)

local function decodeValue(virtualValue, instanceMap)
	-- Refs are represented as IDs in the same space that Rojo's protocol uses.
	if virtualValue.Type == "Ref" then
		if virtualValue.Value == nil then
			return true, nil
		end

		local instance = instanceMap.fromIds[virtualValue.Value]

		if instance ~= nil then
			return true, instance
		else
			return false, Error.new(Error.RefDidNotExist, {
				virtualValue = virtualValue,
			})
		end
	end

	local ok, decodedValue = RbxDom.EncodedValue.decode(virtualValue)

	if not ok then
		return false, Error.new(Error.CannotDecodeValue, {
			virtualValue = virtualValue,
			innerError = decodedValue,
		})
	end

	return true, decodedValue
end

return decodeValue