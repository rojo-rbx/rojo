local Config = require(script.Parent.Config)
local Promise = require(script.Parent.Promise)
local Version = require(script.Parent.Version)
local Http = require(script.Parent.Http)

local ApiContext = {}
ApiContext.__index = ApiContext

ApiContext.Error = {
	ServerIdMismatch = "ServerIdMismatch",
}

setmetatable(ApiContext.Error, {
	__index = function(_, key)
		error("Invalid API.Error name " .. key, 2)
	end
})

function ApiContext.new(url, onMessage)
	assert(type(url) == "string")
	assert(type(onMessage) == "function")

	local context = {
		url = url,
		onMessage = onMessage,
		serverId = nil,
		currentTime = 0,
	}

	setmetatable(context, ApiContext)

	return context
end

function ApiContext:connect()
	return Http.get(self.url)
		:andThen(function(response)
			response = response:json()

			if response.protocolVersion ~= Config.protocolVersion then
				local message = (
					"Found a Rojo dev server, but it's using a different protocol version, and is incompatible." ..
					"\nMake sure you have matching versions of both the Rojo plugin and server!" ..
					"\n\nYour client is version %s, with protocol version %s. It expects server version %s." ..
					"\nYour server is version %s, with protocol version %s." ..
					"\n\nGo to https://github.com/LPGhatguy/rojo for more details."
				):format(
					Version.display(Config.version), Config.protocolVersion,
					Config.expectedApiContextVersionString,
					response.serverVersion, response.protocolVersion
				)

				return Promise.reject(message)
			end

			self.serverId = response.serverId
			self.currentTime = response.currentTime
		end)
end

return ApiContext
