local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Log = require(Packages.Log)

local Assets = require(Plugin.Assets)
local Settings = require(Plugin.Settings)
local Theme = require(Plugin.App.Theme)

local IconButton = require(Plugin.App.Components.IconButton)
local ScrollingFrame = require(Plugin.App.Components.ScrollingFrame)
local Tooltip = require(Plugin.App.Components.Tooltip)
local TextInput = require(Plugin.App.Components.TextInput)
local Setting = require(script.Setting)

local e = Roact.createElement

local function invertTbl(tbl)
	local new = {}
	for key, value in tbl do
		new[value] = key
	end
	return new
end

local invertedLevels = invertTbl(Log.Level)
local confirmationBehaviors = { "Initial", "Always", "Large Changes", "Unlisted PlaceId", "Never" }
local syncReminderModes = { "None", "Notify", "Fullscreen" }

local function Navbar(props)
	return Theme.with(function(theme)
		local navbarTheme = theme.Settings.Navbar

		return e("Frame", {
			Size = UDim2.new(1, 0, 0, 46),
			LayoutOrder = props.layoutOrder,
			BackgroundTransparency = 1,
		}, {
			Back = e(IconButton, {
				icon = Assets.Images.Icons.Back,
				iconSize = 24,
				color = navbarTheme.BackButtonColor,
				transparency = props.transparency,

				position = UDim2.new(0, 0, 0.5, 0),
				anchorPoint = Vector2.new(0, 0.5),

				onClick = props.onBack,
			}, {
				Tip = e(Tooltip.Trigger, {
					text = "Back",
				}),
			}),

			Text = e("TextLabel", {
				Text = "Settings",
				FontFace = theme.Font.Thin,
				TextSize = theme.TextSize.Large,
				TextColor3 = navbarTheme.TextColor,
				TextTransparency = props.transparency,

				Size = UDim2.new(1, 0, 1, 0),

				BackgroundTransparency = 1,
			}),
		})
	end)
end

local SettingsPage = Roact.Component:extend("SettingsPage")

function SettingsPage:init()
	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))
end

