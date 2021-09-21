--[[
	The ChangeBatcher is responsible for collecting and dispatching changes made
	to tracked instances during two-way sync.
]]

local RunService = game:GetService("RunService")

local PatchSet = require(script.Parent.PatchSet)
local RbxDom = require(script.Parent.Parent.RbxDom)
local Log = require(script.Parent.Parent.Log)

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
		__pendingChanges = {},
	}, ChangeBatcher)

	return self
end

function ChangeBatcher:stop()
	self.__heartbeatConnection:Disconnect()
	self.__pendingChanges = {}
end

function ChangeBatcher:add(instance, propertyName)
	local properties = self.__pendingChanges[instance]

	if not properties then
		properties = {}
		self.__pendingChanges[instance] = properties
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
	if not next(self.__pendingChanges) then
		return nil
	end

	local patch = {
		updated = {},
		removed = {},
		added = {},
	}

	for instance, properties in pairs(self.__pendingChanges) do
		local instanceId = self.__instanceMap.fromInstances[instance]

		if instanceId == nil then
			Log.warn("Ignoring change for instance {:?} as it is unknown to Rojo", instance)
			continue
		end

		for propertyName in pairs(properties) do
			local remove = nil
			local update = {
				id = instanceId,
				changedProperties = {},
			}

			if propertyName == "Name" then
				update.changedName = instance.Name
			elseif propertyName == "Parent" then
				if instance.Parent == nil then
					update = nil
					remove = instanceId
					break
				else
					Log.warn("Cannot sync non-nil Parent property changes yet")
					continue
				end
			else
				local propertyDescriptor = RbxDom.findCanonicalPropertyDescriptor(
					instance.ClassName,
					propertyName
				)

				if not propertyDescriptor then
					Log.debug("Could not sync back property {:?}.{}", instance, propertyName)
					continue
				end

				local readSuccess, readResult = propertyDescriptor:read(instance)

				if not readSuccess then
					Log.warn("Could not sync back property {:?}.{}: {}",
						instance, propertyName, readResult)
					continue
				end

				local dataType = propertyDescriptor.dataType
				local encodeSuccess, encodeResult = RbxDom.EncodedValue.encode(readResult, dataType)

				if not encodeSuccess then
					Log.warn("Could not sync back property {:?}.{}: {}",
						instance, propertyName, encodeResult)
					continue
				end

				update.changedProperties[propertyName] = encodeResult
			end

			table.insert(patch.updated, update)
			table.insert(patch.removed, remove)
		end

		self.__pendingChanges[instance] = nil
	end

	if PatchSet.isEmpty(patch) then
		return nil
	end

	return patch
end

return ChangeBatcher
