--[[
	The ChangeBatcher is responsible for collecting and dispatching changes made
	to tracked instances during two-way sync.
]]

local RunService = game:GetService("RunService")

local PatchSet = require(script.Parent.PatchSet)

local createPatchSet = require(script.createPatchSet)

local ChangeBatcher = {}
ChangeBatcher.__index = ChangeBatcher

local BATCH_INTERVAL = 0.2

function ChangeBatcher.new(instanceMap, onChangesFlushed)
	local self

	local heartbeatConnection = RunService.Heartbeat:Connect(function(dt)
		self:__cycle(dt)
	end)

	self = setmetatable({
		__accumulator = 0,
		__heartbeatConnection = heartbeatConnection,
		__instanceMap = instanceMap,
		__instancesToUnpause = {},
		__onChangesFlushed = onChangesFlushed,
		__pendingPropertyChanges = {},
	}, ChangeBatcher)

	return self
end

function ChangeBatcher:stop()
	self.__heartbeatConnection:Disconnect()
	self.__pendingPropertyChanges = {}
end

function ChangeBatcher:add(instance, propertyName)
	local properties = self.__pendingPropertyChanges[instance]

	if not properties then
		properties = {}
		self.__pendingPropertyChanges[instance] = properties
	end

	properties[propertyName] = true
end

function ChangeBatcher:__cycle(dt)
	self.__accumulator += dt

	if self.__accumulator >= BATCH_INTERVAL then
		self.__accumulator -= BATCH_INTERVAL

		local patch = self:__flush()

		if patch then
			self.__onChangesFlushed(patch)
		end
	end

	-- Instance updates that were paused during the previous cycle should be
	-- unpaused.
	for instance in pairs(self.__instancesToUnpause) do
		self.__instanceMap.pausedBatchInstances[instance] = nil
		self.__instancesToUnpause[instance] = nil
	end

	-- Instance updates that were paused during this cycle should be unpaused
	-- in the next cycle.
	for instance in pairs(self.__instanceMap.pausedBatchInstances) do
		self.__instancesToUnpause[instance] = true
	end
end

function ChangeBatcher:__flush()
	if not next(self.__pendingPropertyChanges) then
		return nil
	end

	local patch = createPatchSet(self.__instanceMap, self.__pendingPropertyChanges)

	if PatchSet.isEmpty(patch) then
		return nil
	end

	return patch
end

return ChangeBatcher
