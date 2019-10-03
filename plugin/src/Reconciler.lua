--[[
	This module defines the meat of the Rojo plugin and how it manages tracking
	and mutating the Roblox DOM.
]]

local RbxDom = require(script.Parent.Parent.RbxDom)
local t = require(script.Parent.Parent.t)

local InstanceMap = require(script.Parent.InstanceMap)
local Types = require(script.Parent.Types)
local setCanonicalProperty = require(script.Parent.setCanonicalProperty)

--[[
	This interface represents either a patch created by the hydrate method, or a
	patch returned from the API.

	This type should be a subset of Types.ApiInstanceUpdate.
]]
local IPatch = t.interface({
	removed = t.array(t.union(Types.RbxId, t.Instance)),
	added = t.map(Types.RbxId, Types.ApiInstance),
	updated = t.array(Types.ApiInstanceUpdate),
})

--[[
	Attempt to safely set the parent of an instance.

	This function will always succeed, even if the actual set failed. This is
	important for some types like services that will throw even if their current
	parent is already set to the requested parent.

	TODO: See if we can eliminate this by being more nuanced with property
	assignment?
]]
local function safeSetParent(instance, newParent)
	pcall(function()
		instance.Parent = newParent
	end)
end

--[[
	Similar to setting Parent, some instances really don't like being renamed.

	TODO: Should we be throwing away these results or can we be more careful?
]]
local function safeSetName(instance, name)
	pcall(function()
		instance.Name = name
	end)
end

local Reconciler = {}
Reconciler.__index = Reconciler

function Reconciler.new()
	local self = {
		-- Tracks all of the instances known by the reconciler by ID.
		__instanceMap = InstanceMap.new(),
	}

	return setmetatable(self, Reconciler)
end

--[[
	See Reconciler:__hydrateInternal().
]]
function Reconciler:hydrate(apiInstances, id, instance)
	local hydratePatch = {
		removed = {},
		added = {},
		updated = {},
	}

	self:__hydrateInternal(apiInstances, id, instance, hydratePatch)

	return hydratePatch
end

--[[
	Transforms a value encoded by rbx_dom_weak on the server side into a value
	usable by Rojo's reconciler, potentially using RbxDom.
]]
function Reconciler:__decodeApiValue(apiValue)
	assert(Types.ApiValue(apiValue))

	-- Refs are represented as IDs in the same space that Rojo's protocol uses.
	if apiValue.Type == "Ref" then
		-- TODO: This ref could be pointing at an instance we haven't created
		-- yet!

		return self.__instanceMap.fromIds[apiValue.Value]
	end

	local success, decodedValue = RbxDom.EncodedValue.decode(apiValue)

	if not success then
		error(decodedValue, 2)
	end

	return decodedValue
end

--[[
	Constructs an instance from an ApiInstance without any of its children.
]]
local reifySingleInstanceSchema = Types.ifEnabled(t.tuple(
	Types.ApiInstance
))
function Reconciler:__reifySingleInstance(apiInstance)
	assert(reifySingleInstanceSchema(apiInstance))

	-- Instance.new can fail if we're passing in something that can't be
	-- created, like a service, something enabled with a feature flag, or
	-- something that requires higher security than we have.
	local ok, instance = pcall(Instance.new, apiInstance.ClassName)
	if not ok then
		return false, instance
	end

	-- TODO: When can setting Name fail here?
	safeSetName(instance, apiInstance.Name)

	for key, value in pairs(apiInstance.Properties) do
		setCanonicalProperty(instance, key, self:__decodeApiValue(value))
	end

	return instance
end

--[[
	Construct an instance and all of its descendants, parent it to the given
	instance, and insert it into the reconciler's internal state.
]]
local reifyInstanceSchema = Types.ifEnabled(t.tuple(
	t.map(Types.RbxId, Types.VirtualInstance),
	Types.RbxId,
	t.Instance
))
function Reconciler:__reifyInstance(apiInstances, id, parentInstance)
	assert(reifyInstanceSchema(apiInstances, id, parentInstance))

	local apiInstance = apiInstances[id]
	local ok, instance = self:__reifySingleInstance(apiInstance)

	-- TODO: Propagate this error upward to handle it elsewhere?
	if not ok then
		error(("Couldn't create an instance of type %q, a child of %s"):format(
			apiInstance.ClassName,
			parentInstance:GetFullName()
		))
	end

	for _, childId in ipairs(apiInstance.Children) do
		self:__reify(apiInstances, childId, instance)
	end

	safeSetParent(instance, parentInstance)
	self.__instanceMap:insert(id, instance)

	return instance
end

--[[
	Populates the reconciler's internal state, maps IDs to instances that the
	Rojo plugin knows about, and generates a patch that would update the Roblox
	tree to match Rojo's view of the tree.
]]
local hydrateSchema = Types.ifEnabled(t.tuple(
	t.map(Types.RbxId, Types.VirtualInstance),
	Types.RbxId,
	t.Instance,
	IPatch
))
function Reconciler:__hydrateInternal(apiInstances, id, instance, hydratePatch)
	assert(hydrateSchema(apiInstances, id, instance, hydratePatch))

	local apiInstance = apiInstances[id]

	local function markIdAdded(id)
		local apiInstance = apiInstances[id]
		hydratePatch.added[id] = apiInstance

		for _, childId in ipairs(apiInstance.Children) do
			markIdAdded(childId)
		end
	end

	-- TODO: Measure differences in properties and add them to
	-- hydratePatch.updates

	local existingChildren = instance:GetChildren()

	-- For each existing child, we'll track whether it's been paired with an
	-- instance that the Rojo server knows about.
	local isExistingChildVisited = {}
	for i = 1, #existingChildren do
		isExistingChildVisited[i] = false
	end

	for _, childId in ipairs(apiInstance.Children) do
		local apiChild = apiInstances[childId]

		local childInstance

		for childIndex, instance in ipairs(existingChildren) do
			if not isExistingChildVisited[childIndex] then
				-- We guard accessing Name and ClassName in order to avoid
				-- tripping over children of DataModel that Rojo won't have
				-- permissions to access at all.
				local ok, name, className = pcall(function()
					return instance.Name, instance.ClassName
				end)

				-- This rule is very conservative and could be loosened in the
				-- future, or more heuristics could be introduced.
				if ok and name == apiChild.Name and className == apiChild.ClassName then
					childInstance = instance
					isExistingChildVisited[childIndex] = true
					break
				end
			end
		end

		if childInstance ~= nil then
			-- We found an instance that matches the instance from the API, yay!
			self:__hydrateInternal(apiInstances, childId, childInstance, hydratePatch)
		else
			markIdAdded(childId)
		end
	end

	-- Any unvisited children at this point aren't known by Rojo and we can
	-- destroy them unless the user has explicitly asked us to preserve children
	-- of this instance.
	local shouldClearUnknown = self:__shouldClearUnknownChildren(apiInstance)
	if shouldClearUnknown then
		for childIndex, visited in ipairs(isExistingChildVisited) do
			if not visited then
				table.insert(hydratePatch.removedInstances, existingChildren[childIndex])
			end
		end
	end
end

function Reconciler:__shouldClearUnknownChildren(apiInstance)
	if apiInstance.Metadata ~= nil then
		return not apiInstance.Metadata.ignoreUnknownInstances
	else
		return true
	end
end

return Reconciler