local RbxDom = require(script:FindFirstAncestor("Rojo").RbxDom)

local Logging = require(script.Parent.Logging)

--[[
	Attempts to set a property on the given instance.
]]
local function setCanonicalProperty(instance, key, value)
	if not RbxDom.CanonicalProperty.isScriptable(instance.ClassName, key) then
		return false
	end

	-- If we don't have permissions to access this value at all, we can skip it.
	local readSuccess, existingValue = RbxDom.CanonicalProperty.read(instance, key)

	if not readSuccess then
		-- An error will be thrown if there was a permission issue or if the
		-- property doesn't exist. In the latter case, we should tell the user
		-- because it's probably their fault.
		if existingValue:find("lacking permission") then
			Logging.trace("Permission error reading property %s on class %s", tostring(key), instance.ClassName)
			return false
		else
			error(("Invalid property %s on class %s: %s"):format(tostring(key), instance.ClassName, existingValue), 2)
		end
	end

	local writeSuccess, err = RbxDom.CanonicalProperty.write(instance, key, value)

	if not writeSuccess then
		error(("Cannot set property %s on class %s: %s"):format(tostring(key), instance.ClassName, err), 2)
	end

	return true
end

return setCanonicalProperty