--[[
	"Reifies" a virtual DOM, constructing a real DOM with the same shape.
]]

local invariant = require(script.Parent.Parent.invariant)
local Error = require(script.Parent.Error)
local setProperty = require(script.Parent.setProperty)
local decodeValue = require(script.Parent.decodeValue)

local reifyInner

local function reify(instanceMap, virtualInstances, rootId, parentInstance)
	-- Tracks a map from ID to added instance that should be inserted into
	-- instanceMap if this operation is successful.
	local idsToAdd = {}

	local ok, instanceOrErr = reifyInner(virtualInstances, rootId, parentInstance, idsToAdd)

	if not ok then
		return false, instanceOrErr
	end

	for id, instance in pairs(idsToAdd) do
		instanceMap:insert(id, instance)
	end

	return true, instanceOrErr
end

local function debugInstancePath(virtualInstances, id)
	local virtualInstance = virtualInstances[id]
	local name = virtualInstance.Name
	id = virtualInstance.Parent

	while id ~= nil do
		local virtualInstance = virtualInstances[id]

		if virtualInstance == nil then
			break
		end

		name = virtualInstance.Name .. "." .. name
		id = virtualInstance.Parent
	end

	return name
end

function reifyInner(virtualInstances, rootId, parentInstance, idsToAdd)
	local virtualInstance = virtualInstances[rootId]

	if virtualInstance == nil then
		invariant("Cannot reify an instance not present in virtualInstances\nID: {}", rootId)
	end

	-- Instance.new can fail if we're passing in something that can't be
	-- created, like a service, something enabled with a feature flag, or
	-- something that requires higher security than we have.
	local ok, instance = pcall(Instance.new, virtualInstance.ClassName)

	if not ok then
		return false, Error.new(Error.CannotCreateInstance, {
			instanceId = rootId,
			instancePath = debugInstancePath(virtualInstances, rootId),
			className = virtualInstance.ClassName,
		})
	end

	-- TODO: Can this fail? Previous versions of Rojo guarded against this, but
	-- the reason why was uncertain.
	instance.Name = virtualInstance.Name

	for propertyName, virtualValue in pairs(virtualInstance.Properties) do
		local ok, value = decodeValue(virtualValue)

		if not ok then
			value.details.propertyName = propertyName
			value.details.instanceId = rootId
			value.details.instancePath = debugInstancePath(virtualInstances, rootId)
			return false, value
		end

		local ok, err = setProperty(instance, propertyName, value)

		if not ok then
			err.details.instanceId = rootId
			err.details.instancePath = debugInstancePath(virtualInstances, rootId)
			return false, err
		end
	end

	for _, childId in ipairs(virtualInstance.Children) do
		local ok, err = reifyInner(virtualInstances, childId, instance, idsToAdd)

		if not ok then
			return false, err
		end
	end

	instance.Parent = parentInstance
	idsToAdd[rootId] = instance

	return true, instance
end

return reify