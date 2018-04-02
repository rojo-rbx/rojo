local HttpService = game:GetService("HttpService")

local Config = require(script.Parent.Config)
local Promise = require(script.Parent.Promise)
local Version = require(script.Parent.Version)

local Server = {}
Server.__index = Server

--[[
	Create a new Server using the given HTTP implementation and replacer.

	If the context becomes invalid, `replacer` will be invoked with a new
	context that should be suitable to replace this one.

	Attempting to invoke methods on an invalid conext will throw errors!
]]
function Server.connect(http)
	local context = {
		http = http,
		serverId = nil,
		currentTime = 0,
	}

	setmetatable(context, Server)

	return context:_start()
end

function Server:_start()
	return self:getInfo()
		:andThen(function(response)
			if response.protocolVersion ~= Config.protocolVersion then
				local message = (
					"Found a Rojo dev server, but it's using a different protocol version, and is incompatible." ..
					"\nMake sure you have matching versions of both the Rojo plugin and server!" ..
					"\n\nYour client is version %s, with protocol version %s. It expects server version %s." ..
					"\nYour server is version %s, with protocol version %s." ..
					"\n\nGo to https://github.com/LPGhatguy/rojo for more details."
				):format(
					Version.display(Config.version), Config.protocolVersion,
					Config.expectedServerVersionString,
					response.serverVersion, response.protocolVersion
				)

				return Promise.reject(message)
			end

			self.serverId = response.serverId
			self.currentTime = response.currentTime

			return self
		end)
end

function Server:getInfo()
	return self.http:get("/")
		:andThen(function(response)
			response = response:json()

			return response
		end)
end

function Server:read(paths)
	local body = HttpService:JSONEncode(paths)

	return self.http:post("/read", body)
		:andThen(function(response)
			response = response:json()

			return response.items
		end)
end

function Server:getChanges()
	local url = ("/changes/%f"):format(self.currentTime)

	return self.http:get(url)
		:andThen(function(response)
			response = response:json()

			self.currentTime = response.currentTime

			return response.changes
		end)
end

return Server
