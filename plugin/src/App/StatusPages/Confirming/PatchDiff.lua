local HttpService = game:GetService("HttpService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Log = require(Rojo.Log)
local Roact = require(Rojo.Roact)

local decodeValue = require(Plugin.Reconciler.decodeValue)

local BorderedContainer = require(Plugin.App.Components.BorderedContainer)
local ScrollingFrame = require(Plugin.App.Components.ScrollingFrame)

local e = Roact.createElement

local function alphabeticalNext(t, state)
    -- Equivalent of the next function, but returns the keys in the alphabetic
    -- order of node names. We use a temporary ordered key table that is stored in the
    -- table being iterated.

    local key = nil
    if state == nil then
        -- First iteration, generate the index
		local orderedIndex, i = {}, 0
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
                key = t.__orderedIndex[i+1]
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

local DomLabel = require(script.Parent.DomLabel)

local PatchDiff = Roact.Component:extend("PatchDiff")

function PatchDiff:init()
	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))
end

function PatchDiff:render()
	local patch = self.props.confirmData.patch
	local instanceMap = self.props.confirmData.instanceMap

	local tree = {
		root = {
			className = "DataModel",
			name = "root",
			children = {},
		}
	}
	local domLabels = {}

	local function getNode(id, target)
		for nodeId, node in target or tree do
			if nodeId == id then
				return node
			end
			local descendant = getNode(id, node.children)
			if descendant then return descendant end
		end
	end

	local function addNode(parent, props)
		parent = parent or "root"

		local node = getNode(props.id)
		if node then
			for k, v in props do
				node[k] = v
			end
			return node
		end

		node = table.clone(props)
		node.children = {}

		local parentNode = getNode(parent)
		if not parentNode then
			Log.warn("Failed to create node since parent doesnt exist", parent, props)
			return
		end

		parentNode.children[node.id] = node

		return node
	end

	local function buildAncestryNodes(ancestry)
		-- Build nodes for ancestry
		local previousId = "root"
		for _, ancestorId in ancestry do
			local value = instanceMap.fromIds[ancestorId] or patch.added[ancestorId]
			addNode(previousId, {
				id = ancestorId,
				className = value.ClassName,
				name = value.Name,
			})
			previousId = ancestorId
		end
	end

	for _, change in patch.updated do
		local instance = instanceMap.fromIds[change.id]
		if not instance then continue end

		-- Gather ancestors from existing DOM
		local ancestry = {}
		local parentObject = instance.Parent
		local parentId = instanceMap.fromInstances[parentObject]
		while parentObject do
			table.insert(ancestry, 1, parentId)
			parentObject = parentObject.Parent
			parentId = instanceMap.fromInstances[parentObject]
		end

		buildAncestryNodes(ancestry)

		-- Gather detail text
		local diffTable = {
			{"Property", "Current", "Incoming"},
		}

		local hint, i = {}, 0
		for prop, incoming in change.changedProperties do
			i += 1
			if i < 5 then
				hint[i] = prop
			elseif i == 5 then
				hint[i] = "..."
			end

			local success, incomingValue = decodeValue(incoming)
			if success then
				table.insert(diffTable, {prop, instance[prop], incomingValue})
			else
				table.insert(diffTable, {prop, instance[prop], next(incoming)})
			end
		end

		-- Add this node to tree
		addNode(instanceMap.fromInstances[instance.Parent], {
			id = change.id,
			patchType = "Edit",
			className = instance.ClassName,
			name = instance.Name,
			hint = table.concat(hint, ", "),
			diffTable = #diffTable > 1 and diffTable or nil,
		})
	end

	for _, instance in self.props.confirmData.patch.removed do
		-- Gather ancestors from existing DOM, note that they may have no ID
		local ancestry = {}
		local parentObject = instance.Parent
		local parentId = instanceMap.fromInstances[parentObject] or HttpService:GenerateGUID(false)
		while parentObject do
			instanceMap:insert(parentId, parentObject)
			table.insert(ancestry, 1, parentId)
			parentObject = parentObject.Parent
			parentId = instanceMap.fromInstances[parentObject] or HttpService:GenerateGUID(false)
		end

		buildAncestryNodes(ancestry)

		-- Add this node to tree
		local nodeId = instanceMap.fromInstances[instance] or HttpService:GenerateGUID(false)
		instanceMap:insert(nodeId, instance)
		addNode(instanceMap.fromInstances[instance.Parent], {
			id = nodeId,
			patchType = "Remove",
			className = instance.ClassName,
			name = instance.Name,
		})
	end

	for _, change in patch.added do
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

		buildAncestryNodes(ancestry)

		-- Gather detail text
		local hint, i = {}, 0
		for prop in change.Properties do
			i += 1
			if i >= 5 then
				hint[i] = "..."
				break
			end
			hint[i] = prop
		end

		-- Add this node to tree
		addNode(change.Parent, {
			id = change.Id,
			patchType = "Add",
			className = change.ClassName,
			name = change.Name,
			hint = table.concat(hint, ", "),
		})
	end

	-- Recusively draw tree
	local function drawNode(node, depth)
		table.insert(domLabels, e(DomLabel, {
			patchType = node.patchType,
			className = node.className,
			name = node.name,
			hint = node.hint,
			diffTable = node.diffTable,
			depth = depth,
			transparency = self.props.transparency,
		}))

		for _, childNode in alphabeticalPairs(node.children) do
			drawNode(childNode, depth + 1)
		end
	end
	for _, node in alphabeticalPairs(tree.root.children) do
		drawNode(node, 0)
	end

	return e(BorderedContainer, {
		transparency = self.props.transparency,
		size = UDim2.new(1, 0, 1, -150),
		layoutOrder = self.props.layoutOrder,
	}, {
		e(ScrollingFrame, {
			size = UDim2.new(1, 0, 1, 0),
			contentSize = self.contentSize,
			transparency = self.props.transparency,
		}, {
			Layout = e("UIListLayout", {
				FillDirection = Enum.FillDirection.Vertical,
				SortOrder = Enum.SortOrder.LayoutOrder,
				HorizontalAlignment = Enum.HorizontalAlignment.Right,
				VerticalAlignment = Enum.VerticalAlignment.Top,

				[Roact.Change.AbsoluteContentSize] = function(object)
					self.setContentSize(object.AbsoluteContentSize)
				end,
			}),
			table.unpack(domLabels),
		})
	})
end

return PatchDiff
