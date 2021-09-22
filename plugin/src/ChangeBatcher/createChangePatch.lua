local Log = require(script.Parent.Parent.Parent.Log)
local PatchSet = require(script.Parent.Parent.PatchSet)
local RbxDom = require(script.Parent.Parent.Parent.RbxDom)

-- Take an InstanceMap and a dictionary mapping instances to sets of property
-- names. Populate the patch with updates to all the given properties (or, if
-- any of changes set Parent to nil, removals of the instances) and return the
-- patch.
return function(instanceMap, propertyChanges)
	local patch = PatchSet.newEmpty()

	for instance, properties in pairs(propertyChanges) do
		local instanceId = instanceMap.fromInstances[instance]

		if instanceId == nil then
			Log.warn("Ignoring change for instance {:?} as it is unknown to Rojo", instance)
			continue
		end

		local remove = nil
		local update = {
			id = instanceId,
			changedProperties = {},
		}

		for propertyName in pairs(properties) do
			if propertyName == "Name" then
				update.changedName = instance.Name
			elseif propertyName == "Parent" then
				if instance.Parent == nil then
					update = nil
					remove = instanceId
					break
				else
					Log.warn("Cannot sync non-nil Parent property changes yet")
					continue
				end
			else
				local descriptor = RbxDom.findCanonicalPropertyDescriptor(instance.ClassName, propertyName)

				if not descriptor then
					Log.debug("Could not sync back property {:?}.{}", instance, propertyName)
					continue
				end

				local readSuccess, readResult = descriptor:read(instance)

				if not readSuccess then
					Log.warn("Could not sync back property {:?}.{}: {}", instance, propertyName, readResult)
					continue
				end

				local dataType = descriptor.dataType
				local encodeSuccess, encodeResult = RbxDom.EncodedValue.encode(readResult, dataType)

				if not encodeSuccess then
					Log.warn("Could not sync back property {:?}.{}: {}", instance, propertyName, encodeResult)
					continue
				end

				update.changedProperties[propertyName] = encodeResult
			end
		end

		if update and next(update.changedProperties) then
			table.insert(patch.updated, update)
		end

		table.insert(patch.removed, remove)

		propertyChanges[instance] = nil
	end

	return patch
end
