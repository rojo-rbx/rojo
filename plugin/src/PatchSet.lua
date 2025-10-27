--[[
	Methods to operate on either a patch created by the hydrate method, or a
	patch returned from the API.
]]

local Packages = script.Parent.Parent.Packages
local t = require(Packages.t)

local Types = require(script.Parent.Types)

local function deepEqual(a: any, b: any): boolean
	local typeA = typeof(a)
	if typeA ~= typeof(b) then
		return false
	end

	if typeof(a) == "table" then
		local checkedKeys = {}

		for key, value in a do
			checkedKeys[key] = true

			if deepEqual(value, b[key]) == false then
				return false
			end
		end

		for key, value in b do
			if checkedKeys[key] then
				continue
			end
			if deepEqual(value, a[key]) == false then
				return false
			end
		end

		return true
	end

	if a == b then
		return true
	end

	return false
end

local PatchSet = {}

PatchSet.validate = t.interface({
	removed = t.array(t.union(Types.RbxId, t.Instance)),
	added = t.map(Types.RbxId, Types.ApiInstance),
	updated = t.array(Types.ApiInstanceUpdate),
})

--[[
	Create a new, empty PatchSet.
]]
function PatchSet.newEmpty()
	return {
		removed = {},
		added = {},
		updated = {},
	}
end

--[[
	Tells whether the given PatchSet is empty.
]]
function PatchSet.isEmpty(patchSet)
	return next(patchSet.removed) == nil and next(patchSet.added) == nil and next(patchSet.updated) == nil
end

--[[
	Tells whether the given PatchSet has any remove operations.
]]
function PatchSet.hasRemoves(patchSet)
	return next(patchSet.removed) ~= nil
end

--[[
	Tells whether the given PatchSet has any add operations.
]]
function PatchSet.hasAdditions(patchSet)
	return next(patchSet.added) ~= nil
end

--[[
	Tells whether the given PatchSet has any update operations.
]]
function PatchSet.hasUpdates(patchSet)
	return next(patchSet.updated) ~= nil
end

--[[
	Tells whether the given PatchSet contains changes to the given instance id
]]
function PatchSet.containsId(patchSet, instanceMap, id)
	if patchSet.added[id] ~= nil then
		return true
	end

	for _, idOrInstance in patchSet.removed do
		local removedId = if Types.RbxId(idOrInstance) then idOrInstance else instanceMap.fromInstances[idOrInstance]
		if removedId == id then
			return true
		end
	end

	for _, update in patchSet.updated do
		if update.id == id then
			return true
		end
	end

	return false
end

--[[
	Tells whether the given PatchSet contains changes to the given instance.
	If the given InstanceMap does not contain the instance, this function always returns false.
]]
function PatchSet.containsInstance(patchSet, instanceMap, instance)
	local id = instanceMap.fromInstances[instance]
	if id == nil then
		return false
	end

	return PatchSet.containsId(patchSet, instanceMap, id)
end

--[[
	Tells whether the given PatchSet contains changes to nothing but the given instance id
]]
function PatchSet.containsOnlyId(patchSet, instanceMap, id)
	if not PatchSet.containsId(patchSet, instanceMap, id) then
		-- Patch doesn't contain the id at all
		return false
	end

	for addedId in patchSet.added do
		if addedId ~= id then
			return false
		end
	end

	for _, idOrInstance in patchSet.removed do
		local removedId = if Types.RbxId(idOrInstance) then idOrInstance else instanceMap.fromInstances[idOrInstance]
		if removedId ~= id then
			return false
		end
	end

	for _, update in patchSet.updated do
		if update.id ~= id then
			return false
		end
	end

	return true
end

--[[
	Tells whether the given PatchSet contains changes to nothing but the given instance.
	If the given InstanceMap does not contain the instance, this function always returns false.
]]
function PatchSet.containsOnlyInstance(patchSet, instanceMap, instance)
	local id = instanceMap.fromInstances[instance]
	if id == nil then
		return false
	end

	return PatchSet.containsOnlyId(patchSet, instanceMap, id)
end

--[[
	Returns the update to the given instance id, or nil if there aren't any
]]
function PatchSet.getUpdateForId(patchSet, id)
	for _, update in patchSet.updated do
		if update.id == id then
			return update
		end
	end

	return nil
end

--[[
	Returns the update to the given instance, or nil if there aren't any.
	If the given InstanceMap does not contain the instance, this function always returns nil.
]]
function PatchSet.getUpdateForInstance(patchSet, instanceMap, instance)
	local id = instanceMap.fromInstances[instance]
	if id == nil then
		return nil
	end

	return PatchSet.getUpdateForId(patchSet, id)
end

