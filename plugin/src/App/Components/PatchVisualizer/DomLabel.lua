local SelectionService = game:GetService("Selection")
local StudioService = game:GetService("StudioService")

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
local Tooltip = require(script.Parent.Parent.Tooltip)

local Expansion = Roact.Component:extend("Expansion")

function Expansion:render()
	local props = self.props

	if not props.rendered then
		return nil
	end

	return e("Frame", {
		BackgroundTransparency = 1,
		Size = UDim2.new(1, -props.indent, 1, -30),
		Position = UDim2.new(0, props.indent, 0, 30),
	}, {
		ChangeList = e(ChangeList, {
			changes = props.changeList,
			transparency = props.transparency,
			columnVisibility = props.columnVisibility,
		}),
	})
end

local DomLabel = Roact.Component:extend("DomLabel")

function DomLabel:init()
	local initHeight = self.props.elementHeight:getValue()
	self.expanded = initHeight > 30

	self.motor = Flipper.SingleMotor.new(initHeight)
	self.binding = bindingUtil.fromMotor(self.motor)

	self:setState({
		renderExpansion = self.expanded,
	})
	self.motor:onStep(function(value)
		local renderExpansion = value > 30

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

function DomLabel:render()
	local props = self.props

	return Theme.with(function(theme)
		local iconProps = StudioService:GetClassIcon(props.className)
		local indent = (props.depth or 0) * 20 + 25

		-- Line guides help indent depth remain readable
		local lineGuides = {}
		for i = 1, props.depth or 0 do
			table.insert(
				lineGuides,
				e("Frame", {
					Name = "Line_" .. i,
					Size = UDim2.new(0, 2, 1, 2),
					Position = UDim2.new(0, (20 * i) + 15, 0, -1),
					BorderSizePixel = 0,
					BackgroundTransparency = props.transparency,
					BackgroundColor3 = theme.BorderedContainer.BorderColor,
				})
			)
		end

		return e("Frame", {
			Name = "Change",
			ClipsDescendants = true,
			BackgroundColor3 = if props.patchType then theme.Diff[props.patchType] else nil,
			BorderSizePixel = 0,
			BackgroundTransparency = props.patchType and props.transparency or 1,
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
							local goalHeight = 30
								+ (if self.expanded then math.clamp(#self.props.changeList * 30, 30, 30 * 6) else 0)
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
					columnVisibility = props.columnVisibility,
				})
				else nil,
			DiffIcon = if props.patchType
				then e("ImageLabel", {
					Image = Assets.Images.Diff[props.patchType],
					ImageColor3 = theme.AddressEntry.PlaceholderColor,
					ImageTransparency = props.transparency,
					BackgroundTransparency = 1,
					Size = UDim2.new(0, 20, 0, 20),
					Position = UDim2.new(0, 0, 0, 15),
					AnchorPoint = Vector2.new(0, 0.5),
				})
				else nil,
			ClassIcon = e("ImageLabel", {
				Image = iconProps.Image,
				ImageTransparency = props.transparency,
				ImageRectOffset = iconProps.ImageRectOffset,
				ImageRectSize = iconProps.ImageRectSize,
				BackgroundTransparency = 1,
				Size = UDim2.new(0, 20, 0, 20),
				Position = UDim2.new(0, indent, 0, 15),
				AnchorPoint = Vector2.new(0, 0.5),
			}),
			InstanceName = e("TextLabel", {
				Text = (if props.isWarning then "⚠ " else "") .. props.name .. (props.hint and string.format(
					'  <font color="#%s">%s</font>',
					theme.AddressEntry.PlaceholderColor:ToHex(),
					props.hint
				) or ""),
				RichText = true,
				BackgroundTransparency = 1,
				Font = Enum.Font.GothamMedium,
				TextSize = 14,
				TextColor3 = if props.isWarning then theme.Diff.Warning else theme.Settings.Setting.DescriptionColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = props.transparency,
				TextTruncate = Enum.TextTruncate.AtEnd,
				Size = UDim2.new(1, -indent - 50, 0, 30),
				Position = UDim2.new(0, indent + 30, 0, 0),
			}),
			LineGuides = e("Folder", nil, lineGuides),
		})
	end)
end

return DomLabel
