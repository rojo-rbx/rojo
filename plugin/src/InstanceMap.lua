local Log = require(script.Parent.Parent.Log)

--[[
	A bidirectional map between instance IDs and Roblox instances. It lets us
	keep track of every instance we know about.

	TODO: Track ancestry to catch when stuff moves?
]]
local InstanceMap = {}
InstanceMap.__index = InstanceMap

function InstanceMap.new()
	local self = {
		fromIds = {},
		fromInstances = {},
	}

	return setmetatable(self, InstanceMap)
end

function InstanceMap:insert(id, instance)
	self.fromIds[id] = instance
	self.fromInstances[instance] = id
end

function InstanceMap:removeId(id)
	local instance = self.fromIds[id]

	if instance ~= nil then
		self.fromIds[id] = nil
		self.fromInstances[instance] = nil
	else
		Log.warn("Attempted to remove nonexistant ID %s", tostring(id))
	end
end

function InstanceMap:removeInstance(instance)
	local id = self.fromInstances[instance]

	if id ~= nil then
		self.fromInstances[instance] = nil
		self.fromIds[id] = nil
	else
		Log.warn("Attempted to remove nonexistant instance %s", tostring(instance))
	end
end

function InstanceMap:destroyInstance(instance)
	local id = self.fromInstances[instance]

	if id ~= nil then
		self:destroyId(id)
	else
		Log.warn("Attempted to destroy untracked instance %s", tostring(instance))
	end
end

function InstanceMap:destroyId(id)
	local instance = self.fromIds[id]
	self:removeId(id)

	if instance ~= nil then
		local descendantsToDestroy = {}

		for otherInstance in pairs(self.fromInstances) do
			if otherInstance:IsDescendantOf(instance) then
				table.insert(descendantsToDestroy, otherInstance)
			end
		end

		for _, otherInstance in ipairs(descendantsToDestroy) do
			self:removeInstance(otherInstance)
		end

		instance:Destroy()
	else
		Log.warn("Attempted to destroy nonexistant ID %s", tostring(id))
	end
end

return InstanceMap