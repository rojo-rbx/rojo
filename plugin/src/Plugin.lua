local Config = require(script.Parent.Config)
local Http = require(script.Parent.Http)
local Server = require(script.Parent.Server)
local Promise = require(script.Parent.Promise)

local function collectMatch(source, pattern)
	local result = {}

	for match in source:gmatch(pattern) do
		table.insert(result, match)
	end

	return result
end

local function fileToName(filename)
	if filename:find("%.server%.lua$") then
		return filename:match("^(.-)%.server%.lua$"), "Script"
	elseif filename:find("%.client%.lua$") then
		return filename:match("^(.-)%.client%.lua$"), "LocalScript"
	elseif filename:find("%.lua") then
		return filename:match("^(.-)%.lua$"), "ModuleScript"
	else
		return filename, "StringValue"
	end
end

local function nameToInstance(filename, contents)
	local name, className = fileToName(filename)

	local instance = Instance.new(className)
	instance.Name = name

	if className:find("Script$") then
		instance.Source = contents
	else
		instance.Value = contents
	end

	return instance
end

local function make(item, name)
	if item.type == "dir" then
		local instance = Instance.new("Folder")
		instance.Name = name

		for childName, child in pairs(item.children) do
			make(child, childName).Parent = instance
		end

		return instance
	elseif item.type == "file" then
		return nameToInstance(name, item.contents)
	else
		error("not implemented")
	end
end

local function write(parent, route, item)
	local location = parent

	for index = 1, #route - 1 do
		local piece = route[index]
		local newLocation = location:FindFirstChild(piece)

		if not newLocation then
			newLocation = Instance.new("Folder")
			newLocation.Name = piece
			newLocation.Parent = location
		end

		location = newLocation
	end

	local fileName = route[#route]
	local name = fileToName(fileName)

	local existing = location:FindFirstChild(name)

	local new
	if item then
		new = make(item, fileName)
	end

	if existing then
		existing:Destroy()
	end

	if new then
		new.Parent = location
	end
end

local Plugin = {}
Plugin.__index = Plugin

function Plugin.new()
	local address = "localhost"
	local port = 8081

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
		local data = items[index]

		local fullRoute = collectMatch(partition.target, "[^.]+")
		for i = 2, #route do
			table.insert(fullRoute, routes[index][i])
		end

		write(game, fullRoute, data)
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
