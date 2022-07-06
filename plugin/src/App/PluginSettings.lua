--[[
	Persistent plugin settings that can be accessed via Roact context.
]]

local Rojo = script:FindFirstAncestor("Rojo")

local Roact = require(Rojo.Roact)
local Log = require(Rojo.Log)

local defaultSettings = {
	openScriptsExternally = false,
	twoWaySync = false,
	showNotifications = true,
	playSounds = true,
	typecheckingEnabled = false,
	logLevel = Log.Level.Info,
}

local Settings = {}
Settings.__index = Settings

function Settings.fromPlugin(plugin)
	local values = {}

	for name, defaultValue in pairs(defaultSettings) do
		local savedValue = plugin:GetSetting("Rojo_" .. name)

		if savedValue == nil then
			plugin:SetSetting("Rojo_" .. name, defaultValue)
			values[name] = defaultValue
		else
			values[name] = savedValue
		end
	end

	return setmetatable({
		__values = values,
		__plugin = plugin,
		__updateListeners = {},
	}, Settings)
end

function Settings:get(name)
	if defaultSettings[name] == nil then
		error("Invalid setings name " .. tostring(name), 2)
	end

	return self.__values[name]
end

function Settings:set(name, value)
	self.__plugin:SetSetting("Rojo_" .. name, value)
	self.__values[name] = value

	for callback in pairs(self.__updateListeners) do
		callback(name, value)
	end
end

function Settings:onUpdate(newCallback)
	local newListeners = {}
	for callback in pairs(self.__updateListeners) do
		newListeners[callback] = true
	end

	newListeners[newCallback] = true
	self.__updateListeners = newListeners

	return function()
		local newListeners = {}
		for callback in pairs(self.__updateListeners) do
			if callback ~= newCallback then
				newListeners[callback] = true
			end
		end

		self.__updateListeners = newListeners
	end
end

local Context = Roact.createContext(nil)

local StudioProvider = Roact.Component:extend("StudioProvider")

function StudioProvider:init()
	self.settings = Settings.fromPlugin(self.props.plugin)
end

function StudioProvider:render()
	return Roact.createElement(Context.Provider, {
		value = self.settings,
	}, self.props[Roact.Children])
end

local InternalConsumer = Roact.Component:extend("InternalConsumer")

function InternalConsumer:render()
	return self.props.render(self.props.settings)
end

function InternalConsumer:didMount()
	self.disconnect = self.props.settings:onUpdate(function()
		-- Trigger a dummy state update to update the settings consumer.
		self:setState({})
	end)
end

function InternalConsumer:willUnmount()
	self.disconnect()
end

local function with(callback)
	return Roact.createElement(Context.Consumer, {
		render = function(settings)
			return Roact.createElement(InternalConsumer, {
				settings = settings,
				render = callback,
			})
		end,
	})
end

return {
	StudioProvider = StudioProvider,
	with = with,
}
