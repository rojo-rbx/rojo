local Config = require(script.Parent.Config)
local ApiContext = require(script.Parent.ApiContext)

local REMOTE_URL = ("http://localhost:%d"):format(Config.port)

local Session = {}
Session.__index = Session

function Session.new()
	local self = {}

	setmetatable(self, Session)

	local api

	api = ApiContext.new(REMOTE_URL, function(message)
		if message.type == "InstanceChanged" then
			print("Instance", message.id, "changed!")
			-- readAll()
		else
			warn("Unknown message type " .. message.type)
		end
	end)

	api:connect()
		:andThen(function()
			return api:read({api.rootInstanceId})
		end)
		:andThen(function()
			return api:retrieveMessages()
		end)

	return self
end

return Session
