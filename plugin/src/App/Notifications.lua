local TextService = game:GetService("TextService")
local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)
local Flipper = require(Rojo.Flipper)

local bindingUtil = require(script.Parent.bindingUtil)

local Theme = require(Plugin.App.Theme)
local Assets = require(Plugin.Assets)

local IconButton = require(Plugin.App.Components.IconButton)
local BorderedContainer = require(Plugin.App.Components.BorderedContainer)

local baseClock = DateTime.now().UnixTimestampMillis

local e = Roact.createElement

local Notification = Roact.Component:extend("Notification")

function Notification:init()
	self.motor = Flipper.SingleMotor.new(0)
	self.binding = bindingUtil.fromMotor(self.motor)

	self.motor:onStep(function(value)
		if value <= 0 then
			if self.props.onClose then
				self.props.onClose()
			end
		end
	end)

	self.timeout = task.delay(self.props.timeout, self.dismiss, self)
end

function Notification:dismiss()
	self.motor:setGoal(
		Flipper.Spring.new(0, {
			frequency = 5,
			dampingRatio = 1,
		})
	)
end

function Notification:willUnmount()
	task.cancel(self.timeout)
end

function Notification:didMount()
	self.motor:setGoal(
		Flipper.Spring.new(1, {
			frequency = 3,
			dampingRatio = 1,
		})
	)
end

function Notification:render()
	local time = DateTime.fromUnixTimestampMillis(self.props.timestamp)

	local textBounds = TextService:GetTextSize(
		self.props.text,
		15,
		Enum.Font.GothamSemibold,
		Vector2.new(350, 700)
	)

	local transparency = self.binding:map(function(value)
		return 1 - value
	end)

	local size = self.binding:map(function(value)
		return UDim2.fromOffset(
			(35+75+textBounds.X)*value,
			math.max(14+20+textBounds.Y, 32+20)
		)
	end)

	return Theme.with(function(theme)
		return e(BorderedContainer, {
			transparency = transparency,
			size = size,
			layoutOrder = self.props.layoutOrder,
		}, {
			TextContainer = e("Frame", {
				Size = UDim2.new(0, 35+textBounds.X, 1, -20),
				Position = UDim2.new(0, 0, 0, 10),
				BackgroundTransparency = 1
			}, {
				Logo = e("ImageLabel", {
					ImageTransparency = transparency,
					Image = Assets.Images.PluginButton,
					BackgroundTransparency = 1,
					Size = UDim2.new(0, 32, 0, 32),
					Position = UDim2.new(0, 0, 0.5, 0),
					AnchorPoint = Vector2.new(0, 0.5),
				}),
				Info = e("TextLabel", {
					Text = self.props.text,
					Font = Enum.Font.GothamSemibold,
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
				Time = e("TextLabel", {
					Text = time:FormatLocalTime("LTS", "en-us"),
					Font = Enum.Font.Code,
					TextSize = 12,
					TextColor3 = theme.Notification.InfoColor,
					TextTransparency = transparency,
					TextXAlignment = Enum.TextXAlignment.Left,

					Size = UDim2.new(1, -35, 0, 14),
					Position = UDim2.new(0, 35, 1, -14),

					LayoutOrder = 1,
					BackgroundTransparency = 1,
				}),
			}),

			Close = e(IconButton, {
				icon = Assets.Images.Icons.Close,
				iconSize = 24,
				color = theme.Notification.CloseColor,
				transparency = transparency,

				position = UDim2.new(1, 0, 0.5, 0),
				anchorPoint = Vector2.new(1, 0.5),

				onClick = function()
					self:dismiss()
				end,
			}),

			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 17),
				PaddingRight = UDim.new(0, 15),
			}),
		})
	end)
end

local Notifications = Roact.Component:extend("Notifications")

function Notifications:render()
	local notifs = {}

	for index, notif in ipairs(self.props.notifications) do
		notifs[notif] = e(Notification, {
			text = notif.text,
			timestamp = notif.timestamp,
			timeout = notif.timeout,
			layoutOrder = (notif.timestamp - baseClock),
			onClose = function()
				self.props.onClose(index)
			end,
		})
	end

	return Roact.createFragment(notifs)
end

return Notifications