local TextService = game:GetService("TextService")
local HttpService = game:GetService("HttpService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Theme = require(Plugin.App.Theme)

local BorderedContainer = require(Plugin.App.Components.BorderedContainer)

local e = Roact.createElement
local DELAY = 0.75
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

local TooltipContext = Roact.createContext({})

local function Popup(props)
	local textSize = TextService:GetTextSize(
		props.Text, 16, Enum.Font.GothamMedium, Vector2.new(math.min(props.canvasSize.X, 160), math.huge)
	) + PADDING

	local X = math.min(props.Position.X + OFFSET.X, props.canvasSize.X - OFFSET.X - textSize.X)
	local Y = math.min(props.Position.Y + OFFSET.Y, props.canvasSize.Y - OFFSET.Y - textSize.Y)

	return Theme.with(function(theme)
		return e(BorderedContainer, {
			position = UDim2.fromOffset(X, Y),
			size = UDim2.fromOffset(textSize.X, textSize.Y),
			zIndex = 2,
			transparency = props.transparency,
		}, {
			Label = e("TextLabel", {
				BackgroundTransparency = 1,
				Position = UDim2.fromOffset(PADDING.X/2, PADDING.Y/2),
				Size = UDim2.new(1, -PADDING.X, 1, -PADDING.Y),
				Text = props.Text,
				TextSize = 16,
				Font = Enum.Font.GothamMedium,
				TextWrapped = true,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextColor3 = theme.Button.Bordered.Enabled.TextColor,
				TextTransparency = props.transparency,
			})
		})
	end)
end

local Provider = Roact.Component:extend("TooltipManager")

function Provider:init()
	self.canvas = Roact.createRef()
	self:setState({
		tips = {},
		canvasSize = Vector2.new(200, 100),
		addTip = function(id: string, data: { Text: string, Position: Vector2 })
			self:setState(function(state)
				state.tips[id] = data
				return state
			end)
		end,
		removeTip = function(id: string)
			self:setState(function(state)
				state.tips[id] = nil
				return state
			end)
		end,
	})
end

function Provider:render()
	local tips = self.state.tips
	local popups = {}

	for key, value in tips do
		popups[key] = e(Popup, {
			Text = value.Text or "",
			Position = value.Position or Vector2.zero,

			canvasSize = self.state.canvasSize,
		})
	end

	return Roact.createElement(TooltipContext.Provider, {
        value = self.state,
    }, {
		TooltipCanvas = e("Frame", {
			[Roact.Change.AbsoluteSize] = function(rbx)
				self:setState({
					canvasSize = rbx.AbsoluteSize,
				})
			end,
			ZIndex = 2,
			BackgroundTransparency = 1,
			Size = UDim2.fromScale(1, 1),
		}, popups),
		Container = e("Frame", {
			ZIndex = 1,
			BackgroundTransparency = 1,
			Size = UDim2.fromScale(1, 1),
		}, self.props[Roact.Children]),
	})
end

local Trigger = Roact.Component:extend("TooltipTrigger")

function Trigger:init()
	self.id = HttpService:GenerateGUID(false)
	self.mousePos = Vector2.zero

	self.destroy = nil
end

function Trigger:willUnmount()
	if self.destroy then
		self.destroy()
	end
end

function Trigger:render()
	return Roact.createElement(TooltipContext.Consumer, {
        render = function(context)
			self.destroy = function()
				context.removeTip(self.id)
			end

			return e("Frame", {
				Size = UDim2.fromScale(1, 1),
				BackgroundTransparency = 1,
				ZIndex = self.props.zIndex or 100,

				[Roact.Event.MouseMoved] = function(_rbx, x, y)
					self.mousePos = Vector2.new(x, y)
				end,
				[Roact.Event.MouseEnter] = function()
					self.showDelayThread = task.delay(DELAY, function()
						context.addTip(self.id, {
							Text = self.props.text,
							Position = self.mousePos,
						})
					end)
				end,
				[Roact.Event.MouseLeave] = function()
					if self.showDelayThread then
						task.cancel(self.showDelayThread)
					end
					context.removeTip(self.id)
				end,
			})
		end,
	})
end

return {
	Provider = Provider,
	Trigger = Trigger,
}
