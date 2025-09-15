local StudioService = game:GetService("StudioService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Log = require(Packages.Log)

local Theme = require(Plugin.App.Theme)
local Assets = require(Plugin.Assets)

local TextButton = require(Plugin.App.Components.TextButton)

local e = Roact.createElement

local FullscreenNotification = Roact.Component:extend("FullscreeFullscreenNotificationnNotification")

function FullscreenNotification:init()
	self.transparency, self.setTransparency = Roact.createBinding(0)
	self.lifetime = self.props.timeout
end

function FullscreenNotification:dismiss()
	if self.props.onClose then
		self.props.onClose()
	end
end

function FullscreenNotification:didMount()
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

		self.timeout = nil
	end)
end

function FullscreenNotification:willUnmount()
	if self.timeout and coroutine.status(self.timeout) ~= "dead" then
		task.cancel(self.timeout)
	end
end

function FullscreenNotification:render()
	return Theme.with(function(theme)
		local actionButtons = {}
		if self.props.actions then
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
					transparency = self.transparency,
				})
			end
		end

		return e("Frame", {
			BackgroundColor3 = theme.BackgroundColor,
			Size = UDim2.fromScale(1, 1),
			ZIndex = self.props.layoutOrder,
		}, {
			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 17),
				PaddingRight = UDim.new(0, 15),
				PaddingTop = UDim.new(0, 10),
				PaddingBottom = UDim.new(0, 10),
			}),
			Layout = e("UIListLayout", {
				SortOrder = Enum.SortOrder.LayoutOrder,
				FillDirection = Enum.FillDirection.Vertical,
				HorizontalAlignment = Enum.HorizontalAlignment.Center,
				VerticalAlignment = Enum.VerticalAlignment.Center,
				Padding = UDim.new(0, 10),
			}),
			Logo = e("ImageLabel", {
				ImageTransparency = self.transparency,
				Image = Assets.Images.Logo,
				ImageColor3 = theme.Header.LogoColor,
				BackgroundTransparency = 1,
				Size = UDim2.fromOffset(60, 27),
				LayoutOrder = 1,
			}),
			Info = e("TextLabel", {
				Text = self.props.text,
				FontFace = theme.Font.Main,
				TextSize = theme.TextSize.Body,
				TextColor3 = theme.Notification.InfoColor,
				TextTransparency = self.transparency,
				TextXAlignment = Enum.TextXAlignment.Center,
				TextYAlignment = Enum.TextYAlignment.Center,
				TextWrapped = true,
				BackgroundTransparency = 1,

				AutomaticSize = Enum.AutomaticSize.Y,
				Size = UDim2.fromScale(0.4, 0),
				LayoutOrder = 2,
			}),
			Actions = if self.props.actions
				then e("Frame", {
					Size = UDim2.new(1, -40, 0, 37),
					BackgroundTransparency = 1,
					LayoutOrder = 3,
				}, {
					Layout = e("UIListLayout", {
						FillDirection = Enum.FillDirection.Horizontal,
						HorizontalAlignment = Enum.HorizontalAlignment.Center,
						VerticalAlignment = Enum.VerticalAlignment.Center,
						SortOrder = Enum.SortOrder.LayoutOrder,
						Padding = UDim.new(0, 5),
					}),
					Buttons = Roact.createFragment(actionButtons),
				})
				else nil,
		})
	end)
end

return FullscreenNotification
