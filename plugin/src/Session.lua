local Config = require(script.Parent.Config)
local ApiContext = require(script.Parent.ApiContext)
local Logging = require(script.Parent.Logging)

local REMOTE_URL = ("http://localhost:%d"):format(Config.port)

local function reify(instanceData, id, parent)
	local data = instanceData[id]

	local instance = Instance.new(data.ClassName)

	for key, value in pairs(data.Properties) do
		-- TODO: Branch on value.Type
		instance[key] = value.Value
	end

	instance.Name = data.Name

	for _, childId in ipairs(data.Children) do
		reify(instanceData, childId, instance)
	end

	instance.Parent = parent

	return instance
end

local Session = {}
Session.__index = Session

function Session.new()
	local self = {}

	local api

	api = ApiContext.new(REMOTE_URL, function(message)
		if message.type == "InstanceChanged" then
			Logging.trace("Instance %s changed!", message.id)
			-- readAll()
		else
			Logging.warn("Unknown message type %s", message.type)
		end
	end)

	api:connect()
		:andThen(function()
			return api:read({api.rootInstanceId})
		end)
		:andThen(function(response)
			reify(response.instances, api.rootInstanceId, game.ReplicatedStorage)
			return api:retrieveMessages()
		end)
		:catch(function(message)
			Logging.warn("Couldn't start a new Rojo session: %s", tostring(message))
		end)

	return setmetatable(self, Session)
end

return Session
