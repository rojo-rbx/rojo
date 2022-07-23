local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Assets = require(Plugin.Assets)
local Theme = require(Plugin.App.Theme)

local IconButton = require(Plugin.App.Components.IconButton)
local ScrollingFrame = require(Plugin.App.Components.ScrollingFrame)
local Setting = require(script.Setting)

local e = Roact.createElement

local function Navbar(props)
	return Theme.with(function(theme)
		theme = theme.Settings.Navbar

		return e("Frame", {
			Size = UDim2.new(1, 0, 0, 46),
			LayoutOrder = props.layoutOrder,
			BackgroundTransparency = 1,
		}, {
			Back = e(IconButton, {
				icon = Assets.Images.Icons.Back,
				iconSize = 24,
				color = theme.BackButtonColor,
				transparency = props.transparency,

				position = UDim2.new(0, 0, 0.5, 0),
				anchorPoint = Vector2.new(0, 0.5),

				onClick = props.onBack,
			}),

			Text = e("TextLabel", {
				Text = "Settings",
				Font = Enum.Font.Gotham,
				TextSize = 18,
				TextColor3 = theme.TextColor,
				TextTransparency = props.transparency,

				Size = UDim2.new(1, 0, 1, 0),

				BackgroundTransparency = 1,
			})
		})
	end)
end

local SettingsPage = Roact.Component:extend("SettingsPage")

function SettingsPage:init()
	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))
end

function SettingsPage:render()
	return Theme.with(function(theme)
		theme = theme.Settings

		return e(ScrollingFrame, {
			size = UDim2.new(1, 0, 1, 0),
			contentSize = self.contentSize,
			transparency = self.props.transparency,
		}, {
			Navbar = e(Navbar, {
				onBack = self.props.onBack,
				transparency = self.props.transparency,
				layoutOrder = 0,
			}),

			OpenScriptsExternally = e(Setting, {
				id = "openScriptsExternally",
				name = "Open Scripts Externally",
				description = "Attempt to open scripts in an external editor",
				transparency = self.props.transparency,
				layoutOrder = 1,
			}),

			FindServeSessions = e(Setting, {
				id = "findServeSessions",
				name = "Find Serve Sessions",
				description = "Periodically scan for served projects when not yet connected",
				transparency = self.props.transparency,
				layoutOrder = 2,
			}),

			ShowNotifications = e(Setting, {
				id = "showNotifications",
				name = "Show Notifications",
				description = "Popup notifications in viewport",
				transparency = self.props.transparency,
				layoutOrder = 3,
			}),

			PlaySounds = e(Setting, {
				id = "playSounds",
				name = "Play Sounds",
				description = "Toggle sound effects",
				transparency = self.props.transparency,
				layoutOrder = 4,
			}),

			TwoWaySync = e(Setting, {
				id = "twoWaySync",
				name = "Two-Way Sync",
				description = "EXPERIMENTAL! Editing files in Studio will sync them into the filesystem",
				transparency = self.props.transparency,
				layoutOrder = 5,
			}),

			Layout = e("UIListLayout", {
				FillDirection = Enum.FillDirection.Vertical,
				SortOrder = Enum.SortOrder.LayoutOrder,

				[Roact.Change.AbsoluteContentSize] = function(object)
					self.setContentSize(object.AbsoluteContentSize)
				end,
			}),

			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 20),
				PaddingRight = UDim.new(0, 20),
			}),
		})
	end)
end

return SettingsPage
