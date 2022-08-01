local StudioService = game:GetService("StudioService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)
local Flipper = require(Rojo.Flipper)

local Assets = require(Plugin.Assets)
local Theme = require(Plugin.App.Theme)
local bindingUtil = require(Plugin.App.bindingUtil)

local e = Roact.createElement

local DiffTable = require(script.Parent.DiffTable)
local DomLabel = Roact.Component:extend("DomLabel")

function DomLabel:init()
	self.expanded = false
	self.motor = Flipper.SingleMotor.new(self.props.active and 1 or 0)
	self.binding = bindingUtil.fromMotor(self.motor)
end

function DomLabel:render()
	local props = self.props

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
		local expandHeight = 0
		if props.diffTable then
			expandHeight = math.clamp(#props.diffTable * 30, 30, 30*5)
		end

		return e("Frame", {
			Name = "Change",
			ClipsDescendants = true,
			BackgroundColor3 = if props.patchType then theme.Diff[props.patchType] else nil,
			BorderSizePixel = 0,
			BackgroundTransparency = props.patchType and props.transparency or 1,
			Size = self.binding:map(function(expand)
				return UDim2.new(1, 0, 0, 30 + (expand * expandHeight))
			end),
		}, {
			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 10),
				PaddingRight = UDim.new(0, 10),
			}),
			ExpandButton = if props.diffTable then e("TextButton", {
				BackgroundTransparency = 1,
				Text = "",
				Size = UDim2.new(1, 0, 1, 0),
				[Roact.Event.Activated] = function()
					self.expanded = not self.expanded
					self.motor:setGoal(
						Flipper.Spring.new(self.expanded and 1 or 0, {
							frequency = 5,
							dampingRatio = 1,
						})
					)
				end,
			}) else nil,
			Expansion = if props.diffTable then e("Frame", {
				BackgroundTransparency = 1,
				Size = UDim2.new(1, -indent, 1, -30),
				Position = UDim2.new(0, indent, 0, 30),
			}, {
				e(DiffTable, {
					csv = props.diffTable,
					transparency = self.props.transparency,
				})
			}) else nil,
			DiffIcon = if props.patchType then e("ImageLabel", {
				Image = Assets.Images.Diff[props.patchType],
				ImageColor3 = theme.AddressEntry.PlaceholderColor,
				ImageTransparency = props.transparency,
				BackgroundTransparency = 1,
				Size = UDim2.new(0, 20, 0, 20),
				Position = UDim2.new(0, 0, 0, 15),
				AnchorPoint = Vector2.new(0, 0.5),
			}) else nil,
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
				Text = props.name .. (props.hint and string.format('  <font color="#%s">%s</font>', theme.AddressEntry.PlaceholderColor:ToHex(), props.hint) or ""),
				RichText = true,
				BackgroundTransparency = 1,
				Font = Enum.Font.GothamMedium,
				TextSize = 14,
				TextColor3 = theme.Settings.Setting.DescriptionColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = props.transparency,
				TextTruncate = Enum.TextTruncate.AtEnd,
				Size = UDim2.new(1, -indent-50, 0, 30),
				Position = UDim2.new(0, indent + 30, 0, 0),
			}),
			table.unpack(lineGuides),
		})
	end)
end

return DomLabel
