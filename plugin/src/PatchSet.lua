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
	Invert the given PatchSet using the given instance map.
]]
function PatchSet.invert(patchSet, instanceMap)
	error("not yet implemented", 2)
end

return PatchSet