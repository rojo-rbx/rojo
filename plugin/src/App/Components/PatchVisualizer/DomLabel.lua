local SelectionService = game:GetService("Selection")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Flipper = require(Packages.Flipper)

local Assets = require(Plugin.Assets)
local Theme = require(Plugin.App.Theme)
local bindingUtil = require(Plugin.App.bindingUtil)

local e = Roact.createElement

local ChangeList = require(script.Parent.ChangeList)
local Tooltip = require(Plugin.App.Components.Tooltip)
local ClassIcon = require(Plugin.App.Components.ClassIcon)

local Expansion = Roact.Component:extend("Expansion")

function Expansion:render()
	local props = self.props

	if not props.rendered then
		return nil
	end

	return e("Frame", {
		BackgroundTransparency = 1,
		Size = UDim2.new(1, -props.indent, 1, -24),
		Position = UDim2.new(0, props.indent, 0, 24),
	}, {
		ChangeList = e(ChangeList, {
			changes = props.changeList,
			transparency = props.transparency,
			showStringDiff = props.showStringDiff,
			showTableDiff = props.showTableDiff,
		}),
	})
end

local DomLabel = Roact.Component:extend("DomLabel")

function DomLabel:init()
	local initHeight = self.props.elementHeight:getValue()
	self.expanded = initHeight > 24

	self.motor = Flipper.SingleMotor.new(initHeight)
	self.binding = bindingUtil.fromMotor(self.motor)

	self:setState({
		renderExpansion = self.expanded,
	})
	self.motor:onStep(function(value)
		local renderExpansion = value > 24

		self.props.setElementHeight(value)
		if self.props.updateEvent then
			self.props.updateEvent:Fire()
		end

		self:setState(function(state)
			if state.renderExpansion == renderExpansion then
				return nil
			end

			return {
				renderExpansion = renderExpansion,
			}
		end)
	end)
end

function DomLabel:didUpdate(prevProps)
	if
		prevProps.instance ~= self.props.instance
		or prevProps.patchType ~= self.props.patchType
		or prevProps.name ~= self.props.name
		or prevProps.changeList ~= self.props.changeList
	then
		-- Close the expansion when the domlabel is changed to a different thing
		self.expanded = false
		self.motor:setGoal(Flipper.Spring.new(24, {
			frequency = 5,
			dampingRatio = 1,
		}))
	end
end

