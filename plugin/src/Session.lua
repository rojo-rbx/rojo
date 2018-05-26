local Config = require(script.Parent.Config)
local Http = require(script.Parent.Http)
local ApiContext = require(script.Parent.ApiContext)
local Promise = require(script.Parent.Promise)

local REMOTE_URL = ("http://localhost:%d"):format(Config.port)

local Session = {}
Session.__index = Session

function Session.new()
	local self = {}

	setmetatable(self, Session)

	local api = ApiContext.new(REMOTE_URL)
	api:connect()

	return self
end

return Session
