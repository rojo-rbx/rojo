local TextService = game:GetService("TextService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Theme = require(Plugin.App.Theme)

local BorderedContainer = require(Plugin.App.Components.BorderedContainer)

local e = Roact.createElement
local DELAY = 1
local OFFSET = Vector2.new(5, 10)
local PADDING = Vector2.new(16, 16)

local Tooltip = Roact.Component:extend("Tooltip")

function Tooltip:init()
	self.ref = Roact.createRef()
	self.mousePos = Vector2.new()
	self:setState({
		visible = false,
	})
end

function Tooltip:render()
	return Theme.with(function(theme)
		local children = {}

		if self.state.visible and self.ref:getValue() then
			local instance = self.ref:getValue()
			local layer = instance:FindFirstAncestorWhichIsA("LayerCollector")

			local canvasSize = layer.AbsoluteSize
			local textSize = TextService:GetTextSize(
				self.props.text, 16, Enum.Font.GothamMedium, Vector2.new(math.min(canvasSize.X, self.props.maxWidth or 150), 100)
			) + PADDING

			local targetX = math.min(self.mousePos.X + OFFSET.X, canvasSize.X - OFFSET.X - textSize.X)
			local targetY = math.min(self.mousePos.Y + OFFSET.Y, canvasSize.Y - OFFSET.Y - textSize.Y)

			children.Container = e(BorderedContainer, {
				position = UDim2.fromOffset(targetX - instance.AbsolutePosition.X, targetY - instance.AbsolutePosition.Y),
				size = UDim2.fromOffset(textSize.X, textSize.Y),
				zIndex = 2,
				transparency = self.props.transparency,
			}, {
				Label = e("TextLabel", {
					BackgroundTransparency = 1,
					Position = UDim2.fromOffset(PADDING.X/2, PADDING.Y/2),
					Size = UDim2.new(1, -PADDING.X, 1, -PADDING.Y),
					Text = self.props.text,
					TextSize = 16,
					Font = Enum.Font.GothamMedium,
					TextWrapped = true,
					TextXAlignment = Enum.TextXAlignment.Left,
					TextColor3 = theme.Button.Bordered.Enabled.TextColor,
					TextTransparency = self.props.transparency,
				})
			})
		end

		return e("Frame", {
			Size = UDim2.fromScale(1, 1),
			BackgroundTransparency = 1,
			ZIndex = 50,
			[Roact.Ref] = self.ref,
			[Roact.Event.MouseMoved] = function(_rbx, x, y)
				self.mousePos = Vector2.new(x, y)
			end,
			[Roact.Event.MouseEnter] = function()
				self.showDelayThread = task.delay(DELAY, function()
					self:setState({
						visible = true,
					})
				end)
			end,
			[Roact.Event.MouseLeave] = function()
				if self.showDelayThread then
					task.cancel(self.showDelayThread)
				end
				self:setState({
					visible = false,
				})
			end,
		}, children)
	end)
end

return Tooltip
