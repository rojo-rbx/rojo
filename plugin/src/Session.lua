local Config = require(script.Parent.Config)
local ApiContext = require(script.Parent.ApiContext)
local Logging = require(script.Parent.Logging)

local REMOTE_URL = ("http://localhost:%d"):format(Config.port)

local function makeInstanceMap()
	local self = {
		fromIds = {},
		fromInstances = {},
	}

	function self:insert(id, instance)
		self.fromIds[id] = instance
		self.fromInstances[instance] = id
	end

	function self:removeId(id)
		local instance = self.fromIds[id]

		if instance then
			self.fromIds[id] = nil
			self.fromInstances[instance] = nil
		end
	end

	function self:removeInstance(instance)
		local id = self.fromInstances[instance]

		if id then
			self.fromInstances[instance] = nil
			self.fromIds[id] = nil
		end
	end

	return self
end

local function setProperty(instance, key, value)
	local ok, err = pcall(function()
		instance[key] = value
	end)

	if not ok then
		error(("Cannot set property %s on class %s: %s"):format(tostring(key), instance.ClassName, err), 2)
	end
end

local function shouldClearUnknown(id, configMap)
	if configMap[id] then
		return not configMap[id].ignoreUnknown
	else
		return true
	end
end

local function reify(instanceData, instanceMap, configMap, id, parent)
	local data = instanceData[id]

	local instance = Instance.new(data.ClassName)

	for key, value in pairs(data.Properties) do
		-- TODO: Branch on value.Type
		setProperty(instance, key, value.Value)
	end

	instance.Name = data.Name

	for _, childId in ipairs(data.Children) do
		reify(instanceData, instanceMap, configMap, childId, instance)
	end

	setProperty(instance, "Parent", parent)
	instanceMap:insert(id, instance)

	return instance
end

local function reconcile(instanceData, instanceMap, configMap, id, existingInstance)
	local data = instanceData[id]

	assert(data.ClassName == existingInstance.ClassName)

	for key, value in pairs(data.Properties) do
		setProperty(existingInstance, key, value.Value)
	end

	local existingChildren = existingInstance:GetChildren()

	local unvisitedExistingChildren = {}
	for _, child in ipairs(existingChildren) do
		unvisitedExistingChildren[child] = true
	end

	for _, childId in ipairs(data.Children) do
		local childData = instanceData[childId]

		local existingChildInstance
		for instance in pairs(unvisitedExistingChildren) do
			local ok, name, className = pcall(function()
				return instance.Name, instance.ClassName
			end)

			if ok then
				if name == childData.Name and className == childData.ClassName then
					existingChildInstance = instance
					break
				end
			end
		end

		if existingChildInstance ~= nil then
			unvisitedExistingChildren[existingChildInstance] = nil
			reconcile(instanceData, instanceMap, configMap, childId, existingChildInstance)
		else
			reify(instanceData, instanceMap, configMap, childId, existingInstance)
		end
	end

	if shouldClearUnknown(id, configMap) then
		for existingChildInstance in pairs(unvisitedExistingChildren) do
			instanceMap:removeInstance(existingChildInstance)
			existingChildInstance:Destroy()
		end
	end

	return existingInstance
end

local Session = {}
Session.__index = Session

function Session.new()
	local self = {}

	local instanceMap = makeInstanceMap()

	local api = ApiContext.new(REMOTE_URL)

	ApiContext:onMessage(function(message)
		local idsToGet = {}

		for _, id in ipairs(message.added) do
			table.insert(idsToGet, id)
		end

		for _, id in ipairs(message.updated) do
			table.insert(idsToGet, id)
		end

		for _, id in ipairs(message.removed) do
			table.insert(idsToGet, id)
		end

		coroutine.wrap(function()
			-- TODO: This section is a mess
			local _, response = assert(api:read(idsToGet):await())

			for _, id in ipairs(idsToGet) do
				local data = response.instances[id]
				local instance = instanceMap.fromIds[id]

				if data == nil then
					-- TOO: Destroy descendants too
					if instance ~= nil then
						instanceMap:removeInstance(instance)
						instance:Destroy()
					end
				else
					if instance ~= nil then
						reconcile(response.instances, instanceMap, api.configMap, id, instance)
					else
						error("TODO: Crawl up to nearest parent, use that?")
					end
				end
			end
		end)()
	end)

	api:connect()
		:andThen(function()
			return api:read({api.rootInstanceId})
		end)
		:andThen(function(response)
			reconcile(response.instances, instanceMap, api.configMap, api.rootInstanceId, game)
			-- reify(response.instances, instanceMap, configMap, api.rootInstanceId, game.ReplicatedStorage)
			return api:retrieveMessages()
		end)
		:catch(function(message)
			Logging.warn("Couldn't start a new Rojo session: %s", tostring(message))
		end)

	return setmetatable(self, Session)
end

return Session