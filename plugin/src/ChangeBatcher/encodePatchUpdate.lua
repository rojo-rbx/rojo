local Log = require(script.Parent.Parent.Parent.Log)
local RbxDom = require(script.Parent.Parent.Parent.RbxDom)

local encodeProperty = require(script.Parent.encodeProperty)

return function(instance, instanceId, properties)
	local update = {
		id = instanceId,
		changedProperties = {},
	}

	for propertyName in pairs(properties) do
		if propertyName == "Name" then
			update.changedName = instance.Name
		else
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

			if encodeSuccess then
				update.changedProperties[propertyName] = encodeResult
			end
		end
	end

	if not next(update.changedProperties) and not update.changedName then
		return nil
	end

	return update
end
