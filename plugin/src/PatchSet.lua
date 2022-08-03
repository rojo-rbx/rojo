--[[
	Methods to operate on either a patch created by the hydrate method, or a
	patch returned from the API.
]]

local Packages = script.Parent.Parent.Packages
local t = require(Packages.t)

local Types = require(script.Parent.Types)

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
	return next(patchSet.removed) == nil and
		next(patchSet.added) == nil and
		next(patchSet.updated) == nil
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

--[[
	Create a list of human-readable statements summarizing the contents of this
	patch, intended to be displayed to users.
]]
function PatchSet.humanSummary(instanceMap, patchSet)
	local statements = {}

	for _, idOrInstance in ipairs(patchSet.removed) do
		local instance, id

		if type(idOrInstance) == "string" then
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

		table.insert(statements, string.format(
			"- Add instance %q (ClassName %q) to %s",
			virtualInstance.Name, virtualInstance.ClassName, parentDisplayName))
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

		table.insert(statements, string.format(
			"- Update properties on %s: %s",
			displayName, table.concat(updatedProperties, ",")))
	end

	return table.concat(statements, "\n")
end

return PatchSet
