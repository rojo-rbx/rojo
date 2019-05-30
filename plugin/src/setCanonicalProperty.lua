local RbxDom = require(script:FindFirstAncestor("Rojo").RbxDom)

--[[
	Attempts to set a property on the given instance.
]]
local function setCanonicalProperty(instance, key, value)
	-- If we don't have permissions to access this value at all, we can skip it.
	local readSuccess, existingValue = RbxDom.readProperty(instance, key)

	if not readSuccess then
		if existingValue.kind == RbxDom.Error.Kind.UnknownProperty
			or existingValue.kind == RbxDom.Error.Kind.PropertyNotReadable then
			-- this is fine
			return false
		end

		-- If we don't have permission to write a property, we just silently
		-- ignore it.
		if existingValue.kind == RbxDom.Error.Kind.Roblox and existingValue.extra:find("lacking permission") then
			return false
		end

		error(("Invalid property %s on class %s: %s"):format(tostring(key), instance.ClassName, tostring(existingValue)), 2)
	end

	local writeSuccess, err = RbxDom.writeProperty(instance, key, value)

	if not writeSuccess then
		error(("Cannot set property %s on class %s: %s"):format(tostring(key), instance.ClassName, tostring(err)), 2)
	end

	return true
end

return setCanonicalProperty