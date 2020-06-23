--[[
	"Reifies" a virtual DOM, constructing a real DOM with the same shape.
]]

local invariant = require(script.Parent.Parent.invariant)
local Error = require(script.Parent.Error)
local setProperty = require(script.Parent.setProperty)
local decodeValue = require(script.Parent.decodeValue)

local function reify(virtualInstances, rootId, parentInstance)
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
			return false, value
		end

		local ok, err = setProperty(instance, propertyName, value)

		if not ok then
			err.details.instanceId = rootId
			return false, err
		end
	end

	for _, childId in ipairs(virtualInstance.Children) do
		local ok, err = reify(virtualInstances, childId, instance)

		if not ok then
			return false, err
		end
	end

	instance.Parent = parentInstance

	return true, instance
end

return reify