local HttpService = game:GetService("HttpService")

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
