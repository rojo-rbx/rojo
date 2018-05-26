local Config = require(script.Parent.Config)
local ApiContext = require(script.Parent.ApiContext)

local REMOTE_URL = ("http://localhost:%d"):format(Config.port)

local Session = {}
Session.__index = Session

function Session.new()
	local self = {}

	setmetatable(self, Session)

	local created = {}

	created["0"] = game:GetService("ReplicatedFirst")

	local api
	local function readAll()
		print("Reading all...")

		return api:readAll()
			:andThen(function(instances)
				local visited = {}
				for id, instance in pairs(instances) do
					visited[id] = true
					if id ~= "0" then
						local existing = created[id]
						if existing ~= nil then
							pcall(existing.Destroy, existing)
						end

						local real = Instance.new(instance.className)
						real.Name = instance.name

						for key, value in pairs(instance.properties) do
							real[key] = value
						end

						created[id] = real
					end
				end

				for id, instance in pairs(instances) do
					if id ~= "0" then
						print("parent", created[id], created[tostring(instance.parent)])
						created[id].Parent = created[tostring(instance.parent)]
					end
				end

				for id, object in pairs(created) do
					if not visited[id] then
						object:Destroy()
					end
				end
			end)
	end

	api = ApiContext.new(REMOTE_URL, function(message)
		print("got message", message)

		if message.type == "InstanceChanged" then
			print("Instance", message.id, "changed!")
			readAll()
		else
			warn("Unknown message type " .. message.type)
		end
	end)

	api:connect()
		:andThen(readAll)
		:andThen(function()
			return api:retrieveMessages()
		end)

	return self
end

return Session
