local HttpService = game:GetService("HttpService")
local StudioService = game:GetService("StudioService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Log = require(Rojo.Log)
local Roact = require(Rojo.Roact)
local Assets = require(Plugin.Assets)

local Theme = require(Plugin.App.Theme)
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

local function domLabel(props)
	return Theme.with(function(theme)
		local iconProps = StudioService:GetClassIcon(props.className)
		local lineGuides = {}
		for i=1, props.depth or 0 do
			table.insert(lineGuides, e("Frame", {
				Name = "Line_"..i,
				Size = UDim2.new(0, 2, 1, 2),
				Position = UDim2.new(0, (20*i) + 15, 0, -1),
				BorderSizePixel = 0,
				BackgroundTransparency = props.transparency,
				BackgroundColor3 = theme.BorderedContainer.BorderColor,
			}))
		end

		local indent = (props.depth or 0) * 20 + 25

		return e("Frame", {
			Name = "Change",
			BackgroundColor3 = if props.patchType then theme.Diff[props.patchType] else nil,
			BorderSizePixel = 0,
			BackgroundTransparency = props.patchType and props.transparency or 1,
			Size = UDim2.new(1, 0, 0, 30),
		}, {
			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 10),
				PaddingRight = UDim.new(0, 10),
			}),
			DiffIcon = if props.patchType then e("ImageLabel", {
				Image = Assets.Images.Diff[props.patchType],
				ImageColor3 = theme.AddressEntry.PlaceholderColor,
				ImageTransparency = props.transparency,
				BackgroundTransparency = 1,
				Size = UDim2.new(0, 20, 0, 20),
				Position = UDim2.new(0, 0, 0.5, 0),
				AnchorPoint = Vector2.new(0, 0.5),
			}) else nil,
			ClassIcon = e("ImageLabel", {
				Image = iconProps.Image,
				ImageTransparency = props.transparency,
				ImageRectOffset = iconProps.ImageRectOffset,
				ImageRectSize = iconProps.ImageRectSize,
				BackgroundTransparency = 1,
				Size = UDim2.new(0, 20, 0, 20),
				Position = UDim2.new(0, indent, 0.5, 0),
				AnchorPoint = Vector2.new(0, 0.5),
			}),
			InstanceName = e("TextLabel", {
				Text = props.name .. (props.details and string.format('  <font color="#%s">%s</font>', theme.AddressEntry.PlaceholderColor:ToHex(), props.details) or ""),
				RichText = true,
				BackgroundTransparency = 1,
				Font = Enum.Font.GothamMedium,
				TextSize = 14,
				TextColor3 = theme.Settings.Setting.DescriptionColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = props.transparency,
				TextTruncate = Enum.TextTruncate.AtEnd,
				Size = UDim2.new(1, -indent-50, 1, 0),
				Position = UDim2.new(0, indent + 30, 0, 0),
			}),
			table.unpack(lineGuides),
		})
	end)
end

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
		local propNames, i = {}, 0
		for prop in change.changedProperties do
			i += 1
			if i > 5 then
				propNames[i] = "..."
				break
			end
			propNames[i] = prop
		end

		-- Add this node to tree
		addNode(instanceMap.fromInstances[instance.Parent], {
			id = change.id,
			patchType = "Edit",
			className = instance.ClassName,
			name = instance.Name,
			details = table.concat(propNames, ", "),
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
		local propNames, i = {}, 0
		for prop in change.Properties do
			i += 1
			if i > 5 then
				propNames[i] = "..."
				break
			end
			propNames[i] = prop
		end

		-- Add this node to tree
		addNode(change.Parent, {
			id = change.Id,
			patchType = "Add",
			className = change.ClassName,
			name = change.Name,
			details = table.concat(propNames, ", "),
		})
	end

	-- Recusively draw tree
	local function drawNode(node, depth)
		table.insert(domLabels, e(domLabel, {
			patchType = node.patchType,
			className = node.className,
			name = node.name,
			details = node.details,
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
