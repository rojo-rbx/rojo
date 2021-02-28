local Log = require(script.Parent.Parent.Log)
local PatchSet = require(script.Parent.PatchSet)
local Promise = require(script.Parent.Parent.Promise)

local function tryGetObjects(instanceMap, apiContext, patch)
	assert(PatchSet.validate(patch))

	local unappliedPatch = PatchSet.newEmpty()

	-- GetObjects won't help with anything that failed to remove
	unappliedPatch.removed = patch.removed

	local assetsToRequest = {}
	local receiveCallbacks = {}

	Log.trace("tryGetObjects({:#?})", patch)

	for id, addition in pairs(patch.added) do
		unappliedPatch.added[id] = addition

		table.insert(assetsToRequest, id)
		table.insert(receiveCallbacks, function(newInstance)
			for _, childId in ipairs(addition.Children) do
				local child = instanceMap.fromIds[childId]
				if child == nil then
					Log.warn("Got child ID that wasn't in the instance map: {}", childId)
					continue
				end

				child.Parent = newInstance
			end

			if addition.Parent ~= nil then
				local parent = instanceMap.fromIds[addition.Parent]
				if parent == nil then
					Log.warn("Instance tried to be parented to non-existent parent: {}", addition.Parent)
					return
				end

				local ok, problem = pcall(function()
					newInstance.Parent = parent
				end)

				if not ok then
					Log.warn("GetObjects couldn't parent {} to {}: {}", newInstance, parent, problem)
					return
				end
			end

			unappliedPatch.added[id] = nil
			instanceMap:insert(id, newInstance)
		end)
	end

	-- GetObjects only create instances, we can't update the properties of existing ones.
	-- Instead, just create them again, move their children, and replace the instance.
	for _, update in ipairs(patch.updated) do
		-- If no properties were changed during an update, GetObjects isn't going to do anything that hasn't already been tried
		if next(update.changedProperties) == nil then
			continue
		end

		table.insert(assetsToRequest, update.id)
		table.insert(unappliedPatch.updated, update)

		table.insert(receiveCallbacks, function(newInstance)
			local oldInstance = instanceMap.fromIds[update.id]

			for _, oldChild in ipairs(oldInstance:GetChildren()) do
				oldChild.Parent = newInstance
			end

			local oldParent = oldInstance.Parent
			instanceMap:destroyInstance(oldInstance)

			local ok = pcall(function()
				newInstance.Parent = oldParent
			end)

			if ok then
				table.remove(unappliedPatch.updated, table.find(unappliedPatch.updated, update))
				instanceMap:insert(update.id, newInstance)
			end
		end)
	end

	Log.trace("assetsToRequest = {:?}", assetsToRequest)

	if #assetsToRequest == 0 then
		return Promise.resolve(unappliedPatch)
	end

	return apiContext:createAssets(assetsToRequest):andThen(function(assetId)
		-- GetObjects doesn't yield, it blocks.
		-- There is no way to promisify this unless GetObjectsAsync is opened up.
		local createdAssets = game:GetObjects(assetId)

		for index, assetInstance in ipairs(createdAssets) do
			receiveCallbacks[index](assetInstance)
		end

		return unappliedPatch
	end)
end

return tryGetObjects
