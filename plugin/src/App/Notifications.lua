local TextService = game:GetService("TextService")
local StudioService = game:GetService("StudioService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Flipper = require(Packages.Flipper)

local bindingUtil = require(script.Parent.bindingUtil)

local Theme = require(Plugin.App.Theme)
local Assets = require(Plugin.Assets)

local BorderedContainer = require(Plugin.App.Components.BorderedContainer)

local baseClock = DateTime.now().UnixTimestampMillis

local e = Roact.createElement

local Notification = Roact.Component:extend("Notification")

function Notification:init()
	self.motor = Flipper.SingleMotor.new(0)
	self.binding = bindingUtil.fromMotor(self.motor)

	self.lifetime = self.props.timeout

	self.motor:onStep(function(value)
		if value <= 0 then
			if self.props.onClose then
				self.props.onClose()
			end
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
			(35+40+textBounds.X)*value,
			math.max(14+20+textBounds.Y, 32+20)
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

	for index, notif in ipairs(self.props.notifications) do
		notifs[notif] = e(Notification, {
			soundPlayer = self.props.soundPlayer,
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
