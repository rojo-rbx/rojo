local ContentProvider = game:GetService("ContentProvider")

local Packages = script.Parent.Parent.Packages
local Log = require(Packages.Log)

local Assets = require(script.Parent.Assets)

local gatherAssetUrlsRecursive
function gatherAssetUrlsRecursive(currentTable, currentUrls)
	currentUrls = currentUrls or {}

	for _, value in currentTable do
		if typeof(value) == "string" then
			table.insert(currentUrls, value)
		elseif typeof(value) == "table" then
			gatherAssetUrlsRecursive(value, currentUrls)
		end
	end

	return currentUrls
end

local function preloadAssets()
	local contentUrls = {}
	local soundUrls = {}

	for key, value in Assets do
		if key == "Sounds" then
			gatherAssetUrlsRecursive(value, soundUrls)
		elseif typeof(value) == "string" then
			table.insert(contentUrls, value)
		elseif typeof(value) == "table" then
			gatherAssetUrlsRecursive(value, contentUrls)
		end
	end

	Log.trace("Preloading assets: {:?}", contentUrls)
	Log.trace("Preloading sounds: {:?}", soundUrls)

	-- PreloadAsync fetches a bare string as an image, so sounds go in as Sound
	-- instances, which load their SoundId as audio.
	local preloadList = table.clone(contentUrls)
	local sounds = {}
	for _, soundId in soundUrls do
		local sound = Instance.new("Sound")
		sound.SoundId = soundId
		table.insert(sounds, sound)
		table.insert(preloadList, sound)
	end

	coroutine.wrap(function()
		ContentProvider:PreloadAsync(preloadList)
		for _, sound in sounds do
			sound:Destroy()
		end
	end)()
end

return preloadAssets
