local InstanceReferences = {}
InstanceReferences.__index = InstanceReferences

function InstanceReferences.new(sessionId: string)
	assert(type(sessionId) == "string" and sessionId ~= "", "sessionId must be a non-empty string")
	return setmetatable({
		__sessionId = sessionId,
		__nextId = 1,
		__byId = setmetatable({}, { __mode = "v" }),
		__byInstance = setmetatable({}, { __mode = "k" }),
	}, InstanceReferences)
end

local function isInDataModel(instance)
	local ok, result = pcall(function()
		return instance == game or instance:IsDescendantOf(game)
	end)
	return ok and result
end

function InstanceReferences:reference(instance, path: string)
	if typeof(instance) ~= "Instance" or not isInDataModel(instance) then
		return nil, "Instance is destroyed or outside the current DataModel"
	end

	local id = self.__byInstance[instance]
	if id == nil then
		id = string.format("pinst-%08d", self.__nextId)
		self.__nextId += 1
		self.__byInstance[instance] = id
		self.__byId[id] = instance
	end

	return {
		sessionId = self.__sessionId,
		id = id,
		path = path,
		name = instance.Name,
		className = instance.ClassName,
	}
end

function InstanceReferences:resolve(id: string)
	local instance = self.__byId[id]
	if instance == nil or not isInDataModel(instance) then
		self.__byId[id] = nil
		return nil, "Instance reference is stale"
	end
	return instance
end

function InstanceReferences:clear()
	table.clear(self.__byId)
	table.clear(self.__byInstance)
end

return InstanceReferences
