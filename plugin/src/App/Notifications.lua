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
local TextButton = require(Plugin.App.Components.TextButton)

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
	local textBounds = TextService:GetTextSize(
		self.props.text,
		15,
		Enum.Font.GothamMedium,
		Vector2.new(350, 700)
	)

	local transparency = self.binding:map(function(value)
		return 1 - value
	end)

	local size = self.binding:map(function(value)
		return UDim2.fromOffset(
			(40+textBounds.X)*value,
			textBounds.Y + 34 + 20 + 10
		)
	end)

	local actionButtons = {}
	actionButtons.Layout = e("UIListLayout", {
		FillDirection = Enum.FillDirection.Horizontal,
		SortOrder = Enum.SortOrder.LayoutOrder,
		HorizontalAlignment = Enum.HorizontalAlignment.Right,
		Padding = UDim.new(0, 5),
	})

	if self.props.actions and next(self.props.actions) then
		for text, action in self.props.actions do
			actionButtons[text] = e(TextButton, {
				layoutOrder = -(action.layoutOrder or 1),
				text = action.text or text,
				style = action.style or "Bordered",
				transparency = transparency,
				onClick = function()
					task.spawn(pcall, action.onClick)
					self:dismiss()
				end,
			})
		end
	else
		actionButtons["Dismiss"] = e(TextButton, {
			text = "Dismiss",
			style = "Bordered",
			transparency = transparency,
			onClick = function()
				self:dismiss()
			end,
		})
	end

	return Theme.with(function(theme)
		return e(BorderedContainer, {
			size = size,
			layoutOrder = self.props.layoutOrder,
			clipsDescendants = true,
			transparency = transparency,
		}, {
			Info = e("TextLabel", {
				Text = self.props.text,
				Font = Enum.Font.GothamMedium,
				TextSize = 15,
				TextColor3 = theme.Notification.InfoColor,
				TextTransparency = transparency,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextWrapped = true,

				Size = UDim2.new(1, 0, 0, textBounds.Y),

				LayoutOrder = 1,
				BackgroundTransparency = 1,
			}),

			Logo = e("ImageLabel", {
				Image = Assets.Images.Logo,
				ImageColor3 = theme.Header.LogoColor,
				ImageTransparency = transparency,
				ScaleType = Enum.ScaleType.Fit,
				BackgroundTransparency = 1,

				Size = UDim2.new(0, 60, 0, 34),
				Position = UDim2.new(0, 0, 1, 0),
				AnchorPoint = Vector2.new(0, 1),
			}),

			Actions = e("Frame", {
				Size = UDim2.new(1, -70, 0, 34),
				Position = UDim2.new(1, 0, 1, 0),
				AnchorPoint = Vector2.new(1, 1),
				BackgroundTransparency = 1,
			}, actionButtons),

			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 17),
				PaddingRight = UDim.new(0, 10),
				PaddingTop = UDim.new(0, 10),
				PaddingBottom = UDim.new(0, 10),
			}),
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
			actions = notif.actions,
			layoutOrder = (notif.timestamp - baseClock),
			onClose = function()
				self.props.onClose(index)
			end,
		})
	end

	return Roact.createFragment(notifs)
end

return Notifications
