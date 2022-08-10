--[[
	Defines the process for diffing a virtual DOM and the real DOM to compute a
	patch that can be later applied.
]]

local Packages = script.Parent.Parent.Parent.Packages
local Log = require(Packages.Log)

local invariant = require(script.Parent.Parent.invariant)
local getProperty = require(script.Parent.getProperty)
local Error = require(script.Parent.Error)
local decodeValue = require(script.Parent.decodeValue)

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

	-- Add a virtual instance and all of its descendants to the patch, marked as
	-- being added.
	local function markIdAdded(id)
		local virtualInstance = virtualInstances[id]
		patch.added[id] = virtualInstance

		for _, childId in ipairs(virtualInstance.Children) do
			markIdAdded(childId)
		end
	end

	-- Internal recursive kernel for diffing an instance with the given ID.
	local function diffInternal(id)
		local virtualInstance = virtualInstances[id]
		local instance = instanceMap.fromIds[id]

		if virtualInstance == nil then
			invariant("Cannot diff an instance not present in virtualInstances\nID: {}", id)
		end

		if instance == nil then
			invariant("Cannot diff an instance not present in InstanceMap\nID: {}", id)
		end

		local changedClassName = nil
		if virtualInstance.ClassName ~= instance.ClassName then
			changedClassName = virtualInstance.ClassName
		end

		local changedName = nil
		if virtualInstance.Name ~= instance.Name then
			changedName = virtualInstance.Name
		end

		local changedProperties = {}
		for propertyName, virtualValue in pairs(virtualInstance.Properties) do
			local ok, existingValueOrErr = getProperty(instance, propertyName)

			if ok then
				local existingValue = existingValueOrErr
				local ok, decodedValue = decodeValue(virtualValue, instanceMap)

				if ok then
					if existingValue ~= decodedValue then
						changedProperties[propertyName] = virtualValue
					end
				else
					local propertyType = next(virtualValue)
					Log.warn(
						"Failed to decode property {}.{}. Encoded property was: {:#?}",
						virtualInstance.ClassName,
						propertyName,
						virtualValue
					)
				end
			else
				local err = existingValueOrErr

				if err.kind == Error.UnknownProperty then
					Log.trace("Skipping unknown property {}.{}", err.details.className, err.details.propertyName)
				elseif err.kind == Error.UnreadableProperty then
					Log.trace("Skipping unreadable property {}.{}", err.details.className, err.details.propertyName)
				else
					return false, err
				end
			end
		end

		if changedName ~= nil or changedClassName ~= nil or not isEmpty(changedProperties) then
			table.insert(patch.updated, {
				id = id,
				changedName = changedName,
				changedClassName = changedClassName,
				changedProperties = changedProperties,
				changedMetadata = nil,
			})
		end

		-- Traverse the list of children in the DOM. Any instance that has no
		-- corresponding virtual instance should be removed. Any instance that
		-- does have a corresponding virtual instance is recursively diffed.
		for _, childInstance in ipairs(instance:GetChildren()) do
			local childId = instanceMap.fromInstances[childInstance]

			if childId == nil then
				-- pcall to avoid security permission errors
				local success, skip = pcall(function()
					-- We don't remove instances that aren't going to be saved anyway,
					-- such as the Rojo session lock value.
					return childInstance.Archivable == false
				end)
				if success and skip then
					continue
				end

				-- This is an existing instance not present in the virtual DOM.
				-- We can mark it for deletion unless the user has asked us not
				-- to delete unknown stuff.
				if shouldDeleteUnknownInstances(virtualInstance) then
					table.insert(patch.removed, childInstance)
				end
			else
				local ok, err = diffInternal(childId)

				if not ok then
					return false, err
				end
			end
		end

		-- Traverse the list of children in the virtual DOM. Any virtual
		-- instance that has no corresponding real instance should be created.
		for _, childId in ipairs(virtualInstance.Children) do
			local childInstance = instanceMap.fromIds[childId]

			if childInstance == nil then
				-- This instance is present in the virtual DOM, but doesn't
				-- exist in the real DOM.
				markIdAdded(childId)
			end
		end

		return true
	end

	local ok, err = diffInternal(rootId)

	if not ok then
		return false, err
	end

	return true, patch
end

return diff
