local Config = require(script.Parent.Config)
local ApiContext = require(script.Parent.ApiContext)
local Logging = require(script.Parent.Logging)

local REMOTE_URL = ("http://localhost:%d"):format(Config.port)

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
		:andThen(function()
			return api:retrieveMessages()
		end)
		:catch(function(message)
			Logging.warn("Couldn't start a new Rojo session: %s", tostring(message))
		end)

	return setmetatable(self, Session)
end

return Session
