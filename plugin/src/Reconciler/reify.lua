--[[
	"Reifies" a virtual DOM, constructing a real DOM with the same shape.
]]

local Rojo = script:FindFirstAncestor("Rojo")

local invariant = require(script.Parent.Parent.invariant)
local PatchSet = require(script.Parent.Parent.PatchSet)
local setProperty = require(script.Parent.setProperty)
local decodeValue = require(script.Parent.decodeValue)

local Log = require(Rojo.Packages.Log)

local reifyInner, fetchUnapplied, applyDeferredRefs

local function reify(instanceMap, virtualInstances, rootId, parentInstance, serveSession)
	-- Create an empty patch that will be populated with any parts of this reify
	-- that could not happen, like instances that couldn't be created and
	-- properties that could not be assigned.
	local unappliedPatch = PatchSet.newEmpty()

	-- Contains a list of all of the ref properties that we'll need to assign
	-- after all instances are created. We apply refs in a second pass, after
	-- we create as many instances as we can, so that we ensure that referents
	-- can be mapped to instances correctly.
	local deferredRefs = {}

	reifyInner(instanceMap, virtualInstances, rootId, parentInstance, unappliedPatch, deferredRefs)
	applyDeferredRefs(instanceMap, deferredRefs, unappliedPatch)

	return unappliedPatch
end

--[[
	Add the given ID and all of its descendants in virtualInstances to the given
	PatchSet, marked for addition.
]]
local function addAllToPatch(patchSet, virtualInstances, id)
	local virtualInstance = virtualInstances[id]
	patchSet.added[id] = virtualInstance

	for _, childId in ipairs(virtualInstance.Children) do
		addAllToPatch(patchSet, virtualInstances, childId)
	end
end

--[[
	Inner function that defines the core routine.
]]
function reifyInner(instanceMap, virtualInstances, id, parentInstance, unappliedPatch, deferredRefs)
	local virtualInstance = virtualInstances[id]

	if virtualInstance == nil then
		invariant("Cannot reify an instance not present in virtualInstances\nID: {}", id)
	end

	-- Instance.new can fail if we're passing in something that can't be
	-- created, like a service, something enabled with a feature flag, or
	-- something that requires higher security than we have.
	local ok, instance = pcall(Instance.new, virtualInstance.ClassName)

	if not ok then
		addAllToPatch(unappliedPatch, virtualInstances, id)
		return
	end

	-- TODO: Can this fail? Previous versions of Rojo guarded against this, but
	-- the reason why was uncertain.
	instance.Name = virtualInstance.Name

	-- Track all of the properties that we've failed to assign to this instance.
	local unappliedProperties = {}

	for propertyName, virtualValue in pairs(virtualInstance.Properties) do
		-- Because refs may refer to instances that we haven't constructed yet,
		-- we defer applying any ref properties until all instances are created.
		if next(virtualValue) == "Ref" then
			table.insert(deferredRefs, {
				id = id,
				instance = instance,
				propertyName = propertyName,
				virtualValue = virtualValue,
			})
			continue
		end

		local ok, value = decodeValue(virtualValue, instanceMap)
		if not ok then
			unappliedProperties[propertyName] = virtualValue
			continue
		end

		local ok = setProperty(instance, propertyName, value)
		if not ok then
			unappliedProperties[propertyName] = virtualValue
		end
	end

	-- If there were any properties that we failed to assign, push this into our
	-- unapplied patch as an update that would need to be applied.
	if next(unappliedProperties) ~= nil then
		table.insert(unappliedPatch.updated, {
			id = id,
			changedProperties = unappliedProperties,
		})
	end

	for _, childId in ipairs(virtualInstance.Children) do
		reifyInner(instanceMap, virtualInstances, childId, instance, unappliedPatch, deferredRefs)
	end

	instance.Parent = parentInstance
	instanceMap:insert(id, instance)
end

function applyDeferredRefs(instanceMap, deferredRefs, unappliedPatch)
	local function markFailed(id, propertyName, virtualValue)
		-- If there is already an updated entry in the unapplied patch for this
		-- ref, use the existing one. This could match other parts of the
		-- instance that failed to be created, or even just other refs that
		-- failed to apply.
		--
		-- This is important for instances like selectable GUI objects, which
		-- have many similar referent properties.
		for _, existingUpdate in ipairs(unappliedPatch.updated) do
			if existingUpdate.id == id then
				existingUpdate.changedProperties[propertyName] = virtualValue
				return
			end
		end

		-- We didn't find an existing entry that matched, so push a new entry
		-- into our unapplied patch.
		table.insert(unappliedPatch.updated, {
			id = id,
			changedProperties = {
				[propertyName] = virtualValue,
			},
		})
	end

	for _, entry in ipairs(deferredRefs) do
		local _, refId = next(entry.virtualValue)

		if refId == nil then
			continue
		end

		local targetInstance = instanceMap.fromIds[refId]
		if targetInstance == nil then
			markFailed(entry.id, entry.propertyName, entry.virtualValue)
			continue
		end

		local ok = setProperty(entry.instance, entry.propertyName, targetInstance)
		if not ok then
			markFailed(entry.id, entry.propertyName, entry.virtualValue)
		end
	end
end

function fetchInstances(idList, instanceMap, serveSession)
	return serveSession.__apiContext:fetch(idList)
		:andThen(function(body: {sessionId: string, path: string})
			-- The endpoint `api/fetech/idlist` returns a table that contains
			-- `sessionId` and `path`. The value of `path` is the name of a
			-- file in the Content folder that may be loaded via `GetObjects`.
			Log.debug("Loading assets for {}", idList)
			local objects = game:GetObjects("rbxasset://" .. body.path)
			-- `ReferentMap` is a folder that contains nothing but
			-- ObjectValues which will be named after entries in `instanceMap`
			-- and have their `Value` property point towards a new Instance
			-- that it can be swapped out with. In turn, `reified` is a
			-- container for all of the objects pointed to by those instances.
			local map = objects[1]:FindFirstChild("ReferentMap")
			local reified = objects[1]:FindFirstChild("Reified")
			if map == nil then
				invariant("The fetch endpoint returned a malformed folder: missing ReferentMap")
			end
			if reified == nil then
				invariant("The fetch endpoint returned a malformed folder: missing Reified")
			end
			for _, entry in map:GetChildren() do
				if entry:IsA("ObjectValue") then
					local key, value = entry.Name, entry.Value
					if value == nil or value.Parent ~= reified then
						invariant("ReferentMap contained entry {} that was parented to an outside source", key)
					else
						-- This could be a problem if Roblox ever supports
						-- parallel access to the DataModel but right now,
						-- there's no way this results in a data race.
						local oldInstance: Instance = instanceMap.fromIds[key]
						instanceMap:insert(key, value)
						Log.debug("Swapping Instance {} out", oldInstance:GetFullName())

						local oldParent = oldInstance.Parent
						local children = oldInstance:GetChildren()
						for _, child in children do
							child.Parent = value
						end
						value.Parent = oldParent

						-- So long and thanks for all the fish :-)
						oldInstance:Destroy()
					end
				else
					invariant("ReferentMap entry `{}` was a `{}` and not an ObjectValue", entry.Name, entry.ClassName)
				end
			end

		end)
end

return reify