function SettingsPage:render()
	local layoutOrder = 0
	local function layoutIncrement()
		layoutOrder += 1
		return layoutOrder
	end

	return Roact.createFragment({
		Navbar = e(Navbar, {
			onBack = self.props.onBack,
			transparency = self.props.transparency,
			layoutOrder = layoutIncrement(),
		}),
		Content = e(ScrollingFrame, {
			size = UDim2.new(1, 0, 1, -47),
			position = UDim2.new(0, 0, 0, 47),
			contentSize = self.contentSize,
			transparency = self.props.transparency,
		}, {
			AutoReconnect = e(Setting, {
				id = "autoReconnect",
				name = "Auto Reconnect",
				description = "Reconnect to server on place open if the served project matches the last sync to the place",
				transparency = self.props.transparency,
				layoutOrder = layoutIncrement(),
			}),

			ShowNotifications = e(Setting, {
				id = "showNotifications",
				name = "Show Notifications",
				description = "Popup notifications in viewport",
				transparency = self.props.transparency,
				layoutOrder = layoutIncrement(),
			}),

			SyncReminderMode = e(Setting, {
				id = "syncReminderMode",
				name = "Sync Reminder",
				description = "What type of reminders you receive for syncing your project",
				transparency = self.props.transparency,
				layoutOrder = layoutIncrement(),
				visible = Settings:getBinding("showNotifications"),

				options = syncReminderModes,
			}),

			SyncReminderPolling = e(Setting, {
				id = "syncReminderPolling",
				name = "Sync Reminder Polling",
				description = "Look for available sync servers periodically",
				transparency = self.props.transparency,
				layoutOrder = layoutIncrement(),
				visible = Settings:getBindings("syncReminderMode", "showNotifications"):map(function(values)
					return values.syncReminderMode ~= "None" and values.showNotifications
				end),
			}),

			ConfirmationBehavior = e(Setting, {
				id = "confirmationBehavior",
				name = "Confirmation Behavior",
				description = "When to prompt for confirmation before syncing",
				transparency = self.props.transparency,
				layoutOrder = layoutIncrement(),

				options = confirmationBehaviors,
			}),

			LargeChangesConfirmationThreshold = e(Setting, {
				id = "largeChangesConfirmationThreshold",
				name = "Confirmation Threshold",
				description = "How many modified instances to be considered a large change",
				transparency = self.props.transparency,
				layoutOrder = layoutIncrement(),
				visible = Settings:getBinding("confirmationBehavior"):map(function(value)
					return value == "Large Changes"
				end),
				input = e(TextInput, {
					size = UDim2.new(0, 40, 0, 28),
					text = Settings:getBinding("largeChangesConfirmationThreshold"):map(function(value)
						return tostring(value)
					end),
					transparency = self.props.transparency,
					enabled = true,
					onEntered = function(text)
						local number = tonumber(string.match(text, "%d+"))
						if number then
							Settings:set("largeChangesConfirmationThreshold", math.clamp(number, 1, 999))
						else
							-- Force text back to last valid value
							Settings:set(
								"largeChangesConfirmationThreshold",
								Settings:get("largeChangesConfirmationThreshold")
							)
						end
					end,
				}),
			}),

			PlaySounds = e(Setting, {
				id = "playSounds",
				name = "Play Sounds",
				description = "Toggle sound effects",
				transparency = self.props.transparency,
				layoutOrder = layoutIncrement(),
			}),

			EnableSyncFallback = e(Setting, {
				id = "enableSyncFallback",
				name = "Enable Sync Fallback",
				description = "Whether Instances that fail to sync are remade as a fallback. If this is enabled, Instances may be destroyed and remade when syncing.",
				transparency = self.props.transparency,
				layoutOrder = layoutIncrement(),
			}),

			CheckForUpdates = e(Setting, {
				id = "checkForUpdates",
				name = "Check For Updates",
				description = "Notify about newer compatible Rojo releases",
				transparency = self.props.transparency,
				layoutOrder = layoutIncrement(),
			}),

			CheckForPreleases = e(Setting, {
				id = "checkForPrereleases",
				name = "Include Prerelease Updates",
				description = "Include prereleases when checking for updates",
				transparency = self.props.transparency,
				layoutOrder = layoutIncrement(),
				visible = if string.find(debug.traceback(), "\n[^\n]-user_.-$") == nil
					then false -- Must be a local install to allow prerelease checks
					else Settings:getBinding("checkForUpdates"),
			}),

			AutoConnectPlaytestServer = e(Setting, {
				id = "autoConnectPlaytestServer",
				name = "Auto Connect Playtest Server",
				description = "Automatically connect game server to Rojo when playtesting while connected in Edit",
				tag = "unstable",
				transparency = self.props.transparency,
				layoutOrder = layoutIncrement(),
			}),

			OpenScriptsExternally = e(Setting, {
				id = "openScriptsExternally",
				name = "Open Scripts Externally",
				description = "Attempt to open scripts in an external editor",
				tag = "unstable",
				transparency = self.props.transparency,
				layoutOrder = layoutIncrement(),
			}),

			TwoWaySync = e(Setting, {
				id = "twoWaySync",
				name = "Two-Way Sync",
				description = "Editing files in Studio will sync them into the filesystem",
				locked = self.props.syncActive,
				lockedTooltip = "(Cannot change while currently syncing. Disconnect first.)",
				tag = "unstable",
				transparency = self.props.transparency,
				layoutOrder = layoutIncrement(),
			}),

			LogLevel = e(Setting, {
				id = "logLevel",
				name = "Log Level",
				description = "Plugin output verbosity level",
				tag = "debug",
				transparency = self.props.transparency,
				layoutOrder = layoutIncrement(),

				options = invertedLevels,
				showReset = Settings:getBinding("logLevel"):map(function(value)
					return value ~= "Info"
				end),
				onReset = function()
					Settings:set("logLevel", "Info")
				end,
			}),

			TypecheckingEnabled = e(Setting, {
				id = "typecheckingEnabled",
				name = "Typechecking",
				description = "Toggle typechecking on the API surface",
				tag = "debug",
				transparency = self.props.transparency,
				layoutOrder = layoutIncrement(),
			}),

			TimingLogsEnabled = e(Setting, {
				id = "timingLogsEnabled",
				name = "Timing Logs",
				description = "Toggle logging timing of internal actions for benchmarking Rojo performance",
				tag = "debug",
				transparency = self.props.transparency,
				layoutOrder = layoutIncrement(),
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
		}),
	})
end

return SettingsPage
