local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local PatchTree = require(Plugin.PatchTree)
local PatchSet = require(Plugin.PatchSet)

local Theme = require(Plugin.App.Theme)
local VirtualScroller = require(Plugin.App.Components.VirtualScroller)
local BorderedContainer = require(Plugin.App.Components.BorderedContainer)

local e = Roact.createElement

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
	if self.props.patchTree ~= nextProps.patchTree then
		return true
	end

	local currentPatch, nextPatch = self.props.patch, nextProps.patch
	if currentPatch ~= nil or nextPatch ~= nil then
		return not PatchSet.isEqual(currentPatch, nextPatch)
	end

	return false
end

function PatchVisualizer:render()
	local patchTree = self.props.patchTree
	if patchTree == nil and self.props.patch ~= nil then
		patchTree = PatchTree.build(
			self.props.patch,
			self.props.instanceMap,
			self.props.changeListHeaders or { "Property", "Current", "Incoming" }
		)
		if self.props.unappliedPatch then
			patchTree =
				PatchTree.updateMetadata(patchTree, self.props.patch, self.props.instanceMap, self.props.unappliedPatch)
		end
	end

	-- Recusively draw tree
	local scrollElements, elementHeights, elementIndex = {}, {}, 0

	if patchTree then
		local elementTotal = patchTree:getCount()
		local depthsComplete = {}
		local function drawNode(node, depth)
			elementIndex += 1

			local parentNode = patchTree:getNode(node.parentId)
			local isFinalChild = true
			if parentNode then
				for _id, sibling in parentNode.children do
					if type(sibling) == "table" and sibling.name and sibling.name > node.name then
						isFinalChild = false
						break
					end
				end
			end

			local elementHeight, setElementHeight = Roact.createBinding(24)
			elementHeights[elementIndex] = elementHeight
			scrollElements[elementIndex] = e(DomLabel, {
				transparency = self.props.transparency,
				showStringDiff = self.props.showStringDiff,
				showTableDiff = self.props.showTableDiff,
				updateEvent = self.updateEvent,
				elementHeight = elementHeight,
				setElementHeight = setElementHeight,
				elementIndex = elementIndex,
				isFinalElement = elementIndex == elementTotal,
				depth = depth,
				depthsComplete = table.clone(depthsComplete),
				hasChildren = (node.children ~= nil and next(node.children) ~= nil),
				isFinalChild = isFinalChild,
				patchType = node.patchType,
				className = node.className,
				isWarning = node.isWarning,
				instance = node.instance,
				name = node.name,
				changeInfo = node.changeInfo,
				changeList = node.changeList,
			})

			if isFinalChild then
				depthsComplete[depth] = true
			end
		end

		patchTree:forEach(function(node, depth)
			depthsComplete[depth] = false
			for i = depth + 1, #depthsComplete do
				depthsComplete[i] = nil
			end

			drawNode(node, depth)
		end)
	end

	return Theme.with(function(theme)
		return e(BorderedContainer, {
			transparency = self.props.transparency,
			size = self.props.size,
			position = self.props.position,
			anchorPoint = self.props.anchorPoint,
			layoutOrder = self.props.layoutOrder,
		}, {
			CleanMerge = e("TextLabel", {
				Visible = #scrollElements == 0,
				Text = "No changes to sync, project is up to date.",
				Font = Enum.Font.GothamMedium,
				TextSize = 15,
				TextColor3 = theme.TextColor,
				TextWrapped = true,
				Size = UDim2.new(1, 0, 1, 0),
				BackgroundTransparency = 1,
			}),

			VirtualScroller = e(VirtualScroller, {
				size = UDim2.new(1, 0, 1, -2),
				position = UDim2.new(0, 0, 0, 2),
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
	end)
end

return PatchVisualizer
