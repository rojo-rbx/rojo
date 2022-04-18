local Packages = script.Parent.Parent.Parent.Packages
local Log = require(Packages.Log)
local RbxDom = require(Packages.RbxDom)

local encodeProperty = require(script.Parent.encodeProperty)

return function(instance, instanceId, properties)
	local update = {
		id = instanceId,
		changedProperties = {},
		requiresRecreate = false
	}

	for propertyName in pairs(properties) do
		if propertyName == "Name" then
			update.changedName = instance.Name
		elseif propertyName == "MeshId" then
			update.requiresRecreate = true
		elseif propertyName == "ClassName" then
			update.requiresRecreate, update.changedClassName = true, instance.ClassName
		end
		
		local descriptor = RbxDom.findCanonicalPropertyDescriptor(instance.ClassName, propertyName)

		if not descriptor then
			Log.debug("Could not sync back property {:?}.{}", instance, propertyName)
			continue
		end

		local encodeSuccess, encodeResult = encodeProperty(instance, propertyName, descriptor)

		if not encodeSuccess then
			Log.debug("Could not sync back property {:?}.{}: {}", instance, propertyName, encodeResult)
			continue
		end

		update.changedProperties[propertyName] = encodeResult
	end

	if next(update.changedProperties) == nil and update.changedName == nil then
		return nil
	end

	return update
end
