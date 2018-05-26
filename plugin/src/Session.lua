local Promise = require(script.Parent.Parent.modules.Promise)

local Config = require(script.Parent.Config)
local Http = require(script.Parent.Http)
local ApiContext = require(script.Parent.ApiContext)

local REMOTE_URL = ("http://localhost:%d"):format(Config.port)

local Session = {}
Session.__index = Session

function Session.new()
	local self = {}

	setmetatable(self, Session)

	local api = ApiContext.new(REMOTE_URL, function(message)
		print("Got message:", message)
	end)
	api:connect()

	return self
end

return Session
