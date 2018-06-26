local Config = require(script.Parent.Config)
local ApiContext = require(script.Parent.ApiContext)

local REMOTE_URL = ("http://localhost:%d"):format(Config.port)

local Session = {}
Session.__index = Session

function Session.new()
	local self = {}

	setmetatable(self, Session)

	local function createFoldersUntil(location, route)
		for i = 1, #route - 1 do
			local piece = route[i]

			local child = location:FindFirstChild(piece)

			if child == nil then
				child = Instance.new("Folder")
				child.Name = piece
				child.Parent = location
			end

			location = child
		end

		return location
	end

	local function reify(instancesById, id)
		local object = instancesById[tostring(id)]
		local instance = Instance.new(object.className)
		instance.Name = object.name

		for key, property in pairs(object.properties) do
			instance[key] = property.value
		end

		for _, childId in ipairs(object.children) do
			reify(instancesById, childId).Parent = instance
		end

		return instance
	end

	local api
	local function readAll()
		print("Reading all...")

		return api:readAll()
			:andThen(function(response)
				for partitionName, partitionRoute in pairs(api.partitionRoutes) do
					local parent = createFoldersUntil(game, partitionRoute)

					local rootInstanceId = response.partitionInstances[partitionName]

					print("Root for", partitionName, "is", rootInstanceId)

					reify(response.instances, rootInstanceId).Parent = parent
				end
			end)
	end

	api = ApiContext.new(REMOTE_URL, function(message)
		if message.type == "InstanceChanged" then
			print("Instance", message.id, "changed!")
			-- readAll()
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
