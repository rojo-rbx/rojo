local TextService = game:GetService("TextService")
local StudioService = game:GetService("StudioService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Flipper = require(Packages.Flipper)
local Log = require(Packages.Log)

local bindingUtil = require(script.Parent.bindingUtil)

local Theme = require(Plugin.App.Theme)
local Assets = require(Plugin.Assets)

local BorderedContainer = require(Plugin.App.Components.BorderedContainer)
local TextButton = require(Plugin.App.Components.TextButton)

local baseClock = DateTime.now().UnixTimestampMillis

local e = Roact.createElement

local Notification = Roact.Component:extend("Notification")

function Notification:init()
	self.motor = Flipper.SingleMotor.new(0)
	self.binding = bindingUtil.fromMotor(self.motor)

	self.lifetime = self.props.timeout

	self.motor:onStep(function(value)
		if value <= 0 and self.props.onClose then
			self.props.onClose()
		end
	end)
end

function Notification:dismiss()
	self.motor:setGoal(
		Flipper.Spring.new(0, {
			frequency = 5,
			dampingRatio = 1,
		})
	)
end

function Notification:didMount()
	self.motor:setGoal(
		Flipper.Spring.new(1, {
			frequency = 3,
			dampingRatio = 1,
		})
	)

	self.props.soundPlayer:play(Assets.Sounds.Notification)

	self.timeout = task.spawn(function()
		local clock = os.clock()
		local seen = false
		while task.wait(1/10) do
			local now = os.clock()
			local dt = now - clock
			clock = now

			if not seen then
				seen = StudioService.ActiveScript == nil
			end

			if not seen then
				-- Don't run down timer before being viewed
				continue
			end

			self.lifetime -= dt
			if self.lifetime <= 0 then
				self:dismiss()
				break
			end
		end
	end)
end

function Notification:willUnmount()
	task.cancel(self.timeout)
end

function Notification:render()
	local transparency = self.binding:map(function(value)
		return 1 - value
	end)

	local textBounds = TextService:GetTextSize(
		self.props.text,
		15,
		Enum.Font.GothamMedium,
		Vector2.new(350, 700)
	)

	local actionButtons = {}
	local buttonsX = 0
	if self.props.actions then
		local count = 0
		for key, action in self.props.actions do
			actionButtons[key] = e(TextButton, {
				text = action.text,
				style = action.style,
				onClick = function()
					local success, err = pcall(action.onClick, self)
					if not success then
						Log.warn("Error in notification action: " .. tostring(err))
					end
				end,
				layoutOrder = -action.layoutOrder,
				transparency = transparency,
			})

			buttonsX += TextService:GetTextSize(
				action.text, 18, Enum.Font.GothamMedium,
				Vector2.new(math.huge, math.huge)
			).X + 30

			count += 1
		end

		buttonsX += (count - 1) * 5
	end

	local paddingY, logoSize = 20, 32
	local actionsY = if self.props.actions then 35 else 0
	local contentX = math.max(textBounds.X, buttonsX)

	local size = self.binding:map(function(value)
		return UDim2.fromOffset(
			(35 + 40 + contentX) * value,
			5 + actionsY + paddingY + math.max(logoSize, textBounds.Y)
		)
	end)

	return Theme.with(function(theme)
		return e("TextButton", {
			BackgroundTransparency = 1,
			Size = size,
			LayoutOrder = self.props.layoutOrder,
			Text = "",
			ClipsDescendants = true,

			[Roact.Event.Activated] = function()
				self:dismiss()
			end,
		}, {
			e(BorderedContainer, {
				transparency = transparency,
				size = UDim2.new(1, 0, 1, 0),
			}, {
				Contents = e("Frame", {
					Size = UDim2.new(0, 35 + contentX, 1, -paddingY),
					Position = UDim2.new(0, 0, 0, paddingY / 2),
					BackgroundTransparency = 1
				}, {
					Logo = e("ImageLabel", {
						ImageTransparency = transparency,
						Image = Assets.Images.PluginButton,
						BackgroundTransparency = 1,
						Size = UDim2.new(0, logoSize, 0, logoSize),
						Position = UDim2.new(0, 0, 0, 0),
						AnchorPoint = Vector2.new(0, 0),
					}),
					Info = e("TextLabel", {
						Text = self.props.text,
						Font = Enum.Font.GothamMedium,
						TextSize = 15,
						TextColor3 = theme.Notification.InfoColor,
						TextTransparency = transparency,
						TextXAlignment = Enum.TextXAlignment.Left,
						TextWrapped = true,

						Size = UDim2.new(0, textBounds.X, 0, textBounds.Y),
						Position = UDim2.fromOffset(35, 0),

						LayoutOrder = 1,
						BackgroundTransparency = 1,
					}),
					Actions = if self.props.actions then e("Frame", {
						Size = UDim2.new(1, -40, 0, 35),
						Position = UDim2.new(1, 0, 1, 0),
						AnchorPoint = Vector2.new(1, 1),
						BackgroundTransparency = 1,
					}, {
						Layout = e("UIListLayout", {
							FillDirection = Enum.FillDirection.Horizontal,
							HorizontalAlignment = Enum.HorizontalAlignment.Right,
							VerticalAlignment = Enum.VerticalAlignment.Center,
							SortOrder = Enum.SortOrder.LayoutOrder,
							Padding = UDim.new(0, 5),
						}),
						Buttons = Roact.createFragment(actionButtons),
					}) else nil,
				}),

				Padding = e("UIPadding", {
					PaddingLeft = UDim.new(0, 17),
					PaddingRight = UDim.new(0, 15),
				}),
			})
		})
	end)
end

local Notifications = Roact.Component:extend("Notifications")

function Notifications:render()
	local notifs = {}

	for id, notif in self.props.notifications do
		notifs["NotifID_" .. id] = e(Notification, {
			soundPlayer = self.props.soundPlayer,
			text = notif.text,
			timestamp = notif.timestamp,
			timeout = notif.timeout,
			actions = notif.actions,
			layoutOrder = (notif.timestamp - baseClock),
			onClose = function()
				self.props.onClose(id)
			end,
		})
	end

	return Roact.createFragment(notifs)
end

return Notifications
