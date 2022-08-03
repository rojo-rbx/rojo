--[[
	Take an InstanceMap and a dictionary mapping instances to sets of property
	names. Populate a patch with the encoded values of all the given properties
	on all the given instances (or, if any changes set Parent to nil, removals
	of instances) and return the patch.
]]

local Packages = script.Parent.Parent.Parent.Packages
local Log = require(Packages.Log)

local PatchSet = require(script.Parent.Parent.PatchSet)

local encodePatchUpdate = require(script.Parent.encodePatchUpdate)

return function(instanceMap, propertyChanges)
	local patch = PatchSet.newEmpty()

	for instance, properties in pairs(propertyChanges) do
		local instanceId = instanceMap.fromInstances[instance]

		if instanceId == nil then
			Log.warn("Ignoring change for instance {:?} as it is unknown to Rojo", instance)
			continue
		end

		if properties.Parent then
			if instance.Parent == nil then
				table.insert(patch.removed, instanceId)
			else
				Log.warn("Cannot sync non-nil Parent property changes yet")
			end
		else
			local update = encodePatchUpdate(instance, instanceId, properties)
			table.insert(patch.updated, update)
		end

		propertyChanges[instance] = nil
	end

	return patch
end
