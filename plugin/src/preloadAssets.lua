local ContentProvider = game:GetService("ContentProvider")

local Packages = script.Parent.Parent.Packages
local Log = require(Packages.Log)

local Assets = require(script.Parent.Assets)

local gatherAssetUrlsRecursive
function gatherAssetUrlsRecursive(currentTable, currentUrls)
	currentUrls = currentUrls or {}

	for _, value in pairs(currentTable) do
		if typeof(value) == "string" then
			table.insert(currentUrls, value)
		elseif typeof(value) == "table" then
			gatherAssetUrlsRecursive(value)
		end
	end

	return currentUrls
end

local function preloadAssets()
	local contentUrls = gatherAssetUrlsRecursive(Assets)

	Log.trace("Preloading assets: {:?}", contentUrls)

	coroutine.wrap(function()
		ContentProvider:PreloadAsync(contentUrls)
	end)()
end

return preloadAssets
