--[[
	Persistent plugin settings.
]]

local plugin = plugin or script:FindFirstAncestorWhichIsA("Plugin")
local Rojo = script:FindFirstAncestor("Rojo")

local Log = require(Rojo.Log)

local defaultSettings = {
	openScriptsExternally = false,
	twoWaySync = false,
	showNotifications = true,
	findServeSessions = true,
	playSounds = true,
}

local Settings = {}

Settings._values = table.clone(defaultSettings)
Settings._updateListeners = {}

if plugin then
	for name, defaultValue in pairs(Settings._values) do
		local savedValue = plugin:GetSetting("Rojo_" .. name)

		if savedValue == nil then
			-- plugin:SetSetting hits disc instead of memory, so it can be slow. Spawn so we don't hang.
			task.spawn(plugin.SetSetting, plugin, "Rojo_" .. name, defaultValue)
			Settings._values[name] = defaultValue
		else
			Settings._values[name] = savedValue
		end
	end
	Log.trace("Loaded settings from plugin store")
end

function Settings:get(name)
	if defaultSettings[name] == nil then
		error("Invalid setings name " .. tostring(name), 2)
	end

	return self._values[name]
end

function Settings:set(name, value)
	self._values[name] = value

	if plugin then
		-- plugin:SetSetting hits disc instead of memory, so it can be slow. Spawn so we don't hang.
		task.spawn(plugin.SetSetting, plugin, "Rojo_" .. name, value)
	end

	if self._updateListeners[name] then
		for callback in pairs(self._updateListeners[name]) do
			task.spawn(callback, value)
		end
	end

	Log.trace(string.format("Set setting '%s' to '%s'", name, tostring(value)))
end

function Settings:onChanged(name, callback)
	local listeners = self._updateListeners[name]
	if listeners == nil then
		listeners = {}
		self._updateListeners[name] = listeners
	end
	listeners[callback] = true

	Log.trace(string.format("Added listener for setting '%s' changes", name))

	return function()
		listeners[callback] = nil
		Log.trace(string.format("Removed listener for setting '%s' changes", name))
	end
end

return Settings
