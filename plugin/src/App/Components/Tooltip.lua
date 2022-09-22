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
local OFFSET = Vector2.new(30, 12)
local PADDING = Vector2.new(16, 16)

local TooltipContext = Roact.createContext({})

local function Popup(props)
	local textSize = TextService:GetTextSize(
		props.Text, 16, Enum.Font.GothamMedium, Vector2.new(math.min(props.canvasSize.X, 160), math.huge)
	) + PADDING

	-- Don't go out of bounds
	local X = math.min(props.Position.X - OFFSET.X, props.canvasSize.X - textSize.X)
	local Y = math.min(props.Position.Y + OFFSET.Y, props.canvasSize.Y - textSize.Y)

	return Theme.with(function(theme)
		return e(BorderedContainer, {
			position = UDim2.fromOffset(X, Y),
			size = UDim2.fromOffset(textSize.X, textSize.Y),
			transparency = props.transparency,
		}, {
			Label = e("TextLabel", {
				BackgroundTransparency = 1,
				Position = UDim2.fromScale(0.5, 0.5),
				Size = UDim2.new(1, -PADDING.X, 1, -PADDING.Y),
				AnchorPoint = Vector2.new(0.5, 0.5),
				Text = props.Text,
				TextSize = 16,
				Font = Enum.Font.GothamMedium,
				TextWrapped = true,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextColor3 = theme.Button.Bordered.Enabled.TextColor,
				TextTransparency = props.transparency,
			}),

			Triangle = e("ImageLabel", {
				ZIndex = 100,
				Position = UDim2.fromOffset(
					math.clamp(props.Position.X - X, 6, textSize.X-6),
					-12
				),
				Size = UDim2.fromOffset(16, 16),
				BackgroundTransparency = 1,
				Image = "rbxassetid://10981445863",
				ImageColor3 = theme.BorderedContainer.BackgroundColor,
				ImageTransparency = props.transparency,
			}, {
				Border = e("ImageLabel", {
					Size = UDim2.fromScale(1, 1),
					BackgroundTransparency = 1,
					Image = "rbxassetid://10981549159",
					ImageColor3 = theme.BorderedContainer.BorderColor,
					ImageTransparency = props.transparency,
				}),
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
	if self.showDelayThread then
		task.cancel(self.showDelayThread)
	end
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

				[Roact.Event.MouseMoved] = function(rbx, x, _y)
					self.mousePos = Vector2.new(
						x,
						rbx.AbsolutePosition.Y + rbx.AbsoluteSize.Y - 5
					)
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
