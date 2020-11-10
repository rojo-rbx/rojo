--[[
	Methods to operate on either a patch created by the hydrate method, or a
	patch returned from the API.
]]

local t = require(script.Parent.Parent.t)

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

return PatchSet