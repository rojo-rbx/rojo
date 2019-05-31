local RbxDom = require(script:FindFirstAncestor("Rojo").RbxDom)

--[[
	Attempts to set a property on the given instance.
]]
local function setCanonicalProperty(instance, propertyName, value)
	local descriptor = RbxDom.findCanonicalPropertyDescriptor(instance.ClassName, propertyName)

	-- We can skip unknown properties; they're not likely reflected to Lua.
	--
	-- A good example of a property like this is `Model.ModelInPrimary`, which
	-- is serialized but not reflected to Lua.
	if descriptor == nil then
		return false, "unknown property"
	end

	if descriptor.scriptability == "None" or descriptor.scriptability == "Read" then
		return false, "unwritable property"
	end

	local success, err = descriptor:write(instance, value)

	if not success then
		-- If we don't have permission to write a property, we just silently
		-- ignore it.
		if err.kind == RbxDom.Error.Kind.Roblox and err.extra:find("lacking permission") then
			return false, "permission error"
		end

		local message = ("Invalid property %s.%s: %s"):format(descriptor.className, descriptor.name, tostring(err))
		error(message, 2)
	end

	return true
end

return setCanonicalProperty