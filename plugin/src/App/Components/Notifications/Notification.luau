local StudioService = game:GetService("StudioService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Flipper = require(Packages.Flipper)
local Log = require(Packages.Log)

local Theme = require(Plugin.App.Theme)
local Assets = require(Plugin.Assets)
local bindingUtil = require(Plugin.App.bindingUtil)
local getTextBoundsAsync = require(Plugin.App.getTextBoundsAsync)

local BorderedContainer = require(Plugin.App.Components.BorderedContainer)
local TextButton = require(Plugin.App.Components.TextButton)

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
	self.motor:setGoal(Flipper.Spring.new(0, {
		frequency = 5,
		dampingRatio = 1,
	}))
end

function Notification:didMount()
	self.motor:setGoal(Flipper.Spring.new(1, {
		frequency = 3,
		dampingRatio = 1,
	}))

	self.props.soundPlayer:play(Assets.Sounds.Notification)

	self.timeout = task.spawn(function()
		local clock = os.clock()
		local seen = false
		while task.wait(1 / 10) do
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
	if self.timeout and coroutine.status(self.timeout) ~= "dead" then
		task.cancel(self.timeout)
	end
end

function Notification:render()
	local transparency = self.binding:map(function(value)
		return 1 - value
	end)

	return Theme.with(function(theme)
		local actionButtons = {}
		local buttonsX = 0
		if self.props.actions then
			local count = 0
			for key, action in self.props.actions do
				actionButtons[key] = e(TextButton, {
					text = action.text,
					style = action.style,
					onClick = function()
						self:dismiss()
						if action.onClick then
							local success, err = pcall(action.onClick, self)
							if not success then
								Log.warn("Error in notification action: " .. tostring(err))
							end
						end
					end,
					layoutOrder = -action.layoutOrder,
					transparency = transparency,
				})

				buttonsX += getTextBoundsAsync(action.text, theme.Font.Main, theme.TextSize.Large, math.huge).X + (theme.TextSize.Body * 2)

				count += 1
			end

			buttonsX += (count - 1) * 5
		end

		local paddingY, logoSize = 20, 32
		local actionsY = if self.props.actions then 37 else 0
		local textXSpace = math.max(250, buttonsX) + 35
		local textBounds = getTextBoundsAsync(self.props.text, theme.Font.Main, theme.TextSize.Body, textXSpace)
		local contentX = math.max(textBounds.X, buttonsX)

		local size = self.binding:map(function(value)
			return UDim2.fromOffset(
				(35 + 40 + contentX) * value,
				5 + actionsY + paddingY + math.max(logoSize, textBounds.Y)
			)
		end)

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
				size = UDim2.fromScale(1, 1),
			}, {
				Contents = e("Frame", {
					Size = UDim2.fromScale(1, 1),
					BackgroundTransparency = 1,
				}, {
					Logo = e("ImageLabel", {
						ImageTransparency = transparency,
						Image = Assets.Images.PluginButton,
						BackgroundTransparency = 1,
						Size = UDim2.fromOffset(logoSize, logoSize),
						Position = UDim2.new(0, 0, 0, 0),
						AnchorPoint = Vector2.new(0, 0),
					}),
					Info = e("TextLabel", {
						Text = self.props.text,
						FontFace = theme.Font.Main,
						TextSize = theme.TextSize.Body,
						TextColor3 = theme.Notification.InfoColor,
						TextTransparency = transparency,
						TextXAlignment = Enum.TextXAlignment.Left,
						TextYAlignment = Enum.TextYAlignment.Center,
						TextWrapped = true,

						Size = UDim2.new(0, textBounds.X, 1, -actionsY),
						Position = UDim2.fromOffset(35, 0),

						LayoutOrder = 1,
						BackgroundTransparency = 1,
					}),
					Actions = if self.props.actions
						then e("Frame", {
							Size = UDim2.new(1, -40, 0, actionsY),
							Position = UDim2.fromScale(1, 1),
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
						})
						else nil,
				}),

				Padding = e("UIPadding", {
					PaddingLeft = UDim.new(0, 17),
					PaddingRight = UDim.new(0, 15),
					PaddingTop = UDim.new(0, paddingY / 2),
					PaddingBottom = UDim.new(0, paddingY / 2),
				}),
			}),
		})
	end)
end

return Notification
