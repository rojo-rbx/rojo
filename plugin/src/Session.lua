local Rojo = script:FindFirstAncestor("Rojo")

local Promise = require(Rojo.Promise)

local ApiContext = require(script.Parent.ApiContext)
local Reconciler = require(script.Parent.Reconciler)

local Session = {}
Session.__index = Session

function Session.new(config)
	local remoteUrl = ("http://%s:%s"):format(config.address, config.port)
	local api = ApiContext.new(remoteUrl)

	local self = {
		onError = config.onError,
		disconnected = false,
		reconciler = Reconciler.new(),
		api = api,
	}

	api:connect()
		:andThen(function()
			if self.disconnected then
				return
			end

			return api:read({api.rootInstanceId})
		end)
		:andThen(function(response)
			if self.disconnected then
				return
			end

			self.reconciler:reconcile(response.instances, api.rootInstanceId, game)
			return self:__processMessages()
		end)
		:catch(function(message)
			self.disconnected = true
			self.onError(message)
		end)

	return not self.disconnected, setmetatable(self, Session)
end

function Session:__processMessages()
	if self.disconnected then
		return Promise.resolve()
	end

	return self.api:retrieveMessages()
		:andThen(function(messages)
			local promise = Promise.resolve(nil)

			for _, message in ipairs(messages) do
				promise = promise:andThen(function()
					return self:__onMessage(message)
				end)
			end

			return promise
		end)
		:andThen(function()
			return self:__processMessages()
		end)
end

function Session:__onMessage(message)
	if self.disconnected then
		return Promise.resolve()
	end

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

	return self.api:read(requestedIds)
		:andThen(function(response)
			return self.reconciler:applyUpdate(requestedIds, response.instances)
		end)
end

function Session:disconnect()
	self.disconnected = true
end

return Session