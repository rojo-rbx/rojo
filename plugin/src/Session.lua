local ApiContext = require(script.Parent.ApiContext)
local Config = require(script.Parent.Config)
local Logging = require(script.Parent.Logging)

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
		else
			Logging.warn("Attempted to remove nonexistant ID %s", tostring(id))
		end
	end

	function self:removeInstance(instance)
		local id = self.fromInstances[instance]

		if id ~= nil then
			self.fromInstances[instance] = nil
			self.fromIds[id] = nil
		else
			Logging.warn("Attempted to remove nonexistant instance %s", tostring(instance))
		end
	end

	function self:destroyId(id)
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
			Logging.warn("Attempted to destroy nonexistant ID %s", tostring(id))
		end
	end

	return self
end

local function setProperty(instance, key, value)
	local ok, err = pcall(function()
		if instance[key] ~= value then
			instance[key] = value
		end
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

local function reify(virtualInstancesById, instanceMap, instanceMetadataMap, id, parent)
	local virtualInstance = virtualInstancesById[id]

	local instance = Instance.new(virtualInstance.ClassName)

	for key, value in pairs(virtualInstance.Properties) do
		-- TODO: Branch on value.Type
		setProperty(instance, key, value.Value)
	end

	instance.Name = virtualInstance.Name

	for _, childId in ipairs(virtualInstance.Children) do
		reify(virtualInstancesById, instanceMap, instanceMetadataMap, childId, instance)
	end

	setProperty(instance, "Parent", parent)
	instanceMap:insert(id, instance)

	return instance
end

--[[
	Update an existing instance, including its properties and children, to match
	the given information.
]]
local function reconcile(virtualInstancesById, instanceMap, instanceMetadataMap, id, existingInstance)
	local virtualInstance = virtualInstancesById[id]

	-- If an instance changes ClassName, we assume it's very different. That's
	-- not always the case!
	if virtualInstance.ClassName ~= existingInstance.ClassName then
		-- TODO: Preserve existing children instead?
		local parent = existingInstance.Parent
		instanceMap:destroyId(id)
		reify(virtualInstancesById, instanceMap, instanceMetadataMap, id, parent)
		return
	end

	instanceMap:insert(id, existingInstance)

	-- Some instances don't like being named, even if their name already matches
	setProperty(existingInstance, "Name", virtualInstance.Name)

	for key, value in pairs(virtualInstance.Properties) do
		setProperty(existingInstance, key, value.Value)
	end

	local existingChildren = existingInstance:GetChildren()

	local unvisitedExistingChildren = {}
	for _, child in ipairs(existingChildren) do
		unvisitedExistingChildren[child] = true
	end

	for _, childId in ipairs(virtualInstance.Children) do
		local childData = virtualInstancesById[childId]

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
			reconcile(virtualInstancesById, instanceMap, instanceMetadataMap, childId, existingChildInstance)
		else
			reify(virtualInstancesById, instanceMap, instanceMetadataMap, childId, existingInstance)
		end
	end

	if shouldClearUnknown(id, instanceMetadataMap) then
		for existingChildInstance in pairs(unvisitedExistingChildren) do
			instanceMap:removeInstance(existingChildInstance)
			existingChildInstance:Destroy()
		end
	end

	-- The root instance of a project won't have a parent, like the DataModel,
	-- so we need to be careful here.
	if virtualInstance.Parent ~= nil then
		local parent = instanceMap.fromIds[virtualInstance.Parent]

		if parent == nil then
			Logging.info("Instance %s wanted parent of %s", tostring(id), tostring(virtualInstance.Parent))
			error("Rojo bug: During reconciliation, an instance referred to an instance ID as parent that does not exist.")
		end

		-- Some instances, like services, don't like having their Parent
		-- property poked, even if we're setting it to the same value.
		setProperty(existingInstance, "Parent", parent)
		if existingInstance.Parent ~= parent then
			existingInstance.Parent = parent
		end
	end

	return existingInstance
end

local function applyUpdatePiece(id, visitedIds, virtualInstancesById, instanceMap, instanceMetadataMap)
	if visitedIds[id] then
		return
	end

	visitedIds[id] = true

	local virtualInstance = virtualInstancesById[id]
	local instance = instanceMap.fromIds[id]

	-- The instance was deleted in this update
	if virtualInstance == nil then
		instanceMap:destroyId(id)
		return
	end

	-- An instance we know about was updated
	if instance ~= nil then
		reconcile(virtualInstancesById, instanceMap, instanceMetadataMap, id, instance)
		return instance
	end

	-- If the instance's parent already exists, we can stick it there
	local parentInstance = instanceMap.fromIds[virtualInstance.Parent]
	if parentInstance ~= nil then
		reify(virtualInstancesById, instanceMap, instanceMetadataMap, id, parentInstance)
		return
	end

	-- Otherwise, we can check if this response payload contained the parent and
	-- work from there instead.
	local parentData = virtualInstancesById[virtualInstance.Parent]
	if parentData ~= nil then
		if visitedIds[virtualInstance.Parent] then
			error("Rojo bug: An instance was present and marked as visited but its instance was missing")
		end

		applyUpdatePiece(virtualInstance.Parent, visitedIds, virtualInstancesById, instanceMap, instanceMetadataMap)
		return
	end

	Logging.trace("Instance ID %s, parent ID %s", tostring(id), tostring(virtualInstance.Parent))
	error("Rojo NYI: Instances with parents that weren't mentioned in an update payload")
end

local function applyUpdate(requestedIds, virtualInstancesById, instanceMap, instanceMetadataMap)
	-- This function may eventually be asynchronous; it will require calls to
	-- the server to resolve instances that don't exist yet.
	local visitedIds = {}

	for _, id in ipairs(requestedIds) do
		applyUpdatePiece(id, visitedIds, virtualInstancesById, instanceMap, instanceMetadataMap)
	end
end

local Session = {}
Session.__index = Session

function Session.new(config)
	local self = {}

	self.onError = config.onError

	local instanceMap = makeInstanceMap()

	local remoteUrl = ("http://%s:%s"):format(config.address, config.port)

	local api = ApiContext.new(remoteUrl)

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
			:catch(function(message)
				Logging.warn("%s", tostring(message))
				self.onError()
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
			Logging.warn("%s", tostring(message))
			self.onError()
		end)

	return setmetatable(self, Session)
end

return Session