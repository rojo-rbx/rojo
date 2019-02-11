local Logging = require(script.Parent.Logging)

--[[
	Attempts to set a property on the given instance, correctly handling
	'virtual properties', which aren't reflected directly to Lua.
]]
local function setProperty(instance, key, value)
	-- The 'Contents' property of LocalizationTable isn't directly exposed, but
	-- has corresponding (deprecated) getters and setters.
	if instance.ClassName == "LocalizationTable" and key == "Contents" then
		instance:SetContents(value)
		return
	end

	-- If we don't have permissions to access this value at all, we can skip it.
	local readSuccess, existingValue = pcall(function()
		return instance[key]
	end)

	if not readSuccess then
		-- An error will be thrown if there was a permission issue or if the
		-- property doesn't exist. In the latter case, we should tell the user
		-- because it's probably their fault.
		if existingValue:find("lacking permission") then
			Logging.trace("Permission error reading property %s on class %s", tostring(key), instance.ClassName)
			return
		else
			error(("Invalid property %s on class %s: %s"):format(tostring(key), instance.ClassName, existingValue), 2)
		end
	end

	local writeSuccess, err = pcall(function()
		if existingValue ~= value then
			instance[key] = value
		end
	end)

	if not writeSuccess then
		error(("Cannot set property %s on class %s: %s"):format(tostring(key), instance.ClassName, err), 2)
	end

	return true
end

return setProperty