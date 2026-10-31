local ContentProvider = game:GetService("ContentProvider")

local Packages = script.Parent.Parent.Packages
local Log = require(Packages.Log)

local Assets = require(script.Parent.Assets)

local function gatherAssetUrlsRecursive(currentTable, typeCallbacks)
	for key, value in currentTable do
		const callback = typeCallbacks[type(value)]
		if type(callback) == `function` then
			callback(value, key)
		end
	end
end

local function preloadAssets()
	local contentUrls = {}
	local soundInstances = {}
	
	const function addToContentUrls(value)
		table.insert(contentUrls, value)
	end
	
	local gatherAssetUrlsRules
	gatherAssetUrlsRules = {
		[`table`] = function(value)
			gatherAssetUrlsRecursive(value, gatherAssetUrlsRules)
		end,
		[`string`] = addToContentUrls,
	}
	
	local gatherSoundUrlsRules
	gatherSoundUrlsRules = {
		[`table`] = function(value)
			gatherAssetUrlsRecursive(value, gatherSoundUrlsRules)
		end,
		[`string`] = function(value)
			const sound = Instance.new(`Sound`)
			sound.SoundId = value
			table.insert(soundInstances, sound)
			table.insert(contentUrls, sound)
		end,
	}
	
	gatherAssetUrlsRecursive(Assets, {
		[`table`] = function(value, key)
			if key ~= `Sounds` then -- an exception!
				gatherAssetUrlsRecursive(value, gatherAssetUrlsRules)
				return
			end
			gatherAssetUrlsRecursive(value, gatherSoundUrlsRules)
		end,
		[`string`] = addToContentUrls,
	})

	Log.trace("Preloading assets: {:?}", contentUrls)

	coroutine.wrap(function()
		ContentProvider:PreloadAsync(contentUrls)
		for _, sound in soundInstances do
			sound:Destroy()
		end
	end)()
end

return preloadAssets