local Config = require(script.Parent.Config)
local Http = require(script.Parent.Http)
local Api = require(script.Parent.Api)
local Promise = require(script.Parent.Promise)
local Reconciler = require(script.Parent.Reconciler)

local function collectMatch(source, pattern)
	local result = {}

	for match in source:gmatch(pattern) do
		table.insert(result, match)
	end

	return result
end

local Plugin = {}
Plugin.__index = Plugin

function Plugin.new()
	local address = "localhost"
	local port = Config.dev and 8001 or 8000

	local remote = ("http://%s:%d"):format(address, port)

	local self = {
		_http = Http.new(remote),
		_reconciler = Reconciler.new(),
		_api = nil,
		_polling = false,
		_syncInProgress = false,
	}

	setmetatable(self, Plugin)

	do
		local screenGui = Instance.new("ScreenGui")
		screenGui.Name = "Rojo UI"
		screenGui.Parent = game.CoreGui
		screenGui.DisplayOrder = -1
		screenGui.Enabled = false

		local label = Instance.new("TextLabel")
		label.Font = Enum.Font.SourceSans
		label.TextSize = 20
		label.Text = "Rojo polling..."
		label.BackgroundColor3 = Color3.fromRGB(31, 31, 31)
		label.BackgroundTransparency = 0.5
		label.BorderSizePixel = 0
		label.TextColor3 = Color3.new(1, 1, 1)
		label.Size = UDim2.new(0, 120, 0, 28)
		label.Position = UDim2.new(0, 0, 0, 0)
		label.Parent = screenGui

		self._label = screenGui
	end

	return self
end

--[[
	Clears all state and issues a notice to the user that the plugin has
	restarted.
]]
function Plugin:restart()
	warn("Rojo: The server has changed since the last request, reloading plugin...")

	self._reconciler:clear()
	self._api = nil
	self._polling = false
	self._syncInProgress = false
end

function Plugin:api()
	if not self._api then
		self._api = Api.connect(self._http)
			:catch(function(err)
				self._api = nil
				return Promise.reject(err)
			end)
	end

	return self._api
end

function Plugin:connect()
	print("Rojo: Testing connection...")

	return self:api()
		:andThen(function(api)
			local ok, info = api:getInfo():await()

			if not ok then
				return Promise.reject(info)
			end

			print("Rojo: Server found!")
			print("Rojo: Protocol version:", info.protocolVersion)
			print("Rojo: Server version:", info.serverVersion)
		end)
		:catch(function(err)
			if err == Api.Error.ServerIdMismatch then
				self:restart()
				return self:connect()
			else
				return Promise.reject(err)
			end
		end)
end

function Plugin:togglePolling()
	if self._polling then
		return self:stopPolling()
	else
		return self:startPolling()
	end
end

function Plugin:stopPolling()
	if not self._polling then
		return Promise.resolve(false)
	end

	print("Rojo Stopped polling server for changes.")

	self._polling = false
	self._label.Enabled = false

	return Promise.resolve(true)
end

function Plugin:_pull(api, project, routes)
	return api:read(routes)
		:andThen(function(items)
			for index = 1, #routes do
				local itemRoute = routes[index]
				local partitionName = itemRoute[1]
				local partition = project.partitions[partitionName]
				local item = items[index]

				local partitionRoute = collectMatch(partition.target, "[^.]+")

				-- If the item route's length was 1, we need to rename the instance to
				-- line up with the partition's root object name.
				--
				-- This is a HACK!
				if #itemRoute == 1 then
					if item then
						local objectName = partition.target:match("[^.]+$")
						item.Name = objectName
					end
				end

				local fullRoute = {}
				for _, piece in ipairs(partitionRoute) do
					table.insert(fullRoute, piece)
				end

				for i = 2, #itemRoute do
					table.insert(fullRoute, itemRoute[i])
				end

				self._reconciler:reconcileRoute(fullRoute, item, itemRoute)
			end
		end)
end

function Plugin:startPolling()
	if self._polling then
		return
	end

	print("Rojo: Polling server for changes...")

	self._polling = true
	self._label.Enabled = true

	return self:api()
		:andThen(function(api)
			local syncOk, result = self:syncIn():await()

			if not syncOk then
				return Promise.reject(result)
			end

			local infoOk, info = api:getInfo():await()

			if not infoOk then
				return Promise.reject(info)
			end

			while self._polling do
				local changesOk, changes = api:getChanges():await()

				if not changesOk then
					return Promise.reject(changes)
				end

				if #changes > 0 then
					local routes = {}

					for _, change in ipairs(changes) do
						table.insert(routes, change.route)
					end

					local pullOk, pullResult = self:_pull(api, info.project, routes):await()

					if not pullOk then
						return Promise.reject(pullResult)
					end
				end

				wait(Config.pollingRate)
			end
		end)
		:catch(function(err)
			self:stopPolling()

			if err == Api.Error.ServerIdMismatch then
				self:restart()
				return self:startPolling()
			else
				return Promise.reject(err)
			end
		end)
end

function Plugin:syncIn()
	if self._syncInProgress then
		warn("Rojo: Can't sync right now, because a sync is already in progress.")

		return Promise.resolve()
	end

	self._syncInProgress = true
	print("Rojo: Syncing from server...")

	return self:api()
		:andThen(function(api)
			local ok, info = api:getInfo():await()

			if not ok then
				return Promise.reject(info)
			end

			local routes = {}

			for name in pairs(info.project.partitions) do
				table.insert(routes, {name})
			end

			self:_pull(api, info.project, routes)

			self._syncInProgress = false
			print("Rojo: Sync successful!")
		end)
		:catch(function(err)
			self._syncInProgress = false

			if err == Api.Error.ServerIdMismatch then
				self:restart()
				return self:syncIn()
			else
				return Promise.reject(err)
			end
		end)
end

return Plugin
