--[[
	Defines the process for diffing a virtual DOM and the real DOM to compute a
	patch that can be later applied.
]]

local invariant = require(script.Parent.Parent.invariant)

local function isEmpty(table)
	return next(table) == nil
end

local function shouldDeleteUnknownInstances(virtualInstance)
	if virtualInstance.Metadata ~= nil then
		return not virtualInstance.Metadata.ignoreUnknownInstances
	else
		return true
	end
end

local function diff(instanceMap, virtualInstances, rootId)
	local patch = {
		removed = {},
		added = {},
		updated = {},
	}

	local function diffInternal(id)
		local virtualInstance = virtualInstances[id]
		local instance = instanceMap.fromIds[id]

		if virtualInstance == nil then
			invariant("Cannot diff an instance not present in virtualInstances\nID: {}", id)
		end

		if instance == nil then
			invariant("Cannot diff an instance not present in InstanceMap\nID: {}", id)
		end

		if virtualInstance.ClassName ~= instance.ClassName then
			error("unimplemented: support changing ClassName")
		end

		local changedName = nil
		if virtualInstance.Name ~= instance.Name then
			changedName = virtualInstance.Name
		end

		local changedProperties = {}
		-- TODO: Enumerate properties and calculate changed

		if changedName ~= nil or not isEmpty(changedProperties) then
			table.insert(patch.updated, {
				id = id,
				changedName = changedName,
				changedClassName = nil,
				changedProperties = changedProperties,
				changedMetadata = nil,
			})
		end

		for _, childInstance in ipairs(instance:GetChildren()) do
			local childId = instanceMap.fromInstances[childInstance]

			if childId == nil then
				if shouldDeleteUnknownInstances(virtualInstance) then
					table.insert(patch.removed, childInstance)
				end
			else
				diffInternal(childId)
			end
		end
	end

	diffInternal(rootId)

	return patch
end

return diff