function DomLabel:render()
	local props = self.props
	local depth = props.depth or 1

	return Theme.with(function(theme)
		local color = if props.isWarning
			then theme.Diff.Warning
			elseif props.patchType then theme.Diff.Background[props.patchType]
			else theme.TextColor

		local indent = (depth - 1) * 12 + 15

		-- Line guides help indent depth remain readable
		local lineGuides = {}
		for i = 2, depth do
			if props.depthsComplete[i] then
				continue
			end
			if props.isFinalChild and i == depth then
				-- This line stops halfway down to merge with our connector for the right angle
				lineGuides["Line_" .. i] = e("Frame", {
					Size = UDim2.new(0, 2, 0, 15),
					Position = UDim2.new(0, (12 * (i - 1)) + 6, 0, -1),
					BorderSizePixel = 0,
					BackgroundTransparency = props.transparency,
					BackgroundColor3 = theme.BorderedContainer.BorderColor,
				})
			else
				-- All other lines go all the way
				-- with the exception of the final element, which stops halfway down
				lineGuides["Line_" .. i] = e("Frame", {
					Size = UDim2.new(0, 2, 1, if props.isFinalElement then -9 else 2),
					Position = UDim2.new(0, (12 * (i - 1)) + 6, 0, -1),
					BorderSizePixel = 0,
					BackgroundTransparency = props.transparency,
					BackgroundColor3 = theme.BorderedContainer.BorderColor,
				})
			end
		end

		if depth ~= 1 then
			lineGuides["Connector"] = e("Frame", {
				Size = UDim2.new(0, 8, 0, 2),
				Position = UDim2.new(0, 2 + (12 * props.depth), 0, 12),
				AnchorPoint = Vector2.xAxis,
				BorderSizePixel = 0,
				BackgroundTransparency = props.transparency,
				BackgroundColor3 = theme.BorderedContainer.BorderColor,
			})
		end

		return e("Frame", {
			ClipsDescendants = true,
			BackgroundTransparency = if props.elementIndex % 2 == 0 then 0.985 else 1,
			BackgroundColor3 = theme.Diff.Row,
			Size = self.binding:map(function(expand)
				return UDim2.new(1, 0, 0, expand)
			end),
		}, {
			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 10),
				PaddingRight = UDim.new(0, 10),
			}),
			Button = e("TextButton", {
				BackgroundTransparency = 1,
				Text = "",
				Size = UDim2.new(1, 0, 1, 0),
				[Roact.Event.Activated] = function(_rbx: Instance, _input: InputObject, clickCount: number)
					if clickCount == 1 then
						-- Double click opens the instance in explorer
						self.lastDoubleClickTime = os.clock()
						if props.instance then
							SelectionService:Set({ props.instance })
						end
					elseif clickCount == 0 then
						-- Single click expands the changes
						task.wait(0.25)
						if os.clock() - (self.lastDoubleClickTime or 0) <= 0.25 then
							-- This is a double click, so don't expand
							return
						end

						if props.changeList then
							self.expanded = not self.expanded
							local goalHeight = 24
								+ (if self.expanded then math.clamp(#props.changeList * 24, 24, 24 * 6) else 0)
							self.motor:setGoal(Flipper.Spring.new(goalHeight, {
								frequency = 5,
								dampingRatio = 1,
							}))
						end
					end
				end,
			}, {
				StateTip = if (props.instance or props.changeList)
					then e(Tooltip.Trigger, {
						text = (if props.changeList
							then "Click to " .. (if self.expanded then "hide" else "view") .. " changes"
							else "") .. (if props.instance
							then (if props.changeList then " & d" else "D") .. "ouble click to open in Explorer"
							else ""),
					})
					else nil,
			}),
			Expansion = if props.changeList
				then e(Expansion, {
					rendered = self.state.renderExpansion,
					indent = indent,
					transparency = props.transparency,
					changeList = props.changeList,
					showStringDiff = props.showStringDiff,
					showTableDiff = props.showTableDiff,
				})
				else nil,
			DiffIcon = if props.patchType
				then e("ImageLabel", {
					Image = Assets.Images.Diff[props.patchType],
					ImageColor3 = color,
					ImageTransparency = props.transparency,
					BackgroundTransparency = 1,
					Size = UDim2.new(0, 14, 0, 14),
					Position = UDim2.new(0, 0, 0, 12),
					AnchorPoint = Vector2.new(0, 0.5),
				})
				else nil,
			ClassIcon = e(ClassIcon, {
				className = props.className,
				color = color,
				transparency = props.transparency,
				size = UDim2.new(0, 16, 0, 16),
				position = UDim2.new(0, indent + 2, 0, 12),
				anchorPoint = Vector2.new(0, 0.5),
			}),
			InstanceName = e("TextLabel", {
				Text = (if props.isWarning then "⚠ " else "") .. props.name,
				RichText = true,
				BackgroundTransparency = 1,
				FontFace = if props.patchType then theme.Font.Bold else theme.Font.Main,
				TextSize = theme.TextSize.Body,
				TextColor3 = color,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = props.transparency,
				TextTruncate = Enum.TextTruncate.AtEnd,
				Size = UDim2.new(1, -indent - 50, 0, 24),
				Position = UDim2.new(0, indent + 22, 0, 0),
			}),
			ChangeInfo = e("Frame", {
				BackgroundTransparency = 1,
				Size = UDim2.new(1, -indent - 80, 0, 24),
				Position = UDim2.new(1, -2, 0, 0),
				AnchorPoint = Vector2.new(1, 0),
			}, {
				Layout = e("UIListLayout", {
					FillDirection = Enum.FillDirection.Horizontal,
					HorizontalAlignment = Enum.HorizontalAlignment.Right,
					VerticalAlignment = Enum.VerticalAlignment.Center,
					SortOrder = Enum.SortOrder.LayoutOrder,
					Padding = UDim.new(0, 4),
				}),
				Edits = if props.changeInfo and props.changeInfo.edits
					then e("TextLabel", {
						Text = props.changeInfo.edits .. if props.changeInfo.failed then "," else "",
						BackgroundTransparency = 1,
						FontFace = theme.Font.Thin,
						TextSize = theme.TextSize.Body,
						TextColor3 = theme.SubTextColor,
						TextTransparency = props.transparency,
						Size = UDim2.new(0, 0, 0, theme.TextSize.Body),
						AutomaticSize = Enum.AutomaticSize.X,
						LayoutOrder = 2,
					})
					else nil,
				Failed = if props.changeInfo and props.changeInfo.failed
					then e("TextLabel", {
						Text = props.changeInfo.failed,
						BackgroundTransparency = 1,
						FontFace = theme.Font.Thin,
						TextSize = theme.TextSize.Body,
						TextColor3 = theme.Diff.Warning,
						TextTransparency = props.transparency,
						Size = UDim2.new(0, 0, 0, theme.TextSize.Body),
						AutomaticSize = Enum.AutomaticSize.X,
						LayoutOrder = 6,
					})
					else nil,
			}),
			LineGuides = e("Folder", nil, lineGuides),
		})
	end)
end

return DomLabel
