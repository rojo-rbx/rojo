local RbxDom = require(script.Parent.Parent.RbxDom)

--[[
	Attempts to set a property on the given instance.
]]
local function getCanonincalProperty(instance, propertyName)
	local descriptor = RbxDom.findCanonicalPropertyDescriptor(instance.ClassName, propertyName)

	-- We can skip unknown properties; they're not likely reflected to Lua.
	--
	-- A good example of a property like this is `Model.ModelInPrimary`, which
	-- is serialized but not reflected to Lua.
	if descriptor == nil then
		return false, "unknown property"
	end

	if descriptor.scriptability == "None" or descriptor.scriptability == "Write" then
		return false, "unreadable property"
	end

	local success, valueOrErr = descriptor:read(instance)

	if not success then
		local err = valueOrErr

		-- If we don't have permission to read a property, we can chalk that up
		-- to our database being out of date and the engine being right.
		if err.kind == RbxDom.Error.Kind.Roblox and err.extra:find("lacking permission") then
			return false, "permission error"
		end

		local message = ("Invalid property %s.%s: %s"):format(descriptor.className, descriptor.name, tostring(err))
		error(message, 2)
	end

	return true, valueOrErr
end

return getCanonincalProperty