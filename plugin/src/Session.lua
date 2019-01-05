local ApiContext = require(script.Parent.ApiContext)
local Logging = require(script.Parent.Logging)
local Reconciler = require(script.Parent.Reconciler)

local Session = {}
Session.__index = Session

function Session.new(config)
	local self = {}

	self.onError = config.onError

	local reconciler

	local remoteUrl = ("http://%s:%s"):format(config.address, config.port)

	local api = ApiContext.new(remoteUrl)

	ApiContext:onMessage(function(message)
		local requestedIds = {}

		for _, id in ipairs(message.added) do
			table.insert(requestedIds, id)
		end

		for _, id in ipairs(message.updated) do
			table.insert(requestedIds, id)
		end

		for _, id in ipairs(message.removed) do
			table.insert(requestedIds, id)
		end

		return api:read(requestedIds)
			:andThen(function(response)
				return reconciler:applyUpdate(requestedIds, response.instances)
			end)
			:catch(function(message)
				Logging.warn("%s", tostring(message))
				self.onError()
			end)
	end)

	api:connect()
		:andThen(function()
			reconciler = Reconciler.new(api.instanceMetadataMap)

			return api:read({api.rootInstanceId})
		end)
		:andThen(function(response)
			reconciler:reconcile(response.instances, api.rootInstanceId, game)
			return api:retrieveMessages()
		end)
		:catch(function(message)
			Logging.warn("%s", tostring(message))
			self.onError()
		end)

	return setmetatable(self, Session)
end

return Session