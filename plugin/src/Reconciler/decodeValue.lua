--[[
	Transforms a value encoded by rbx_dom_weak on the server side into a value
	usable by Rojo's reconciler, potentially using RbxDom.
]]

local Packages = script.Parent.Parent.Parent.Packages
local RbxDom = require(Packages.RbxDom)
local Error = require(script.Parent.Error)

local function decodeValue(encodedValue, instanceMap)
	local ty, value = next(encodedValue)

	-- Refs are represented as IDs in the same space that Rojo's protocol uses.
	if ty == "Ref" then
		if value == "00000000000000000000000000000000" then
			return true, nil
		end

		local instance = instanceMap.fromIds[value]

		if instance ~= nil then
			return true, instance
		else
			return false, Error.new(Error.RefDidNotExist, {
				encodedValue = encodedValue,
			})
		end
	end

	local ok, decodedValue = RbxDom.EncodedValue.decode(encodedValue)

	if not ok then
		return false, Error.new(Error.CannotDecodeValue, {
			encodedValue = encodedValue,
			innerError = decodedValue,
		})
	end

	return true, decodedValue
end

return decodeValue
