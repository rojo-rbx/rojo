local Fmt = require(script.Parent.Parent.Fmt)
local Log = require(script.Parent.Parent.Log)
local PatchSet = require(script.Parent.PatchSet)
local Promise = require(script.Parent.Parent.Promise)

local function tryGetObjects(instanceMap, apiContext, patch)
	assert(PatchSet.validate(patch))

	local unappliedPatch = PatchSet.newEmpty()

	-- GetObjects won't help with anything that failed to remove
	unappliedPatch.removed = patch.removed
	-- TODO: Implement this
	unappliedPatch.added = patch.added

	local assetsToRequest = {}
	local receiveCallbacks = {}

	-- TODO: added

	-- GetObjects only create instances, we can't update the properties of existing ones.
	-- Instead, just create them again, move their children, and replace the instance.
	for _, update in ipairs(patch.updated) do
		table.insert(assetsToRequest, update.id)
		table.insert(unappliedPatch.updated, update)

		receiveCallbacks[update.id] = function(newInstance)
			table.remove(unappliedPatch.updated, table.find(unappliedPatch.updated, update))

			local oldInstance = instanceMap.fromIds[update.id]

			-- TODO: What if oldInstance is nil?
			for _, oldChild in ipairs(oldInstance:GetChildren()) do
				oldChild.Parent = newInstance
			end

			local oldParent = oldInstance.Parent
			instanceMap:destroyInstance(oldInstance)
			newInstance.Parent = oldParent
		end
	end

	if #assetsToRequest == 0 then
		return Promise.resolve(unappliedPatch)
	end

	return apiContext:createAssets(assetsToRequest):andThen(function(assetId)
		--[[
			The assets Rojo creates that we will be loading is in the following structure:

			Assuming we requested the following IDs:
			- DOGE: A doge mesh named Doge
			- ROCK: A rock mesh named MyPet

			Root: Folder
			* DOGE: Folder
				* Doge: MeshPart
			* ROCK: Folder
				* MyPet: MeshPart
		]]

		-- GetObjects doesn't yield, it blocks.
		-- There is no way to promisify this unless GetObjectsAsync is opened up.
		local createdAssets = game:GetObjects(assetId)[1]

		if createdAssets == nil then
			Log.warn("Request to create assets returned an asset that was empty.")
			return unappliedPatch
		end

		for _, assetFolder in ipairs(createdAssets:GetChildren()) do
			local requestedId = assetFolder.Name

			-- TODO: Does this need to support multi-rooted instances? Probably not?
			local assetInstance = assetFolder:GetChildren()[1]

			receiveCallbacks[requestedId](assetInstance)
			instanceMap:insert(requestedId, assetInstance)
		end

		return unappliedPatch
	end)
end

return tryGetObjects