--[[
	Tells whether the given PatchSets are equal.
]]
function PatchSet.isEqual(patchA, patchB)
	return deepEqual(patchA, patchB)
end

--[[
	Count the number of changes in the given PatchSet.
]]
function PatchSet.countChanges(patch)
	local count = 0

	for _, add in patch.added do
		-- Adding an instance is 1 change per property
		for _ in add.Properties do
			count += 1
		end
	end
	for _ in patch.removed do
		-- Removing an instance is 1 change
		count += 1
	end
	for _, update in patch.updated do
		-- Updating an instance is 1 change per property updated
		for _ in update.changedProperties do
			count += 1
		end
		if update.changedName ~= nil then
			count += 1
		end
		if update.changedClassName ~= nil then
			count += 1
		end
	end

	return count
end

--[[
	Count the number of instances affected by the given PatchSet.
]]
function PatchSet.countInstances(patch)
	local count = 0

	-- Added instances
	for _ in patch.added do
		count += 1
	end
	-- Removed instances
	for _ in patch.removed do
		count += 1
	end
	-- Updated instances
	for _ in patch.updated do
		count += 1
	end

	return count
end

--[[
	Merge multiple PatchSet objects into the given PatchSet.
]]
function PatchSet.assign(target, ...)
	for i = 1, select("#", ...) do
		local sourcePatch = select(i, ...)

		for _, removed in ipairs(sourcePatch.removed) do
			table.insert(target.removed, removed)
		end

		for id, added in pairs(sourcePatch.added) do
			target.added[id] = added
		end

		for _, update in ipairs(sourcePatch.updated) do
			table.insert(target.updated, update)
		end
	end

	return target
end

function PatchSet.addedIdList(patchSet): { string }
	local idList = table.create(#patchSet.added)
	for id in patchSet.added do
		table.insert(idList, id)
	end
	return table.freeze(idList)
end

function PatchSet.updatedIdList(patchSet): { string }
	local idList = table.create(#patchSet.updated)
	for _, item in patchSet.updated do
		table.insert(idList, item.id)
	end
	return table.freeze(idList)
end

--[[
	Create a list of human-readable statements summarizing the contents of this
	patch, intended to be displayed to users.
]]
function PatchSet.humanSummary(instanceMap, patchSet)
	local statements = {}

	for _, idOrInstance in ipairs(patchSet.removed) do
		local instance, id

		if Types.RbxId(idOrInstance) then
			id = idOrInstance
			instance = instanceMap.fromIds[id]
		else
			instance = idOrInstance
			id = instanceMap.fromInstances[instance]
		end

		if instance ~= nil then
			table.insert(statements, string.format("- Delete instance %s", instance:GetFullName()))
		else
			table.insert(statements, string.format("- Delete instance with ID %s", id))
		end
	end

	local additionsMentioned = {}

	local function addAllDescendents(virtualInstance)
		additionsMentioned[virtualInstance.Id] = true

		for _, childId in ipairs(virtualInstance.Children) do
			addAllDescendents(patchSet.added[childId])
		end
	end

	for id, addition in pairs(patchSet.added) do
		if additionsMentioned[id] then
			continue
		end

		local virtualInstance = addition
		while true do
			if virtualInstance.Parent == nil then
				break
			end

			local virtualParent = patchSet.added[virtualInstance.Parent]
			if virtualParent == nil then
				break
			end

			virtualInstance = virtualParent
		end

		local parentDisplayName = "nil (how strange!)"
		if virtualInstance.Parent ~= nil then
			local parent = instanceMap.fromIds[virtualInstance.Parent]
			if parent ~= nil then
				parentDisplayName = parent:GetFullName()
			end
		end

		table.insert(
			statements,
			string.format(
				"- Add instance %q (ClassName %q) to %s",
				virtualInstance.Name,
				virtualInstance.ClassName,
				parentDisplayName
			)
		)
	end

	for _, update in ipairs(patchSet.updated) do
		local updatedProperties = {}

		if update.changedMetadata ~= nil then
			table.insert(updatedProperties, "Rojo's Metadata")
		end

		if update.changedName ~= nil then
			table.insert(updatedProperties, "Name")
		end

		if update.changedClassName ~= nil then
			table.insert(updatedProperties, "ClassName")
		end

		for name in pairs(update.changedProperties) do
			table.insert(updatedProperties, name)
		end

		local instance = instanceMap.fromIds[update.id]
		local displayName
		if instance ~= nil then
			displayName = instance:GetFullName()
		else
			displayName = "[unknown instance]"
		end

		table.insert(
			statements,
			string.format("- Update properties on %s: %s", displayName, table.concat(updatedProperties, ","))
		)
	end

	return table.concat(statements, "\n")
end

return PatchSet
