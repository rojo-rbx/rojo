local HttpService = game:GetService("HttpService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Log = require(Packages.Log)

local Types = require(Plugin.Types)
local PatchSet = require(Plugin.PatchSet)
local decodeValue = require(Plugin.Reconciler.decodeValue)
local getProperty = require(Plugin.Reconciler.getProperty)

local BorderedContainer = require(Plugin.App.Components.BorderedContainer)
local VirtualScroller = require(Plugin.App.Components.VirtualScroller)

local e = Roact.createElement

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

local function Tree()
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

	function tree:getNode(id, target)
		if self.idToNode[id] then
			return self.idToNode[id]
		end

		for nodeId, node in target or tree.ROOT.children do
			if nodeId == id then
				self.idToNode[id] = node
				return node
			end
			local descendant = self:getNode(id, node.children)
			if descendant then
				return descendant
			end
		end

		return nil
	end

	function tree:addNode(parent, props)
		parent = parent or "ROOT"

		local node = self:getNode(props.id)
		if node then
			for k, v in props do
				node[k] = v
			end
			return node
		end

		node = table.clone(props)
		node.children = {}

		local parentNode = self:getNode(parent)
		if not parentNode then
			Log.warn("Failed to create node since parent doesnt exist: {}, {}", parent, props)
			return
		end

		parentNode.children[node.id] = node
		self.idToNode[node.id] = node

		return node
	end

	function tree:buildAncestryNodes(ancestry, patch, instanceMap)
		-- Build nodes for ancestry by going up the tree
		local previousId = "ROOT"
		for _, ancestorId in ancestry do
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

	return tree
end

local function findUnappliedPropsForId(unappliedPatch, id)
	for _, change in unappliedPatch.updated do
		if change.id == id then
			return change.changedProperties or {}
		end
	end
	return {}
end

local DomLabel = require(script.DomLabel)

local PatchVisualizer = Roact.Component:extend("PatchVisualizer")

function PatchVisualizer:init()
	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))

	self.updateEvent = Instance.new("BindableEvent")
end

function PatchVisualizer:willUnmount()
	self.updateEvent:Destroy()
end

function PatchVisualizer:shouldUpdate(nextProps)
	local currentPatch, nextPatch = self.props.patch, nextProps.patch

	return not PatchSet.isEqual(currentPatch, nextPatch)
end

function PatchVisualizer:buildTree(patch, unappliedPatch, instanceMap)
	local tree = Tree()

	for _, change in patch.updated do
		local instance = instanceMap.fromIds[change.id]
		if not instance then
			continue
		end

		-- Gather ancestors from existing DOM
		local ancestry = {}
		local parentObject = instance.Parent
		local parentId = instanceMap.fromInstances[parentObject]
		while parentObject do
			table.insert(ancestry, 1, parentId)
			parentObject = parentObject.Parent
			parentId = instanceMap.fromInstances[parentObject]
		end

		tree:buildAncestryNodes(ancestry, patch, instanceMap)

		-- Gather detail text
		local changeList, hint = nil, nil
		if next(change.changedProperties) or change.changedName then
			local unappliedChanges = findUnappliedPropsForId(unappliedPatch, change.id)

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
					if incomingSuccess then incomingValue else next(incoming),
					{
						isWarning = unappliedChanges[prop] ~= nil
					}
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
			table.insert(changeList, 1, { "Property", "Current", "Incoming" })
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
		local ancestry = {}
		local parentObject = instance.Parent
		local parentId = instanceMap.fromInstances[parentObject] or HttpService:GenerateGUID(false)
		while parentObject do
			instanceMap:insert(parentId, parentObject)
			table.insert(ancestry, 1, parentId)
			parentObject = parentObject.Parent
			parentId = instanceMap.fromInstances[parentObject] or HttpService:GenerateGUID(false)
		end

		tree:buildAncestryNodes(ancestry, patch, instanceMap)

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
		local ancestry = {}
		local parentId = change.Parent
		local parentData = patch.added[parentId]
		local parentObject = instanceMap.fromIds[parentId]
		while parentId do
			table.insert(ancestry, 1, parentId)
			parentId = nil

			if parentData then
				parentId = parentData.Parent
				parentData = patch.added[parentId]
				parentObject = instanceMap.fromIds[parentId]
			elseif parentObject then
				parentObject = parentObject.Parent
				parentId = instanceMap.fromInstances[parentObject]
				parentData = patch.added[parentId]
			end
		end

		tree:buildAncestryNodes(ancestry, patch, instanceMap)

		-- Gather detail text
		local changeList, hint = nil, nil
		if next(change.Properties) then
			local unappliedChanges = findUnappliedPropsForId(unappliedPatch, change.Id)

			changeList = {}

			local hintBuffer, i = {}, 0
			for prop, incoming in change.Properties do
				i += 1
				hintBuffer[i] = prop

				local success, incomingValue = decodeValue(incoming, instanceMap)
				if success then
					table.insert(changeList, { prop, "N/A", incomingValue, {
						isWarning = unappliedChanges[prop] ~= nil
					} })
				else
					table.insert(changeList, { prop, "N/A", next(incoming), {
						isWarning = unappliedChanges[prop] ~= nil
					} })
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
			table.insert(changeList, 1, { "Property", "Current", "Incoming" })
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

function PatchVisualizer:render()
	local patch = self.props.patch or PatchSet.newEmpty()
	local unappliedPatch = self.props.unappliedPatch or PatchSet.newEmpty()
	local instanceMap = self.props.instanceMap

	local tree = self:buildTree(patch, unappliedPatch, instanceMap)

	-- Recusively draw tree
	local scrollElements, elementHeights = {}, {}
	local function drawNode(node, depth)
		local elementHeight, setElementHeight = Roact.createBinding(30)
		table.insert(elementHeights, elementHeight)
		table.insert(
			scrollElements,
			e(DomLabel, {
				columnVisibility = self.props.columnVisibility,
				updateEvent = self.updateEvent,
				elementHeight = elementHeight,
				setElementHeight = setElementHeight,
				patchType = node.patchType,
				className = node.className,
				isWarning = next(findUnappliedPropsForId(unappliedPatch, node.id)) ~= nil,
				instance = node.instance,
				name = node.name,
				hint = node.hint,
				changeList = node.changeList,
				depth = depth,
				transparency = self.props.transparency,
			})
		)

		for _, childNode in alphabeticalPairs(node.children) do
			drawNode(childNode, depth + 1)
		end
	end
	for _, node in alphabeticalPairs(tree.ROOT.children) do
		drawNode(node, 0)
	end

	return e(BorderedContainer, {
		transparency = self.props.transparency,
		size = self.props.size,
		position = self.props.position,
		layoutOrder = self.props.layoutOrder,
	}, {
		VirtualScroller = e(VirtualScroller, {
			size = UDim2.new(1, 0, 1, 0),
			transparency = self.props.transparency,
			count = #scrollElements,
			updateEvent = self.updateEvent.Event,
			render = function(i)
				return scrollElements[i]
			end,
			getHeightBinding = function(i)
				return elementHeights[i]
			end,
		}),
	})
end

return PatchVisualizer
