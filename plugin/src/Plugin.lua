local Config = require(script.Parent.Config)
local Http = require(script.Parent.Http)
local Server = require(script.Parent.Server)
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

	local foop = {
		_http = Http.new(remote),
		_server = nil,
		_polling = false,
	}

	setmetatable(foop, Plugin)

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

		foop._label = screenGui
	end

	return foop
end

function Plugin:server()
	if not self._server then
		self._server = Server.connect(self._http)
			:catch(function(err)
				self._server = nil
				return Promise.reject(err)
			end)
	end

	return self._server
end

function Plugin:connect()
	print("Testing connection...")

	return self:server()
		:andThen(function(server)
			return server:getInfo()
		end)
		:andThen(function(result)
			print("Server found!")
			print("Protocol version:", result.protocolVersion)
			print("Server version:", result.serverVersion)
		end)
end

function Plugin:togglePolling()
	if self._polling then
		self:stopPolling()

		return Promise.resolve(nil)
	else
		return self:startPolling()
	end
end

function Plugin:stopPolling()
	if not self._polling then
		return
	end

	print("Stopped polling.")

	self._polling = false
	self._label.Enabled = false
end

function Plugin:_pull(server, project, routes)
	local items = server:read(routes):await()

	for index = 1, #routes do
		local route = routes[index]
		local partitionName = route[1]
		local partition = project.partitions[partitionName]
		local item = items[index]

		local fullRoute = collectMatch(partition.target, "[^.]+")
		for i = 2, #route do
			table.insert(fullRoute, routes[index][i])
		end

		Reconciler.reconcileRoute(fullRoute, item)
	end
end

function Plugin:startPolling()
	if self._polling then
		return
	end

	print("Starting to poll...")

	self._polling = true
	self._label.Enabled = true

	return self:server()
		:andThen(function(server)
			self:syncIn():await()

			local project = server:getInfo():await().project

			while self._polling do
				local changes = server:getChanges():await()

				local routes = {}

				for _, change in ipairs(changes) do
					table.insert(routes, change.route)
				end

				self:_pull(server, project, routes)

				wait(Config.pollingRate)
			end
		end)
		:catch(function()
			self:stopPolling()
		end)
end

function Plugin:syncIn()
	print("Syncing from server...")

	return self:server()
		:andThen(function(server)
			local project = server:getInfo():await().project

			local routes = {}

			for name in pairs(project.partitions) do
				table.insert(routes, {name})
			end

			self:_pull(server, project, routes)

			print("Sync successful!")
		end)
end

return Plugin
