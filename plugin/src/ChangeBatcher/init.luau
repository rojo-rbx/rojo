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

	local renderSteppedConnection = RunService.RenderStepped:Connect(function(dt)
		self:__cycle(dt)
	end)

	self = setmetatable({
		__accumulator = 0,
		__renderSteppedConnection = renderSteppedConnection,
		__instanceMap = instanceMap,
		__onChangesFlushed = onChangesFlushed,
		__pendingPropertyChanges = {},
	}, ChangeBatcher)

	return self
end

function ChangeBatcher:stop()
	self.__renderSteppedConnection:Disconnect()
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

	self.__instanceMap:unpauseAllInstances()
end

function ChangeBatcher:__flush()
	if next(self.__pendingPropertyChanges) == nil then
		return nil
	end

	local patch = createPatchSet(self.__instanceMap, self.__pendingPropertyChanges)

	if PatchSet.isEmpty(patch) then
		return nil
	end

	return patch
end

return ChangeBatcher
