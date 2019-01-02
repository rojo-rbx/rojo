local ApiContext = require(script.Parent.ApiContext)
local Config = require(script.Parent.Config)
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

		if instance ~= nil then
			self.fromIds[id] = nil
			self.fromInstances[instance] = nil
		end
	end

	function self:removeInstance(instance)
		local id = self.fromInstances[instance]

		if id ~= nil then
			self.fromInstances[instance] = nil
			self.fromIds[id] = nil
		end
	end

	function self:destroyId(id)
		self:removeId(id)
		local instance = self.fromIds[id]

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

local function shouldClearUnknown(id, instanceMetadataMap)
	if instanceMetadataMap[id] then
		return not instanceMetadataMap[id].ignoreUnknown
	else
		return true
	end
end

local function reify(instanceData, instanceMap, instanceMetadataMap, id, parent)
	local data = instanceData[id]

	local instance = Instance.new(data.ClassName)

	for key, value in pairs(data.Properties) do
		-- TODO: Branch on value.Type
		setProperty(instance, key, value.Value)
	end

	instance.Name = data.Name

	for _, childId in ipairs(data.Children) do
		reify(instanceData, instanceMap, instanceMetadataMap, childId, instance)
	end

	setProperty(instance, "Parent", parent)
	instanceMap:insert(id, instance)

	return instance
end

local function reconcile(instanceData, instanceMap, instanceMetadataMap, id, existingInstance)
	local data = instanceData[id]

	if data.ClassName ~= existingInstance.ClassName then
		-- TODO: Preserve existing children instead?
		local parent = existingInstance.Parent
		instanceMap:destroyId(id)
		reify(instanceData, instanceMap, instanceMetadataMap, id, parent)
		return
	end

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
			reconcile(instanceData, instanceMap, instanceMetadataMap, childId, existingChildInstance)
		else
			reify(instanceData, instanceMap, instanceMetadataMap, childId, existingInstance)
		end
	end

	if shouldClearUnknown(id, instanceMetadataMap) then
		for existingChildInstance in pairs(unvisitedExistingChildren) do
			instanceMap:removeInstance(existingChildInstance)
			existingChildInstance:Destroy()
		end
	end

	return existingInstance
end

local function applyUpdatePiece(id, visitedIds, responseData, instanceMap, instanceMetadataMap)
	if visitedIds[id] then
		return
	end

	visitedIds[id] = true

	local data = responseData[id]
	local instance = instanceMap.fromIds[id]

	-- The instance was deleted in this update
	if data == nil then
		instanceMap:destroyId(id)
		return
	end

	-- An instance we know about was updated
	if instance ~= nil then
		reconcile(responseData, instanceMap, instanceMetadataMap, id, instance)
		return instance
	end

	-- If the instance's parent already exists, we can stick it there
	local parentInstance = instanceMap.fromIds[data.Parent]
	if parentInstance ~= nil then
		reify(responseData, instanceMap, instanceMetadataMap, id, parentInstance)
		return
	end

	-- Otherwise, we can check if this response payload contained the parent and
	-- work from there instead.
	local parentData = responseData[data.Parent]
	if parentData ~= nil then
		if visitedIds[data.Parent] then
			error("Rojo bug: An instance was present and marked as visited but its instance was missing")
		end

		applyUpdatePiece(data.Parent, visitedIds, responseData, instanceMap, instanceMetadataMap)
		return
	end

	error("Rojo NYI: Instances with parents that weren't mentioned in an update payload")
end

local function applyUpdate(requestedIds, responseData, instanceMap, instanceMetadataMap)
	-- This function may eventually be asynchronous; it will require calls to
	-- the server to resolve instances that don't exist yet.
	local visitedIds = {}

	for _, id in ipairs(requestedIds) do
		applyUpdatePiece(id, visitedIds, responseData, instanceMap, instanceMetadataMap)
	end
end

local Session = {}
Session.__index = Session

function Session.new()
	local self = {}

	local instanceMap = makeInstanceMap()

	local api = ApiContext.new(REMOTE_URL)

	ApiContext:onMessage(function(message)
		local requestedIds = {}

		for _, id in ipairs(message.added) do
			table.insert(requestedIds, id)
		end

		for _, id in ipairs(message.updated) do
			table.insert(requestedIds, id)
		end

		for _, id in ipairs(message.removed) do
			table.insert(requestedIds, id)
		end

		return api:read(requestedIds)
			:andThen(function(response)
				return applyUpdate(requestedIds, response.instances, instanceMap, api.instanceMetadataMap)
			end)
	end)

	api:connect()
		:andThen(function()
			return api:read({api.rootInstanceId})
		end)
		:andThen(function(response)
			reconcile(response.instances, instanceMap, api.instanceMetadataMap, api.rootInstanceId, game)
			-- reify(response.instances, instanceMap, instanceMetadataMap, api.rootInstanceId, game.ReplicatedStorage)
			return api:retrieveMessages()
		end)
		:catch(function(message)
			Logging.warn("Couldn't start a new Rojo session: %s", tostring(message))
		end)

	return setmetatable(self, Session)
end

return Session