local TextService = game:GetService("TextService")
local HttpService = game:GetService("HttpService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Theme = require(Plugin.App.Theme)

local BorderedContainer = require(Plugin.App.Components.BorderedContainer)

local e = Roact.createElement

local DELAY = 0.75 -- How long to hover before a popup is shown (seconds)
local TEXT_PADDING = Vector2.new(8 * 2, 6 * 2) -- Padding for the popup text containers
local TAIL_SIZE = 16 -- Size of the triangle tail piece
local X_OFFSET = 30 -- How far right (from left) the tail will be (assuming enough space)
local Y_OVERLAP = 10 -- Let the triangle tail piece overlap the target a bit to help "connect" it

local TooltipContext = Roact.createContext({})

local function Popup(props)
	local textSize = TextService:GetTextSize(
		props.Text,
		16,
		Enum.Font.GothamMedium,
		Vector2.new(math.min(props.parentSize.X, 160), math.huge)
	) + TEXT_PADDING + (Vector2.one * 2)

	local trigger = props.Trigger:getValue()

	local spaceBelow = props.parentSize.Y
		- (trigger.AbsolutePosition.Y + trigger.AbsoluteSize.Y - Y_OVERLAP + TAIL_SIZE)
	local spaceAbove = trigger.AbsolutePosition.Y + Y_OVERLAP - TAIL_SIZE

	-- If there's not enough space below, and there's more space above, then show the tooltip above the trigger
	local displayAbove = spaceBelow < textSize.Y and spaceAbove > spaceBelow

	local X = math.clamp(props.Position.X - X_OFFSET, 0, props.parentSize.X - textSize.X)
	local Y = 0

	if displayAbove then
		Y = math.max(trigger.AbsolutePosition.Y - TAIL_SIZE - textSize.Y + Y_OVERLAP, 0)
	else
		Y = math.min(
			trigger.AbsolutePosition.Y + trigger.AbsoluteSize.Y + TAIL_SIZE - Y_OVERLAP,
			props.parentSize.Y - textSize.Y
		)
	end

	return Theme.with(function(theme)
		return e(BorderedContainer, {
			position = UDim2.fromOffset(X, Y),
			size = UDim2.fromOffset(textSize.X, textSize.Y),
			transparency = props.transparency,
		}, {
			Label = e("TextLabel", {
				BackgroundTransparency = 1,
				Position = UDim2.fromScale(0.5, 0.5),
				Size = UDim2.new(1, -TEXT_PADDING.X, 1, -TEXT_PADDING.Y),
				AnchorPoint = Vector2.new(0.5, 0.5),
				Text = props.Text,
				TextSize = 16,
				Font = Enum.Font.GothamMedium,
				TextWrapped = true,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextColor3 = theme.Button.Bordered.Enabled.TextColor,
				TextTransparency = props.transparency,
			}),

			Tail = e("ImageLabel", {
				ZIndex = 100,
				Position = if displayAbove
					then UDim2.new(0, math.clamp(props.Position.X - X, 6, textSize.X - 6), 1, -1)
					else UDim2.new(0, math.clamp(props.Position.X - X, 6, textSize.X - 6), 0, -TAIL_SIZE + 1),
				Size = UDim2.fromOffset(TAIL_SIZE, TAIL_SIZE),
				AnchorPoint = Vector2.new(0.5, 0),
				Rotation = if displayAbove then 180 else 0,
				BackgroundTransparency = 1,
				Image = "rbxassetid://10983945016",
				ImageColor3 = theme.BorderedContainer.BackgroundColor,
				ImageTransparency = props.transparency,
			}, {
				Border = e("ImageLabel", {
					Size = UDim2.fromScale(1, 1),
					BackgroundTransparency = 1,
					Image = "rbxassetid://10983946430",
					ImageColor3 = theme.BorderedContainer.BorderColor,
					ImageTransparency = props.transparency,
				}),
			}),
		})
	end)
end

local Provider = Roact.Component:extend("TooltipManager")

function Provider:init()
	self:setState({
		tips = {},
		addTip = function(id: string, data: { Text: string, Position: Vector2, Trigger: any })
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
	return Roact.createElement(TooltipContext.Provider, {
		value = self.state,
	}, self.props[Roact.Children])
end

local Container = Roact.Component:extend("TooltipContainer")

function Container:init()
	self:setState({
		size = Vector2.new(200, 100),
	})
end

function Container:render()
	return Roact.createElement(TooltipContext.Consumer, {
		render = function(context)
			local tips = context.tips
			local popups = {}

			for key, value in tips do
				popups[key] = e(Popup, {
					Text = value.Text or "",
					Position = value.Position or Vector2.zero,
					Trigger = value.Trigger,

					parentSize = self.state.size,
				})
			end

			return e("Frame", {
				[Roact.Change.AbsoluteSize] = function(rbx)
					self:setState({
						size = rbx.AbsoluteSize,
					})
				end,
				ZIndex = 100,
				BackgroundTransparency = 1,
				Size = UDim2.fromScale(1, 1),
			}, popups)
		end,
	})
end

local Trigger = Roact.Component:extend("TooltipTrigger")

function Trigger:init()
	self.id = HttpService:GenerateGUID(false)
	self.ref = Roact.createRef()
	self.mousePos = Vector2.zero
	self.showingPopup = false

	self.destroy = function()
		self.props.context.removeTip(self.id)
		self.showingPopup = false
	end
end

function Trigger:willUnmount()
	if self.showDelayThread then
		pcall(task.cancel, self.showDelayThread)
	end
	if self.destroy then
		self.destroy()
	end
end

function Trigger:didUpdate(prevProps)
	if prevProps.text ~= self.props.text then
		-- Any existing popup is now invalid
		self.props.context.removeTip(self.id)
		self.showingPopup = false

		-- Let the new text propagate
		self:managePopup()
	end
end

function Trigger:isHovering()
	local rbx = self.ref.current
	if rbx then
		local pos = rbx.AbsolutePosition
		local size = rbx.AbsoluteSize
		local mousePos = self.mousePos

		return mousePos.X >= pos.X
			and mousePos.X <= pos.X + size.X
			and mousePos.Y >= pos.Y
			and mousePos.Y <= pos.Y + size.Y
	end
	return false
end

function Trigger:managePopup()
	if self:isHovering() then
		if self.showingPopup or self.showDelayThread then
			-- Don't duplicate popups
			return
		end

		self.showDelayThread = task.delay(DELAY, function()
			self.props.context.addTip(self.id, {
				Text = self.props.text,
				Position = self.mousePos,
				Trigger = self.ref,
			})
			self.showDelayThread = nil
			self.showingPopup = true
		end)
	else
		if self.showDelayThread then
			pcall(task.cancel, self.showDelayThread)
			self.showDelayThread = nil
		end
		self.props.context.removeTip(self.id)
		self.showingPopup = false
	end
end

function Trigger:render()
	local function recalculate(rbx)
		local widget = rbx:FindFirstAncestorOfClass("DockWidgetPluginGui")
		if not widget then
			return
		end
		self.mousePos = widget:GetRelativeMousePosition()

		self:managePopup()
	end

	return e("Frame", {
		Size = UDim2.fromScale(1, 1),
		BackgroundTransparency = 1,
		ZIndex = self.props.zIndex or 100,
		[Roact.Ref] = self.ref,

		[Roact.Change.AbsolutePosition] = recalculate,
		[Roact.Change.AbsoluteSize] = recalculate,
		[Roact.Event.MouseMoved] = recalculate,
		[Roact.Event.MouseLeave] = recalculate,
		[Roact.Event.MouseEnter] = recalculate,
	})
end

local function TriggerConsumer(props)
	return Roact.createElement(TooltipContext.Consumer, {
		render = function(context)
			local innerProps = table.clone(props)
			innerProps.context = context

			return e(Trigger, innerProps)
		end,
	})
end

return {
	Provider = Provider,
	Container = Container,
	Trigger = TriggerConsumer,
}
