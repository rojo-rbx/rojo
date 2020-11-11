--[[
	"Reifies" a virtual DOM, constructing a real DOM with the same shape.
]]

local invariant = require(script.Parent.Parent.invariant)
local PatchSet = require(script.Parent.Parent.PatchSet)
local setProperty = require(script.Parent.setProperty)
local decodeValue = require(script.Parent.decodeValue)

local reifyInner, applyDeferredRefs

local function reify(instanceMap, virtualInstances, rootId, parentInstance)
	-- Create an empty patch that will be populated with any parts of this reify
	-- that could not happen, like instances that couldn't be created and
	-- properties that could not be assigned.
	local unappliedPatch = PatchSet.newEmpty()

	-- Contains a list of all of the ref properties that we'll need to assign
	-- after all instances are created. We apply refs in a second pass, after
	-- we create as many instances as we can, so that we ensure that referents
	-- can be mapped to instances correctly.
	local deferredRefs = {}

	reifyInner(instanceMap, virtualInstances, rootId, parentInstance, unappliedPatch, deferredRefs)
	applyDeferredRefs(instanceMap, deferredRefs)

	return unappliedPatch
end

function applyDeferredRefs(instanceMap, deferredRefs)
	-- FIXME: Implement this function.
end

--[[
	Add the given ID and all of its descendants in virtualInstances to the given
	PatchSet, marked for addition.
]]
local function addAllToPatch(patchSet, virtualInstances, id)
	local virtualInstance = virtualInstances[id]
	patchSet.added[id] = virtualInstance

	for _, childId in ipairs(virtualInstance.Children) do
		addAllToPatch(patchSet, virtualInstances, childId)
	end
end

--[[
	Inner function that defines the core routine.
]]
function reifyInner(instanceMap, virtualInstances, id, parentInstance, unappliedPatch, deferredRefs)
	local virtualInstance = virtualInstances[id]

	if virtualInstance == nil then
		invariant("Cannot reify an instance not present in virtualInstances\nID: {}", id)
	end

	-- Instance.new can fail if we're passing in something that can't be
	-- created, like a service, something enabled with a feature flag, or
	-- something that requires higher security than we have.
	local ok, instance = pcall(Instance.new, virtualInstance.ClassName)

	if not ok then
		addAllToPatch(unappliedPatch, virtualInstances, id)
		return
	end

	-- TODO: Can this fail? Previous versions of Rojo guarded against this, but
	-- the reason why was uncertain.
	instance.Name = virtualInstance.Name

	-- Track all of the properties that we've failed to assign to this instance.
	local unappliedProperties = {}

	for propertyName, virtualValue in pairs(virtualInstance.Properties) do
		-- FIXME: Check for Ref properties and add them to deferredRefs instead
		-- of trying to resolve them here.

		local ok, value = decodeValue(virtualValue, instanceMap)
		if not ok then
			unappliedProperties[propertyName] = virtualValue
			continue
		end

		local ok = setProperty(instance, propertyName, value)
		if not ok then
			unappliedProperties[propertyName] = virtualValue
		end
	end

	-- If there were any properties that we failed to assign, push this into our
	-- unapplied patch as an update that would need to be applied.
	if next(unappliedProperties) ~= nil then
		table.insert(unappliedPatch.updated, {
			id = id,
			changedProperties = unappliedProperties,
		})
	end

	for _, childId in ipairs(virtualInstance.Children) do
		reifyInner(instanceMap, virtualInstances, childId, instance, unappliedPatch, deferredRefs)
	end

	instance.Parent = parentInstance
	instanceMap:insert(id, instance)
end

return reify