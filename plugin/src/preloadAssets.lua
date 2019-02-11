local ContentProvider = game:GetService("ContentProvider")

local Logging = require(script.Parent.Logging)
local Assets = require(script.Parent.Assets)

local function preloadAssets()
	local contentUrls = {}

	for _, sprite in pairs(Assets.Sprites) do
		table.insert(contentUrls, sprite.asset)
	end

	for _, slice in pairs(Assets.Slices) do
		table.insert(contentUrls, slice.asset)
	end

	for _, url in pairs(Assets.Images) do
		table.insert(contentUrls, url)
	end

	Logging.trace("Preloading assets: %s", table.concat(contentUrls, ", "))

	coroutine.wrap(function()
		ContentProvider:PreloadAsync(contentUrls)
	end)()
end

return preloadAssets