local Rojo = script:FindFirstAncestor("Rojo")
local Log = require(Rojo.Log)

local exposedSettings = nil

return {
	consumer = function(props)
		exposedSettings = props.settings
		return nil
	end,

	get = function(self, setting: string)
		if exposedSettings == nil then
			Log.warn("Attempted to externally get a setting before portal was initialized")
			return
		end

		return exposedSettings:get(setting)
	end,
}
