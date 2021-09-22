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

-- The kinds of instances that need to be paused for a longer time.
local ExtraPauseCycles = {
	LuaSourceContainer = 1,
}

local function getExtraCycleCount(instance)
	for instanceKind, cycleCount in pairs(ExtraPauseCycles) do
		if instance:IsA(instanceKind) then
			return cycleCount
		end
	end

	return 0
end

function ChangeBatcher.new(instanceMap, onChangesFlushed)
	local self

	local heartbeatConnection = RunService.Heartbeat:Connect(function(dt)
		self:__cycle(dt)
	end)

	self = setmetatable({
		__accumulator = 0,
		__heartbeatConnection = heartbeatConnection,
		__instanceMap = instanceMap,

		-- A map of paused instances to numbers indicating how many additional
		-- cycles the instance must remain paused. Most kinds of instances never
		-- end up in this table and are unpaused as soon as possible. However,
		-- some have properties that can cause their changed signals to fire
		-- more times and later than expected, and should stay paused for longer.
		__remainingPauseCycles = {},

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

	-- Decrement the cycle counter of each instance waiting to be unpaused and
	-- unpause any that are ready.
	for instance in pairs(self.__remainingPauseCycles) do
		local remainingCycles = self.__remainingPauseCycles[instance]

		if remainingCycles - 1 == 0 then
			self.__remainingPauseCycles[instance] = nil
			self.__instanceMap.pausedUpdateInstances[instance] = nil
		else
			self.__remainingPauseCycles[instance] = remainingCycles - 1
		end
	end

	for instance in pairs(self.__instanceMap.pausedUpdateInstances) do
		-- We can unpause most kinds of instances right away. However, setting
		-- some properties may cause additional changed events after this code
		-- runs. This causes us to detect a spurious property change.
		--
		-- For example, changing a script's Source property from a script or the
		-- command bar while that same script happens to be open in Roblox
		-- Studio's script editor causes its changed signal to fire three
		-- distinct times: once when the property is set, then two more times
		-- just after Heartbeat fires (after we run!). In this particular
		-- example, the pause needs to be held over until the next cycle.
		local extraCycleCount = getExtraCycleCount(instance)

		if extraCycleCount > 0 then
			if self.__remainingPauseCycles[instance] then
				continue
			end

			self.__remainingPauseCycles[instance] = extraCycleCount
		else
			self.__instanceMap.pausedUpdateInstances[instance] = nil
		end
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
