local RunService = game:GetService("RunService")

local Packages = script.Parent.Parent.Packages
local Log = require(Packages.Log)

--[[
	A bidirectional map between instance IDs and Roblox instances. It lets us
	keep track of every instance we know about.

	TODO: Track ancestry to catch when stuff moves?
]]
local InstanceMap = {}
InstanceMap.__index = InstanceMap

function InstanceMap.new(onInstanceChanged)
	local self = {
		-- A map from IDs to instances.
		fromIds = {},

		-- A map from instances to IDs.
		fromInstances = {},

		-- A set of all instances that updates should be paused for. This set
		-- should generally be empty, and will be filled by pauseInstance
		-- temporarily.
		pausedUpdateInstances = {},

		-- A map from instances to a signal or list of signals connected to it.
		instancesToSignal = {},

		-- Callback that's invoked whenever an instance is changed and it was
		-- not paused.
		onInstanceChanged = onInstanceChanged,
	}

	return setmetatable(self, InstanceMap)
end

function InstanceMap:size()
	local size = 0

	for _ in pairs(self.fromIds) do
		size = size + 1
	end

	return size
end

--[[
	Disconnect all connections and release all instance references.
]]
function InstanceMap:stop()
	-- I think this is safe.
	for instance in pairs(self.fromInstances) do
		self:removeInstance(instance)
	end
end

function InstanceMap:__fmtDebug(output)
	output:writeLine("InstanceMap {{")
	output:indent()

	-- Collect all of the entries in the InstanceMap and sort them by their
	-- label, which helps make our output deterministic.
	local entries = {}
	for id, instance in pairs(self.fromIds) do
		local label = string.format("%s (%s)", instance:GetFullName(), instance.ClassName)

		table.insert(entries, {id, label})
	end

	table.sort(entries, function(a, b)
		return a[2] < b[2]
	end)

	for _, entry in ipairs(entries) do
		output:writeLine("{}: {}", entry[1], entry[2])
	end

	output:unindent()
	output:write("}")
end

function InstanceMap:insert(id, instance)
	self:removeId(id)
	self:removeInstance(instance)

	self.fromIds[id] = instance
	self.fromInstances[instance] = id
	self:__connectSignals(instance)
end

function InstanceMap:removeId(id)
	local instance = self.fromIds[id]

	if instance ~= nil then
		self:__disconnectSignals(instance)
		self.fromIds[id] = nil
		self.fromInstances[instance] = nil
	end
end

function InstanceMap:removeInstance(instance)
	local id = self.fromInstances[instance]
	self:__disconnectSignals(instance)

	if id ~= nil then
		self.fromInstances[instance] = nil
		self.fromIds[id] = nil
	end
end

function InstanceMap:destroyInstance(instance)
	local id = self.fromInstances[instance]

	if id ~= nil then
		self:removeId(id)
	end

	for _, descendantInstance in ipairs(instance:GetDescendants()) do
		self:removeInstance(descendantInstance)
	end

	instance:Destroy()
end

function InstanceMap:destroyId(id)
	local instance = self.fromIds[id]
	self:removeId(id)

	if instance ~= nil then
		for _, descendantInstance in ipairs(instance:GetDescendants()) do
			self:removeInstance(descendantInstance)
		end

		instance:Destroy()
	end
end

--[[
	Pause updates for an instance.
]]
function InstanceMap:pauseInstance(instance)
	local id = self.fromInstances[instance]

	-- If we don't know about this instance, ignore it.
	if id == nil then
		return
	end

	self.pausedUpdateInstances[instance] = true
end

--[[
	Unpause updates for an instance.
]]
function InstanceMap:unpauseInstance(instance)
	self.pausedUpdateInstances[instance] = nil
end

--[[
	Unpause updates for all instances.
]]
function InstanceMap:unpauseAllInstances()
	table.clear(self.pausedUpdateInstances)
end

function InstanceMap:__connectSignals(instance)
	-- ValueBase instances have an overriden version of the Changed signal that
	-- only detects changes to their Value property.
	--
	-- We can instead connect listener to each individual property that we care
	-- about on those objects (Name and Value) to emulate the same idea.
	if instance:IsA("ValueBase") then
		local signals = {
			instance:GetPropertyChangedSignal("Name"):Connect(function()
				self:__maybeFireInstanceChanged(instance, "Name")
			end),

			instance:GetPropertyChangedSignal("Value"):Connect(function()
				self:__maybeFireInstanceChanged(instance, "Value")
			end),

			instance:GetPropertyChangedSignal("Parent"):Connect(function()
				self:__maybeFireInstanceChanged(instance, "Parent")
			end),
		}

		self.instancesToSignal[instance] = signals
	else
		self.instancesToSignal[instance] = instance.Changed:Connect(function(propertyName)
			self:__maybeFireInstanceChanged(instance, propertyName)
		end)
	end
end

function InstanceMap:__maybeFireInstanceChanged(instance, propertyName)
	Log.trace("{}.{} changed", instance:GetFullName(), propertyName)

	if self.pausedUpdateInstances[instance] then
		return
	end

	if self.onInstanceChanged == nil then
		return
	end

	if RunService:IsRunning() then
		-- We probably don't want to pick up property changes to save to the
		-- filesystem in a running game.
		return
	end

	self.onInstanceChanged(instance, propertyName)
end

function InstanceMap:__disconnectSignals(instance)
	local signals = self.instancesToSignal[instance]

	if signals ~= nil then
		-- In most cases, we only have a single signal, so we avoid keeping
		-- around the extra table. ValueBase objects force us to use multiple
		-- signals to emulate the Instance.Changed event, however.
		if typeof(signals) == "table" then
			for _, signal in ipairs(signals) do
				signal:Disconnect()
			end
		else
			signals:Disconnect()
		end

		self.instancesToSignal[instance] = nil
	end
end

return InstanceMap
