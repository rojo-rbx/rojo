--[[
	Methods to turn PatchSets into trees matching the DataModel containing
	the changes and metadata for use in the PatchVisualizer component.
]]

local HttpService = game:GetService("HttpService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Log = require(Packages.Log)

local Types = require(Plugin.Types)
local decodeValue = require(Plugin.Reconciler.decodeValue)
local getProperty = require(Plugin.Reconciler.getProperty)

local function alphabeticalNext(t, state)
	-- Equivalent of the next function, but returns the keys in the alphabetic
	-- order of node names. We use a temporary ordered key table that is stored in the
	-- table being iterated.

	local key = nil
	if state == nil then
		-- First iteration, generate the index
		local orderedIndex, i = table.create(5), 0
		for k in t do
			i += 1
			orderedIndex[i] = k
		end
		table.sort(orderedIndex, function(a, b)
			local nodeA, nodeB = t[a], t[b]
			return (nodeA.name or "") < (nodeB.name or "")
		end)

		t.__orderedIndex = orderedIndex
		key = orderedIndex[1]
	else
		-- Fetch the next value
		for i, orderedState in t.__orderedIndex do
			if orderedState == state then
				key = t.__orderedIndex[i + 1]
				break
			end
		end
	end

	if key then
		return key, t[key]
	end

	-- No more value to return, cleanup
	t.__orderedIndex = nil

	return
end

local function alphabeticalPairs(t)
	-- Equivalent of the pairs() iterator, but sorted
	return alphabeticalNext, t, nil
end

local Tree = {}
Tree.__index = Tree

function Tree.new()
	local tree = {
		idToNode = {},
		ROOT = {
			className = "DataModel",
			name = "ROOT",
			children = {},
		},
	}
	-- Add ROOT to idToNode or it won't be found by getNode since that searches *within* ROOT
	tree.idToNode["ROOT"] = tree.ROOT

	return setmetatable(tree, Tree)
end

-- Iterates over all sub-nodes, depth first
-- node is where to start from, defaults to root
-- depth is used for recursion but can be used to set the starting depth
function Tree:forEach(callback, node, depth)
	depth = depth or 1
	for _, child in alphabeticalPairs(if node then node.children else self.ROOT.children) do
		callback(child, depth)
		if type(child.children) == "table" then
			self:forEach(callback, child, depth + 1)
		end
	end
end

-- Finds a node by id, depth first
-- searchNode is the node to start the search within, defaults to root
function Tree:getNode(id, searchNode)
	if self.idToNode[id] then
		return self.idToNode[id]
	end

	local searchChildren = (searchNode or self.ROOT).children
	for nodeId, node in searchChildren do
		if nodeId == id then
			self.idToNode[id] = node
			return node
		end
		local descendant = self:getNode(id, node)
		if descendant then
			return descendant
		end
	end

	return nil
end

function Tree:doesNodeExist(id)
	return self.idToNode[id] ~= nil
end

-- Adds a node to the tree as a child of the node with id == parent
-- If parent is nil, it defaults to root
-- props must contain id, and cannot contain children or parentId
-- other than those three, it can hold anything
function Tree:addNode(parent, props)
	assert(props.id, "props must contain id")

	parent = parent or "ROOT"

	if self:doesNodeExist(props.id) then
		-- Update existing node
		local node = self:getNode(props.id)
		for k, v in props do
			node[k] = v
		end
		return node
	end

	local node = table.clone(props)
	node.children = {}
	node.parentId = parent

	local parentNode = self:getNode(parent)
	if not parentNode then
		Log.warn("Failed to create node since parent doesnt exist: {}, {}", parent, props)
		return
	end

	parentNode.children[node.id] = node
	self.idToNode[node.id] = node

	return node
end

-- Given a list of ancestor ids in descending order, builds the nodes for them
-- using the patch and instanceMap info
function Tree:buildAncestryNodes(previousId: string?, ancestryIds: { string }, patch, instanceMap)
	-- Build nodes for ancestry by going up the tree
	previousId = previousId or "ROOT"

	for _, ancestorId in ancestryIds do
		local value = instanceMap.fromIds[ancestorId] or patch.added[ancestorId]
		if not value then
			Log.warn("Failed to find ancestor object for " .. ancestorId)
			continue
		end
		self:addNode(previousId, {
			id = ancestorId,
			className = value.ClassName,
			name = value.Name,
			instance = if typeof(value) == "Instance" then value else nil,
		})
		previousId = ancestorId
	end
end

local PatchTree = {}

-- Builds a new tree from a patch and instanceMap
-- uses changeListHeaders in node.changeList
function PatchTree.build(patch, instanceMap, changeListHeaders)
	local tree = Tree.new()

	local knownAncestors = {}

	for _, change in patch.updated do
		local instance = instanceMap.fromIds[change.id]
		if not instance then
			continue
		end

		-- Gather ancestors from existing DOM
		local ancestryIds = {}
		local parentObject = instance.Parent
		local parentId = instanceMap.fromInstances[parentObject]
		local previousId = nil
		while parentObject do
			if knownAncestors[parentId] then
				-- We've already added this ancestor
				previousId = parentId
				break
			end

			table.insert(ancestryIds, 1, parentId)
			knownAncestors[parentId] = true
			parentObject = parentObject.Parent
			parentId = instanceMap.fromInstances[parentObject]
		end

		tree:buildAncestryNodes(previousId, ancestryIds, patch, instanceMap)

		-- Gather detail text
		local changeList, hint = nil, nil
		if next(change.changedProperties) or change.changedName then
			changeList = {}

			local hintBuffer, i = {}, 0
			local function addProp(prop: string, current: any?, incoming: any?, metadata: any?)
				i += 1
				hintBuffer[i] = prop
				changeList[i] = { prop, current, incoming, metadata }
			end

			-- Gather the changes

			if change.changedName then
				addProp("Name", instance.Name, change.changedName)
			end

			for prop, incoming in change.changedProperties do
				local incomingSuccess, incomingValue = decodeValue(incoming, instanceMap)
				local currentSuccess, currentValue = getProperty(instance, prop)

				addProp(
					prop,
					if currentSuccess then currentValue else "[Error]",
					if incomingSuccess then incomingValue else select(2, next(incoming))
				)
			end

			-- Finalize detail values

			-- Trim hint to top 3
			table.sort(hintBuffer)
			if #hintBuffer > 3 then
				hintBuffer = {
					hintBuffer[1],
					hintBuffer[2],
					hintBuffer[3],
					i - 3 .. " more",
				}
			end
			hint = table.concat(hintBuffer, ", ")

			-- Sort changes and add header
			table.sort(changeList, function(a, b)
				return a[1] < b[1]
			end)
			table.insert(changeList, 1, changeListHeaders)
		end

		-- Add this node to tree
		tree:addNode(instanceMap.fromInstances[instance.Parent], {
			id = change.id,
			patchType = "Edit",
			className = instance.ClassName,
			name = instance.Name,
			instance = instance,
			hint = hint,
			changeList = changeList,
		})
	end

	for _, idOrInstance in patch.removed do
		local instance = if Types.RbxId(idOrInstance) then instanceMap.fromIds[idOrInstance] else idOrInstance
		if not instance then
			-- If we're viewing a past patch, the instance is already removed
			-- and we therefore cannot get the tree for it anymore
			continue
		end

		-- Gather ancestors from existing DOM
		-- (note that they may have no ID if they're being removed as unknown)
		local ancestryIds = {}
		local parentObject = instance.Parent
		local parentId = instanceMap.fromInstances[parentObject] or HttpService:GenerateGUID(false)
		local previousId = nil
		while parentObject do
			if knownAncestors[parentId] then
				-- We've already added this ancestor
				previousId = parentId
				break
			end

			instanceMap:insert(parentId, parentObject) -- This ensures we can find the parent later
			table.insert(ancestryIds, 1, parentId)
			knownAncestors[parentId] = true
			parentObject = parentObject.Parent
			parentId = instanceMap.fromInstances[parentObject] or HttpService:GenerateGUID(false)
		end

		tree:buildAncestryNodes(previousId, ancestryIds, patch, instanceMap)

		-- Add this node to tree
		local nodeId = instanceMap.fromInstances[instance] or HttpService:GenerateGUID(false)
		instanceMap:insert(nodeId, instance)
		tree:addNode(instanceMap.fromInstances[instance.Parent], {
			id = nodeId,
			patchType = "Remove",
			className = instance.ClassName,
			name = instance.Name,
			instance = instance,
		})
	end

	for id, change in patch.added do
		-- Gather ancestors from existing DOM or future additions
		local ancestryIds = {}
		local parentId = change.Parent
		local parentData = patch.added[parentId]
		local parentObject = instanceMap.fromIds[parentId]
		local previousId = nil
		while parentId do
			if knownAncestors[parentId] then
				-- We've already added this ancestor
				previousId = parentId
				break
			end

			table.insert(ancestryIds, 1, parentId)
			knownAncestors[parentId] = true
			parentId = nil

			if parentData then
				-- object is parented to an instance that does not exist yet
				parentId = parentData.Parent
				parentData = patch.added[parentId]
				parentObject = instanceMap.fromIds[parentId]
			elseif parentObject then
				-- object is parented to an instance that exists
				parentObject = parentObject.Parent
				parentId = instanceMap.fromInstances[parentObject]
				parentData = patch.added[parentId]
			end
		end

		tree:buildAncestryNodes(previousId, ancestryIds, patch, instanceMap)

		-- Gather detail text
		local changeList, hint = nil, nil
		if next(change.Properties) then
			changeList = {}

			local hintBuffer, i = {}, 0
			for prop, incoming in change.Properties do
				i += 1
				hintBuffer[i] = prop

				local success, incomingValue = decodeValue(incoming, instanceMap)
				if success then
					table.insert(changeList, { prop, "N/A", incomingValue })
				else
					table.insert(changeList, { prop, "N/A", select(2, next(incoming)) })
				end
			end

			-- Finalize detail values

			-- Trim hint to top 3
			table.sort(hintBuffer)
			if #hintBuffer > 3 then
				hintBuffer = {
					hintBuffer[1],
					hintBuffer[2],
					hintBuffer[3],
					i - 3 .. " more",
				}
			end
			hint = table.concat(hintBuffer, ", ")

			-- Sort changes and add header
			table.sort(changeList, function(a, b)
				return a[1] < b[1]
			end)
			table.insert(changeList, 1, changeListHeaders)
		end

		-- Add this node to tree
		tree:addNode(change.Parent, {
			id = change.Id,
			patchType = "Add",
			className = change.ClassName,
			name = change.Name,
			hint = hint,
			changeList = changeList,
			instance = instanceMap.fromIds[id],
		})
	end

	return tree
end

-- Creates a deep copy of a tree for immutability purposes in Roact
function PatchTree.clone(tree)
	if not tree then
		return
	end

	local newTree = Tree.new()
	tree:forEach(function(node)
		newTree:addNode(node.parentId, table.clone(node))
	end)

	return newTree
end

-- Updates the metadata of a tree with the unapplied patch and currently existing instances
-- Builds a new tree from the data if one isn't provided
-- Always returns a new tree for immutability purposes in Roact
function PatchTree.updateMetadata(tree, patch, instanceMap, unappliedPatch)
	if tree then
		tree = PatchTree.clone(tree)
	else
		tree = PatchTree.build(patch, instanceMap)
	end

	-- Update isWarning metadata
	for _, failedChange in unappliedPatch.updated do
		local node = tree:getNode(failedChange.id)
		if node then
			node.isWarning = true
			Log.trace("Marked node as warning: {} {}", node.id, node.name)

			if node.changeList then
				for _, change in node.changeList do
					if failedChange.changedProperties[change[1]] then
						Log.trace("  Marked property as warning: {}", change[1])
						if change[4] == nil then
							change[4] = {}
						end
						change[4].isWarning = true
					end
				end
			end
		end
	end

	-- Update if instances exist
	tree:forEach(function(node)
		if node.instance then
			if node.instance.Parent == nil and node.instance ~= game then
				-- This instance has been removed
				Log.trace("Removed instance from node: {} {}", node.id, node.name)
				node.instance = nil
			end
		else
			-- This instance may have been added
			node.instance = instanceMap.fromIds[node.id]
			if node.instance then
				Log.trace("Added instance to node: {} {}", node.id, node.name)
			end
		end
	end)

	return tree
end

return PatchTree
