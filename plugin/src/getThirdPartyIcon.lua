local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Assets = require(Plugin.Assets)

return function(source: string?)
	if not source then
		return Assets.Images.ThirdPartyPlugin
	end

	local sourceId = string.match(source, "cloud_(%d+)")
	if not sourceId then
		return Assets.Images.ThirdPartyPlugin
	end

	return string.format("rbxthumb://type=Asset&id=%s&w=150&h=150", sourceId)
end
